// 벤치 — 측정기도 코퍼스도 계약이 소유한다. 이 파일은 이 유닛의 미러를 그 측정기에 세울 뿐이다.
//   SOKSAK_BENCH_OUT=<dir> cargo test --release --test bench -- --ignored --nocapture
//
// ④ 메모리 축이 재는 것은 미러가 붙든 힙이다 — 계약의 계수 할당자를 이 바이너리에 끼운다
// (RSS 가 아니라 순 할당 바이트를 재는 이유는 계약 bench 모듈에 적혀 있다).
#[global_allocator]
static ALLOC: soksak_contract_terminal::bench::CountingAlloc =
    soksak_contract_terminal::bench::CountingAlloc::new();

mod common;

#[test]
#[ignore]
fn bench() {
    let report = soksak_contract_terminal::bench::run::<common::Unit>("wezterm");
    println!("{}", report.to_line());
    // 기록이 판정보다 먼저다. 떨어진 유닛의 숫자야말로 표에 가장 있어야 할 숫자인데, 판정을
    // 먼저 하면 그 유닛은 아무 기록도 남기지 못하고 사라진다.
    if let Ok(dir) = std::env::var("SOKSAK_BENCH_OUT") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(dir.join("wezterm.bench"), report.to_line()).expect("write");
    }
    // 예산은 게이트다(SPEC.md §14.2) — 어기면 여기서 떨어진다. 후보끼리 견주지 않는다:
    // 이 유닛이 잰 수요와 이 유닛의 성적만 본다.
    soksak_contract_terminal::bench::assert_within_budget(&report);
}
