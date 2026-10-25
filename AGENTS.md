# Agents

Follow README's "Principles": if a task seems to need memory reads, disassembly, decompiled code or Blizzard's or Nintendo's files in the repo, stop and measure from outside instead. The one exception is Smashcraft Servers (README Principles); its internals live in the private repository and never here.

## Structure

Where code lives and where it is moving: docs/architecture.md. Place a change by its row there.

- Take the UI's types for padbridge messages from TypeScript generated from padbridge's Rust types (ts-rs, approved by Tom); never hand-copy them (`ui/src/model.ts` is the last copy, removed in C1).
- Read preset names, preset labels and the pad's face layout from padbridge's snapshot; never hardcode them.
- Read Smashcraft's file names, module names and record format from the client kit; never restate them in client code.
- Pin padbridge and Smashcraft by release tag or a main SHA that is never rewritten.
- Regenerate generated files from their producer instead of editing them, and never edit `src-tauri/lua-5.3.6/`.

## Tests

Tests follow the testing skill (north:harness/skills/testing/SKILL.md). Every
permanent test is one of five kinds:

1. A recorded scenario: real or seeded input replayed through the whole
   system, checking invariants or agreement.
2. A property over a pure core: generated inputs with a fixed seed and count,
   checked against a rule that always holds.
3. A product measurement: an owner-decided number with its source.
4. An external reference: truth from outside the code, cited in the test
   title or fixture.
5. One integration check per real boundary: build, protocol, file format,
   device or service.

Any other test is scaffolding, and you delete it before landing.

Run: `bun scripts/kit.ts && cd ui && bun install && bun test && bun test ../scripts`,
then `cd ../src-tauri && nix-shell ../shell.nix --run 'cargo test --jobs 2'` (README: Build, test, install).
