// The clean-room check for the whole tracked tree. Runs before every push and in CI.
mod rules;

use rules::{ALLOWLIST, File, JASS_DECLARATION, JASS_SCAN_BYTES, Scan, content_rules, is_code, parse_allowlist, problems};
use std::{collections::HashMap, fs, io::Read, path::Path, process::Command};

const BATCH: usize = 1000;

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new("git").arg("-C").arg(root).args(args).output().map_err(|error| format!("clean room: git {}: {error}", args[0]))?;
    let code = output.status.code().unwrap_or(-1);
    if code > 1 || (code == 1 && args[0] != "grep") {
        return Err(format!("clean room: git {} failed: {}", args[0], String::from_utf8_lossy(&output.stderr).trim()));
    }
    Ok(output.stdout)
}

fn grep_counts(root: &Path, mode: &str, pattern: &str, paths: &[&String]) -> Result<Vec<(String, usize)>, String> {
    let mut counts = Vec::new();
    for batch in paths.chunks(BATCH) {
        let literals: Vec<String> = batch.iter().map(|path| format!(":(literal){path}")).collect();
        let mut args = vec!["grep", "--cached", "-z", "-c", "-I", mode, pattern, "--"];
        args.extend(literals.iter().map(String::as_str));
        let stdout = git(root, &args)?;
        for line in String::from_utf8_lossy(&stdout).lines().filter(|line| !line.is_empty()) {
            if let Some((path, count)) = line.split_once('\0') {
                counts.push((path.to_string(), count.parse().unwrap_or(0)));
            }
        }
    }
    Ok(counts)
}

fn scan(root: &Path) -> Result<Scan, String> {
    let listed = git(root, &["ls-files", "-z"])?;
    let names: Vec<String> = listed.split(|byte| *byte == 0).filter(|name| !name.is_empty()).map(|name| String::from_utf8_lossy(name).into_owned()).collect();
    let mut files = Vec::new();
    let mut scanned = Vec::new();
    for path in names {
        let mut head = vec![0u8; 4];
        let opened = fs::File::open(root.join(&path));
        let length = opened.as_ref().map_or(0, |file| file.metadata().map_or(0, |meta| meta.len()));
        let read = opened.and_then(|mut file| file.read(&mut head)).unwrap_or(0);
        head.truncate(read);
        if length <= JASS_SCAN_BYTES {
            scanned.push(path.clone());
        }
        files.push(File { path, head });
    }
    let scanned: Vec<&String> = scanned.iter().collect();
    let jass_lines: HashMap<String, usize> = grep_counts(root, "-E", JASS_DECLARATION, &scanned)?.into_iter().collect();
    let mut content_hits: HashMap<String, Vec<&'static str>> = HashMap::new();
    for rule in content_rules() {
        let candidates: Vec<&String> = scanned.iter().copied().filter(|path| !rule.exempt.contains(&path.as_str()) && (!rule.code_only || is_code(path))).collect();
        for (path, count) in grep_counts(root, "-P", &rule.pattern, &candidates)? {
            if count >= rule.minimum_lines {
                content_hits.entry(path).or_default().push(rule.name);
            }
        }
    }
    let allowlist = parse_allowlist(&fs::read_to_string(root.join(ALLOWLIST)).unwrap_or_default());
    Ok(Scan { files, jass_lines, content_hits, allowlist })
}

fn main() {
    let root = std::env::args().nth(1).map_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join(".."), Into::into);
    match scan(&root).map(|scan| problems(&scan)) {
        Ok(found) if found.is_empty() => println!("clean room: no game files, copied game scripts, memory access, address tables or private imports outside the allowlist"),
        Ok(found) => {
            found.iter().for_each(|problem| eprintln!("{problem}"));
            std::process::exit(1);
        }
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    fn repository(files: &[(&str, &[u8])], allowlist: &str) -> std::path::PathBuf {
        let name = files.iter().map(|(path, _)| *path).collect::<Vec<_>>().join("-").replace('/', "_");
        let root = std::env::temp_dir().join(format!("clean-room-{}-{name}-{}", std::process::id(), allowlist.len()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let run = |args: &[&str]| assert!(Command::new("git").arg("-C").arg(&root).args(args).stdout(Stdio::null()).status().unwrap().success());
        run(&["init", "-q"]);
        for (path, bytes) in files {
            if let Some(parent) = root.join(path).parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(root.join(path), bytes).unwrap();
        }
        fs::write(root.join(ALLOWLIST), allowlist).unwrap();
        run(&["add", "."]);
        root
    }

    fn checked(files: &[(&str, &[u8])], allowlist: &str) -> Vec<String> {
        problems(&scan(&repository(files, allowlist)).unwrap())
    }

    #[test]
    fn refuses_what_wisps_check_refuses() {
        let jass: String = (0..21).map(|n| format!("native Foo{n} takes integer a returns nothing\n")).collect();
        let memory = format!("{}(pid)", concat!("process_vm_", "readv"));
        let address = concat!("unit_off", "set = 0x00A1B2\n").repeat(3);
        let cases: Vec<(&str, Vec<u8>, &str)> = vec![
            ("Footman.mdx", b"MDLXdata".to_vec(), "game-format file"),
            ("blob.dat", b"BLP1\0\0\0\0".to_vec(), "game-format file"),
            ("shot.png", b"\x89PNG".to_vec(), "game-format file"),
            ("common.txt", jass.into_bytes(), "JASS declaration lines"),
            ("a.c", memory.into_bytes(), "memory access"),
            ("b.c", address.into_bytes(), "binary address table"),
            ("c.c", concat!("see zz_", "80ABCDEF here").as_bytes().to_vec(), "decomp symbol"),
        ];
        for (path, bytes, rule) in cases {
            let found = checked(&[(path, bytes.as_slice())], "");
            assert!(found.iter().any(|problem| problem.contains(path) && problem.contains(rule)), "{path} must be refused for {rule}, got {found:?}");
        }
    }

    #[test]
    fn allows_our_own_files_and_listed_exceptions() {
        assert_eq!(checked(&[("src/a.rs", b"fn main() {}")], ""), Vec::<String>::new());
        let listed = "icon.png\tgenerated\tscripts/icons.ts\n";
        assert_eq!(checked(&[("icon.png", b"\x89PNG")], listed), Vec::<String>::new());
        let bad = "icon.png\tstolen\tx\nmissing.png\tgenerated\tx\n";
        let found = checked(&[("icon.png", b"\x89PNG")], bad);
        assert!(found[0].contains("names origin") && found[1].contains("isn't committed"), "{found:?}");
        assert_eq!(found.len(), 2);
    }
}
