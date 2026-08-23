use crate::engine::Engine;
use soksak_kit_sidecar_terminal::mirror::{RecoveryMirror, TerminalCell, TerminalModes};

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
    pub fn modes(&self) -> TerminalModes {
        self.0.modes()
    }
    pub fn history_size(&self) -> usize {
        self.0.history_size()
    }
    pub fn line_cells(&self, line: i32) -> Vec<TerminalCell> {
        self.0.line_cells(line)
    }
    pub fn frame(&self) -> soksak_kit_sidecar_terminal::mirror::TerminalFrame {
        self.0.frame()
    }
}
