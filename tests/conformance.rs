// 계약 합격시험 — 픽스처도 골든도 여기 없다. 정본은 soksak-contract-terminal 이고, 이 파일은
// 그 시험을 부르는 일곱 개의 평범한 테스트다. 이 유닛의 미러를 정규형으로 옮기는 좌석은
// tests/common/mod.rs 에 있다(벤치도 같은 좌석을 쓴다 — 사본 0).
mod common;

use common::Unit;
use soksak_contract_terminal as contract;
use soksak_contract_terminal::Fixture;

#[test]
fn mid_escape_tail() {
    contract::assert_conforms::<Unit>(Fixture::MidEscapeTail);
}

#[test]
fn cjk_width() {
    contract::assert_conforms::<Unit>(Fixture::CjkWidth);
}

#[test]
fn alt_screen() {
    contract::assert_conforms::<Unit>(Fixture::AltScreen);
}

#[test]
fn private_modes() {
    contract::assert_conforms::<Unit>(Fixture::PrivateModes);
}

#[test]
fn replay_guard() {
    contract::assert_conforms::<Unit>(Fixture::ReplayGuard);
}

#[test]
fn cold_paint_alt() {
    contract::assert_conforms::<Unit>(Fixture::ColdPaintAlt);
}

#[test]
fn dec_line_drawing() {
    contract::assert_conforms::<Unit>(Fixture::DecLineDrawing);
}

// resize→rehydrate 폭 정합(공유 단언) — 코어 resize 는 데몬 PTY 만 바꾸고 미러엔 전파 안 되므로 kit 이
// rehydrate 직전(그리고 리사이즈마다) 미러를 pane 폭으로 맞춘다. 그 전제(다른 폭 resize 후 rehydrate 가
// 왕복 충실·내용 보존)를 계약이 못박고, 각 엔진은 여기서 한 줄로 부른다 — 개별 엔진에 복붙하지 않는다.
#[test]
fn resize_reflow() {
    contract::assert_resize_reflow::<Unit>();
}

// 골든 부트스트랩 — 이 엔진이 코퍼스를 어떻게 해석하는지 정규형 텍스트로 뱉는다. 골든이 아니라
// **후보**다: 엔진끼리 대조하고 VT 스펙으로 판정한 뒤에만 계약의 골든이 된다(SPEC.md §12).
// 평시 시험에 끼지 않는다(#[ignore]).
//   SOKSAK_GOLDEN_OUT=<dir> cargo test --test conformance -- --ignored dump_goldens
#[test]
#[ignore]
fn dump_goldens() {
    let dir = std::env::var("SOKSAK_GOLDEN_OUT").expect("SOKSAK_GOLDEN_OUT=<dir> 로 산출 경로를 준다");
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    for f in Fixture::ALL {
        for (stem, text) in contract::dump::<Unit>(f) {
            let path = dir.join(format!("{stem}.golden"));
            std::fs::write(&path, text).expect("write golden candidate");
            println!("wrote {}", path.display());
        }
    }
}
