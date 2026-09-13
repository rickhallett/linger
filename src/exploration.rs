//! Local documentation spans. The command is data, never a program to execute.
use crate::{inspector::Tab, state::App};
use serde::Deserialize;
use std::collections::HashMap;

pub const MAX_COMMAND: usize = 32 * 1024;

#[derive(Clone, Debug, Deserialize)]
pub struct Part {
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub source: String,
    pub extractor: String,
    pub kind: String,
    pub known: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Explanation {
    #[serde(default)]
    pub spans: Vec<Part>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Default)]
pub struct Explorer {
    pub cache: HashMap<String, Explanation>,
    pub pending: Option<String>,
    pub revision: u64,
}

pub fn command(tool: &str, input: &str) -> Option<String> {
    if crate::command_literals::is_orchestration(tool) {
        return crate::command_literals::commands(tool, input)
            .into_iter()
            .next()
            .map(|c| c.text);
    }
    if !matches!(
        tool,
        "Bash"
            | "shell"
            | "exec_command"
            | "shell_command"
            | "functions.exec_command"
            | "functions.shell_command"
    ) {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(input).ok()?;
    let value = v.get("cmd").or_else(|| v.get("command"))?;
    if let Some(command) = value.as_str() {
        return Some(command.into());
    }
    let argv = value
        .as_array()?
        .iter()
        .map(|v| v.as_str().map(str::to_owned))
        .collect::<Option<Vec<_>>>()?;
    crate::patterns::shell_wrapper(&argv).map(|(_, command)| command.to_owned())
}

impl Explanation {
    pub fn message(text: impl Into<String>) -> Self {
        Self {
            error: Some(text.into()),
            ..Self::default()
        }
    }

    /// Python offsets count Unicode scalars; Rust slices need byte offsets.
    /// Reject malformed ranges, preserve overlaps for nested shell constructs,
    /// and expose uncovered non-whitespace as explicitly unknown.
    pub fn validate(mut self, command: &str) -> Self {
        if self.error.is_some() {
            return self;
        }
        let offsets: Vec<_> = command
            .char_indices()
            .map(|(n, _)| n)
            .chain([command.len()])
            .collect();
        if self
            .spans
            .iter()
            .any(|p| p.start >= p.end || p.end >= offsets.len())
        {
            return Self::message(
                "The matcher returned invalid spans. Inspect the original Input instead.",
            );
        }
        let mut covered = vec![false; offsets.len() - 1];
        for p in &mut self.spans {
            covered[p.start..p.end].fill(true);
            p.start = offsets[p.start];
            p.end = offsets[p.end];
        }
        let chars: Vec<_> = command.chars().collect();
        let mut n = 0;
        while n < chars.len() {
            if covered[n] || chars[n].is_whitespace() {
                n += 1;
                continue;
            }
            let start = n;
            while n < chars.len() && !covered[n] && !chars[n].is_whitespace() {
                n += 1;
            }
            self.spans.push(Part { start: offsets[start], end: offsets[n], text: "No documentation matched this part. Embedded program bodies need their own language explanation; use i for context.".into(), source: "Unmatched input".into(), extractor: String::new(), kind: "unknown".into(), known: false });
        }
        self.spans.sort_by_key(|p| (p.start, p.end));
        self
    }

    pub fn notes(&self, command: &str, index: usize) -> String {
        if let Some(error) = &self.error {
            return error.clone();
        }
        let Some(p) = self.spans.get(index) else {
            return "No command parts to explore.".into();
        };
        let extraction = if p.extractor.is_empty() {
            String::new()
        } else {
            format!(" · extraction: {}", p.extractor)
        };
        format!(
            "Part {}/{} · {} · {}\n\n{}\n\nSource: {}{}\n\n{}\n\nLocal matching; no command executed. The manual platform is a reference, not a detected execution host.\nUse 1 for original input; i for Mercury on the whole call.",
            index + 1,
            self.spans.len(),
            p.kind,
            if p.known { "matched" } else { "unknown" },
            &command[p.start..p.end],
            p.source,
            extraction,
            p.text
        )
    }
}

impl App {
    pub fn prepare_exploration(&mut self) {
        if !self
            .inspector
            .as_ref()
            .is_some_and(|i| i.tab == Tab::Explain)
        {
            return;
        }
        let mut from_argv = false;
        let mut shell_count = 0;
        let cmd = self.inspected_call().and_then(|call| {
            let i = self.inspector.as_ref()?;
            let evidence = self.session.tool_evidence(&i.agent, &call.id, false);
            let input = evidence.last()?;
            from_argv = serde_json::from_str::<serde_json::Value>(input)
                .ok()
                .and_then(|v| {
                    v.get("cmd")
                        .or_else(|| v.get("command"))
                        .map(|c| c.is_array())
                })
                .unwrap_or(false);
            if crate::command_literals::is_orchestration(&call.name) {
                let literals = crate::command_literals::commands(&call.name, input);
                shell_count = literals.len();
                literals
                    .get(i.shell_index.min(shell_count.saturating_sub(1)))
                    .map(|c| c.text.clone())
            } else {
                command(&call.name, input)
            }
        });
        let i = self.inspector.as_mut().unwrap();
        i.command_from_argv = from_argv;
        i.shell_count = shell_count;
        i.shell_index = i.shell_index.min(shell_count.saturating_sub(1));
        if i.command != cmd {
            i.command = cmd.clone();
            i.part = 0;
            i.scroll = 0;
            i.horizontal = 0;
        }
        let Some(cmd) = cmd else { return };
        if self.explorer.cache.contains_key(&cmd) {
            return;
        }
        if self.explorer.cache.len() >= 32 {
            self.explorer.cache.clear();
        }
        let message = if cmd.len() > MAX_COMMAND {
            "Command exceeds the 32 KiB local matcher limit. The complete recorded input is available in tab 1."
        } else if !cfg!(feature = "native") {
            "The local command matcher is available in the native terminal build."
        } else {
            if let Some(old) = self.explorer.pending.replace(cmd.clone()) {
                self.explorer.cache.remove(&old);
            }
            "Matching command parts against local documentation…"
        };
        self.explorer
            .cache
            .insert(cmd, Explanation::message(message));
    }

    pub fn move_command_part(&mut self, delta: isize) {
        let Some(i) = &mut self.inspector else { return };
        let len = i
            .command
            .as_ref()
            .and_then(|c| self.explorer.cache.get(c))
            .map_or(0, |e| e.spans.len());
        i.part = i
            .part
            .saturating_add_signed(delta)
            .min(len.saturating_sub(1));
        i.scroll = 0;
        i.horizontal = 0;
    }
}

#[cfg(feature = "native")]
pub async fn run(command: String) -> (String, Explanation) {
    let result = lookup(&command).await.unwrap_or_else(Explanation::message);
    (command, result)
}

#[cfg(feature = "native")]
async fn lookup(command: &str) -> Result<Explanation, String> {
    use std::{path::PathBuf, process::Stdio};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let root = std::env::var_os("LINGER_EXPLAINSHELL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
                })
                .join("linger/explainshell")
        });
    if !root.join(".venv/bin/python").is_file() || !root.join("manpages.db").is_file() {
        return Err("Command exploration needs the optional local explainshell pack.\n\nFrom the Linger checkout run:\nuv run scripts/setup-explainshell.py\n\nThen restart Linger. Setup downloads documentation; lookups stay local. Existing reference notes follow below.".into());
    }
    let work = async {
        let mut child = tokio::process::Command::new(root.join(".venv/bin/python"))
            .args(["-I", "-c", include_str!("explainshell_bridge.py")])
            .arg(&root)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| "Could not start the local command matcher.".to_string())?;
        let bytes = serde_json::to_vec(&serde_json::json!({"command": command})).unwrap();
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(&bytes)
            .await
            .map_err(|_| "Could not send input to the local matcher.")?;
        drop(stdin);
        let mut output = Vec::new();
        child
            .stdout
            .take()
            .unwrap()
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut output)
            .await
            .map_err(|_| "Could not read local documentation.")?;
        if output.len() > 2 * 1024 * 1024 {
            return Err("Local documentation exceeded the response limit.".into());
        }
        let status = child
            .wait()
            .await
            .map_err(|_| "Local matcher did not complete.")?;
        if !status.success() {
            return Err("Local matcher failed; original input remains available.".into());
        }
        let response: Explanation = serde_json::from_slice(&output)
            .map_err(|_| "Local matcher returned an unreadable response.")?;
        Ok(response.validate(command))
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), work)
        .await
        .map_err(|_| {
            "Local documentation lookup timed out; original input remains available.".to_string()
        })?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_offsets_and_unknown_gaps_preserve_exact_input() {
        let cmd = "rg café -n";
        let e = Explanation {
            spans: vec![Part {
                start: 3,
                end: 7,
                text: "pattern".into(),
                source: "fixture".into(),
                extractor: String::new(),
                kind: "operand".into(),
                known: true,
            }],
            error: None,
        }
        .validate(cmd);
        assert_eq!(
            e.spans
                .iter()
                .map(|p| &cmd[p.start..p.end])
                .collect::<Vec<_>>(),
            ["rg", "café", "-n"]
        );
        assert!(!e.spans[0].known);
        assert!(e.spans[1].known);
    }
    #[test]
    fn rejects_invalid_offsets() {
        let raw = r#"{"spans":[{"start":0,"end":99,"text":"x","source":"x","extractor":"","kind":"x","known":true}]}"#;
        assert!(
            serde_json::from_str::<Explanation>(raw)
                .unwrap()
                .validate("ls")
                .error
                .is_some()
        );
    }
    #[test]
    fn only_recognized_shell_argv_exposes_its_inner_string() {
        assert_eq!(
            command("shell", r#"{"command":["/bin/zsh","-lc","rg -n foo src"]}"#).as_deref(),
            Some("rg -n foo src")
        );
        assert!(
            command(
                "shell",
                r#"{"command":["/bin/zsh","-lc","rg -n foo src","arg0"]}"#
            )
            .is_none()
        );
        assert!(command("shell", r#"{"command":["python3","-c","print(1)"]}"#).is_none());
    }
    #[test]
    fn embedded_code_and_argv_are_not_shell_strings() {
        assert!(command("functions.exec", r#"{"cmd":"rg -n foo"}"#).is_none());
        assert!(command("shell", r#"{"command":["python","-c","print('rg')"]}"#).is_none());
        assert_eq!(
            command("Bash", r#"{"command":"rg -n foo"}"#).as_deref(),
            Some("rg -n foo")
        );
    }
}
