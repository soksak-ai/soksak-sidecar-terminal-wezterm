use soksak_kit_sidecar_terminal::mirror::{
    EnginePointerInput, PointerButton, PointerPhase, SelectionModifiers, TerminalEngine,
};
use soksak_sidecar_terminal_wezterm::engine::Engine;

fn pointer(
    phase: PointerPhase,
    button: PointerButton,
    modifiers: SelectionModifiers,
) -> EnginePointerInput {
    EnginePointerInput {
        row: 2,
        col: 1,
        phase,
        button,
        click_count: if phase == PointerPhase::Move { 0 } else { 1 },
        modifiers,
    }
}

#[test]
fn dec9_and_dec1001_map_from_distinct_native_engine_state() {
    let mut engine = Engine::new(120, 40);

    engine.feed(b"\x1b[?9h");
    let x10 = engine.modes();
    assert!(x10.mouse_x10);
    assert!(!x10.mouse_click, "DEC9 must not alias DEC1000");
    assert!(!x10.mouse_highlight, "DEC9 must not alias DEC1001");
    assert!(x10.mouse_reporting());
    assert!(x10.reports_pointer(PointerPhase::Down, PointerButton::Left));
    assert!(!x10.reports_pointer(PointerPhase::Up, PointerButton::Left));

    engine.feed(b"\x1b[?9l\x1b[?1001h");
    let highlight = engine.modes();
    assert!(!highlight.mouse_x10, "DEC1001 must not alias DEC9");
    assert!(!highlight.mouse_click, "DEC1001 must not alias DEC1000");
    assert!(highlight.mouse_highlight);
    assert!(highlight.mouse_reporting());
    assert!(highlight.reports_pointer(PointerPhase::Down, PointerButton::Left));
    assert!(highlight.reports_pointer(PointerPhase::Up, PointerButton::Left));
    assert!(!highlight.reports_pointer(PointerPhase::Move, PointerButton::Left));

    engine.feed(b"\x1b[?1001l\x1b[?1000h");
    let normal = engine.modes();
    assert!(!normal.mouse_x10, "DEC1000 must not alias DEC9");
    assert!(normal.mouse_click);
    assert!(!normal.mouse_highlight, "DEC1000 must not alias DEC1001");
}

#[test]
fn native_pointer_encoder_obeys_kit_phase_admission() {
    let modifiers = SelectionModifiers {
        shift: true,
        alt: true,
        control: true,
        meta: true,
    };
    let mut x10 = Engine::new(120, 40);
    x10.feed(b"\x1b[?9h");
    assert_eq!(
        TerminalEngine::pointer_input(
            &mut x10,
            pointer(PointerPhase::Down, PointerButton::Left, modifiers),
        )
        .unwrap(),
        b"\x1b[M \"#",
    );
    assert_eq!(
        TerminalEngine::pointer_input(
            &mut x10,
            pointer(PointerPhase::Up, PointerButton::Left, modifiers),
        )
        .unwrap_err(),
        "POINTER_MODE_CHANGED: pointer phase is not reported by live modes",
    );

    let mut highlight = Engine::new(120, 40);
    highlight.feed(b"\x1b[?1001h");
    assert_eq!(
        TerminalEngine::pointer_input(
            &mut highlight,
            pointer(PointerPhase::Down, PointerButton::Left, modifiers),
        )
        .unwrap(),
        b"\x1b[M<\"#",
    );
    assert_eq!(
        TerminalEngine::pointer_input(
            &mut highlight,
            pointer(PointerPhase::Up, PointerButton::Left, modifiers),
        )
        .unwrap(),
        b"\x1b[M?\"#",
    );
    assert_eq!(
        TerminalEngine::pointer_input(
            &mut highlight,
            pointer(PointerPhase::Move, PointerButton::Left, modifiers),
        )
        .unwrap_err(),
        "POINTER_MODE_CHANGED: pointer phase is not reported by live modes",
    );
}
