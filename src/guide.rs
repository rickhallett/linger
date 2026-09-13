//! A field guide projected from actual collected command encounters.
use crate::patterns::{Level, Library, Occurrence, Pattern, Row};
use std::collections::BTreeMap;

type SpecimenId = (String, String, String);
fn identity(o: &Occurrence) -> SpecimenId {
    (o.session.clone(), o.agent.clone(), o.call.clone())
}

pub struct Entry {
    pub program: Row,
    pub forms: Vec<Row>,
}
#[derive(Default)]
pub struct Guide {
    pub open: bool,
    pub detail: bool,
    pub selected: Option<String>,
    pub form: Option<String>,
    pub specimen: usize,
    pub scroll: usize,
    pub columns: usize,
    pub here_only: bool,
    pub query: String,
    pub searching: bool,
    pub editing: Option<(String, String, usize)>,
    pub notes: BTreeMap<String, String>,
    pub pending_notes: Vec<(String, String)>,
    pub notice: Option<String>,
    pub entries: Vec<Entry>,
    pub collected: BTreeMap<SpecimenId, Occurrence>,
    pub pending_collections: Vec<Occurrence>,
    collection_revision: u64,
    specimens: BTreeMap<String, Vec<SpecimenId>>,
    revision: Option<(u64, String)>,
}

pub fn original(row: &Row) -> Pattern {
    crate::patterns::pattern(&row.tool, &row.example)
        .or_else(|| {
            crate::patterns::pattern(
                &row.tool,
                &serde_json::json!({"command": row.example}).to_string(),
            )
        })
        .unwrap_or_else(|| Pattern {
            key: String::new(),
            label: String::new(),
            command: row.example.clone(),
            tool: row.tool.clone(),
        })
}
impl Guide {
    pub fn collect(&mut self, o: Occurrence, persist: bool) -> bool {
        if self.collected.contains_key(&identity(&o)) {
            return false;
        }
        self.collected.insert(identity(&o), o.clone());
        if persist {
            self.pending_collections.push(o);
        }
        self.collection_revision += 1;
        true
    }
    pub fn examples(&self, key: &str, session: &str) -> Vec<&Occurrence> {
        let mut examples: Vec<_> = self
            .specimens
            .get(key)
            .into_iter()
            .flatten()
            .filter_map(|id| self.collected.get(id))
            .collect();
        examples.sort_by_key(|o| {
            (
                o.session != session,
                &o.order,
                &o.session,
                &o.agent,
                &o.call,
            )
        });
        examples
    }
    pub fn refresh(&mut self, library: &Library) {
        let revision = (self.collection_revision, library.session.clone());
        if self.revision.as_ref() == Some(&revision) {
            return;
        }
        self.specimens.clear();
        let mut rows = BTreeMap::<String, Row>::new();
        let mut sessions = BTreeMap::<String, std::collections::BTreeSet<String>>::new();
        for (id, o) in &self.collected {
            for p in crate::patterns::projections(&o.pattern) {
                self.specimens
                    .entry(p.key.clone())
                    .or_default()
                    .push(id.clone());
                let spread = sessions.entry(p.key.clone()).or_default();
                spread.insert(o.session.clone());
                let row = rows.entry(p.key.clone()).or_insert_with(|| Row {
                    key: p.key,
                    label: p.label,
                    example: o.pattern.command.clone(),
                    tool: o.pattern.tool.clone(),
                    here: 0,
                    total: 0,
                    spread: 0,
                    level: p.level,
                    inline: p.inline,
                    first_seen: None,
                });
                row.total += 1;
                row.here += usize::from(o.session == library.session);
                row.spread = spread.len();
                if let Some(first) = crate::patterns::recorded_time(&o.order)
                    && row.first_seen.as_ref().is_none_or(|old| first < *old)
                {
                    row.first_seen = Some(first);
                }
            }
        }
        let rows: Vec<_> = rows.into_values().collect();
        let mut entries: BTreeMap<String, Entry> = rows
            .iter()
            .filter(|r| r.level == Level::Programs)
            .map(|r| {
                (
                    r.key.clone(),
                    Entry {
                        program: r.clone(),
                        forms: vec![r.clone()],
                    },
                )
            })
            .collect();
        for row in rows
            .iter()
            .filter(|r| r.level != Level::Programs && !r.inline)
        {
            let pattern = original(row);
            let ranges = crate::patterns::highlight_ranges(&pattern, &row.key);
            for program in crate::patterns::projections(&pattern)
                .into_iter()
                .filter(|p| p.level == Level::Programs)
            {
                let member = crate::patterns::highlight_ranges(&pattern, &program.key)
                    .iter()
                    .any(|&(start, end)| ranges.iter().any(|&(s, e)| s <= start && end <= e));
                if member && let Some(entry) = entries.get_mut(&program.key) {
                    entry.forms.push(row.clone());
                }
            }
        }
        self.entries = entries.into_values().collect();
        self.entries
            .sort_by(|a, b| a.program.label.cmp(&b.program.label));
        for e in &mut self.entries {
            e.forms[1..].sort_by(|a, b| a.label.cmp(&b.label));
        }
        self.revision = Some(revision);
        self.reconcile();
    }
    pub fn visible(&self) -> Vec<&Entry> {
        let query = self.query.to_lowercase();
        let mut entries: Vec<_> = self
            .entries
            .iter()
            .filter(|e| {
                (!self.here_only || e.program.here > 0)
                    && (query.is_empty()
                        || e.program.label.to_lowercase().contains(&query)
                        || e.forms
                            .iter()
                            .any(|r| r.label.to_lowercase().contains(&query))
                        || self
                            .notes
                            .get(&e.program.key)
                            .is_some_and(|n| n.to_lowercase().contains(&query)))
            })
            .collect();
        if !query.is_empty() {
            entries.sort_by_key(|e| {
                let label = e.program.label.to_lowercase();
                if label == query {
                    0
                } else if label.contains(&query) {
                    1
                } else {
                    2
                }
            });
        }
        entries
    }
    pub fn reconcile(&mut self) {
        let visible = self.visible();
        if !visible
            .iter()
            .any(|e| Some(&e.program.key) == self.selected.as_ref())
        {
            self.selected = visible.first().map(|e| e.program.key.clone());
            self.form = None;
            self.specimen = 0;
            self.scroll = 0;
        }
        if let Some(entry) = self.entry()
            && !entry
                .forms
                .iter()
                .any(|r| Some(&r.key) == self.form.as_ref())
        {
            self.form = Some(entry.program.key.clone());
            self.specimen = 0;
            self.scroll = 0;
        }
    }
    pub fn entry(&self) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|e| Some(&e.program.key) == self.selected.as_ref())
    }
    pub fn row(&self) -> Option<&Row> {
        self.entry()?
            .forms
            .iter()
            .find(|r| Some(&r.key) == self.form.as_ref())
    }
    pub fn move_entry(&mut self, delta: isize) {
        let visible = self.visible();
        let index = visible
            .iter()
            .position(|e| Some(&e.program.key) == self.selected.as_ref())
            .unwrap_or(0);
        let next = index
            .saturating_add_signed(delta)
            .min(visible.len().saturating_sub(1));
        self.selected = visible.get(next).map(|e| e.program.key.clone());
        self.form = None;
        self.specimen = 0;
        self.scroll = 0;
        self.reconcile();
    }
    pub fn move_form(&mut self, delta: isize) {
        let Some(entry) = self.entry() else { return };
        let at = entry
            .forms
            .iter()
            .position(|r| Some(&r.key) == self.form.as_ref())
            .unwrap_or(0);
        self.form = entry
            .forms
            .get(
                at.saturating_add_signed(delta)
                    .min(entry.forms.len().saturating_sub(1)),
            )
            .map(|r| r.key.clone());
        self.specimen = 0;
        self.scroll = 0;
    }
    pub fn edit_note(&mut self) {
        if let Some(key) = &self.selected {
            let text = self.notes.get(key).cloned().unwrap_or_default();
            let cursor = text.len();
            self.editing = Some((key.clone(), text, cursor));
        }
    }
    pub fn save_note(&mut self) {
        if let Some((key, note, _)) = self.editing.take() {
            self.notes.insert(key.clone(), note.clone());
            self.pending_notes.push((key, note));
            self.notice = Some("Saving field note…".into());
        }
    }
}
impl crate::state::App {
    pub fn collect_inspected(&mut self) {
        let Some(i) = self.inspector.as_ref().filter(|i| i.detail) else {
            return;
        };
        let Some(call) = i.call.clone() else {
            return;
        };
        let id = (self.library.session.clone(), i.agent.clone(), call.clone());
        if self.guide.collected.contains_key(&id) {
            return;
        }
        let Some(info) = self.inspected_call() else {
            return;
        };
        let Some(pattern) = self
            .session
            .tool_evidence(&i.agent, &call, false)
            .into_iter()
            .find_map(|input| crate::patterns::pattern(&info.name, input))
        else {
            return;
        };
        let o = Occurrence {
            session: id.0,
            agent: id.1,
            call: id.2,
            order: info
                .ts
                .or(self.timeline.cursor)
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
            pattern,
        };
        if self.guide.collect(o, true) {
            self.inspector.as_mut().unwrap().notice = Some("Collected for the field guide.".into());
        }
    }
    pub fn open_field_guide(&mut self) {
        self.guide.refresh(&self.library);
        self.guide.open = true;
    }
    pub fn inspect_specimen(&mut self) {
        let Some(row) = self.guide.row().cloned() else {
            return;
        };
        let examples = self.guide.examples(&row.key, &self.library.session);
        let Some(o) = examples
            .get(self.guide.specimen.min(examples.len().saturating_sub(1)))
            .cloned()
            .cloned()
        else {
            return;
        };
        if o.session != self.library.session {
            self.guide.notice =
                Some("Collected specimen: open its original recording to inspect output.".into());
            return;
        }
        self.open_occurrence(o);
        self.guide.open = false;
    }
}

pub fn description(name: &str) -> (&'static str, [&'static str; 3]) {
    match name.rsplit('/').next().unwrap_or(name) {
        "rg" | "grep" => ("Search through text", ["  ╭────╮", "  │ .* │", "  ╰──┬─╯"]),
        "git" => (
            "Follow changes through time",
            ["  ●──●──●", "     ╲", "      ●──●"],
        ),
        "sed" | "awk" => ("Read and reshape text", ["  ┌───┐", "  │a→b│", "  └───┘"]),
        "python" | "python3" => (
            "Work with data and programs",
            ["  ╭─╮ ╭─╮", "  │ ╰─╯ │", "  ╰─╮ ╭─╯"],
        ),
        "ssh" => (
            "Reach another machine",
            ["  ┌─┐ ┌─┐", "  │ ├─┤ │", "  └─┘ └─┘"],
        ),
        "zsh" | "bash" | "sh" | "dash" | "ksh" => (
            "Compose commands in a shell",
            ["  ┌─────┐", "  │ >_  │", "  └─────┘"],
        ),
        _ => (
            "Collected from your sessions",
            ["  ┌─────┐", "  │ ◇   │", "  └─────┘"],
        ),
    }
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use super::*;
    use crate::{
        fact::{Fact, FactKind},
        state::{App, Mode},
    };
    fn observe(library: &mut Library, call: &str, command: &str, time: i64) {
        let make = |kind| Fact {
            agent: Some("main".into()),
            ts: chrono::DateTime::from_timestamp(1_800_000_000 + time, 0),
            kind,
        };
        library.observe(
            "test",
            [
                make(FactKind::ToolStart {
                    call: call.into(),
                    name: "Bash".into(),
                    summary: None,
                }),
                make(FactKind::ToolEvidence {
                    call: call.into(),
                    output: false,
                    text: serde_json::json!({"command": command}).to_string().into(),
                }),
            ]
            .iter(),
        );
    }
    #[test]
    fn guide_only_grows_from_collected_specimens_and_relates_their_forms() {
        let mut lib = Library::default();
        let mut guide = Guide::default();
        guide.refresh(&lib);
        assert!(guide.entries.is_empty());
        observe(&mut lib, "one", "rg -n git src && git status --short", 0);
        observe(&mut lib, "one", "rg -n git src && git status --short", 0);
        guide.refresh(&lib);
        assert!(guide.entries.is_empty());
        for o in lib.current.values() {
            guide.collect(o.clone(), true);
            guide.collect(o.clone(), true);
        }
        guide.refresh(&lib);
        assert_eq!(guide.pending_collections.len(), 1);
        assert_eq!(guide.entries.len(), 2);
        let rg = guide
            .entries
            .iter()
            .find(|e| e.program.label == "rg")
            .unwrap();
        assert_eq!(rg.program.total, 1);
        assert!(rg.forms.iter().any(|r| r.label.contains("&&")));
        assert!(!rg.forms.iter().any(|r| r.label == "git status --short"));
        assert!(rg.program.first_seen.is_some());
        guide.selected = Some(rg.program.key.clone());
        observe(&mut lib, "two", "awk '{print $1}' file", 10);
        guide.refresh(&lib);
        assert_eq!(guide.entries.len(), 2);
        for o in lib.current.values() {
            guide.collect(o.clone(), true);
        }
        guide.refresh(&lib);
        assert_eq!(guide.entries.len(), 3);
        assert_eq!(guide.entry().unwrap().program.label, "rg");
    }
    #[test]
    fn note_editor_is_modal_and_unicode_safe() {
        use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::new("test".into(), Mode::Replay);
        observe(&mut app.library, "one", "rg -n TODO src", 0);
        for o in app.library.current.values() {
            app.guide.collect(o.clone(), true);
        }
        app.open_field_guide();
        app.guide.detail = true;
        let key = |app: &mut App, code| {
            crate::handler::handle_event(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)), app)
        };
        key(&mut app, KeyCode::Char('n'));
        for c in "café qGp/".chars() {
            assert!(!key(&mut app, KeyCode::Char(c)));
        }
        assert!(app.guide.open);
        assert!(app.pending_interpretations.is_empty());
        key(&mut app, KeyCode::Home);
        key(&mut app, KeyCode::Right);
        key(&mut app, KeyCode::Char('X'));
        key(&mut app, KeyCode::End);
        key(&mut app, KeyCode::Enter);
        assert_eq!(app.guide.notes.values().next().unwrap(), "cXafé qGp/");
        assert_eq!(app.guide.pending_notes.len(), 1);
        key(&mut app, KeyCode::Char('n'));
        key(&mut app, KeyCode::Char('z'));
        key(&mut app, KeyCode::Esc);
        assert_eq!(app.guide.notes.values().next().unwrap(), "cXafé qGp/");
        key(&mut app, KeyCode::Char('n'));
        crate::handler::handle_event(&Event::Paste("界".repeat(3000)), &mut app);
        assert!(app.guide.editing.as_ref().unwrap().1.len() <= 4096);
    }
    #[test]
    fn cached_specimen_does_not_jump_to_unrelated_current_evidence() {
        let mut app = App::new("test".into(), Mode::Replay);
        observe(&mut app.library, "one", "rg -n TODO src", 0);
        for o in app.library.current.values() {
            app.guide.collect(o.clone(), true);
        }
        app.library.reset("other");
        app.open_field_guide();
        app.guide.detail = true;
        app.inspect_specimen();
        assert!(app.guide.open);
        assert!(app.inspector.is_none());
        assert!(
            app.guide
                .notice
                .as_ref()
                .unwrap()
                .contains("Collected specimen")
        );
    }
}
