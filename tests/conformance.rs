// 계약 합격시험 — 픽스처도 reference state도 여기 없다. 정본은 soksak-contract-terminal 이고, 이 파일은
// 그 시험을 부르는 일곱 개의 평범한 테스트다. 이 유닛의 미러를 정규형으로 옮기는 좌석은
// tests/common/mod.rs 에 있다(벤치도 같은 좌석을 쓴다 — 사본 0).
mod common;

use common::SidecarMirror;
use soksak_contract_terminal as contract;
use soksak_contract_terminal::Fixture;
use soksak_kit_sidecar_terminal::frame::{FrameBaseline, delta};
use soksak_sidecar_terminal_wezterm::Mirror;

#[test]
fn process_label_control_contract() {
    soksak_kit_sidecar_terminal::integration::assert_process_label_contract();
}

#[test]
fn cursor_style() {
    contract::assert_cursor_style_conforms::<SidecarMirror>();
}

#[test]
fn mid_escape_tail() {
    contract::assert_conforms::<SidecarMirror>(Fixture::MidEscapeTail);
}

#[test]
fn cjk_width() {
    contract::assert_conforms::<SidecarMirror>(Fixture::CjkWidth);
}

#[test]
fn alt_screen() {
    contract::assert_conforms::<SidecarMirror>(Fixture::AltScreen);
}

#[test]
fn private_modes() {
    contract::assert_conforms::<SidecarMirror>(Fixture::PrivateModes);
}

#[test]
fn replay_guard() {
    contract::assert_conforms::<SidecarMirror>(Fixture::ReplayGuard);
}

#[test]
fn cold_paint_alt() {
    contract::assert_conforms::<SidecarMirror>(Fixture::ColdPaintAlt);
}

#[test]
fn dec_line_drawing() {
    contract::assert_conforms::<SidecarMirror>(Fixture::DecLineDrawing);
}

// resize→rehydrate 폭 정합(공유 단언) — 코어 resize 는 데몬 PTY 만 바꾸고 미러엔 전파 안 되므로 kit 이
// rehydrate 직전(그리고 리사이즈마다) 미러를 pane 폭으로 맞춘다. 그 전제(다른 폭 resize 후 rehydrate 가
// 왕복 충실·내용 보존)를 계약이 못박고, 각 엔진은 여기서 한 줄로 부른다 — 개별 엔진에 복붙하지 않는다.
#[test]
fn resize_reflow() {
    contract::assert_resize_reflow::<SidecarMirror>();
}

// reference state 부트스트랩 — 이 엔진이 코퍼스를 어떻게 해석하는지 정규형 텍스트로 뱉는다. reference state이 아니라
// **후보**다: 엔진끼리 대조하고 VT 스펙으로 판정한 뒤에만 계약의 reference state이 된다(SPEC.md §12).
// 평시 시험에 끼지 않는다(#[ignore]).
//   SOKSAK_REFERENCE_STATE_OUT=<dir> cargo test --test conformance -- --ignored dump_reference_states
#[test]
#[ignore]
fn dump_reference_states() {
    let dir = std::env::var("SOKSAK_REFERENCE_STATE_OUT")
        .expect("SOKSAK_REFERENCE_STATE_OUT=<dir> 로 산출 경로를 준다");
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    for f in Fixture::ALL {
        for (stem, text) in contract::dump::<SidecarMirror>(f) {
            let path = dir.join(format!("{stem}.reference-state"));
            std::fs::write(&path, text).expect("write reference state candidate");
            println!("wrote {}", path.display());
        }
    }
}

// frame 와이어 시리즈 — 스트림을 셋으로 잘라 먹이며 subscriber 하나가 받는 세 reply. kit 의 delta 가
// 만들고 계약의 apply 가 접는다; 접은 결과는 해석을 채점하는 그 reference state 와 같아야 한다.
fn frame_series(fixture: Fixture) -> contract::frame::FrameSeries {
    let stream = fixture.stream();
    let cuts = contract::frame::cut_points(stream.len());
    let mut mirror = Mirror::new(contract::COLS, contract::ROWS);
    let mut baseline: Option<FrameBaseline> = None;
    let mut fed = 0;
    let mut replies = Vec::new();
    for cut in cuts {
        mirror.feed(&stream[fed..cut]);
        fed = cut;
        let frame = mirror.frame_at(0);
        let (reply, next) = delta(baseline.as_ref(), &frame, cut as u64);
        baseline = Some(next);
        let wire = serde_json::to_value(&reply).expect("frame reply serializes");
        replies.push(serde_json::from_value(wire).expect("kit reply parses as the contract wire"));
    }
    contract::frame::FrameSeries {
        fixture: fixture.stem().to_string(),
        cols: contract::COLS,
        rows: contract::ROWS,
        cuts: cuts.to_vec(),
        replies,
    }
}

#[test]
fn frame_delta_reproduces_reference_states() {
    for fixture in Fixture::ALL {
        contract::frame::assert_series_reproduces(&frame_series(fixture), fixture);
    }
}

// 시리즈 부트스트랩 — 계약의 reference_states/frames/<stem>.frames.json 후보를 뱉는다(#[ignore]).
//   SOKSAK_FRAME_SERIES_OUT=<dir> cargo test --release --test conformance -- --ignored dump_frame_series
#[test]
#[ignore]
fn dump_frame_series() {
    let dir = std::env::var("SOKSAK_FRAME_SERIES_OUT")
        .expect("SOKSAK_FRAME_SERIES_OUT=<dir> 로 산출 경로를 준다");
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    for fixture in Fixture::ALL {
        let path = dir.join(format!("{}.frames.json", fixture.stem()));
        std::fs::write(&path, frame_series(fixture).to_json()).expect("write frame series");
        println!("wrote {}", path.display());
    }
}

#[test]
fn mode_report_restores() {
    contract::assert_mode_report_restores::<SidecarMirror>();
}
