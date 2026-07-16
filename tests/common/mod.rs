// 계약 합격시험 — 픽스처도 골든도 여기 없다. 정본은 soksak-contract-terminal 이고, 이 파일은
// 이 유닛의 미러를 그 시험대에 세우는 좌석이다.
//
// 좌석이 지는 일은 하나뿐이다: 이 엔진의 화면 표현을 계약의 **정규형**(ScreenState, SPEC.md §11)
// 으로 옮기는 것. 무엇을 같다고 볼지(팔레트 색·wide 스페이서·행 꼬리 공백)는 계약이 판정했고,
// 여기서는 그 판정을 따를 뿐이다.

use soksak_contract_terminal as contract;
use soksak_contract_terminal::{Fixture, MirrorUnderTest};
use soksak_sidecar_terminal_wezterm::engine::{ColorSnap, GridCell, ModeSnap};
use soksak_sidecar_terminal_wezterm::Mirror;

pub struct Unit(Mirror);

impl MirrorUnderTest for Unit {
    fn new(cols: u16, rows: u16) -> Self {
        Unit(Mirror::new(cols, rows))
    }

    fn feed(&mut self, bytes: &[u8]) {
        self.0.feed(bytes)
    }

    fn resize(&mut self, cols: u16, rows: u16) {
        self.0.resize(cols, rows)
    }

    fn rehydrate(&self) -> Vec<u8> {
        self.0.rehydrate()
    }

    fn cold_paint(&self) -> Vec<u8> {
        self.0.cold_paint()
    }

    fn suppressed_replies(&self) -> u64 {
        self.0.suppressed_replies()
    }

    fn screen_state(&self) -> contract::ScreenState {
        let (row, col) = self.0.cursor();
        let hist = self.0.history_size() as i32;
        contract::ScreenState {
            cols: self.0.cols(),
            rows: self.0.rows(),
            alt: self.0.alt_active(),
            cursor: (col as u16, row as u16),
            modes: modes_of(self.0.modes()),
            history: (-hist..0).map(|l| row_of(self.0.line_cells(l))).collect(),
            visible: (0..self.0.rows() as i32).map(|l| row_of(self.0.line_cells(l))).collect(),
        }
    }
}

// 엔진 행 → 정규형 행. 스페이서는 담지 않고(wide 본체가 두 칸을 먹는다는 사실에서 유도된다),
// 꼬리의 빈 기본 칸은 잘린다 — 둘 다 계약의 정규화 규칙이다.
fn row_of(cells: Vec<GridCell>) -> contract::Row {
    let mut out = Vec::with_capacity(cells.len());
    for c in cells {
        if c.spacer {
            continue;
        }
        let mut text = String::new();
        text.push(c.ch);
        for z in &c.zerowidth {
            text.push(*z);
        }
        out.push(contract::Cell {
            text,
            fg: color_of(c.fg),
            bg: color_of(c.bg),
            attrs: contract::Attrs {
                bold: c.bold,
                dim: c.dim,
                italic: c.italic,
                underline: c.underline,
                inverse: c.inverse,
                strikeout: c.strikeout,
                hidden: c.hidden,
            },
            wide: c.wide,
        });
    }
    contract::Row::normalized(out)
}

// 팔레트 색은 인덱스 그대로 — 0..16 을 따로 접지 않는다(계약의 정규화 규칙).
fn color_of(c: ColorSnap) -> contract::Color {
    match c {
        ColorSnap::Default => contract::Color::Default,
        ColorSnap::Named(i) | ColorSnap::Indexed(i) => contract::Color::Palette(i),
        ColorSnap::Rgb(r, g, b) => contract::Color::Rgb(r, g, b),
    }
}

fn modes_of(m: ModeSnap) -> contract::Modes {
    contract::Modes {
        bracketed_paste: m.bracketed_paste,
        app_cursor: m.app_cursor,
        app_keypad: m.app_keypad,
        mouse_click: m.mouse_click,
        mouse_drag: m.mouse_drag,
        mouse_motion: m.mouse_motion,
        sgr_mouse: m.sgr_mouse,
        utf8_mouse: m.utf8_mouse,
        focus_in_out: m.focus_in_out,
        alternate_scroll: m.alternate_scroll,
        show_cursor: m.show_cursor,
        line_wrap: m.line_wrap,
        insert: m.insert,
    }
}
