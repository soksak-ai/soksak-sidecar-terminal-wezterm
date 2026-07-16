//! 데몬 wire 계약의 재구현 — 코어 크레이트(soksak-pty-proto)를 링크하지 않는다.
//! SPEC.md §4·§6 이 문서화한 바이트를 그대로 구현한다. 여기 상수·경로 파생·
//! 프레임 포맷이 데몬(soksak-ptyd)의 것과 어긋나면 통합이 깨진다 — 정본은 데몬 crate,
//! 이 파일은 그 계약의 소비자 구현이다.
//!
//! 서비스 소켓 경로는 **계약-키드(엔진명 미포함)** 다 — 두 엔진 유닛이 같은 경로를 프로브해
//! 계약당 하나만 돌게 하는 싱글턴의 근거(SPEC §2). CLIENT_ID(관측 라벨)만 유닛별로 다르다.

use std::path::{Path, PathBuf};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;

/// 데몬 wire 프로토콜 판(soksak-pty-proto PTYD_PROTOCOL_VERSION 과 일치해야 한다).
pub const PTYD_PROTOCOL_VERSION: u32 = 1;

/// tee 프레임 kind — SPEC.md §6.2. data = 원시 출력 사본, gap = 백프레셔 유실 마커.
pub const TEE_FRAME_DATA: u8 = 0;
pub const TEE_FRAME_GAP: u8 = 1;

/// 이 사이드카 유닛이 데몬 hello 에 싣는 client id(관측 라벨 — 인증은 토큰). 유닛별로
/// 다르다(엔진 식별) — 서비스 소켓 경로와 달리 계약-키드가 아니다.
pub const CLIENT_ID: &str = "soksak-sidecar-terminal-wezterm";

// ── 데몬 소켓·토큰 경로(identity home 파생, 프로토콜-키드) ─────────────────────

pub fn run_dir(home: &Path) -> PathBuf {
    home.join("run")
}

pub fn control_socket_path(home: &Path) -> PathBuf {
    run_dir(home).join(format!("ptyd-p{PTYD_PROTOCOL_VERSION}.sock"))
}

pub fn stream_socket_path(home: &Path) -> PathBuf {
    run_dir(home).join(format!("ptyd-p{PTYD_PROTOCOL_VERSION}-stream.sock"))
}

pub fn token_path(home: &Path) -> PathBuf {
    run_dir(home).join(format!("ptyd-p{PTYD_PROTOCOL_VERSION}.token"))
}

// ── 이 사이드카의 서비스 소켓(계약-키드, 엔진 무관) ───────────────────────────
// 싱글턴이 계약당 하나이도록 엔진 이름을 넣지 않는다 — 모든 엔진 유닛이 같은 경로를
// 프로브해, 하나가 이미 서빙 중이면 물러난다(SPEC §2 singleton). 프로토콜-키드.

/// 서비스 소켓: `<home>/run/soksak-sidecar-terminal-p<N>.sock`.
pub fn service_socket_path(home: &Path) -> PathBuf {
    run_dir(home).join(format!("soksak-sidecar-terminal-p{PTYD_PROTOCOL_VERSION}.sock"))
}

/// 서비스 로그: `<home>/run/soksak-sidecar-terminal-p<N>.log`(데몬 ptyd-p<N>.log 와 동형).
/// 플러그인은 사이드카를 stdio Channel 로 스폰해 stderr 가 사라진다 — 생존 서비스의 진단
/// 가시성을 위해 사이드카가 자기 stderr 를 이 파일로 물린다(체크포인트 실패 등 loud 보존).
pub fn service_log_path(home: &Path) -> PathBuf {
    run_dir(home).join(format!("soksak-sidecar-terminal-p{PTYD_PROTOCOL_VERSION}.log"))
}

// ── 봉인 체크포인트 경로(데몬 StoreBlob 이 쓰는 자리 — 통합 테스트 단언용) ──────
// 데몬 proto 의 base64url 컴포넌트별 인코딩과 동일(전단사 stem — 손실 sanitize·구분자
// 모호성 없음).

fn ckpt_component(s: &str) -> String {
    URL_SAFE_NO_PAD.encode(s)
}

pub fn checkpoint_dir(home: &Path) -> PathBuf {
    home.join("pty").join("checkpoints")
}

pub fn checkpoint_path(home: &Path, window_label: &str, pane_id: &str) -> PathBuf {
    checkpoint_dir(home)
        .join(format!("ckpt-{}.{}.json", ckpt_component(window_label), ckpt_component(pane_id)))
}

// ── 서비스/데몬 hello + 봉투 헬퍼 ─────────────────────────────────────────────

/// 데몬 control/stream hello 를 만든다(camelCase 와이어). session/from_seq/subscribe 는
/// 필요할 때만 실린다(unset 이면 와이어에서 생략 — 데몬의 additive-optional 규약과 동형).
pub fn hello(token: &str, session: Option<u64>, subscribe: bool) -> serde_json::Value {
    let mut h = serde_json::json!({
        "version": PTYD_PROTOCOL_VERSION,
        "token": token,
        "clientId": CLIENT_ID,
    });
    if let Some(s) = session {
        h["session"] = serde_json::json!(s);
    }
    if subscribe {
        h["subscribe"] = serde_json::json!(true);
    }
    h
}

/// 대칭 봉투 판정 — `ok` 가 참이 아니면 code/message 를 담은 에러 문장으로 접는다.
pub fn require_ok(reply: &serde_json::Value) -> Result<(), String> {
    if reply.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        return Ok(());
    }
    let code = reply.get("code").and_then(|v| v.as_str()).unwrap_or("ERR");
    let msg = reply.get("message").and_then(|v| v.as_str()).unwrap_or("(no message)");
    Err(format!("{code}: {msg}"))
}
