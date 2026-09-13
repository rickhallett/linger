# Pattern library

The library directs attention to command forms a person repeatedly encounters. It indexes supported tool input during ingestion; the replay model continues to fold only evidence available at the playhead. This separation lets a person browse the whole recording without accidentally displaying a future result in the inspector.

## Identity and grouping

An occurrence is `(session, agent, call)`. Input and start facts can arrive in either order. The earliest usable recorded input, with a lexical tie-break, supplies its pattern. Duplicate facts, replay folds and repeated opens do not become new occurrences. The command cache keeps previously observed calls after truncation or partial reopen; the current-recording count reflects only the loaded recording.

A usage key captures a bounded command form: executable, supported options, operand roles and ordered composition operators. Different `rg -n` patterns/paths can share a key; `rg -l` stays separate. Numeric sed line/range printing has its own shape. Git status flags and SSH configuration values remain significant. No flags are reordered or assumed equivalent. Only supported segments are grouped; unsupported shell grammar makes the whole command exact.

Inline code, heredocs, expansions, redirection and unfamiliar command options retain exact-input identities. Shell command arrays are retained as structured exact input. Unrelated tools without supported command/code payloads are outside this first index. The original input remains available regardless of grouping. This is a bounded normalizer, not an explainshell implementation or a complete shell AST.

Keys are SHA-256 digests of versioned, explicitly serialized pattern descriptions. `usage-v1` and `exact-v1` distinguish the two modes. Changing normalization rules requires a new key version and an explicit state migration decision; do not silently transfer a Learned label to a different pattern.

## Counts and time

Rows rank by occurrence frequency in the selected scope. They separately show the current recording count, cached occurrence count, and the number of distinct sessions. All-time means observed by Linger, not every transcript on the machine. Patterns are not outcomes and frequency is not an importance score.

The preview cycles exact occurrences in the loaded recording and can jump to a selected call's last recorded event, pausing playback for close reading. After that deliberate seek, normal inspector time boundaries apply. A cached-only row currently provides one deterministic input example; it does not contain output or silently load another recording. The query remains visible after leaving search mode.

Practising markers reveal as their input becomes available at the playhead. They are annotations, not evidence that a process succeeded. Failure and playhead markers retain priority where timeline columns coincide. The library counts themselves deliberately do not rewind.

## Durable state

The native TUI owns a dedicated SQLite worker. It receives command observations and explicit learning changes through a channel; database work does not run on the terminal render loop. Shutdown flushes pending choices. The two databases are:

- `patterns.sqlite3`: rebuildable observations, stable call identities, original command/tool input and pattern descriptions; no tool result bodies.
- `learning.sqlite3`: pattern keys and user-selected Unmarked, Want, Practising or Learned states.

SQLite transactions and primary keys coordinate independent viewers. Upserts merge individual choices rather than overwriting an entire state file. Counts refresh on new observations or `r`; learning choices from other processes load on restart. This is local persistence, not a synchronisation service. In-memory choices update immediately; a failed save is shown as session-only. Damaged or newer databases are retained, not reset automatically.

The default data directory is `$XDG_DATA_HOME/linger` or `~/.local/share/linger`, with `LINGER_DATA_DIR` for isolation. New data directories are private on Unix. Keep the learning database when intentionally rebuilding the command cache, and close viewers before moving/rebuilding database files. There is no automatic retention/deletion policy or cache-management command yet.

## Validation boundary

Fictional fixtures cover duplicate ingestion, out-of-order input, grouping differences, scripts and unsupported grammar, timeline boundaries, independent database connections, restart persistence, corrupt-file preservation and rebuilding only the cache. The PTY scenario opens two recordings across three processes, checks frequency/session counts, persists Practising and Learned, hides Learned, cycles exact inputs and opens a recorded result. It makes no model requests and uses an isolated data directory. Sustained use on a large personal corpus remains to be measured.
