//! Conservative, versioned command patterns and the personal attention view.
//! This index observes ingestion, not playback. It never executes input.
use crate::{
    fact::{Fact, FactKind},
    state::App,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod normalize;
#[cfg(feature = "native")]
pub mod storage;
pub use normalize::pattern;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Learning {
    #[default]
    Unmarked,
    Want,
    Practising,
    Learned,
}
impl Learning {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unmarked => "Unmarked",
            Self::Want => "Want to understand",
            Self::Practising => "Practising",
            Self::Learned => "Learned",
        }
    }
    pub fn mark(self) -> &'static str {
        match self {
            Self::Unmarked => "·",
            Self::Want => "?",
            Self::Practising => "◎",
            Self::Learned => "✓",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pattern {
    pub key: String,
    pub label: String,
    pub command: String,
    pub tool: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Occurrence {
    pub session: String,
    pub agent: String,
    pub call: String,
    pub order: String,
    pub pattern: Pattern,
}
#[derive(Clone, Default)]
pub struct Aggregate {
    pub key: String,
    pub label: String,
    pub example: String,
    pub tool: String,
    pub sessions: BTreeMap<String, usize>,
}
#[derive(Clone)]
pub struct Row {
    pub key: String,
    pub label: String,
    pub example: String,
    pub tool: String,
    pub here: usize,
    pub total: usize,
    pub spread: usize,
}
#[derive(Default)]
pub struct View {
    pub all: bool,
    pub show_learned: bool,
    pub selected: Option<String>,
    pub query: String,
    pub searching: bool,
    pub scroll: usize,
    pub example: usize,
    pub rows: Vec<Row>,
    pub preview: Vec<ratatui::text::Line<'static>>,
    pub preview_stamp: Option<(u64, Option<String>, usize, u16)>,
    stamp: Option<(u64, bool, bool, String)>,
}
#[derive(Default)]
struct RawCall {
    name: Option<String>,
    inputs: BTreeSet<(String, std::sync::Arc<str>)>,
}

#[derive(Default)]
pub struct Library {
    pub session: String,
    raw: BTreeMap<(String, String), RawCall>,
    pub current: BTreeMap<(String, String), Occurrence>,
    pub cached: Vec<Aggregate>,
    pub states: BTreeMap<String, Learning>,
    pub view: Option<View>,
    pub revision: u64,
    pub pending: BTreeMap<(String, String, String), Occurrence>,
    pub choices: Vec<(String, Learning)>,
    pub notice: Option<String>,
    pub saving: usize,
    pub storage_error: Option<String>,
    pub ready: bool,
    pub refresh_requested: bool,
    mark_stamp: Option<(u64, u64, usize)>,
    marks: Vec<usize>,
}
impl Library {
    pub fn reset(&mut self, session: &str) {
        self.session = session.into();
        self.raw.clear();
        self.current.clear();
        self.view = None;
        self.revision += 1;
    }
    pub fn observe<'a>(&mut self, session: &str, facts: impl Iterator<Item = &'a Fact>) {
        if self.session != session {
            self.reset(session);
        }
        let mut changed = BTreeSet::new();
        for f in facts {
            let Some(agent) = &f.agent else { continue };
            match &f.kind {
                FactKind::ToolStart { call, name, .. } => {
                    let id = (agent.clone(), call.clone());
                    let raw = self.raw.entry(id.clone()).or_default();
                    if raw.name.as_ref().is_none_or(|old| name < old) {
                        raw.name = Some(name.clone());
                        changed.insert(id);
                    }
                }
                FactKind::ToolEvidence {
                    call,
                    output: false,
                    text,
                } => {
                    let id = (agent.clone(), call.clone());
                    let raw = self.raw.entry(id.clone()).or_default();
                    // Same deterministic earliest input across arrival orders.
                    let candidate = (
                        f.ts.map(|t| t.to_rfc3339()).unwrap_or_default(),
                        text.clone(),
                    );
                    if raw.inputs.insert(candidate) {
                        changed.insert(id);
                    }
                }
                _ => {}
            }
        }
        for id in changed {
            let raw = &self.raw[&id];
            if let Some(name) = &raw.name
                && let Some((order, input, pattern)) = raw
                    .inputs
                    .iter()
                    .find_map(|(order, input)| pattern(name, input).map(|p| (order, input, p)))
            {
                let occurrence = Occurrence {
                    session: session.into(),
                    agent: id.0.clone(),
                    call: id.1.clone(),
                    order: format!("{order}\n{input}"),
                    pattern,
                };
                if self.current.get(&id) != Some(&occurrence) {
                    self.pending.insert(
                        (session.into(), id.0.clone(), id.1.clone()),
                        occurrence.clone(),
                    );
                    self.current.insert(id, occurrence);
                    self.revision += 1;
                }
            }
        }
    }
    pub fn practising_events(
        &mut self,
        items: &[crate::tailer::ReplayItem],
        generation: u64,
    ) -> &[usize] {
        let stamp = (self.revision, generation, items.len());
        if self.mark_stamp != Some(stamp) {
            let mut seen = BTreeSet::new();
            self.marks = items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    item.facts
                        .iter()
                        .any(|f| {
                            if let (
                                Some(agent),
                                FactKind::ToolEvidence {
                                    call,
                                    output: false,
                                    ..
                                },
                            ) = (&f.agent, &f.kind)
                            {
                                self.state_for_call(agent, call) == Learning::Practising
                                    && seen.insert((agent.clone(), call.clone()))
                            } else {
                                false
                            }
                        })
                        .then_some(index)
                })
                .collect();
            self.mark_stamp = Some(stamp);
        }
        &self.marks
    }
    pub fn learning(&self, key: &str) -> Learning {
        self.states.get(key).copied().unwrap_or_default()
    }
    pub fn state_for_call(&self, agent: &str, call: &str) -> Learning {
        self.current
            .get(&(agent.into(), call.into()))
            .map(|o| self.learning(&o.pattern.key))
            .unwrap_or_default()
    }
    pub fn choose(&mut self, key: String, state: Learning) {
        self.states.insert(key.clone(), state);
        self.choices.push((key, state));
        self.saving += 1;
        self.revision += 1;
        self.notice = Some("Saving learning state…".into());
    }
    pub fn refresh_view(&mut self) {
        let Some(view) = &self.view else { return };
        let stamp = (
            self.revision,
            view.all,
            view.show_learned,
            view.query.clone(),
        );
        if view.stamp.as_ref() == Some(&stamp) {
            return;
        }
        let mut aggregates: BTreeMap<String, Aggregate> = self
            .cached
            .iter()
            .cloned()
            .map(|a| (a.key.clone(), a))
            .collect();
        let mut here: BTreeMap<String, usize> = BTreeMap::new();
        for o in self.current.values() {
            *here.entry(o.pattern.key.clone()).or_default() += 1;
            aggregates
                .entry(o.pattern.key.clone())
                .or_insert_with(|| Aggregate {
                    key: o.pattern.key.clone(),
                    label: o.pattern.label.clone(),
                    example: o.pattern.command.clone(),
                    tool: o.pattern.tool.clone(),
                    sessions: Default::default(),
                });
        }
        let query = view.query.to_lowercase();
        let mut rows: Vec<_> = aggregates
            .into_values()
            .filter_map(|mut a| {
                let n = here.get(&a.key).copied().unwrap_or(0);
                if (!view.all && n == 0)
                    || (!view.show_learned && self.learning(&a.key) == Learning::Learned)
                    || !format!("{} {}", a.label, a.example)
                        .to_lowercase()
                        .contains(&query)
                {
                    return None;
                }
                if n > 0 {
                    let cached = a.sessions.entry(self.session.clone()).or_default();
                    *cached = (*cached).max(n);
                }
                Some(Row {
                    key: a.key,
                    label: a.label,
                    example: a.example,
                    tool: a.tool,
                    here: n,
                    total: a.sessions.values().sum(),
                    spread: a.sessions.len(),
                })
            })
            .collect();
        rows.sort_by(|a, b| {
            (if view.all { b.total } else { b.here })
                .cmp(&(if view.all { a.total } else { a.here }))
                .then_with(|| a.label.cmp(&b.label))
                .then_with(|| a.key.cmp(&b.key))
        });
        let view = self.view.as_mut().unwrap();
        let old_index = view
            .rows
            .iter()
            .position(|r| Some(&r.key) == view.selected.as_ref())
            .unwrap_or(0);
        if !rows.iter().any(|r| Some(&r.key) == view.selected.as_ref()) {
            view.selected = rows
                .get(old_index.min(rows.len().saturating_sub(1)))
                .map(|r| r.key.clone());
            view.scroll = 0;
            view.example = 0;
        }
        view.rows = rows;
        view.stamp = Some(stamp);
    }
    pub fn selected(&self) -> Option<&Row> {
        let v = self.view.as_ref()?;
        v.rows.iter().find(|r| Some(&r.key) == v.selected.as_ref())
    }
    pub fn move_row(&mut self, delta: isize) {
        self.refresh_view();
        let Some(v) = &mut self.view else { return };
        let n = v
            .rows
            .iter()
            .position(|r| Some(&r.key) == v.selected.as_ref())
            .unwrap_or(0)
            .saturating_add_signed(delta)
            .min(v.rows.len().saturating_sub(1));
        v.selected = v.rows.get(n).map(|r| r.key.clone());
        v.scroll = 0;
        v.example = 0;
    }
    pub fn examples(&self, key: &str) -> Vec<&Occurrence> {
        let mut rows: Vec<_> = self
            .current
            .values()
            .filter(|o| o.pattern.key == key)
            .collect();
        rows.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.call.cmp(&b.call)));
        rows
    }
}

impl App {
    pub fn open_library(&mut self) {
        if self.library.view.is_none() {
            self.library.view = Some(View::default());
        }
        self.library.refresh_view();
    }
    pub fn mark_inspected(&mut self, state: Learning) {
        let Some(i) = &self.inspector else { return };
        let Some(o) = self
            .library
            .current
            .get(&(i.agent.clone(), i.call.clone().unwrap_or_default()))
        else {
            return;
        };
        self.library.choose(o.pattern.key.clone(), state);
    }
    pub fn open_pattern_occurrence(&mut self) {
        let Some(row) = self.library.selected() else {
            return;
        };
        let examples = self.library.examples(&row.key);
        let n = self
            .library
            .view
            .as_ref()
            .map_or(0, |v| v.example)
            .min(examples.len().saturating_sub(1));
        let Some(o) = examples.get(n).cloned().cloned() else {
            self.library.notice=Some("This example is cached from another session. Open that recording to inspect its output.".into());
            return;
        };
        // Jump deliberately to the last recorded event for this occurrence.
        let at = self
            .timeline
            .items
            .iter()
            .rev()
            .find(|item| {
                item.facts.iter().any(|f| {
                    f.agent.as_ref() == Some(&o.agent)
                        && match &f.kind {
                            FactKind::ToolStart { call, .. }
                            | FactKind::ToolEvidence { call, .. }
                            | FactKind::ToolEnd { call, .. } => call == &o.call,
                            _ => false,
                        }
                })
            })
            .and_then(|item| item.ts());
        if let Some(at) = at {
            self.seek(at);
        }
        self.timeline.follow_head = false;
        self.is_paused = true;
        self.library.view = None;
        self.inspector = Some(crate::inspector::Inspector {
            agent: o.agent,
            call: Some(o.call),
            detail: true,
            ..Default::default()
        });
    }
}

#[cfg(all(test, feature = "native"))]
mod tests;
