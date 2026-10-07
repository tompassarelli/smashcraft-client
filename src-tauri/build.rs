// Lua 5.3.6 with 32-bit numbers, Warcraft III's number model, for playing a
// replay in its own map's simulation (lua-5.3.6/README.md, src/mapsim.rs).
fn main() {
    let sources = std::fs::read_dir("lua-5.3.6").expect("lua-5.3.6 sources");
    let files: Vec<_> = sources.flatten().map(|entry| entry.path()).filter(|path| path.extension().is_some_and(|ext| ext == "c")).collect();
    println!("cargo:rerun-if-changed=lua-5.3.6");
    cc::Build::new().files(files).include("lua-5.3.6").define("LUA_32BITS", None).warnings(false).compile("lua32");
    tauri_build::build()
}
