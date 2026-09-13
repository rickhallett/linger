# Linger

Watch your agents work. Stay with what matters.

Linger is an open-source terminal viewer for Claude Code and Codex sessions. Follow a run live, move back through time, and drill into the exact inputs and recorded outputs of individual tool calls. Read command reference notes or ask Mercury to interpret the selected evidence.

Early local build: live/replay inspection, a session and cross-session pattern library, and persistent learning states with Practising highlights.

## Run

Requires Rust 1.88 or newer (build with the committed lockfile).

```sh
cargo install --path . --locked
linger                         # newest session for the current project
linger /path/to/project        # follow another project
linger /path/to/session.jsonl  # replay a recording
linger <session-id> --follow    # start at the recorded live edge
linger inspect <file-or-id>     # headless session summary
```

A fictional demo is included in this checkout:

```sh
linger examples/repair.jsonl --follow
```

Press **Enter** to open the tool inspector. Pick a call with **j/k**, then **Enter** to focus its content. **Esc** returns one level. The original transcript is never modified, and no captured command is executed.

## Keyboard

| Key | Action |
|---|---|
| Enter | Graph → calls → content |
| b | Open/close the pattern library |
| w / p / L / u in inspector | Want / Practising / Learned / Unmarked |
| Esc | Content → calls → graph |
| j/k or ↑/↓ | Select calls, or scroll focused content |
| 1 / 2 | Recorded input / output |
| v | Toggle readable / raw payload view |
| 3 or e | Deterministic reference notes |
| 4 | View interpretation for this evidence snapshot |
| i inside inspector | Ask Mercury about selected input/output |
| R inside inspector | Retry interpretation |
| / then Enter | Search the current content |
| n | Next search match |
| h/l or ←/→ | Scroll content horizontally |
| PgUp / PgDn | Scroll content vertically |
| , / . | Previous / next recorded event |
| [ / ] | Previous / next prompt boundary |
| g or End | Return to the latest recorded event |
| Space | Pause/resume playback |
| s in graph | Toggle idle-gap compression |
| ? in graph | Graph controls and help |
| q / Ctrl-C | Quit (q is ordinary text during search) |

Live ingestion continues while the inspector is open. Selection and reading position stay fixed. Time navigation changes the evidence available: a later result is not shown at an earlier playhead.

## Pattern library

Press **b** from the graph or inspector. The library ranks recurring command forms by frequency and previews their original inputs with reference notes.

| Key in library | Action |
|---|---|
| Tab | This recording / all cached sessions |
| j/k or ↑/↓ | Select a pattern |
| h/l or ←/→ | Preview another occurrence in this recording |
| Enter | Jump to that occurrence’s last recorded event and pause |
| w / p / L / u | Want to understand / Practising / Learned / Unmarked |
| H | Show/hide Learned patterns (hidden by default) |
| / then Enter | Filter patterns and recorded examples |
| PgUp / PgDn | Scroll the preview |
| r | Refresh command counts from the local cache |
| b / Esc | Return to the previous view |

Each row distinguishes **here**, **all cached occurrences**, and **distinct sessions**. Counts cover entire opened recordings, independent of the playhead. Replaying or reopening a call does not increment its count. The cache grows as you open sessions; Linger does not scan your unseen history. Cached-only patterns have a recorded input example; open the original recording to inspect its output.

Practising patterns carry a yellow **◎** in the inspector and replay timeline. Learned hides a pattern from the learning view, preserving every call in execution history. States are your explicit choices; Linger does not infer mastery or promote them automatically. Changes made by another running viewer appear on restart; `r` refreshes command counts.

Common `rg`, numeric `sed -n` printing, `git status`, and bounded `ssh` forms have conservative usage keys. Supported command compositions preserve their operators and order. Unsupported options, expansions, redirection and inline scripts keep exact input identities. These are browsing patterns, not a full shell parser or proof of semantic equivalence. Inline Python programs remain separate unless their exact input matches.

The local store defaults to `$XDG_DATA_HOME/linger/` or `~/.local/share/linger/`; `LINGER_DATA_DIR` overrides it. `patterns.sqlite3` retains command/tool input and occurrence identities; `learning.sqlite3` stores only stable pattern keys and your choices. Outputs are not copied into the pattern cache. These local files can contain private command arguments. They are not uploaded by the library. See [pattern storage and grouping](docs/PATTERN-LIBRARY.md).

## Optional Mercury interpretation

Configure either `OPENROUTER_API_KEY` (default model `inception/mercury-2.5`) or `INCEPTION_API_KEY` (default model `mercury-2.5`). Linger reads a `.env` file in the launch directory, falling back to `$XDG_CONFIG_HOME/linger/.env` or `~/.config/linger/.env`. Set `LINGER_ENV_FILE` to choose an exact file. These files are parsed as data; no shell code runs and the process environment is not modified.

Process credentials take precedence over file credentials; if both providers exist in the same source, Inception wins. `LINGER_MODEL` overrides the model identifier (use the selected provider's naming). Keep credentials in the ignored `.env` or your private configuration, never tracked files.

Only an explicit **i** or retry action sends data. It sends the selected call's input and recorded output, plus bounded reference notes, to the selected provider: `https://openrouter.ai/api/v1/chat/completions` or `https://api.inceptionlabs.ai/v1/chat/completions`. Credentials are only sent to their matching host; redirects are disabled. Input and output are each limited to 48 KB for interpretation, with omissions labelled. This build does **not** scrub transcript secrets. Inspect sensitive material locally unless you intend to send it.

Requests run asynchronously with a timeout. Interpretations are cached in memory for the evidence snapshot and are not reused as current when output changes. Without a key or a working connection, inspection and replay still work. Provider response bodies are not printed on HTTP errors.

## What this version does and does not establish

- Preserves recorded Claude tool input/results and Codex function/custom-tool payloads. Codex command-completion records expose available command, cwd, exit and output fields.
- Displays structured content as JSON. Images are not rendered, and there is no recovery of output omitted or truncated by the provider.
- Gives bounded command reference notes for common `rg`, `ssh`, `sed`, Git and Python invocations. This is not yet explainshell's token-by-token parser or full shell coverage. Embedded code remains code and can be interpreted contextually.
- Retains Zoetrope's transcript discovery, graph and replay foundation. Live output is only as current as the transcript; this is not direct process stdout capture.
- The pattern library and persistent learning states are implemented. Episode loops, full syntax-span explanations and opening cached-only recordings directly from the library remain future work. See [the roadmap](docs/LINGER-ROADMAP.md).
- A live Mercury 2.5 response through OpenRouter was verified using the fictional Python failure (13 September 2026). The explanation was visible within approximately 1.4 seconds while navigation stayed responsive. This single sample is not a latency or quality benchmark; direct Inception remains unverified live.

## Development

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
# Optional native PTY check (requires uv):
uv run --with pyte python scripts/terminal-smoke.py
uv run --with pyte python scripts/pattern-smoke.py
# Opt-in: one live provider request, using fictional evidence only:
uv run --with pyte python scripts/terminal-smoke.py --live-mercury
```

Tests cover upstream transcript/replay invariants and Linger's result preservation, time boundaries, stale interpretations, keyboard search, stable selection and a laptop-sized terminal render. Fixtures under `examples/` are fictional.

## Visual direction

Osaka Jade from Omarchy Quattro: deep green surfaces, muted jade focus, warm olive text and vermilion failures. A solid selection rail and filled active tab keep focus visible without relying on colour alone. The graph and timeline retain their layout; square frames and a quiet wordmark connect them to the inspector. Colours and shared frame/selection styles live in [src/ui/theme.rs](src/ui/theme.rs), so the visual language can evolve independently of replay and evidence handling. See [the design notes](docs/VISUAL-DESIGN.md).

## Attribution and licence

Linger is derived from [Zoetrope](https://github.com/furkankly/zoetrope), by Furkan Kalaycioglu, starting at commit `b1f31dd26bd4e9e513885e39edb78d0850a5d1fe`. Its graph, transcript adapters and replay engine are the foundation of this application. Linger's inspector, evidence retention and interpretation layer are additions. The original [MIT licence](LICENSE) is retained; Linger's additions are also MIT-licensed.

No explainshell code or database is incorporated in this version. Its interface is an inspiration and reuse remains an option. Historical upstream documentation, browser code, integrations and release workflows (under `docs/upstream-workflows/`) are retained for provenance; they are not a promise of Linger browser or Herdr-plugin support. The internal library keeps the `zoetrope` name for now to avoid a mechanical rewrite of the inherited code.
