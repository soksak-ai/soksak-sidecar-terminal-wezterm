use crate::engine::Engine;
use soksak_kit_sidecar_terminal::mirror::{
    EnginePointerInput, EngineWheelInput, MirrorCapabilities, RecoveryMirror, SelectionRequest,
    SelectionSnapshot, TerminalCell, TerminalCursorAnimation, TerminalCursorStyle, TerminalFrame,
    TerminalModes, TerminalThemeOverrides,
};

pub struct Mirror(RecoveryMirror<Engine>);

impl Mirror {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self(RecoveryMirror::new(cols, rows))
    }
    pub fn feed(&mut self, bytes: &[u8]) {
        self.0.feed(bytes);
    }
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.0.resize(cols, rows);
    }
    pub fn rehydrate(&self) -> Vec<u8> {
        self.0.rehydrate()
    }
    pub fn cold_paint(&self) -> Vec<u8> {
        self.0.cold_paint()
    }
    pub fn alt_active(&self) -> bool {
        self.0.alt_active()
    }
    pub fn suppressed_replies(&self) -> u64 {
        self.0.suppressed_replies()
    }
    pub fn cols(&self) -> u16 {
        self.0.cols()
    }
    pub fn rows(&self) -> u16 {
        self.0.rows()
    }
    pub fn cursor(&self) -> (usize, usize) {
        self.0.cursor()
    }
    pub fn cursor_style(&self) -> TerminalCursorStyle {
        self.0.cursor_style()
    }
    pub fn cursor_animation(&self) -> TerminalCursorAnimation {
        self.0.cursor_animation()
    }
    pub fn modes(&self) -> TerminalModes {
        self.0.modes()
    }
    pub fn history_size(&self) -> usize {
        self.0.history_size()
    }
    pub fn line_cells(&self, line: i32) -> Vec<TerminalCell> {
        self.0.line_cells(line)
    }
    pub fn capabilities(&self) -> MirrorCapabilities {
        self.0.capabilities()
    }
    pub fn frame_at(&self, offset: usize) -> TerminalFrame {
        self.0.frame_at(offset)
    }
    pub fn frame(&self) -> TerminalFrame {
        self.0.frame()
    }
    pub fn theme_overrides(&self) -> TerminalThemeOverrides {
        self.0.theme_overrides()
    }
    pub fn selection_command(
        &mut self,
        request: &SelectionRequest,
        offset: usize,
    ) -> Result<SelectionSnapshot, String> {
        self.0.selection_command(request, offset)
    }
    pub fn selection_range(&self, line: i32) -> Option<(u16, u16)> {
        self.0.selection_range(line)
    }
    pub fn wheel_input(&mut self, input: EngineWheelInput) -> Result<Vec<u8>, String> {
        self.0.wheel_input(input)
    }
    pub fn pointer_input(&mut self, input: EnginePointerInput) -> Result<Vec<u8>, String> {
        self.0.pointer_input(input)
    }
}
