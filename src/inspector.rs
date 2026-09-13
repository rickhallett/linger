//! Linger's read-only inspector. Selection uses call identity, not a moving
//! row offset. All evidence comes from the model AT the current playhead.
use crate::state::App;
use crate::state::session::{ToolCallInfo, ToolState};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    Input,
    #[default]
    Output,
    Explain,
    Interpret,
}

#[derive(Default)]
pub struct Inspector {
    pub agent: String,
    pub call: Option<String>,
    pub detail: bool,
    pub raw: bool,
    pub nowrap: bool,
    pub content_width: usize,
    pub tab: Tab,
    pub scroll: usize,
    pub horizontal: u16,
    pub query: String,
    pub searching: bool,
    pub lines: Vec<String>,
    pub view_stamp: Option<u64>,
    pub notice: Option<String>,
    pub command: Option<String>,
    pub command_from_argv: bool,
    pub part: usize,
    pub command_lines: Vec<ratatui::text::Line<'static>>,
    pub command_row: usize,
}

#[derive(Clone)]
pub struct InterpretationRequest {
    pub key: String,
    pub tool: String,
    pub input: String,
    pub output: String,
    pub reference: String,
}

pub struct Interpretation {
    pub key: String,
    pub text: String,
}

impl App {
    pub fn open_inspector(&mut self) {
        let agent = self.selected_agent_id().unwrap_or_else(|| "main".into());
        self.flow.select_node(&agent);
        self.camera = crate::state::Camera::Manual;
        self.camera_glide = None;
        let call = self
            .session
            .agent(&agent)
            .and_then(|a| a.tool_calls().last())
            .map(|c| c.id.clone());
        self.inspector = Some(Inspector {
            agent,
            call,
            ..Default::default()
        });
    }

    pub fn inspected_call(&self) -> Option<&ToolCallInfo> {
        let i = self.inspector.as_ref()?;
        self.session
            .agent(&i.agent)?
            .tool_calls()
            .find(|c| Some(&c.id) == i.call.as_ref())
    }

    pub fn move_call(&mut self, delta: isize) {
        let Some(i) = &self.inspector else { return };
        let calls: Vec<_> = self
            .session
            .agent(&i.agent)
            .map(|a| a.tool_calls().map(|c| c.id.clone()).collect())
            .unwrap_or_default();
        if calls.is_empty() {
            return;
        }
        let pos = calls
            .iter()
            .position(|id| Some(id) == i.call.as_ref())
            .unwrap_or(calls.len() - 1);
        let next = pos.saturating_add_signed(delta).min(calls.len() - 1);
        let i = self.inspector.as_mut().unwrap();
        i.call = Some(calls[next].clone());
        i.scroll = 0;
        i.horizontal = 0;
        i.notice = None;
        i.command = None;
        i.part = 0;
    }

    pub fn inspector_evidence(&self) -> (String, String) {
        let Some(i) = &self.inspector else {
            return Default::default();
        };
        let Some(call) = &i.call else {
            return Default::default();
        };
        let input = self
            .session
            .tool_evidence(&i.agent, call, false)
            .join("\n\n--- recorded input version ---\n\n");
        let output = self
            .session
            .tool_evidence(&i.agent, call, true)
            .join("\n\n--- recorded result version ---\n\n");
        (input, output)
    }

    pub fn interpretation_request(&self) -> Option<InterpretationRequest> {
        let i = self.inspector.as_ref()?;
        let call = self.inspected_call()?;
        let (input, output) = self.inspector_evidence();
        let reference = reference_notes(&call.name, &input);
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        (&input, &output, &reference).hash(&mut hash);
        Some(InterpretationRequest {
            key: format!(
                "{}:{}:{}:{:x}",
                self.current_session_id,
                i.agent,
                call.id,
                hash.finish()
            ),
            tool: call.name.clone(),
            input,
            output,
            reference,
        })
    }

    pub fn request_interpretation(&mut self) {
        let Some(request) = self.interpretation_request() else {
            return;
        };
        if !self.interpretations.contains_key(&request.key) {
            self.interpretations.insert(
                request.key.clone(),
                "Interpreting selected input and recorded output with Mercury…".into(),
            );
            self.pending_interpretations.push_back(request);
            self.interpretation_revision = self.interpretation_revision.wrapping_add(1);
        }
        if let Some(i) = &mut self.inspector {
            i.tab = Tab::Interpret;
            i.detail = true;
            i.scroll = 0;
        }
    }

    /// Rebuild display text only when evidence/tab/result changes, never per
    /// output line per animation frame. Raw evidence remains untouched.
    pub fn refresh_inspector(&mut self) {
        let Some(i) = &self.inspector else { return };
        // Arc identity is stable across timeline/snapshot clones. Check the
        // lightweight version signature before joining or formatting payloads.
        let mut signature = std::collections::hash_map::DefaultHasher::new();
        (
            &i.agent,
            &i.call,
            i.tab as u8,
            i.raw,
            i.nowrap,
            i.content_width,
            self.interpretation_revision,
            self.explorer.revision,
            i.part,
        )
            .hash(&mut signature);
        if let Some(call) = &i.call {
            for output in [false, true] {
                if let Some(versions) =
                    self.session
                        .evidence
                        .get(&(i.agent.clone(), call.clone(), output))
                {
                    for ((ts, text), _) in versions {
                        (ts, text.as_ptr() as usize, text.len()).hash(&mut signature);
                    }
                }
            }
        }
        self.inspected_call()
            .map(|c| c.state as u8)
            .hash(&mut signature);
        let stamp = signature.finish();
        if i.view_stamp == Some(stamp) {
            return;
        }
        self.prepare_exploration();
        let i = self.inspector.as_ref().unwrap();
        let request = self.interpretation_request();
        let selected = self.inspected_call();
        let content = match (selected, request.as_ref()) {
            (Some(call), Some(r)) => match i.tab {
                Tab::Input => if r.input.is_empty() { "Input not recorded.".into() } else { if i.raw { r.input.clone() } else { readable_input(&r.input) } },
                Tab::Output => if r.output.is_empty() {
                    if call.state == ToolState::Pending { "No result recorded at this point in time. The call is pending.".into() }
                    else { "Tool completion was recorded, but no result body is available.".into() }
                } else { if i.raw { r.output.clone() } else { readable_output(&r.output, 0) } },
                Tab::Explain => i.command.as_ref().and_then(|cmd| self.explorer.cache.get(cmd).map(|e| {
                    let notes = e.notes(cmd, i.part);
                    if e.error.is_some() { format!("{notes}\n\n{}", r.reference) } else { notes }
                })).unwrap_or_else(|| r.reference.clone()),
                Tab::Interpret => self.interpretations.get(&r.key).cloned().unwrap_or_else(||
                    "No interpretation for this evidence snapshot. Press i to send this call's input and recorded output to Mercury.\n\nAn interpretation from another point in time is not reused when evidence changes.".into()),
            },
            _ => "This call is not present at this playhead. Move forward in time, or select another call. No future output is shown.".into(),
        };
        let i = self.inspector.as_mut().unwrap();
        i.view_stamp = Some(stamp);
        (i.command_lines, i.command_row) = i
            .command
            .as_ref()
            .filter(|command| command.len() <= crate::exploration::MAX_COMMAND)
            .map(|command| {
                let part = self
                    .explorer
                    .cache
                    .get(command)
                    .and_then(|e| e.spans.get(i.part));
                crate::ui::inspector::command_lines(command, part, i.content_width.max(1))
            })
            .unwrap_or_default();
        i.lines = safe_text(&content)
            .split('\n')
            .flat_map(|line| {
                if !i.nowrap && i.content_width > 0 && !line.is_empty() {
                    wrap_evidence_line(line, i.content_width)
                } else {
                    vec![line.to_string()]
                }
            })
            .collect();
    }

    pub fn find_in_inspector(&mut self, next: bool) {
        let Some(i) = &mut self.inspector else { return };
        if i.query.is_empty() || i.lines.is_empty() {
            return;
        }
        let query = i.query.to_lowercase();
        let start = if next { i.scroll.saturating_add(1) } else { 0 };
        let len = i.lines.len();
        if let Some(row) = (0..len)
            .map(|n| (start + n) % len)
            .find(|&n| i.lines[n].to_lowercase().contains(&query))
        {
            i.scroll = row;
            i.notice = Some(format!("Match on line {}", row + 1));
        } else {
            i.notice = Some("No matches".into());
        }
    }

    pub fn step_event(&mut self, forward: bool) {
        let len = self.timeline.items.len();
        if len == 0 {
            return;
        }
        let target = if forward {
            self.timeline.folded.saturating_add(1).min(len)
        } else {
            self.timeline.folded.saturating_sub(1).max(1)
        };
        self.timeline.cursor = self.timeline.ts_at_index(target - 1);
        self.timeline.follow_head = false;
        self.commit_inspector_seek(target);
    }
}

// Preserve indentation and every character: wrapping changes display rows only.
fn wrap_evidence_line(line: &str, width: usize) -> Vec<String> {
    use unicode_width::UnicodeWidthChar;
    let mut rows = Vec::new();
    let mut rest = line;
    while !rest.is_empty() {
        let mut columns = 0;
        let mut boundary = None;
        let mut cut = rest.len();
        for (at, c) in rest.char_indices() {
            let size = c.width().unwrap_or(0);
            if at > 0 && columns + size > width {
                cut = boundary.unwrap_or(at);
                break;
            }
            columns += size;
            if c.is_whitespace() {
                boundary = Some(at + c.len_utf8());
            }
        }
        rows.push(rest[..cut].to_string());
        rest = &rest[cut..];
    }
    rows
}

fn readable_input(text: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text)
        && let Some(cmd) = v
            .get("cmd")
            .or_else(|| v.get("command"))
            .and_then(|v| v.as_str())
    {
        return format!("{}\n\nInvocation arguments\n{}", cmd, pretty(text));
    }
    pretty(text)
}

fn readable_output(text: &str, depth: usize) -> String {
    if depth > 3 {
        return text.into();
    }
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else {
        return text.into();
    };
    if let Some(s) = v.as_str() {
        return readable_output(s, depth + 1);
    }
    if let Some(parts) = v.as_array() {
        return parts
            .iter()
            .map(|p| {
                p.get("text")
                    .and_then(|t| t.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("[Structured content]\n{}", p))
            })
            .collect::<Vec<_>>()
            .join("\n\n");
    }
    if let Some(map) = v.as_object() {
        let mut sections = Vec::new();
        for (key, value) in map {
            if let Some(body) = value.as_str().filter(|_| {
                matches!(
                    key.as_str(),
                    "output" | "stdout" | "stderr" | "aggregated_output"
                )
            }) {
                sections.push(format!("{}\n{}", key, readable_output(body, depth + 1)));
            } else {
                sections.push(format!("{}: {}", key, value));
            }
        }
        return sections.join("\n\n");
    }
    pretty(text)
}

fn pretty(text: &str) -> String {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .filter(|v| v.is_object() || v.is_array())
        .and_then(|v| serde_json::to_string_pretty(&v).ok())
        .unwrap_or_else(|| text.to_string())
}

/// Display control bytes visibly. Never interpret ANSI/OSC (including clipboard
/// and hyperlink escapes) supplied by a tool. Preserve line structure.
pub fn safe_text(text: &str) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\n' => out.push('\n'),
            '\t' => out.push_str("    "),
            c if c.is_control() => {
                let _ = write!(out, "\\u{{{:x}}}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

/// Deliberately bounded reference coverage. This is a command guide, not a
/// claim to have parsed or explained every token, expansion or embedded script.
pub fn reference_notes(tool: &str, input: &str) -> String {
    let value = serde_json::from_str::<serde_json::Value>(input).ok();
    let shell_tool = matches!(
        tool,
        "Bash"
            | "shell"
            | "exec_command"
            | "shell_command"
            | "functions.exec_command"
            | "functions.shell_command"
    );
    if !shell_tool {
        return "No deterministic guide for this tool yet.\n\nThe Input tab preserves its code or structured arguments. Use i for contextual interpretation. Embedded Python or JavaScript is not shell syntax.".into();
    }
    let command = value
        .as_ref()
        .and_then(|v| v.get("cmd").or_else(|| v.get("command")))
        .and_then(|v| v.as_str());
    let Some(command) = command else {
        return "Structured shell invocation. Inspect the recorded command array and shell flags in Input. Use i for contextual interpretation. No token-level explanation is claimed.".into();
    };
    let executable = command.split_whitespace().next().unwrap_or("");
    let notes = match executable {
        "rg" => {
            "rg searches for a pattern in files. -n includes line numbers; --files lists searchable paths. In `rg -n var1 var2`, var1 is the pattern and var2 is a path, not a second pattern. Exit 1 means no match; it is not necessarily an execution failure.\nSource: https://github.com/BurntSushi/ripgrep/blob/master/GUIDE.md"
        }
        "ssh" => {
            "ssh connects to a remote host. -o supplies a configuration option as an argument, for example `-o BatchMode=yes`. BatchMode disables interactive authentication prompts; it does not mean running several commands. The host and any remote command are separate operands.\nSource: https://man.openbsd.org/ssh and https://man.openbsd.org/ssh_config"
        }
        "sed" => {
            "sed applies editing commands to input. `sed -n '20,40p' file` suppresses default printing and prints the inclusive line range 20–40. Without -n, p may print selected lines in addition to the default output. Options such as -i differ between BSD and GNU sed.\nSource: local `man sed` (use the manual for the actual execution host)."
        }
        "git" => {
            "git status reports worktree and index state. --short gives compact output; --porcelain requests a format intended for scripts; --branch includes branch information. Other git subcommands and their options need their own reference.\nSource: https://git-scm.com/docs/git-status"
        }
        "python" | "python3" => {
            "Python's -c executes a supplied program string. A '-' script operand reads the program from standard input, often supplied by a shell heredoc. The script body is Python; shell syntax explanations do not establish what it does.\nSource: https://docs.python.org/3/using/cmdline.html"
        }
        _ => {
            "No deterministic guide for this executable yet. Inspect its exact input and use i for contextual interpretation."
        }
    };
    format!(
        "Command reference · {}\n\n{}\n\nCoverage: reference notes for the leading command only. Pipelines, quoting, expansions and later commands have not been fully parsed. Nothing is executed.",
        executable, notes
    )
}

#[cfg(all(test, feature = "native"))]
mod tests;
