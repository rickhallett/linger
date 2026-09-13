# Command explorer

Open a tool call and press **3** or **e**. The command remains visible above its documentation, with the selected character span highlighted. **h/l** or **←/→** steps through parts; **j/k**, **PgUp/PgDn** and search navigate the explanation. **Esc** returns to call selection. Input/output panning and time navigation keep their existing keys. **i** still asks Mercury about the whole selected call, not only the highlighted part.

## Local setup

```sh
uv run scripts/setup-explainshell.py
```

Setup requires Git and uv. It installs a separate Python environment and source checkout and downloads a roughly 220 MiB compressed manpage pack. Restart Linger after setup. Nothing is installed or downloaded by opening an inspector. Once installed, lookups run locally without network access by the adapter; recorded commands are never executed.

The backend location is `$XDG_DATA_HOME/linger/explainshell` or `~/.local/share/linger/explainshell`. `LINGER_EXPLAINSHELL_DIR` overrides it for both setup and lookup. It is deliberately independent of `LINGER_DATA_DIR`, which isolates personal observations and learning state. Without the pack, bounded reference notes and a setup instruction remain available.

The installer pins explainshell source commit `827ebe38da9822a10e89946551e377330a9de35e`, bashlex 0.18 and pydantic 2.12.5. Its database is `explainshell-2026-06-22-063435.db.zst` from the upstream `db-latest` release, checked against archive SHA-256 `9c6286d533f7f52e07b2420555b9cff93cf4754380c379747166e1f20374ebc6`. The local receipt records these and the decompressed database hash. Missing or differing installations fail visibly; setup does not replace a locally modified checkout.

## What the explanations mean

The optional backend is [explainshell](https://github.com/idank/explainshell): its Bash AST matcher selects text from a read-only SQLite manpage store. Linger validates returned Unicode character ranges before displaying them. This is deterministic runtime matching, **not a guarantee that every extracted manual entry is correct**. The selected manual path and extraction method appear with the text. All 61,322 parsed manpage entries in the installed pack have `extractor=llm`, including all 41,319 Ubuntu 26.04 entries used here (queried 13 September 2026). The upstream model identifies option metadata and manual line ranges; the extractor reconstructs option descriptions from those original manual lines. This is an LLM-built documentation index, not end-to-end deterministic extraction. The [extraction prompt](https://github.com/idank/explainshell/blob/827ebe38da9822a10e89946551e377330a9de35e/explainshell/extraction/llm/prompt.py) and [line-range reconstruction](https://github.com/idank/explainshell/blob/827ebe38da9822a10e89946551e377330a9de35e/explainshell/extraction/llm/response.py) establish this boundary. No LLM runs during a local lookup.

Manpage lookups are pinned to Ubuntu 26.04 for reproducibility. This does not identify the host, shell or executable version that ran the command. Bash syntax is the parser's reference dialect even for a string passed to zsh; zsh-specific grammar can remain unsupported. macOS/BSD and GNU option differences still matter.

The adapter disables the pack's implicit nested-command inference: observed entries incorrectly treated an `rg` path and an SSH hostname as executable names. Actual shell AST command nodes (for example in pipelines) remain separate. Unknown flags and subsequent uncertain operand roles are labelled unknown. Separate operands stay selectable even when their documentation is identical.

Linger adds narrowly matched explanations for numeric `sed` printing, `ssh -o BatchMode=yes`, and simple quoted Python stdin heredocs. These are labelled as Linger rules with references to the [GNU sed manual](https://www.gnu.org/software/sed/manual/sed.html), [OpenSSH configuration manual](https://man.openbsd.org/ssh_config#BatchMode) and [Bash redirection manual](https://www.gnu.org/s/bash/manual/html_node/Redirections.html). A Python heredoc's body remains opaque Python source; Mercury can interpret it on request. No generic Python or JavaScript parser is claimed.

Supported shell tools expose a string `cmd` or `command`. A recognized shell `-c` argument array can expose its inner string, labelled “shell string from argv”; Input preserves the exact array. Other arrays, embedded orchestration code, unsupported grammar and parser failures retain their raw input and contextual interpretation route. The first parsed shell unit may leave later multiline content unknown. General nested language exploration, per-token quote/escape explanations and automatic execution-platform matching remain future work.

## Runtime and validation

Lookups run in a separate Python process, asynchronously, one active plus the latest pending selection. Commands pass through stdin JSON, never shell interpolation or process arguments. The child receives no inherited environment credentials; its logging and stderr are suppressed because parser exceptions can include input. Input is limited to 32 KiB, responses to 2 MiB, and execution to five seconds. Results are bounded in-memory and keyed only by the command; output appends preserve the selected part. Replay derives the command from the latest input available at its current playhead.

Rust tests cover Unicode offsets, invalid ranges, unsupported tool/array handling, selection stability during output appends, keyboard routing and the input-time boundary. `scripts/explainshell-check.py` exercises the real installed matcher and checks that command substitutions and Python bodies never execute. `scripts/exploration-smoke.py` drives a 120×40 native PTY using fictional recordings, including wrapper-array drill-down and structural frequency views. These do not make provider requests or establish complete parser correctness.

## Attribution

Linger's Rust application and adapter source retain their MIT notices. The separately installed [explainshell source](https://github.com/idank/explainshell/blob/827ebe38da9822a10e89946551e377330a9de35e/LICENSE) is GPL-3.0 and retains its licence in the external checkout. Its manpage pack is supplied by that project; underlying manuals retain their respective notices. Neither the upstream Python source nor the database is vendored in this repository. Preserve those licences and notices if distributing the optional backend or data.
