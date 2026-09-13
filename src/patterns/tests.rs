use super::*;
use crate::{
    fact::FactKind,
    state::{App, Mode},
    tailer::{ReplayItem, UiEvent},
};

fn input(command: &str) -> String {
    serde_json::json!({"command":command}).to_string()
}
fn shape(command: &str) -> Pattern {
    pattern("Bash", &input(command)).unwrap()
}
fn facts(call: &str, command: &str, time: i64) -> Vec<Fact> {
    let at = chrono::DateTime::from_timestamp(1_800_000_000 + time, 0);
    vec![
        Fact {
            agent: Some("main".into()),
            ts: at,
            kind: FactKind::ToolStart {
                call: call.into(),
                name: "Bash".into(),
                summary: Some(command.into()),
            },
        },
        Fact {
            agent: Some("main".into()),
            ts: at,
            kind: FactKind::ToolEvidence {
                call: call.into(),
                output: false,
                text: input(command).into(),
            },
        },
    ]
}
#[test]
fn conservative_shapes_preserve_options_compositions_and_code() {
    assert_eq!(
        shape("rg -n needle src").key,
        shape("rg -n 'other words' tests").key
    );
    assert_ne!(shape("rg -n needle src").key, shape("rg -l needle src").key);
    assert_eq!(
        shape("sed -n '8,16p' main.py").key,
        shape("sed -n '20,30p' other.py").key
    );
    assert_ne!(
        shape("sed -n '8p' main.py").key,
        shape("sed -n '8,16p' main.py").key
    );
    assert_eq!(
        shape("git status --short && rg -n foo src").key,
        shape("git status --short && rg -n bar lib").key
    );
    assert_ne!(
        shape("git status --short && rg -n foo src").key,
        shape("git status --short || rg -n foo src").key
    );
    assert_eq!(
        shape("ssh -o BatchMode=yes host-a").key,
        shape("ssh -o BatchMode=yes host-b").key
    );
    assert_ne!(
        shape("ssh -o BatchMode=yes host").key,
        shape("ssh -o BatchMode=no host").key
    );
    for command in [
        "rg -n $PATTERN src",
        "rg -n x *.rs",
        "rg -n x [ab]",
        "rg -n x src > result",
        "rg --unknown=value x src",
        "python3 - <<'PY'\nprint(1)\nPY",
        "git status;$(whoami)",
    ] {
        assert!(
            shape(command).label.starts_with("exact command"),
            "{command}"
        );
    }
    assert_ne!(
        shape("python3 - <<'PY'\nprint(1)\nPY").key,
        shape("python3 - <<'PY'\nprint(2)\nPY").key
    );
    assert_eq!(shape("rg -n 'a|b' src").label, "rg -n <pattern> <path>");
    let code = pattern(
        "functions.exec",
        "text(await tools.exec_command({cmd:'rg -n foo src'}))",
    )
    .unwrap();
    assert!(code.label.starts_with("inline code"));
    assert!(!code.key.contains("exec_command"));
    assert!(pattern("write_stdin", r#"{"chars":"pwd"}"#).is_none());
}
#[test]
fn evidence_can_arrive_before_start_and_replay_is_not_an_occurrence() {
    let f = facts("one", "rg -n first src", 0);
    let mut a = Library::default();
    a.observe("s", f.iter().rev());
    a.observe("s", f.iter());
    assert_eq!(a.current.len(), 1);
    assert_eq!(a.pending.len(), 1);
    let first = a.current.values().next().unwrap().clone();
    a.observe("s", facts("one", "rg -l later src", 10).iter());
    assert_eq!(a.current.values().next().unwrap(), &first);
    a.observe("s", facts("two", "rg -n second tests", 20).iter());
    a.view = Some(View::default());
    a.refresh_view();
    assert_eq!(a.selected().unwrap().here, 2);
}
#[test]
fn counts_keep_occurrences_and_session_spread_distinct_and_selection_stable() {
    let mut a = Library::default();
    a.observe("current", facts("a", "rg -n foo src", 0).iter());
    let p = shape("rg -n bar tests");
    a.cached.push(Aggregate {
        key: p.key.clone(),
        label: p.label,
        example: p.command,
        tool: p.tool,
        sessions: BTreeMap::from([("old".into(), 9), ("current".into(), 1)]),
        ..Default::default()
    });
    a.view = Some(View::default());
    a.refresh_view();
    let r = a.selected().unwrap();
    assert_eq!((r.here, r.total, r.spread), (1, 10, 2));
    a.observe("current", facts("z", "git status --short", 1).iter());
    a.refresh_view();
    assert_eq!(a.selected().unwrap().key, p.key);
    a.choose(p.key.clone(), Learning::Learned);
    a.refresh_view();
    assert!(a.view.as_ref().unwrap().rows.iter().all(|r| r.key != p.key));
    assert_eq!(a.current.len(), 2);
    a.view.as_mut().unwrap().show_learned = true;
    a.refresh_view();
    assert!(a.view.as_ref().unwrap().rows.iter().any(|r| r.key == p.key));
}
#[test]
fn library_counts_whole_recording_but_inspector_respects_time_and_jump() {
    let mut a = App::new("s".into(), Mode::Replay);
    let mut items = Vec::new();
    for f in facts("a", "rg -n foo src", 0)
        .into_iter()
        .chain(facts("b", "rg -n bar tests", 20))
    {
        items.push(ReplayItem::at(f.ts, vec![f]));
    }
    a.handle_ui_event(UiEvent::ReplayLoaded {
        session_id: "s".into(),
        items,
        speed: 8.,
        info: Default::default(),
    });
    a.open_library();
    assert_eq!(a.library.selected().unwrap().here, 2);
    assert_eq!(a.session.tool_count(), 1);
    a.library.view.as_mut().unwrap().example = 1;
    a.open_pattern_occurrence();
    assert_eq!(a.inspected_call().unwrap().id, "b");
    assert!(a.is_paused);
    a.step_event(false);
    a.step_event(false);
    assert!(a.inspected_call().is_none());
    assert_eq!(a.library.current.len(), 2);
}
#[test]
fn library_search_is_modal_and_learning_is_explicit() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    let mut a = App::new("s".into(), Mode::Live);
    a.library
        .observe("s", facts("a", "rg -n foo src", 0).iter());
    a.open_library();
    let key = |c| Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    for c in ['/', 'q', 'p', 'L'] {
        assert!(!crate::handler::handle_event(&key(c), &mut a));
    }
    assert!(a.library.states.is_empty());
    assert_eq!(a.library.view.as_ref().unwrap().query, "qpL");
    a.library.view.as_mut().unwrap().searching = false;
    a.library.view.as_mut().unwrap().query.clear();
    a.library.refresh_view();
    crate::handler::handle_event(&key('p'), &mut a);
    assert_eq!(a.library.state_for_call("main", "a"), Learning::Practising);
    assert_eq!(a.library.choices.len(), 1);
}
#[test]
fn laptop_pattern_browser_renders_counts_and_practising() {
    let mut a = App::new("s".into(), Mode::Live);
    a.library
        .observe("s", facts("a", "rg -n foo src", 0).iter());
    a.open_library();
    a.library.choose(
        a.library.selected().unwrap().key.clone(),
        Learning::Practising,
    );
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
    terminal.draw(|f| crate::ui::draw(f, &mut a)).unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    for value in [
        "patterns",
        "Frequency",
        "Practising",
        "rg -n foo src",
        "1 here",
        "1 all",
    ] {
        assert!(text.contains(value), "missing {value}");
    }
}

#[test]
fn incomplete_input_does_not_make_pattern_depend_on_arrival_order() {
    let mut f = facts("a", "rg -n foo src", 5);
    f.push(Fact {
        agent: Some("main".into()),
        ts: chrono::DateTime::from_timestamp(1_800_000_000, 0),
        kind: FactKind::ToolEvidence {
            call: "a".into(),
            output: false,
            text: "{}".into(),
        },
    });
    let mut forward = Library::default();
    let mut reverse = Library::default();
    for fact in &f {
        forward.observe("s", std::iter::once(fact));
    }
    for fact in f.iter().rev() {
        reverse.observe("s", std::iter::once(fact));
    }
    assert_eq!(forward.current, reverse.current);
    assert_eq!(forward.current.len(), 1);
}
#[test]
fn practising_timeline_marks_do_not_reveal_future_input() {
    let mut a = App::new("s".into(), Mode::Replay);
    let mut all = facts("a", "rg -n foo src", 0);
    all.extend(facts("b", "git status --short", 20));
    all.push(Fact {
        agent: Some("main".into()),
        ts: chrono::DateTime::from_timestamp(1_800_000_025, 0),
        kind: FactKind::Activity,
    });
    let items = all
        .into_iter()
        .map(|f| ReplayItem::at(f.ts, vec![f]))
        .collect();
    a.handle_ui_event(UiEvent::ReplayLoaded {
        session_id: "s".into(),
        items,
        speed: 8.,
        info: Default::default(),
    });
    a.library
        .choose(shape("git status --short").key, Learning::Practising);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
    terminal.draw(|f| crate::ui::draw(f, &mut a)).unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(!text.contains("◎"));
    a.go_live();
    terminal.draw(|f| crate::ui::draw(f, &mut a)).unwrap();
    let text = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    assert!(text.contains("◎"));
}

#[cfg(feature = "native")]
mod persistence {
    use super::super::storage::Store;
    use super::*;
    struct Scratch(std::path::PathBuf);
    impl Scratch {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            Self(std::env::temp_dir().join(format!(
                    "linger-tests-{}-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos(),
                    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                )))
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn occurrence(session: &str, call: &str, command: &str) -> Occurrence {
        Occurrence {
            session: session.into(),
            agent: "main".into(),
            call: call.into(),
            order: format!("0:{command}"),
            pattern: shape(command),
        }
    }
    #[test]
    fn reopen_dedup_two_sessions_and_learning_survives_cache_rebuild() {
        let dir = Scratch::new();
        let mut store = Store::open(&dir.0).unwrap();
        let rows = vec![
            occurrence("s1", "a", "rg -n first src"),
            occurrence("s1", "b", "rg -n second src"),
            occurrence("s2", "a", "rg -n third tests"),
        ];
        store.index(&rows).unwrap();
        store.index(&rows).unwrap();
        store
            .set(&rows[0].pattern.key, Learning::Practising)
            .unwrap();
        drop(store);
        let store = Store::open(&dir.0).unwrap();
        let stats = store.aggregates().unwrap();
        assert_eq!(stats.len(), 2); // usage combination and program projection
        for row in &stats {
            assert_eq!(row.sessions.values().sum::<usize>(), 3);
            assert_eq!(row.sessions.len(), 2);
        }
        assert_eq!(
            store.states().unwrap()[&rows[0].pattern.key],
            Learning::Practising
        );
        drop(store);
        std::fs::remove_file(dir.0.join("patterns.sqlite3")).unwrap();
        let store = Store::open(&dir.0).unwrap();
        assert!(store.aggregates().unwrap().is_empty());
        assert_eq!(
            store.states().unwrap()[&rows[0].pattern.key],
            Learning::Practising
        );
    }
    #[test]
    fn cached_wrapper_projections_rebuild_from_existing_occurrences() {
        let dir = Scratch::new();
        let mut store = Store::open(&dir.0).unwrap();
        for session in ["s1", "s2"] {
            let mut library = Library::default();
            library.observe(
                session,
                argv_facts(
                    "a",
                    &["/bin/zsh", "-lc", "rg -n first src && rg -n second tests"],
                    0,
                )
                .iter(),
            );
            library.observe(
                session,
                argv_facts("b", &["/bin/zsh", "-lc", "python3 -c 'print(42)'"], 1).iter(),
            );
            let rows: Vec<_> = library.current.values().cloned().collect();
            store.index(&rows).unwrap();
            store.index(&rows).unwrap();
        }
        drop(store);
        let store = Store::open(&dir.0).unwrap();
        let rows = store.aggregates().unwrap();
        let wrapper = rows
            .iter()
            .find(|r| r.level == Level::Wrappers && r.label == "/bin/zsh -lc")
            .unwrap();
        assert_eq!(wrapper.sessions.values().sum::<usize>(), 4);
        assert_eq!(wrapper.sessions.len(), 2);
        let rg = rows
            .iter()
            .find(|r| r.label == "rg -n <pattern> <path>")
            .unwrap();
        assert_eq!(rg.sessions.values().sum::<usize>(), 2);
        assert!(
            rows.iter()
                .any(|r| r.inline && r.example.contains("print(42)"))
        );
    }
    #[test]
    fn independent_connections_do_not_overwrite_other_learning_choices() {
        let dir = Scratch::new();
        let a = Store::open(&dir.0).unwrap();
        let b = Store::open(&dir.0).unwrap();
        a.set("pattern-a", Learning::Practising).unwrap();
        b.set("pattern-b", Learning::Learned).unwrap();
        a.set("pattern-a", Learning::Want).unwrap();
        let states = b.states().unwrap();
        assert_eq!(states.len(), 2);
        assert_eq!(states["pattern-a"], Learning::Want);
        assert_eq!(states["pattern-b"], Learning::Learned);
    }
    #[test]
    fn corrupt_learning_database_is_retained() {
        let dir = Scratch::new();
        std::fs::create_dir_all(&dir.0).unwrap();
        let path = dir.0.join("learning.sqlite3");
        std::fs::write(&path, "fictional damaged bytes").unwrap();
        assert!(Store::open(&dir.0).is_err());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "fictional damaged bytes"
        );
    }
}

fn argv_facts(call: &str, argv: &[&str], time: i64) -> Vec<Fact> {
    let mut records = facts(call, "unused", time);
    for f in &mut records {
        if let FactKind::ToolEvidence {
            output: false,
            text,
            ..
        } = &mut f.kind
        {
            *text = serde_json::json!({"command": argv}).to_string().into();
        }
    }
    records
}

#[test]
fn structural_levels_count_wrappers_and_inner_commands_once_per_call() {
    let mut library = Library::default();
    library.observe(
        "one",
        argv_facts(
            "a",
            &["/bin/zsh", "-lc", "rg -n foo src && rg -n bar tests"],
            0,
        )
        .iter(),
    );
    library.observe(
        "one",
        argv_facts("b", &["/bin/zsh", "-lc", "git status --short"], 1).iter(),
    );
    library.observe("one", facts("c", "rg -n baz lib", 2).iter());
    library.view = Some(View::default());
    library.refresh_view();
    let rows = &library.view.as_ref().unwrap().rows;
    let rg = rows
        .iter()
        .find(|r| r.label == "rg -n <pattern> <path>")
        .unwrap();
    assert_eq!(rg.here, 2); // two rg segments in call a still count as one call
    assert_eq!(library.examples(&rg.key).len(), 2);
    assert!(
        rows.iter()
            .any(|r| r.label == "/bin/zsh -lc → git status --short")
    );
    library.view.as_mut().unwrap().level = Level::Programs;
    library.refresh_view();
    let zsh = library
        .view
        .as_ref()
        .unwrap()
        .rows
        .iter()
        .find(|r| r.label == "/bin/zsh")
        .unwrap();
    assert_eq!(zsh.here, 2);
    let key = zsh.key.clone();
    library.choose(key, Learning::Practising);
    assert_eq!(library.state_for_call("main", "a"), Learning::Practising);
    library.view.as_mut().unwrap().level = Level::Wrappers;
    library.refresh_view();
    let rows = &library.view.as_ref().unwrap().rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].label, "/bin/zsh -lc");
    assert_eq!(rows[0].here, 2);
}

#[test]
fn inline_scripts_are_hidden_but_search_and_toggle_recover_originals() {
    let mut library = Library::default();
    library.observe(
        "one",
        facts("a", "python3 - <<'PY'\nprint('unique-script')\nPY", 0).iter(),
    );
    library.observe(
        "one",
        argv_facts("b", &["/bin/zsh", "-lc", "python3 -c 'print(42)'"], 1).iter(),
    );
    library.view = Some(View::default());
    library.refresh_view();
    assert!(library.view.as_ref().unwrap().rows.is_empty());
    library.view.as_mut().unwrap().query = "unique-script".into();
    library.refresh_view();
    assert_eq!(library.view.as_ref().unwrap().rows.len(), 1);
    assert!(
        library
            .selected()
            .unwrap()
            .example
            .contains("unique-script")
    );
    library.view.as_mut().unwrap().query.clear();
    library.view.as_mut().unwrap().show_scripts = true;
    library.refresh_view();
    assert_eq!(library.view.as_ref().unwrap().rows.len(), 2);
    library.view.as_mut().unwrap().show_scripts = false;
    library.view.as_mut().unwrap().level = Level::Wrappers;
    library.refresh_view();
    assert_eq!(library.selected().unwrap().label, "/bin/zsh -lc");
    assert_eq!(library.current.len(), 2);
}
