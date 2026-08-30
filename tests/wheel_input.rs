use soksak_kit_sidecar_terminal::mirror::{
    EngineWheelInput, EngineWheelRoute, SelectionModifiers, TerminalEngine,
};
use soksak_sidecar_terminal_wezterm::engine::Engine;

fn wheel(horizontal: i32, vertical: i32, route: EngineWheelRoute) -> EngineWheelInput {
    EngineWheelInput {
        row: 2,
        col: 1,
        horizontal,
        vertical,
        modifiers: SelectionModifiers::default(),
        route,
    }
}

#[test]
fn native_sgr_wheel_preserves_axes_directions_repetition_and_modifiers() {
    let mut engine = Engine::new(120, 40);
    engine.feed(b"\x1b[?1000h\x1b[?1006h");

    let mut negative = wheel(-2, -2, EngineWheelRoute::MouseReport);
    negative.modifiers = SelectionModifiers {
        shift: true,
        alt: false,
        control: true,
        meta: true,
    };
    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, negative).unwrap(),
        b"\x1b[<92;2;3M\x1b[<92;2;3M\x1b[<94;2;3M\x1b[<94;2;3M",
    );
    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, wheel(1, 1, EngineWheelRoute::MouseReport),)
            .unwrap(),
        b"\x1b[<65;2;3M\x1b[<67;2;3M",
    );
}

#[test]
fn native_legacy_wheel_uses_the_live_x10_and_utf8_encodings() {
    let mut engine = Engine::new(240, 120);
    engine.feed(b"\x1b[?1000h");

    let mut legacy = wheel(0, -1, EngineWheelRoute::MouseReport);
    legacy.modifiers = SelectionModifiers {
        shift: true,
        alt: true,
        control: true,
        meta: false,
    };
    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, legacy).unwrap(),
        [0x1b, b'[', b'M', 124, 34, 35],
    );

    engine.feed(b"\x1b[?1005h");
    let mut utf8 = wheel(0, -1, EngineWheelRoute::MouseReport);
    utf8.col = 100;
    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, utf8).unwrap(),
        [0x1b, b'[', b'M', 96, 0xc2, 0x85, 35],
    );
}

#[test]
fn native_alternate_scroll_uses_live_cursor_encoding_on_both_axes() {
    let mut engine = Engine::new(80, 24);
    engine.feed(b"\x1b[?1049h\x1b[?1007h");

    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, wheel(1, -2, EngineWheelRoute::AlternateScroll),)
            .unwrap(),
        b"\x1b[A\x1b[A\x1b[C",
    );

    engine.feed(b"\x1b[?1h");
    assert_eq!(
        TerminalEngine::wheel_input(&mut engine, wheel(-1, 1, EngineWheelRoute::AlternateScroll),)
            .unwrap(),
        b"\x1bOB\x1bOD",
    );
}

#[test]
fn native_wheel_refuses_routes_selected_from_stale_modes() {
    let mut engine = Engine::new(80, 24);
    engine.feed(b"\x1b[?1000h\x1b[?1000l");
    let mouse_error =
        TerminalEngine::wheel_input(&mut engine, wheel(0, -1, EngineWheelRoute::MouseReport))
            .unwrap_err();
    assert!(
        mouse_error.starts_with("WHEEL_MODE_CHANGED:"),
        "{mouse_error}",
    );

    engine.feed(b"\x1b[?1049h\x1b[?1007h\x1b[?1007l");
    let alternate_error =
        TerminalEngine::wheel_input(&mut engine, wheel(0, -1, EngineWheelRoute::AlternateScroll))
            .unwrap_err();
    assert!(
        alternate_error.starts_with("WHEEL_MODE_CHANGED:"),
        "{alternate_error}",
    );

    engine.feed(b"\x1b[?1007h\x1b[?1000h");
    let superseded_error =
        TerminalEngine::wheel_input(&mut engine, wheel(0, -1, EngineWheelRoute::AlternateScroll))
            .unwrap_err();
    assert!(
        superseded_error.starts_with("WHEEL_MODE_CHANGED:"),
        "{superseded_error}",
    );
}
