//! soksak-sidecar-terminal-wezterm — 라이브러리 면.
//!
//! `soksak-spec-sidecar-terminal@1` 계약(정본 = soksak-contract-terminal)의 **wezterm 엔진
//! 구현**. 도메인 로직(복원 미러·직렬화기)과 엔진 좌석을 모듈로 가른다:
//!   [`engine`]  wezterm-term 을 만지는 유일한 모듈(엔진-중립 뷰만 노출) — 미러의 피고 엔진.
//!   [`mirror`]  엔진-불가지 복원 로직 — [`Mirror`] 와 ANSI 직렬화기.
//!
//! 바이너리(서비스 소켓·데몬 피어링·체크포인트 정책)는 이 라이브러리를 링크한다.
//! 합격 판정은 계약(soksak-contract-terminal)이 소유한다 — 정답은 선언된 골든이고, 픽스처도
//! 골든도 이 크레이트에 사본으로 두지 않는다. tests/conformance.rs 가 그 시험대에 이 미러를 세운다.

pub mod checkpoint;
pub mod daemon;
pub mod engine;
pub mod mirror;
pub mod proto;
pub mod service;

pub use mirror::Mirror;
