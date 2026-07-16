//! RED 재현(SPEC.md §2·§7) — 사이드카 미가동 상태에서 서비스 소켓에 접속하면 명시
//! 연결 실패여야 한다: 무음도, 무한 대기도 아니다. degraded 는 loud 라는 계약을 이
//! 크레이트 경계에서 못박는다(회귀 감지망: connect 가 hang 하거나 에러를 삼키면 깨진다).
//!
//! 서비스 소켓 계약은 엔진-불가지다(미러가 어느 엔진을 쓰든 이 실패 의미론은 같다).

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use soksak_sidecar_terminal_wezterm::service::ServiceClient;

// ~/.soksak-e2e 하위 유니크 홈 — 실 데몬 소켓과 충돌하지 않는 격리 경로. 접두는 짧게 —
// 서비스 소켓 경로가 macOS Unix 소켓 SUN_LEN(~104B)을 넘으면 connect 가 '없는 소켓'이
// 아니라 '경로 초과'로 실패해 테스트 의도가 흐려진다(pid+nanos 로 유일성 확보).
fn fresh_home() -> PathBuf {
    let nanos = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    let home = PathBuf::from(std::env::var("HOME").unwrap())
        .join(".soksak-e2e")
        .join(format!("wz-down-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(home.join("run")).unwrap();
    home
}

#[test]
fn connecting_to_an_absent_sidecar_fails_loudly_and_fast() {
    let home = fresh_home();

    // 소켓이 바인드된 적 없다(사이드카 미가동). connect 는 즉시 명시 에러여야 한다.
    let start = Instant::now();
    let result = ServiceClient::connect(&home, "any-token");
    let elapsed = start.elapsed();

    let err = match result {
        Ok(_) => panic!("connect to an absent sidecar must fail, not succeed silently"),
        Err(e) => e,
    };
    let msg = err.to_string();
    assert!(
        msg.contains("no terminal sidecar"),
        "the failure must name what is missing (got: {msg})"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "connect must fail fast, not hang (took {elapsed:?})"
    );

    let _ = std::fs::remove_dir_all(&home);
}
