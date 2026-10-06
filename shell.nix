# Build shell for the Smashcraft client on NixOS. Rust comes from rustup's
# pinned toolchain (src-tauri/rust-toolchain.toml), Bun from the user profile.
{ pkgs ? import <nixpkgs> { } }:
let runtime = import ./runtime.nix { inherit pkgs; };
in pkgs.mkShell {
  nativeBuildInputs = with pkgs; [ pkg-config stdenv.cc ];
  buildInputs = runtime.paths ++ (map (p: p.dev or p) runtime.paths);
  # The tray library is loaded at run time.
  LD_LIBRARY_PATH = "${runtime}/lib";
}
