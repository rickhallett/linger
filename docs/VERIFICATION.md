# Local verification — 13 September 2026

The first visual identity and OpenRouter update passed 221 library tests and 9 CLI tests, formatting, Clippy with warnings denied, and the native release build. The existing transcript/replay tests and evidence/time boundaries remain included.

The 120×40 PTY fixture exercised opening the inspector, selecting an earlier call, reading multiline Python input and failure output, stepping events, the missing-credential path, returning and quitting. Colour and monochrome captures were inspected; the coloured capture explicitly removes the agent runner's NO_COLOR setting. The application itself respects the terminal's colour preference. Raster previews are reconstructed from captured terminal cells with Menlo; they are not screenshots of a particular terminal application.

## One live provider sample

An explicit inspector request sent only the fictional Python division-by-zero call in `examples/repair.jsonl` to OpenRouter, using `inception/mercury-2.5`. A successful explanation appeared within approximately 1.36 seconds of the key action, including two intervening navigation actions and terminal polling. This is an observed completion upper bound for one sample, not a throughput benchmark or guaranteed latency.

The explanation correctly identified the empty list, division by zero and recorded exit code 1. Navigation to Input and back to Interpretation worked immediately after requesting the explanation. The fixture did not measure time to first token or prove the network request remained in flight at each keypress. Direct Inception has request-format tests but was not called live. No real session transcript was sent in this check.

Reproduce the offline path with `uv run --with pyte python scripts/terminal-smoke.py`; add `--live-mercury` only when intentionally making a live request with the configured provider. Screens and receipts go to ignored `outputs/`. Keys are read from the process environment or parsed private configuration, never included in a fixture or diagnostic output.
