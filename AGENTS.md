# Linger development

Linger is an independent open-source terminal application derived from Zoetrope. Read README.md and docs/LINGER-ROADMAP.md. Existing docs/ARCHITECTURE.md explains the inherited provider/fact, replay and snapshot invariants; preserve them unless a tested change explicitly supersedes one.

- Terminal first; live inspection and time navigation are equally important.
- Keep one owner for integration. Delegate only when the operator asks and the work has a clear independent boundary.
- Never execute a recorded command to explain it. Preserve raw evidence; distinguish documentation, interpretation, missing output and provider completion.
- Reference coverage must be honest. Do not parse embedded Python/JavaScript as shell merely because it contains a command string.
- Keep selection stable during appends; never show future evidence at an earlier playhead.
- Model calls must be explicit and asynchronous. No secrets, personal transcripts, private product notes or credentials in Git, fixtures, logs or screenshots. Use fictional fixtures.
- Local working artifacts go under ignored outputs/. Retain personal evidence in its private owner, not this public-intended repository.
- No eyebrow elements in the UI. Keyboard navigation, readable laptop layouts and immediate response are product requirements.
- Validate meaningful changes with cargo fmt --check, cargo test --locked, and cargo clippy --all-targets --locked -- -D warnings. Run the terminal path as well as headless tests when changing interaction.
- Preserve upstream MIT notices. Do not publish packages, push or release solely because the project is intended to be open source.
