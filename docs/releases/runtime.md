Linger is a terminal application for understanding Claude Code and Codex tool calls: inspect recorded inputs and outputs, replay a run, explore command documentation, and keep a field guide with notes and learning states.

Install with `cargo install linger --version 0.1.0 --locked`, or download the archive for your Mac:

- `aarch64-apple-darwin`: Apple Silicon.
- `x86_64-apple-darwin`: Intel.

Each archive includes the binary, fictional examples, MIT notices and a SHA-256 checksum file. Verify with `shasum -a 256 -c <archive>.sha256` before extracting. Binaries are not Apple Developer ID signed or notarized. Linux binary distribution is to be decided.

Inspection and replay work locally. Optional Mercury interpretation sends selected evidence to the configured provider only on explicit request. No recorded command is executed.

Runtime implementation is the committed baseline `473f63fab877c731a2701ee51d1938e127b533ff`; release packaging, documentation and the website are added separately. See https://lingerer.xyz/guide/ for setup and controls.
