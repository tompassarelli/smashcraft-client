// The rules of wisp:host/checks/cleanRoom.ts (wisp:docs/clean-room.md, "The check"), ported unchanged.
use std::collections::HashMap;

pub const ALLOWLIST: &str = "clean-room-allowlist.tsv";

pub const ORIGINS: &[&str] = &["original", "generated"];

pub const GAME_EXTENSIONS: &[&str] = &[
    "mdx", "mdl", "blp", "dds", "tga", "wav", "mp3", "flac", "ogg", "opus", "w3x", "w3m", "w3n", "w3g", "mpq", "slk", "j", "ai", "fdf", "toc", "imp",
    "wts", "w3u", "w3t", "w3a", "w3b", "w3d", "w3h", "w3q", "w3e", "wpm", "doo", "shd", "mmp", "w3i", "w3r", "w3c", "w3s", "ttf", "otf", "png", "jpg",
    "jpeg", "gif", "webp", "bmp",
];

const MAGIC: &[&[u8]] = &[b"MDLX", b"BLP1", b"BLP2", b"MPQ\x1a", b"MPQ\x1b", b"HM3W"];

pub const JASS_DECLARATION: &str = r"^[[:space:]]*((constant[[:space:]]+)?native[[:space:]]+[A-Za-z0-9_]+[[:space:]]+takes[[:space:]]|function[[:space:]]+[A-Za-z0-9_]+[[:space:]]+takes[[:space:]].*[[:space:]]returns[[:space:]]|type[[:space:]]+[A-Za-z0-9_]+[[:space:]]+extends[[:space:]]|constant[[:space:]]+[A-Za-z0-9_]+[[:space:]]+[A-Za-z0-9_]+[[:space:]]*=)";

pub const JASS_LIMIT: usize = 20;

pub const JASS_SCAN_BYTES: u64 = 2_000_000;

pub const LAN_PLUGIN_LOADER: &str = "host/lan/plugin.ts";

pub struct ContentRule {
    pub name: &'static str,
    pub pattern: String,
    pub minimum_lines: usize,
    pub code_only: bool,
    pub exempt: &'static [&'static str],
    pub why: &'static str,
}

pub const CODE_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "mjs", "cjs", "lua"];

pub fn is_code(path: &str) -> bool {
    path.rsplit_once('.').is_some_and(|(_, extension)| CODE_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()))
}

// Split so this file and its tests never contain the tokens the published history is scanned for.
pub fn content_rules() -> Vec<ContentRule> {
    let memory_calls = [
        concat!("process_vm_", "(?:readv|writev)"),
        concat!("(?<![A-Za-z])p", "trace"),
        concat!("/proc/[^/\\s\"'`]+/m", "em\\b"),
        concat!("(?:Read|Write)Process", "Memory"),
        concat!("VirtualProtect", "Ex"),
        concat!("CreateRemote", "Thread"),
    ];
    vec![
        ContentRule {
            name: "memory access",
            pattern: format!("(?i)(?:{})", memory_calls.join("|")),
            minimum_lines: 1,
            code_only: false,
            exempt: &[],
            why: "names a call or path that reads or writes another process's memory",
        },
        ContentRule {
            name: "binary address table",
            pattern: concat!("(?i)\\w*(?:offset|r", "va|addr|address|base|ptr|pointer)\\w*[\"']?\\s*[:=,]\\s*[\"']?0x[0-9a-f]{4,}").to_string(),
            minimum_lines: 3,
            code_only: false,
            exempt: &[],
            why: "looks like a table of addresses or offsets inside the game binary",
        },
        ContentRule {
            name: "decomp symbol",
            pattern: "\\b[a-zA-Z]{2,4}_80[0-9A-F]{6}\\b".to_string(),
            minimum_lines: 1,
            code_only: false,
            exempt: &[],
            why: "names a decompiled function by its address",
        },
        ContentRule {
            name: "private import",
            pattern: concat!(
                "(?:import|require)\\s*\\(?[^)\\n]*wisp-", "private|\\bfrom\\s*[\"'`][^\"'`\\n]*wisp-", "private|wisp-", "private/[^\"'`\\s]*\\.(?:ts|js|mjs|cjs)\\b"
            )
            .to_string(),
            minimum_lines: 1,
            code_only: true,
            exempt: &[LAN_PLUGIN_LOADER],
            why: "imports private code; only host/lan/plugin.ts loads the private LAN plugin",
        },
    ]
}

pub struct Entry {
    pub path: String,
    pub origin: String,
    pub how: String,
}

pub fn parse_allowlist(text: &str) -> Vec<Entry> {
    text.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let mut columns = line.split('\t');
            let mut next = || columns.next().unwrap_or("").to_string();
            Entry { path: next(), origin: next(), how: next() }
        })
        .collect()
}

pub struct File {
    pub path: String,
    pub head: Vec<u8>,
}

pub struct Scan {
    pub files: Vec<File>,
    pub jass_lines: HashMap<String, usize>,
    pub content_hits: HashMap<String, Vec<&'static str>>,
    pub allowlist: Vec<Entry>,
}

pub fn problems(scan: &Scan) -> Vec<String> {
    let fix = format!(
        "Delete it, regenerate it from our own source, or load it from the user's install at run time; if it is ours, add it to {ALLOWLIST} as PATH<TAB>{}<TAB>HOW (wisp:docs/clean-room.md).",
        ORIGINS.join("|")
    );
    let mut found = Vec::new();
    let allowed: HashMap<&str, &Entry> = scan.allowlist.iter().map(|entry| (entry.path.as_str(), entry)).collect();
    for entry in &scan.allowlist {
        if !ORIGINS.contains(&entry.origin.as_str()) {
            found.push(format!("{ALLOWLIST}: {} names origin \"{}\"; this repository allows {} (wisp:docs/clean-room.md)", entry.path, entry.origin, ORIGINS.join(", ")));
        } else if entry.how.trim().is_empty() {
            found.push(format!("{ALLOWLIST}: {} doesn't say how it was made (wisp:docs/clean-room.md)", entry.path));
        }
        if !scan.files.iter().any(|file| file.path == entry.path) {
            found.push(format!("{ALLOWLIST}: {} isn't committed; remove its row", entry.path));
        }
    }
    for file in &scan.files {
        if allowed.contains_key(file.path.as_str()) {
            continue;
        }
        let extension = file.path.rsplit_once('.').map(|(_, extension)| extension.to_ascii_lowercase()).unwrap_or_default();
        let magic = MAGIC.iter().any(|bytes| file.head.starts_with(bytes));
        let game_extension = GAME_EXTENSIONS.contains(&extension.as_str()) && (extension != "mdx" || file.head.starts_with(b"MDLX"));
        if game_extension || magic {
            found.push(format!("clean room: {} is a game-format file. {fix}", file.path));
        }
    }
    let mut jass: Vec<_> = scan.jass_lines.iter().filter(|(path, lines)| **lines > JASS_LIMIT && !allowed.contains_key(path.as_str())).collect();
    jass.sort();
    for (path, lines) in jass {
        found.push(format!("clean room: {path} has {lines} JASS declaration lines (over {JASS_LIMIT}), like a copy of common.j or blizzard.j. {fix}"));
    }
    let rules = content_rules();
    let mut hits: Vec<_> = scan.content_hits.iter().filter(|(path, _)| !allowed.contains_key(path.as_str())).collect();
    hits.sort();
    for (path, names) in hits {
        for name in names {
            let why = rules.iter().find(|rule| rule.name == *name).map_or(*name, |rule| rule.why);
            found.push(format!("clean room: {path} {why} ({name}). Smashcraft's behaviour comes from outside the game only; remove it (README, Principles; wisp:docs/clean-room.md)."));
        }
    }
    found
}
