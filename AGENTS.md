# Agents

Follow README's "Principles": if a task seems to need memory reads, disassembly, decompiled code or Blizzard's or Nintendo's files in the repo, stop and measure from outside instead.

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
