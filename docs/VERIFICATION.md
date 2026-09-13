# Local verification — 13 September 2026

The first visual identity and OpenRouter update passed 221 library tests and 9 CLI tests, formatting, Clippy with warnings denied, and the native release build. The existing transcript/replay tests and evidence/time boundaries remain included.

The 120×40 PTY fixture exercised opening the inspector, selecting an earlier call, reading multiline Python input and failure output, stepping events, the missing-credential path, returning and quitting. Colour and monochrome captures were inspected; the coloured capture explicitly removes the agent runner's NO_COLOR setting. The application itself respects the terminal's colour preference. Raster previews are reconstructed from captured terminal cells with Menlo; they are not screenshots of a particular terminal application.

## One live provider sample

An explicit inspector request sent only the fictional Python division-by-zero call in `examples/repair.jsonl` to OpenRouter, using `inception/mercury-2.5`. A successful explanation appeared within approximately 1.36 seconds of the key action, including two intervening navigation actions and terminal polling. This is an observed completion upper bound for one sample, not a throughput benchmark or guaranteed latency.

The explanation correctly identified the empty list, division by zero and recorded exit code 1. Navigation to Input and back to Interpretation worked immediately after requesting the explanation. The fixture did not measure time to first token or prove the network request remained in flight at each keypress. Direct Inception has request-format tests but was not called live. No real session transcript was sent in this check.

Reproduce the offline path with `uv run --with pyte python scripts/terminal-smoke.py`; add `--live-mercury` only when intentionally making a live request with the configured provider. Screens and receipts go to ignored `outputs/`. Keys are read from the process environment or parsed private configuration, never included in a fixture or diagnostic output.

## Pattern library — 13 September 2026

The next terminal slice passed 232 library tests plus 9 CLI tests, formatting, Clippy with warnings denied and the native release build. Existing inspector/transport/missing-key PTY checks still pass. No new model request was made for this slice.

The new `scripts/pattern-smoke.py` launched three real 120×40 terminals using two fictional recordings and an isolated data directory. It verified four instances of one pattern across two distinct sessions, stable counts after reopening, persistence of Practising and Learned, hiding/revealing Learned, cycling occurrence input, and jumping to recorded output. Nine total call identities remained nine after the third process. The selected occurrence pauses for close reading. Terminal-cell previews of the library and Practising inspector were visually reviewed.

Focused regressions cover conservative option/composition grouping, exact inline scripts, invalid/partial input arrival order, duplicate ingestion, session spread, stable selection, modal search, no future Practising timeline marker, independent SQLite connections, corrupt learning-file preservation and retaining learning choices while rebuilding the cache. These are local and fictional checks; sustained real-corpus use is not yet measured.

## Command exploration and structural frequency — 13 September 2026

This slice passed 244 library tests plus 9 CLI tests, formatting, Clippy with warnings denied, the native release build and a portable `--no-default-features` check. The original inspector and three-process pattern PTY flows still pass. Added regressions cover Unicode and invalid documentation spans, explicit shell-array unwrapping, selected-part stability during output appends, replay hiding both future input and its learning mark, script filtering/search, structural grouping, per-call deduplication and rebuilding cross-session wrapper counts from existing stored occurrences.

The optional backend was installed from the pinned upstream commit and archive checksum documented in [command exploration](COMMAND-EXPLORER.md). Re-running setup verified the retained database against its receipt. `scripts/explainshell-check.py` passed against the real local manpage pack: rg operand roles with Unicode, pipelines, SSH hostname and BatchMode, numeric sed ranges, Git subcommand flags, unknown flags and affected operands, end-of-options, unknown programs, command substitutions and opaque Python bodies. Sentinel files stayed absent: recorded shell substitutions and Python programs were not executed. These assertions are narrower than complete parser correctness.

`scripts/exploration-smoke.py` exercised the real 120×40 TUI: selected flag/path/range explanations, quoted heredocs and opaque Python bodies, input/output returns, part selection across tabs, hidden script bodies, searchable original scripts, program/wrapper/inner-command frequency, persistent wrapper Practising, reopening without duplicated counts, occurrence jumps and exploration of a shell string inside recorded argv. Terminal-cell renders of the selected rg path and wrapper-frequency view were visually inspected. All fixtures were fictional, caches isolated, and this slice made zero provider requests.

Known boundaries: manuals reference Ubuntu 26.04, not a detected execution environment; all 61,322 indexed manpages have LLM-extracted metadata (41,319 in the selected Ubuntu release); option descriptions are reconstructed from manual line ranges; unsupported syntax and embedded languages remain partial/unknown. The example-panel highlights additionally have checks for wrapper-versus-program spans, repeated command names versus literal operands, JSON escapes and Unicode, with native colour-cell assertions for current and cached-only examples. Structural levels cover bounded shell/usage forms, not arbitrary token subsets. Large personal-corpus profiling and exhaustive Bash/zsh correctness remain open.

## Field guide — 13 September 2026

The first guide passed 248 library tests and 9 CLI tests, formatting, Clippy with warnings denied, the release build and the portable-core check. New regressions cover encounter-driven growth, source-location relationships, stable selection during appends, modal Unicode note editing, cached-only output boundaries, and retaining notes and learning choices when the command cache is rebuilt.

`scripts/guide-smoke.py` exercises three real 120×40 terminal processes with fictional recordings and one isolated cache: growth from four to five entries, search, highlighted current and cached-only specimens, notes surviving restart, bounded specimen cycling, jumping to the correct output, returning to the entry, and Practising propagation. Existing inspector and three-process pattern terminal checks also pass. Gallery and entry terminal-cell renders were visually inspected. No provider requests were made.

This is a local first version for hands-on use. Cached-only forms retain representative input, not every cross-session specimen or output. Large personal-library performance and sustained use are not measured by these fixtures.

## Default inspector wrapping — 13 September 2026

All four inspector tabs now wrap by default, including raw payloads, while preserving spaces and indentation. `W` toggles unwrapped reading and resets horizontal offset; h/l still selects command parts. The native release passed 249 library tests plus 9 CLI tests, formatting, Clippy with warnings denied and the portable-core check. Regressions cover all tabs, resize reflow, Unicode/spacing preservation, cached rendering and wrap/pan interaction.

`scripts/wrap-smoke.py` verified long input/output tails, raw input, toggling wrapping, search/vertical scrolling and the Interpretation placeholder in a real 120×40 terminal. The captured wrapped input was visually reviewed. Existing command-explorer/structural-frequency terminal checks passed, including selected-part navigation, argv exploration and missing-backend fallback. All evidence was fictional; no provider requests were made.

## Deliberate collection and Patterns navigation — 13 September 2026

Collection now requires entering a call's detail view/content tab or jumping to an occurrence from Patterns. It stores the inspected input specimen in the learning database, deduplicated by recording/agent/call. Loading recordings, navigating call-list rows, timeline movement and repeated inspection do not grow the collection. Guide forms, counts and examples come only from these specimens; frequency still counts all recorded calls. Existing frequency history is not silently imported. Collected inputs and notes survive rebuilding the frequency database.

The release passed 252 library tests plus 9 CLI tests, formatting, Clippy with warnings denied and the portable-core check. Regressions cover no passive collection, deduplication, the playhead input boundary, persistence/cache rebuilding, and Patterns timeline/help shortcuts with modal search. The collection terminal test opened three recordings containing 14 call identities and retained exactly three deliberately inspected specimens; it verified cross-session cycling, notes, highlighting, output jumps and Practising. The Patterns terminal test additionally exercised `[`, `g`, `?` and Esc without leaving Patterns. Wrapping checks still passed. All fixtures were fictional; no model requests were made.

## Constituent frequency and syntax colours — 13 September 2026

Frequency now opens on Parts: existing program identities plus new subcommand, option, encountered option-group and shell composition-operator projections. These are rebuilt from retained inputs, counted once per call and mapped back through shell quoting/JSON escapes. Common comments and redirection no longer suppress the whole decomposition; substitutions and heredocs remain conservative. Known option values and `--` operands are excluded from option counts. Parts searches names; other levels retain original-input search. Each command's constituent scan is bounded to 64 argument tokens. Short-option bundles remain literal tokens; unknown option arity is not inferred.

The command explorer colours matched synopsis/name spans, options, arguments and shell syntax, with an underlined unknown style and a filled selection. A terminal check against the actual installed matcher verified option/name colours and the selected background; synthetic regressions cover wrapping, Unicode and unknown styling. The matcher calls executable names `synopsis`, covered by the colour adapter.

Validation: 255 library tests plus 9 CLI tests, formatting, Clippy with warnings denied, native release and portable-core checks. `scripts/parts-smoke.py` covers the default level, recurring flags across different operands, subcommands/operators, original-input highlighting, occurrence jumps, syntax colours and reopening without inflated counts. Existing Patterns, collection and command-exploration terminal checks passed after explicitly selecting their tested levels. Frequency and command-strip cell renders were visually reviewed. All fixtures were fictional and no model request was made. These checks do not establish exhaustive shell parsing or large-corpus performance.

## Nested shell arguments and keyboard feedback — 13 September 2026

260 library tests and 9 CLI tests pass, along with formatting, Clippy with warnings denied, native release build and portable core check. `scripts/nested-smoke.py` uses a fictional Codex custom-tool recording in a 120×40 PTY to verify literal shell extraction, real local documentation matching, nested-command switching, matching-command selection from frequency, source-range highlights, per-outer-call deduplication and cache restart. Dynamic variable arguments stay opaque. Lexical unit tests reject comments, misleading strings, spreads, duplicate/computed properties, interpolated templates and regexps.

The same terminal flow verifies shortcut pulse/expiry, removal of the routine counts explanation, Preview/Reading call identity and centered selection in a 20-call recording. Existing Guide collection/persistence and word-wrap terminal flows also pass. No model requests or recorded command execution occurred. These are local synthetic checks, not broad JavaScript-parser coverage or a guarantee that a nested call actually executed.
