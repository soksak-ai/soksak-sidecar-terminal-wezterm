//! 엔진 격리 좌석 — wezterm-term 을 만지는 유일한 모듈. 미러(복원 직렬화기)는 여기가
//! 내놓는 엔진-중립 뷰(스칼라 상태 + [`GridCell`] 행 읽기)만 쓴다. 판정자는 별도 엔진 위에
//! 채점은 계약이 선언한 골든이 한다 — 이 좌석이 만든 화면과, 그 화면이 만든 페인트가 되살린
//! 화면이 둘 다 같은 골든이어야 한다(자기-일관 오류도 골든이 바깥이라 숨지 못한다).
//!
//! 엔진-중립 타입(이 파일 위쪽의 [`ColorSnap`]·[`ModeSnap`]·[`GridCell`])은 직렬화기가
//! 그리드를 읽는 창이다. [`Engine`] 만 wezterm 이다.
//!
//! wezterm-term 이 흡수한 엔진 차이(엔진-중립 면의 시그니처는 계약이 고정한다):
//!   - 응답 포획: 엔진마다 다르다 — wezterm 은 생성자에 넘긴 writer 로 answerback 을 쓴다 —
//!     [`ReplyTap`] writer 로 흡수한다. 그 writer 는 동기로 불린다(`threaded_writer` = false)
//!     므로, feed 가 돌아온 시점에 그 feed 의 답은 이미 tap 에 들어와 있다.
//!   - private mode 읽기: wezterm 은 대부분의 DEC private mode(app_keypad·마우스·focus 등)에
//!     public getter 가 없다. 같은 파서(termwiz)로 mode 액션을 관찰해 [`ModeTracker`] 로
//!     복원한다(엔진의 authoritative 파서를 재사용 — 자작 상태기계 아님). 파싱은 한 번만:
//!     엔진이 자기 파서를 돌리는 그 한 번을 `advance_bytes_observed` 로 같이 본다.
//!   - 그리드 읽기: wezterm 은 wide 문자를 셀 1개(width 2)로 담는다(계약 정규형 의 본체+스페이서
//!     2셀과 다름). [`materialize_line_into`] 가 컬럼을 확장해 계약 정규형과 동형인 [`GridCell`]
//!     로 정렬한다(wide 본체 + spacer). 라인 wrap 은 `last_cell_was_wrapped()` → 마지막 칸 wrapline.
//!   - 라인 접근: `Screen::line` 으로 한 행을 빌려 읽는다(복제 없음). `scrollback_rows()` 는
//!     전체 라인 수(scrollback+visible)라 스크롤백 수는 total−physical_rows 로 계산한다.

use std::sync::{Arc, Mutex};

use termwiz::cell::{CellAttributes, Intensity, Underline};
use termwiz::color::ColorAttribute;
use termwiz::escape::csi::{
    DecPrivateMode, DecPrivateModeCode, Mode, TerminalMode, TerminalModeCode, CSI,
};
use termwiz::escape::{Action, Esc, EscCode};
use termwiz::surface::line::{CellRef, Line};
use wezterm_term::color::ColorPalette;
use wezterm_term::{Terminal, TerminalConfiguration, TerminalSize};

/// 엔진이 유지하는 스크롤백 행 수. 바이트 충실 복원의 바닥 — 전체 의미 이력은
/// command_blocks(app.data)가 소유하고, 이 수치는 화면 재현용 창이다.
pub const MIRROR_SCROLLBACK_LINES: usize = 1000;

// ── 엔진-중립 스냅샷 타입(계약의 비교 통화 — 두 엔진 유닛 공용) ──────────────

/// 색 스냅샷 — 엔진 타입을 밖으로 새지 않게 자체 표현으로 고정한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorSnap {
    #[default]
    Default,
    Named(u8),
    Indexed(u8),
    Rgb(u8, u8, u8),
}

/// 복원 대상 private mode 집합의 스냅샷(rehydrate 가 재현해야 하는 전부).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModeSnap {
    pub bracketed_paste: bool,
    pub app_cursor: bool,
    pub app_keypad: bool,
    pub mouse_click: bool,
    pub mouse_drag: bool,
    pub mouse_motion: bool,
    pub sgr_mouse: bool,
    pub utf8_mouse: bool,
    pub focus_in_out: bool,
    pub alternate_scroll: bool,
    pub show_cursor: bool,
    pub line_wrap: bool,
    pub insert: bool,
}

/// 직렬화기가 읽는 엔진-중립 셀 — 직렬화에 필요한 것을 다 담는다(spacer·wrapline·zerowidth
/// 포함). 이 타입 하나가 직렬화기의 그리드 읽기 단일 창이다 — 엔진 세부는 이 파일 밖으로
/// 나가지 않는다.
pub struct GridCell {
    pub ch: char,
    pub fg: ColorSnap,
    pub bg: ColorSnap,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub strikeout: bool,
    pub hidden: bool,
    /// wide 문자 본체(2칸 점유의 첫 칸).
    pub wide: bool,
    /// wide 문자 스페이서(본체 뒤 칸 또는 줄끝 선두 스페이서) — 직렬화기가 건너뛴다.
    pub spacer: bool,
    /// WRAPLINE — 마지막 칸에서만 의미: 이 행이 자연 개행(wrap)으로 이어진다.
    pub wrapline: bool,
    /// 결합 문자(zero-width) 후속.
    pub zerowidth: Vec<char>,
}

// ── 응답 tap — 터미널이 PTY 에 answerback 하려는 바이트를 writer 로 포획 ────────
// 엔진에게 이 writer 를 **동기**로 부르라고 말해 둔다(`MirrorConfig::threaded_writer` = false).
// 기본값(배경 스레드+채널)은 writer 가 진짜 PTY 라서 막힐 수 있는 상호작용 터미널의 사정이다 —
// 이 tap 은 메모리에 쌓을 뿐 절대 막히지 않으므로, 그 보호는 값만 치르고 얻는 것이 없다: 미러
// 하나당 스레드 하나, write 마다 할당+채널 send, 그리고 배달이 비동기라 "이 feed 가 무엇을
// 답하려 했는가"를 읽으려면 그 스레드와 동기화해야 한다. 동기 writer 는 그 셋을 한꺼번에
// 없앤다 — feed 가 돌아온 시점에 답은 이미 여기 들어와 있다(배리어 불필요).

struct ReplyState {
    replies: Mutex<Vec<String>>,
}

#[derive(Clone)]
struct ReplyTap(Arc<ReplyState>);

impl std::io::Write for ReplyTap {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .replies
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(String::from_utf8_lossy(buf).into_owned());
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// ── 엔진 설정 — 계약이 요구하는 것만(색 팔레트 + 스크롤백 창) ──────────────────

#[derive(Debug)]
struct MirrorConfig;

impl TerminalConfiguration for MirrorConfig {
    fn color_palette(&self) -> ColorPalette {
        ColorPalette::default()
    }
    fn scrollback_size(&self) -> usize {
        MIRROR_SCROLLBACK_LINES
    }
    /// 응답 writer 를 배경 스레드에 넘기지 않는다 — [`ReplyTap`] 이 응답을 받는다.
    fn threaded_writer(&self) -> bool {
        false
    }
}

// ── mode 관찰기 — 파서 액션에서 DEC private mode 를 복원한다 ────────────────────
// wezterm 은 대부분의 private mode 에 getter 가 없다. 엔진과 같은 파서로 mode 액션을 관찰해
// authoritative 하게 재구성한다(자작 VT 상태기계가 아니라 authoritative 파서의 산출을 읽을
// 뿐 — 그리드·스크롤백·커서 emulation 은 wezterm 이 소유).

struct ModeTracker {
    snap: ModeSnap,
}

impl ModeTracker {
    fn new() -> Self {
        // 계약이 선언한 출생 상태(SPEC.md §11.I) — 엔진의 파워온 기본값이 아니라 이것이 기준이다.
        // 켜진 채 태어나는 것은 커서 가시성(DECTCEM)과 자동 줄바꿈(DECAWM) 둘뿐이다. alternate
        // scroll(1007)은 xterm 의 alternateScroll 리소스 기본값이 "false" 이므로 꺼진 채 태어난다.
        ModeTracker {
            snap: ModeSnap {
                show_cursor: true,
                line_wrap: true,
                ..ModeSnap::default()
            },
        }
    }

    fn observe(&mut self, action: &Action) {
        match action {
            Action::CSI(CSI::Mode(m)) => self.observe_mode(m),
            // DECKPAM/DECKPNM — private mode 가 아니라 Esc 라 DECRQM 로도 안 읽힌다.
            Action::Esc(Esc::Code(EscCode::DecApplicationKeyPad)) => self.snap.app_keypad = true,
            Action::Esc(Esc::Code(EscCode::DecNormalKeyPad)) => self.snap.app_keypad = false,
            // RIS(full reset) — 모드가 초기값으로 돌아간다(계약 정규형과 parity).
            Action::Esc(Esc::Code(EscCode::FullReset)) => self.snap = ModeTracker::new().snap,
            _ => {}
        }
    }

    fn observe_mode(&mut self, m: &Mode) {
        let (set, dm) = match m {
            Mode::SetDecPrivateMode(dm) => (true, dm),
            Mode::ResetDecPrivateMode(dm) => (false, dm),
            Mode::SetMode(tm) => return self.observe_terminal_mode(true, tm),
            Mode::ResetMode(tm) => return self.observe_terminal_mode(false, tm),
            // 질의(DECRQM)·저장/복원·xterm 키 모드는 mode 상태를 바꾸지 않는다 — 무시.
            _ => return,
        };
        match dm {
            DecPrivateMode::Code(code) => match code {
                DecPrivateModeCode::ApplicationCursorKeys => self.snap.app_cursor = set,
                DecPrivateModeCode::AutoWrap => self.snap.line_wrap = set,
                DecPrivateModeCode::ShowCursor => self.snap.show_cursor = set,
                DecPrivateModeCode::MouseTracking => self.snap.mouse_click = set,
                DecPrivateModeCode::ButtonEventMouse => self.snap.mouse_drag = set,
                DecPrivateModeCode::AnyEventMouse => self.snap.mouse_motion = set,
                DecPrivateModeCode::FocusTracking => self.snap.focus_in_out = set,
                DecPrivateModeCode::Utf8Mouse => self.snap.utf8_mouse = set,
                DecPrivateModeCode::SGRMouse => self.snap.sgr_mouse = set,
                DecPrivateModeCode::BracketedPaste => self.snap.bracketed_paste = set,
                _ => {}
            },
            // AlternateScroll(1007) 은 termwiz 에 named code 가 없다 — Unspecified 로 온다.
            DecPrivateMode::Unspecified(1007) => self.snap.alternate_scroll = set,
            DecPrivateMode::Unspecified(_) => {}
        }
    }

    fn observe_terminal_mode(&mut self, set: bool, tm: &TerminalMode) {
        if let TerminalMode::Code(TerminalModeCode::Insert) = tm {
            self.snap.insert = set;
        }
    }
}

// ── Engine — 유일한 wezterm 좌석 ─────────────────────────────────────────────

/// 바이트를 실제 렌더해 화면 상태를 유지하는 헤드리스 VT 엔진(wezterm-term). 미러(복원
/// 로직)가 쓰는 유일한 엔진 면이며, "이 바이트를 먹은 터미널이 PTY 에 무엇을 되쓰려 했는가"
/// (`captured_replies`)의 프로브이기도 하다.
pub struct Engine {
    term: Terminal,
    state: Arc<ReplyState>,
    modes: ModeTracker,
    cols: u16,
    rows: u16,
}

impl Engine {
    pub fn new(cols: u16, rows: u16) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let state = Arc::new(ReplyState { replies: Mutex::new(Vec::new()) });
        let writer = Box::new(ReplyTap(state.clone()));
        let size = term_size(cols, rows);
        let term =
            Terminal::new(size, Arc::new(MirrorConfig), "soksak-sidecar-terminal", "1", writer);
        Engine { term, state, modes: ModeTracker::new(), cols, rows }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        // 엔진의 파서가 낸 액션을 흐르는 채로 관찰한다(`advance_bytes_observed`): mode 관찰과
        // 엔진 적용이 같은 한 번의 파싱을 나눠 쓴다 — 이중 파싱도, 자작 상태기계도, 액션을
        // 통째로 모아 두는 중간 버퍼도 없다. 파서는 엔진 안에서 상태를 유지해 청크 경계의 부분
        // 시퀀스를 재조립한다(mirror.feed 가 ESC 경계로 쪼개도 안전).
        let modes = &mut self.modes;
        self.term.advance_bytes_observed(bytes, |a| modes.observe(a));
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols.max(1);
        self.rows = rows.max(1);
        self.term.resize(term_size(self.cols, self.rows));
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// 이 엔진이 PTY 에 되쓰려 한 응답들(DA1/DSR/OSC 질의 답). 재생 가드의 프로브 —
    /// 복원 시퀀스를 먹인 엔진에서 이게 비어 있지 않으면 이중응답이다.
    pub fn captured_replies(&self) -> Vec<String> {
        self.state.replies.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn alt_active(&self) -> bool {
        self.term.is_alt_screen_active()
    }

    /// 커서 위치(화면 기준 0-base row, col).
    pub fn cursor(&self) -> (usize, usize) {
        let p = self.term.cursor_pos();
        (p.y.max(0) as usize, p.x)
    }

    /// 현재 스크롤백(화면 위로 밀려난) 행 수. wezterm `scrollback_rows()` 는 전체 라인
    /// 수(scrollback+visible)라, 보이는 행 수를 빼야 진짜 스크롤백이다.
    pub fn history_size(&self) -> usize {
        let screen = self.term.screen();
        screen.scrollback_rows().saturating_sub(screen.physical_rows)
    }

    pub fn modes(&self) -> ModeSnap {
        self.modes.snap.clone()
    }

    /// 한 행(line index; 0..rows = 보이는 화면, 음수 = 스크롤백)을 엔진-중립 셀 벡터로
    /// 읽는다. 길이는 항상 `cols` — spacer 포함(직렬화기가 skip 판정을 소유한다).
    pub fn line_cells(&self, line: i32) -> Vec<GridCell> {
        let mut buf = Vec::new();
        self.line_cells_into(line, &mut buf);
        buf
    }

    /// [`Engine::line_cells`] 와 같은 행 읽기를 호출자의 버퍼에 채운다. 스크롤백 전체를 훑는
    /// 직렬화기는 행마다 새 벡터를 얻을 이유가 없다 — 창 하나(1000행)를 페인트하면 그 벡터
    /// 할당이 1000번이다. 버퍼는 재사용되고, 길이는 언제나 `cols` 다.
    pub fn line_cells_into(&self, line: i32, buf: &mut Vec<GridCell>) {
        let cols = self.cols as usize;
        buf.clear();
        let screen = self.term.screen();
        let total = screen.scrollback_rows();
        let phys_rows = screen.physical_rows;
        let hist = total.saturating_sub(phys_rows) as i32;
        let idx = hist + line;
        if idx < 0 || idx as usize >= total {
            buf.resize_with(cols, blank_cell);
            return;
        }
        materialize_line_into(screen.line(idx as usize), cols, buf);
    }
}

fn term_size(cols: u16, rows: u16) -> TerminalSize {
    TerminalSize {
        rows: rows as usize,
        cols: cols as usize,
        pixel_width: 0,
        pixel_height: 0,
        dpi: 0,
    }
}

// 한 wezterm Line 을 계약 정규형과 동형인 GridCell 벡터(길이 = cols)로 정렬한다. wide 문자는
// wezterm 이 셀 1개(width 2)로 담으므로, 본체 칸에 wide 를 세우고 다음 칸을 spacer 로 채운다
// (직렬화기·판정자가 spacer 를 건너뛴다 — 폭은 wide 가 진실). 결합 문자는 셀 문자열의
// 두 번째 자부터 zerowidth 로 분리한다.
fn materialize_line_into(line: &Line, cols: usize, grid: &mut Vec<GridCell>) {
    grid.clear();
    grid.resize_with(cols, blank_cell);
    for cr in line.visible_cells() {
        let col = cr.cell_index();
        if col >= cols {
            break;
        }
        grid[col] = cell_of(&cr);
        if cr.width() == 2 && col + 1 < cols {
            grid[col + 1] = GridCell { spacer: true, ..blank_cell() };
        }
    }
    if line.last_cell_was_wrapped() {
        if let Some(last) = grid.last_mut() {
            last.wrapline = true;
        }
    }
}

fn cell_of(cr: &CellRef) -> GridCell {
    let attrs: &CellAttributes = cr.attrs();
    let mut chars = cr.str().chars();
    let ch = chars.next().unwrap_or(' ');
    let zerowidth: Vec<char> = chars.collect();
    GridCell {
        ch,
        fg: snap_color(attrs.foreground()),
        bg: snap_color(attrs.background()),
        bold: attrs.intensity() == Intensity::Bold,
        dim: attrs.intensity() == Intensity::Half,
        italic: attrs.italic(),
        underline: attrs.underline() != Underline::None,
        inverse: attrs.reverse(),
        strikeout: attrs.strikethrough(),
        hidden: attrs.invisible(),
        wide: cr.width() == 2,
        spacer: false,
        wrapline: false,
        zerowidth,
    }
}

// wezterm ColorAttribute → 엔진-중립 ColorSnap. 팔레트 0..16 은 Named(기본/브라이트 SGR 로
// 왕복), 16..256 은 Indexed(38;5;N), truecolor 는 Rgb 로 낮춘다. wezterm 은 SGR 3x/9x 를
// PaletteIndex 로 담으므로 <16 은 Named 로 두어 기본색이 정확히 왕복한다(판정자 계약 정규형의
// Named 매핑과 동형).
fn snap_color(color: ColorAttribute) -> ColorSnap {
    match color {
        ColorAttribute::Default => ColorSnap::Default,
        ColorAttribute::PaletteIndex(i) => {
            if i < 16 {
                ColorSnap::Named(i)
            } else {
                ColorSnap::Indexed(i)
            }
        }
        ColorAttribute::TrueColorWithDefaultFallback(rgb)
        | ColorAttribute::TrueColorWithPaletteFallback(rgb, _) => {
            let (r, g, b, _) = rgb.to_srgb_u8();
            ColorSnap::Rgb(r, g, b)
        }
    }
}

fn blank_cell() -> GridCell {
    GridCell {
        ch: ' ',
        fg: ColorSnap::Default,
        bg: ColorSnap::Default,
        bold: false,
        dim: false,
        italic: false,
        underline: false,
        inverse: false,
        strikeout: false,
        hidden: false,
        wide: false,
        spacer: false,
        wrapline: false,
        zerowidth: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // wezterm 은 answerback 을 백그라운드 스레드로 비동기 배달한다. feed 의 배수 배리어가
    // 그 스레드를 동기화하므로, feed 직후 captured_replies 가 결정적으로 answerback 을 담는다.
    // 배리어가 없으면 이 읽기가 배달을 앞질러 프로세스별로 0 을 본다(간헐 RED). 배리어가
    // 있으면 매번 참이라 결정적이다 — 각 질의를 신선한 엔진에 먹여 곧바로 단언한다.
    #[test]
    fn feed_synchronously_captures_answerbacks() {
        for q in [&b"\x1b[c"[..], b"\x1b[>c", b"\x1b[6n", b"\x1b]11;?\x07"] {
            let mut e = Engine::new(80, 24);
            e.feed(q);
            assert!(
                !e.captured_replies().is_empty(),
                "feed must synchronously capture the answerback for {q:?}"
            );
        }
    }
}
