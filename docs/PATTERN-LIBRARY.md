# Pattern library

The library directs attention to command forms a person repeatedly encounters. It indexes supported tool input during ingestion; the replay model continues to fold only evidence available at the playhead. This separation lets a person browse the whole recording without accidentally displaying a future result in the inspector.

## Identity and grouping

An occurrence is `(session, agent, call)`. Input and start facts can arrive in either order. The earliest usable recorded input, with a lexical tie-break, supplies its pattern. Duplicate facts, replay folds and repeated opens do not become new occurrences. The command cache keeps previously observed calls after truncation or partial reopen; the current-recording count reflects only the loaded recording.

A usage key captures a bounded command form: executable, supported options, operand roles and ordered composition operators. Different `rg -n` patterns/paths can share a key; `rg -l` stays separate. Numeric sed line/range printing has its own shape. Git status flags and SSH configuration values remain significant. No flags are reordered or assumed equivalent. Only supported segments are grouped; unsupported shell grammar makes the whole command exact.

Inline code, heredocs, expansions, redirection and unfamiliar command options retain exact-input identities. Shell command arrays are retained as structured exact input. Unrelated tools without supported command/code payloads are outside this first index. The original input remains available regardless of grouping. This is a bounded normalizer, not an explainshell implementation or a complete shell AST.

Keys are SHA-256 digests of versioned, explicitly serialized pattern descriptions. `usage-v1` and `exact-v1` distinguish the two modes. Changing normalization rules requires a new key version and an explicit state migration decision; do not silently transfer a Learned label to a different pattern.

## Structural levels and inline scripts

The library defaults to Parts. The Combinations view hides inline script bodies. `S` includes them; a nonempty search searches and displays matching scripts. This filters rows, never deletes observations. Common Python stdin/`-c`, Node/Ruby/Perl eval forms and code tools are recognized conservatively; this is not arbitrary language classification.

`f` cycles Parts → Combinations → Programs → Wrappers; `c` returns directly to Parts. Parts includes program identities, bounded subcommands (such as `git diff` and `cargo test`), individual option spellings, the encountered option group, SSH `-o` configuration pairs, and shell `|`, `&&`, `||` and separator forms. Option group order is retained. Known option values are skipped; `--` ends flag extraction. At most 64 argument tokens per command are examined. These are lexical patterns, not assertions that every displayed fragment is an executable command.

Common compositions, comments and redirects can be split without executing them; quoted strings stay opaque. Heredocs, substitutions and grouped shell grammar stay conservative. One-off inline code bodies never become component patterns. A literal short-option bundle remains one token.

 A recognized shell argument array such as `["/bin/zsh", "-lc", "rg -n needle src"]` contributes `/bin/zsh` and `rg` to Programs, `/bin/zsh -lc` to Wrappers, and both the wrapper-plus-inner usage and the inner usage to Combinations. Supported pipelines contribute their full shape and constituent command forms. Literal executable paths and flag order remain significant. Only bounded shell `-c` forms are unwrapped; positional shell arguments remain opaque; unfamiliar options can appear as lexical parts without inferred argument semantics. General token-subset mining is deferred.

Each row counts distinct **calls containing the form**, at most once within a call even when a pipeline repeats it. Rows and levels overlap, so summing them does not yield unique calls. `Tab` changes session scope independently of structural level. Existing exact/usage identities remain retained; new projections use `structure-v1` keys. They are rebuilt from existing cache observations without changing the SQLite schema or deleting learning choices.

Learning states belong to the selected form. Marking a program or wrapper Practising highlights calls containing it; learning one form does not automatically mark its enclosing combinations Learned. Current-recording previews/jumps work for each structural level. The preview highlights the selected form inside its original input. Shell quote removal and JSON decoding retain source-byte mappings; a wrapper highlights only its executable/flags, and an inner program highlights its command position, not same-word arguments or metadata. Multiple occurrences within one call are all highlighted. Cached-only examples use the same mapping. Unsupported source locations remain unhighlighted and are labelled rather than guessed. Cached search uses retained exact inputs and the available aggregate examples, not a full-text transcript search.

## Counts and time

Rows rank by occurrence frequency in the selected scope. They separately show the current recording count, cached occurrence count, and the number of distinct sessions. All-time means observed by Linger, not every transcript on the machine. Patterns are not outcomes and frequency is not an importance score.

The preview cycles exact occurrences in the loaded recording and can jump to a selected call's last recorded event, pausing playback for close reading. After that deliberate seek, normal inspector time boundaries apply. A cached-only row currently provides one deterministic input example; it does not contain output or silently load another recording. The query remains visible after leaving search mode.

Practising markers reveal as their input becomes available at the playhead. They are annotations, not evidence that a process succeeded. Failure and playhead markers retain priority where timeline columns coincide. The library counts themselves deliberately do not rewind.

## Durable state

The native TUI owns a dedicated SQLite worker. It receives command observations and explicit learning changes through a channel; database work does not run on the terminal render loop. Shutdown flushes pending choices. The two databases are:

- `patterns.sqlite3`: rebuildable observations, stable call identities, original command/tool input and pattern descriptions; no tool result bodies.
- `learning.sqlite3`: pattern keys, user-selected learning states, field notes and explicitly collected input specimens. Collection happens on drill-in and survives rebuilding the frequency cache; ordinary occurrence ingestion never populates it.

SQLite transactions and primary keys coordinate independent viewers. Upserts merge individual choices rather than overwriting an entire state file. Counts refresh on new observations or `r`; learning choices from other processes load on restart. This is local persistence, not a synchronisation service. In-memory choices update immediately; a failed save is shown as session-only. Damaged or newer databases are retained, not reset automatically.

The default data directory is `$XDG_DATA_HOME/linger` or `~/.local/share/linger`, with `LINGER_DATA_DIR` for isolation. New data directories are private on Unix. Keep the learning database when intentionally rebuilding the command cache, and close viewers before moving/rebuilding database files. There is no automatic retention/deletion policy or cache-management command yet.

## Validation boundary

Fictional fixtures cover duplicate ingestion, out-of-order input, grouping differences, scripts and unsupported grammar, timeline boundaries, independent database connections, restart persistence, corrupt-file preservation and rebuilding only the cache. The PTY scenario opens two recordings across three processes, checks frequency/session counts, persists Practising and Learned, hides Learned, cycles exact inputs and opens a recorded result. It makes no model requests and uses an isolated data directory. Sustained use on a large personal corpus remains to be measured.
