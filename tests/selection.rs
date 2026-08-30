use soksak_kit_sidecar_terminal::mirror::{
    CellSide, EngineSelectionPoint, SelectionKind, SelectionModifiers, TerminalEngine,
};
use soksak_sidecar_terminal_wezterm::engine::Engine;

#[test]
fn simple_drag_uses_wezterm_lines_for_text_and_range() {
    let marker = "SELECT_WEZTERM_1234567890";
    let mut engine = Engine::new(40, 3);
    engine.feed(marker.as_bytes());
    TerminalEngine::selection_begin(
        &mut engine,
        SelectionKind::Simple,
        EngineSelectionPoint {
            line: 0,
            col: 0,
            side: CellSide::Left,
        },
        SelectionModifiers::default(),
    )
    .expect("begin selection");
    TerminalEngine::selection_update(
        &mut engine,
        EngineSelectionPoint {
            line: 0,
            col: u16::try_from(marker.len()).unwrap(),
            side: CellSide::Left,
        },
        SelectionModifiers::default(),
    )
    .expect("update selection");
    assert_eq!(
        TerminalEngine::selection_text(&engine).as_deref(),
        Some(marker)
    );
    assert_eq!(
        TerminalEngine::selection_range(&engine, 0),
        Some((0, u16::try_from(marker.len() - 1).unwrap())),
    );
}
