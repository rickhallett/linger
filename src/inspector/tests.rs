use crate::{
    fact::{Fact, FactKind, Outcome},
    inspector::{Tab, safe_text},
    state::{App, Mode},
    tailer::{ReplayItem, UiEvent},
};

fn fact(time: i64, kind: FactKind) -> Fact {
    Fact {
        agent: Some("main".into()),
        ts: chrono::DateTime::from_timestamp(1_800_000_000 + time, 0),
        kind,
    }
}
fn records() -> Vec<Fact> {
    vec![
        fact(
            0,
            FactKind::ToolStart {
                call: "a".into(),
                name: "Bash".into(),
                summary: Some("rg -n TODO src".into()),
            },
        ),
        fact(
            0,
            FactKind::ToolEvidence {
                call: "a".into(),
                output: false,
                text: r#"{"command":"rg -n TODO src"}"#.into(),
            },
        ),
        fact(
            5,
            FactKind::ToolEnd {
                call: "a".into(),
                outcome: Outcome::Ok,
            },
        ),
        fact(
            5,
            FactKind::ToolEvidence {
                call: "a".into(),
                output: true,
                text: "src/main.rs:12:TODO\nsecret-future-line".into(),
            },
        ),
    ]
}
fn app() -> App {
    let mut app = App::new("test".into(), Mode::Replay);
    let items = records()
        .into_iter()
        .map(|f| ReplayItem::at(f.ts, vec![f]))
        .collect();
    app.handle_ui_event(UiEvent::ReplayLoaded {
        session_id: "test".into(),
        items,
        speed: 8.,
        info: Default::default(),
    });
    app.go_live();
    app.open_inspector();
    app
}
#[test]
fn replay_never_leaks_future_output_and_invalidates_interpretation() {
    let mut a = app();
    let r = a.interpretation_request().unwrap();
    a.interpretations
        .insert(r.key.clone(), "future interpretation".into());
    a.inspector.as_mut().unwrap().tab = Tab::Interpret;
    a.refresh_inspector();
    assert!(
        a.inspector
            .as_ref()
            .unwrap()
            .lines
            .join("\n")
            .contains("future interpretation")
    );
    a.step_event(false); // before output evidence, after completion
    assert!(a.inspector_evidence().1.is_empty());
    assert_ne!(a.interpretation_request().unwrap().key, r.key);
    a.refresh_inspector();
    assert!(
        !a.inspector
            .as_ref()
            .unwrap()
            .lines
            .join("\n")
            .contains("future interpretation")
    );
    a.inspector.as_mut().unwrap().tab = Tab::Output;
    a.refresh_inspector();
    assert!(
        a.inspector
            .as_ref()
            .unwrap()
            .lines
            .join("\n")
            .contains("no result body")
    );
    a.go_live();
    assert!(a.inspector_evidence().1.contains("secret-future-line"));
}
#[test]
fn evidence_is_idempotent_and_arrival_order_independent() {
    let mut a = crate::state::session::SessionModel::new("s".into());
    let mut b = a.clone();
    for f in records() {
        a.apply_fact(&f);
        a.apply_fact(&f);
    }
    for f in records().into_iter().rev() {
        b.apply_fact(&f);
    }
    assert_eq!(a.evidence, b.evidence);
    assert_eq!(a.tool_evidence("main", "a", true).len(), 1);
}
#[test]
fn appended_calls_do_not_steal_selection_or_scroll() {
    let mut a = app();
    a.inspector.as_mut().unwrap().scroll = 1;
    a.handle_ui_event(UiEvent::Batch {
        session_id: "test".into(),
        statements: vec![crate::fact::Statement {
            at: chrono::DateTime::from_timestamp(1_800_000_010, 0),
            facts: vec![fact(
                10,
                FactKind::ToolStart {
                    call: "b".into(),
                    name: "Bash".into(),
                    summary: None,
                },
            )],
        }],
    });
    assert_eq!(a.inspector.as_ref().unwrap().call.as_deref(), Some("a"));
    assert_eq!(a.inspector.as_ref().unwrap().scroll, 1);
}
#[test]
fn search_consumes_quit_and_transport_characters() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    let mut a = app();
    a.refresh_inspector();
    for ch in ['/', 'q', '[', 'g'] {
        assert!(!crate::handler::handle_event(
            &Event::Key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE)),
            &mut a
        ));
    }
    assert_eq!(a.inspector.as_ref().unwrap().query, "q[g");
    assert!(!a.should_quit);
}
#[test]
fn literal_controls_cannot_become_terminal_actions() {
    let text = safe_text("hello\x1b]52;c;payload\x07\n你好\tend");
    assert!(!text.contains('\x1b'));
    assert!(!text.contains('\x07'));
    assert!(text.contains("你好    end"));
}
#[test]
fn codex_keeps_structured_output_and_inline_program() {
    let mut s = crate::provider::codex::Stream::new();
    s.push(r#"{"type":"session_meta","payload":{"id":"a","session_id":"a","source":"cli","thread_source":"user"}}"#);
    let input = "print('hello')\nprint(42)";
    let call = serde_json::json!({"type":"response_item","payload":{"type":"custom_tool_call","call_id":"c","name":"exec","input":input}});
    let output = serde_json::json!({"type":"response_item","payload":{"type":"custom_tool_call_output","call_id":"c","output":[{"type":"text","text":"hello\n42"},{"type":"image","image_url":"fixture-only"}]}});
    let mut model = crate::state::session::SessionModel::new("a".into());
    for line in [call, output] {
        for f in s.push(&line.to_string()).unwrap().facts {
            model.apply_fact(&f);
        }
    }
    assert_eq!(model.tool_evidence("main", "c", false), vec![input]);
    assert!(model.tool_evidence("main", "c", true)[0].contains("fixture-only"));
}
#[test]
fn claude_keeps_result_content() {
    use crate::provider::claude::{Record, Source, wire::parse_line};
    let text = r#"{"type":"user","uuid":"r","timestamp":"2026-09-13T10:00:01Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"a","content":"exact\noutput","is_error":false}]}}"#;
    let st = Record::Entry {
        source: Source::Main,
        entry: parse_line(text).unwrap(),
    }
    .statement()
    .unwrap();
    assert!(st.facts.iter().any(|f|matches!(&f.kind,FactKind::ToolEvidence {call,output:true,text} if call=="a" && &**text=="exact\noutput")));
}
#[test]
fn laptop_render_exposes_evidence_and_navigation() {
    let mut a = app();
    let backend = ratatui::backend::TestBackend::new(120, 40);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(|f| crate::ui::draw(f, &mut a)).unwrap();
    let buffer = terminal.backend().buffer();
    let text = (0..40)
        .map(|y| {
            (0..120)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("secret-future-line"));
    assert!(text.contains("linger / main"));
    assert!(text.contains("1 Input"));
    assert!(text.contains("i Mercury"));
}
#[test]
fn reference_does_not_treat_javascript_as_shell() {
    let text = crate::inspector::reference_notes(
        "exec",
        r#"await tools.exec_command({cmd:'rg -n TODO src'})"#,
    );
    assert!(text.contains("No deterministic guide"));
    assert!(!text.contains("var1 is the pattern"));
}

#[test]
fn renamed_demo_opens_through_the_real_provider_boundary() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/repair.jsonl");
    let session = crate::provider::open(&crate::provider::Target::Path(path), None).unwrap();
    assert_eq!(session.id, "linger-demo");
    assert_eq!(session.provider, crate::provider::Provider::Codex);
}

#[test]
fn reference_reflows_on_resize_and_cached_frames_reuse_lines() {
    let mut a = app();
    {
        let i = a.inspector.as_mut().unwrap();
        i.tab = Tab::Explain;
        i.content_width = 35;
    }
    a.refresh_inspector();
    let narrow = a.inspector.as_ref().unwrap().lines.len();
    let ptr = a.inspector.as_ref().unwrap().lines.as_ptr();
    a.refresh_inspector();
    assert_eq!(ptr, a.inspector.as_ref().unwrap().lines.as_ptr());
    a.inspector.as_mut().unwrap().content_width = 90;
    a.refresh_inspector();
    assert!(a.inspector.as_ref().unwrap().lines.len() < narrow);
}

#[test]
fn command_selection_survives_output_but_never_reveals_future_input() {
    use crate::exploration::Explanation;
    let mut a = app();
    a.inspector.as_mut().unwrap().tab = Tab::Explain;
    a.refresh_inspector();
    let command = a.explorer.pending.take().unwrap();
    let explanation = serde_json::from_str::<Explanation>(r#"{"spans":[{"start":0,"end":2,"text":"Search files","source":"fictional documentation","extractor":"fixture","kind":"command","known":true},{"start":3,"end":5,"text":"Include line numbers","source":"fictional documentation","extractor":"fixture","kind":"option","known":true}]}"#).unwrap().validate(&command);
    a.explorer.cache.insert(command.clone(), explanation);
    a.explorer.revision += 1;
    a.refresh_inspector();
    a.move_command_part(1);
    a.refresh_inspector();
    assert!(
        a.inspector
            .as_ref()
            .unwrap()
            .lines
            .join("\n")
            .contains("Include line numbers")
    );
    a.handle_ui_event(UiEvent::Batch {
        session_id: "test".into(),
        statements: vec![crate::fact::Statement {
            at: chrono::DateTime::from_timestamp(1_800_000_010, 0),
            facts: vec![fact(
                10,
                FactKind::ToolEvidence {
                    call: "a".into(),
                    output: true,
                    text: "additional output".into(),
                },
            )],
        }],
    });
    a.refresh_inspector();
    assert_eq!(a.inspector.as_ref().unwrap().part, 1);
    assert!(a.explorer.pending.is_none());
    let key = a
        .library
        .current
        .get(&("main".into(), "a".into()))
        .unwrap()
        .pattern
        .key
        .clone();
    a.library.choose(key, crate::patterns::Learning::Practising);
    assert_eq!(
        a.visible_learning("main", "a"),
        crate::patterns::Learning::Practising
    );
    a.commit_inspector_seek(1); // tool start, before recorded input
    assert_eq!(
        a.visible_learning("main", "a"),
        crate::patterns::Learning::Unmarked
    );
    a.refresh_inspector();
    assert!(a.inspector.as_ref().unwrap().command.is_none());
    assert!(a.inspector.as_ref().unwrap().command_lines.is_empty());
    assert!(
        !a.inspector
            .as_ref()
            .unwrap()
            .lines
            .join("\n")
            .contains("Include line numbers")
    );
    a.go_live();
    a.refresh_inspector();
    assert_eq!(
        a.inspector.as_ref().unwrap().command.as_ref(),
        Some(&command)
    );
    assert_eq!(a.inspector.as_ref().unwrap().part, 0);
}

#[test]
fn explanation_keys_step_parts_without_changing_other_tab_navigation() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    let mut a = app();
    let key = |a: &mut App, c| {
        crate::handler::handle_event(
            &Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)),
            a,
        );
    };
    key(&mut a, '3');
    a.refresh_inspector();
    assert!(a.inspector.as_ref().unwrap().detail);
    let command = a.explorer.pending.take().unwrap();
    a.explorer.cache.insert(
        command.clone(),
        crate::exploration::Explanation::default().validate(&command),
    );
    a.explorer.revision += 1;
    a.refresh_inspector();
    key(&mut a, 'l');
    a.refresh_inspector();
    assert_eq!(a.inspector.as_ref().unwrap().part, 1);
    assert_eq!(a.inspector.as_ref().unwrap().horizontal, 0);
    key(&mut a, '2');
    key(&mut a, 'l');
    assert_eq!(a.inspector.as_ref().unwrap().horizontal, 0);
    key(&mut a, 'W');
    key(&mut a, 'l');
    assert_eq!(a.inspector.as_ref().unwrap().horizontal, 4);
    key(&mut a, 'W');
    assert_eq!(a.inspector.as_ref().unwrap().horizontal, 0);
    key(&mut a, '3');
    key(&mut a, '/');
    key(&mut a, 'h');
    assert_eq!(a.inspector.as_ref().unwrap().query, "h");
    assert_eq!(a.inspector.as_ref().unwrap().part, 1);
}

#[test]
fn every_tab_wraps_without_losing_evidence_and_reflows_after_resize() {
    use unicode_width::UnicodeWidthStr;
    let mut a = app();
    for tab in [Tab::Input, Tab::Output, Tab::Explain, Tab::Interpret] {
        let i = a.inspector.as_mut().unwrap();
        i.tab = tab;
        i.content_width = 12;
        i.nowrap = true;
        a.refresh_inspector();
        let original = a.inspector.as_ref().unwrap().lines.concat();
        a.inspector.as_mut().unwrap().nowrap = false;
        a.refresh_inspector();
        let i = a.inspector.as_ref().unwrap();
        assert_eq!(i.lines.concat(), original);
        assert!(i.lines.iter().all(|line| line.width() <= 12));
        let narrow = i.lines.len();
        a.inspector.as_mut().unwrap().content_width = 80;
        a.refresh_inspector();
        assert!(a.inspector.as_ref().unwrap().lines.len() < narrow);
    }
    let code = "    print('界界界界界界界界界界')  # keep  spaces";
    let rows = super::wrap_evidence_line(code, 16);
    assert_eq!(rows.concat(), code);
    assert!(rows[0].starts_with("    "));
    assert!(rows.iter().all(|line| line.width() <= 16));
}
