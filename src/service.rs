//! 서버 면(SPEC.md §5) — 터미널 플러그인이 붙어 복원을 요청한다. NDJSON over UDS,
//! identity-home 소켓 한 본(계약-키드, 엔진 무관 → 싱글턴은 계약당 하나). hello 후
//! rehydrate/coldPaint/status. 죽음은 소비자 쪽에서 loud(무음 아님).
//!
//! 소비 오케스트레이션·서버·체크포인트 루프는 [`Mirror`] 만 거쳐 엔진-불가지다 — 미러가
//! 미러가 어떤 엔진을 쓰는지 이 파일은 모른다.
//!
//! 공유 세션 상태(미러·체크포인트 정책·소비 seq·gap 카운터)도 여기 산다 — 소비자
//! 스레드(main)가 채우고 서버가 읽는다.

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use interprocess::local_socket::{
    prelude::*, GenericFilePath, Listener, ListenerOptions, RecvHalf, SendHalf, Stream,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use serde_json::{json, Value};

use crate::checkpoint::CheckpointPolicy;
use crate::daemon::{ControlClient, SessionInfo, TeeFrame, TeeStream};
use crate::mirror::Mirror;
use crate::proto;

/// 재부착 키 — 데몬과 같은 (window_label, pane_id). 창 라벨이 pane 을 네임스페이스한다.
pub type PaneKey = (Option<String>, String);

/// 한 세션의 사이드카-측 상태. 소비자 스레드가 tee 프레임으로 미러를 먹이고, 서버가
/// 복원 요청에 이 미러를 직렬화한다.
pub struct SessionState {
    pub session_id: u64,
    pub window: Option<String>,
    pub pane: String,
    pub mirror: Mirror,
    pub policy: CheckpointPolicy,
    /// tee 백프레셔 gap 수신 수 — status 로 loud 보고(무음 유실 아님).
    pub tee_gaps: u64,
    /// 소비한 데몬 링 seq(birth 구독이면 0 기점 정확). warm 핸드오프 좌표 uptoSeq.
    /// Data 프레임 길이만큼 전진, Gap 이면 to_seq 로 점프.
    pub consumed_seq: u64,
    /// 마지막 봉인-블롭 push 시각 — status 의 checkpointAge.
    pub last_checkpoint: Option<Instant>,
    /// tee EOF(세션 종료) 도달 — 소비자·체크포인트 스레드 종료 신호.
    pub closed: bool,
}

impl SessionState {
    pub fn new(session_id: u64, window: Option<String>, pane: String, cols: u16, rows: u16) -> Self {
        SessionState {
            session_id,
            window,
            pane,
            mirror: Mirror::new(cols, rows),
            policy: CheckpointPolicy::default(),
            tee_gaps: 0,
            consumed_seq: 0,
            last_checkpoint: None,
            closed: false,
        }
    }
}

/// 세션 셀 — 소비자·체크포인트·서버 스레드가 공유. cv 로 출력/종료 이벤트를 알린다.
pub struct SessionCell {
    pub st: Mutex<SessionState>,
    pub cv: Condvar,
}

/// (window, pane) → 세션 셀. 소비자 스레드가 등록/해제, 서버가 조회.
pub type Registry = Arc<Mutex<HashMap<PaneKey, Arc<SessionCell>>>>;

pub fn new_registry() -> Registry {
    Arc::new(Mutex::new(HashMap::new()))
}

fn lookup(reg: &Registry, window: Option<&str>, pane: &str) -> Option<Arc<SessionCell>> {
    reg.lock()
        .unwrap()
        .get(&(window.map(String::from), pane.to_string()))
        .cloned()
}

// ── 세션 소비 오케스트레이션(구독→미러 feed→체크포인트 push) ────────────────────
// 부팅 시 나열된 세션과 ensureSession 요청이 공유하는 진입점. 데몬 control 을 한 본
// (ckpt_control)만 공유해 listSessions/StoreBlob 을 직렬화한다(디바운스라 드묾).

/// 세션 소비에 필요한 공유 컨텍스트 — 서비스가 ensureSession 으로 새 세션을 구독할 때 쓴다.
#[derive(Clone)]
pub struct SpawnCtx {
    pub home: PathBuf,
    pub reg: Registry,
    pub ckpt_control: Arc<Mutex<ControlClient>>,
}

/// 한 세션을 구독(동기)→레지스트리 등록(동기)→feed·checkpoint 스레드 기동. 동기 등록이라
/// 반환 시 이미 서비스 면에 뜬다 — 중복 ensureSession 이 재구독하지 않는다(누수·이중 tee 방지).
/// 이미 미러 중이면 격자만 최신화하고 Ok(false). 구독 실패는 명시 에러(등록 안 함, degraded loud).
/// consumed_seq 는 tee 의 startSeq 로 앵커한다 — mid-session 구독이어도 데몬 링과 정합(SPEC §6.4).
pub fn spawn_session_consumer(
    ctx: &SpawnCtx,
    info: SessionInfo,
    cols: u16,
    rows: u16,
) -> io::Result<bool> {
    let key: PaneKey = (info.window_label.clone(), info.pane_id.clone());
    if let Some(cell) = ctx.reg.lock().unwrap().get(&key).cloned() {
        cell.st.lock().unwrap().mirror.resize(cols, rows);
        return Ok(false);
    }
    let tee = TeeStream::subscribe(&ctx.home, info.session)?;
    let start_seq = tee.start_seq();
    let mut state = SessionState::new(info.session, info.window_label.clone(), info.pane_id.clone(), cols, rows);
    state.consumed_seq = start_seq;
    // [근접-birth 씨앗] mid-session 구독(start_seq>0)이면 tee 는 start_seq 부터라 그 이전 출력을
    // 못 본다 — 사이드카 스폰이 초기 출력보다 늦으면 warm 복원에서 그 화면이 사라진다(구독 레이스).
    // subscribe ack 이 구독 순간 데몬 링에 원자 캡처한 backlog(tee.seed())를 실어 준다 — 이를
    // 미러에 선주입해 메운다. 링은 유계라 부분 씨앗(retained window)이고, start_seq 부터의 tee
    // 프레임과 겹치지 않는다(SPEC §6.4). 데몬 미러 직렬화(getSnapshot)에 의존하지 않는다 — 링 씨앗.
    if !tee.seed().is_empty() {
        state.mirror.feed(tee.seed());
    }
    let cell = Arc::new(SessionCell { st: Mutex::new(state), cv: Condvar::new() });
    ctx.reg.lock().unwrap().insert(key.clone(), cell.clone());

    {
        let cell = cell.clone();
        let ckpt = ctx.ckpt_control.clone();
        std::thread::spawn(move || checkpoint_loop(cell, ckpt));
    }
    {
        let reg = ctx.reg.clone();
        std::thread::spawn(move || feed_loop(tee, cell, reg, key));
    }
    Ok(true)
}

/// tee 프레임 소비 루프 — Data 는 미러에 먹이고 consumed_seq 를 전진, Gap 은 loud 카운트 후
/// to_seq 로 점프(무음 유실 금지). EOF(세션 종료)면 closed 표시 후 레지스트리에서 해제.
fn feed_loop(mut tee: TeeStream, cell: Arc<SessionCell>, reg: Registry, key: PaneKey) {
    let session = tee.session();
    loop {
        match tee.next_frame() {
            Ok(Some(TeeFrame::Data(bytes))) => {
                let mut st = cell.st.lock().unwrap();
                st.mirror.feed(&bytes);
                st.consumed_seq += bytes.len() as u64;
                st.policy.on_output(Instant::now());
                cell.cv.notify_all();
            }
            Ok(Some(TeeFrame::Gap { from_seq, to_seq })) => {
                let mut st = cell.st.lock().unwrap();
                st.tee_gaps += 1;
                st.consumed_seq = to_seq;
                eprintln!(
                    "soksak-sidecar-terminal: session {session} tee gap [{from_seq},{to_seq}) — restore fidelity degraded"
                );
                cell.cv.notify_all();
            }
            Ok(None) => break,
            Err(e) => {
                eprintln!("soksak-sidecar-terminal: session {session} tee read error: {e}");
                break;
            }
        }
    }
    let mut st = cell.st.lock().unwrap();
    st.closed = true;
    cell.cv.notify_all();
    drop(st);
    reg.lock().unwrap().remove(&key);
}

/// 체크포인트 디바운스 루프 — cv 이벤트/마감 시각으로만 깬다(고정 틱 폴링 없음). 마감 도달 시
/// cold_paint 를 봉인-블롭으로 데몬에 밀고(키 불접촉) reset 한다.
fn checkpoint_loop(cell: Arc<SessionCell>, ckpt_control: Arc<Mutex<ControlClient>>) {
    let mut st = cell.st.lock().unwrap();
    loop {
        if st.closed {
            return;
        }
        if !st.policy.is_dirty() {
            st = cell.cv.wait(st).unwrap();
            continue;
        }
        let now = Instant::now();
        match st.policy.deadline() {
            Some(deadline) if now < deadline => {
                let (guard, _) = cell.cv.wait_timeout(st, deadline - now).unwrap();
                st = guard;
                continue;
            }
            _ => {}
        }
        let paint = st.mirror.cold_paint();
        let window = st.window.clone();
        let pane = st.pane.clone();
        st.policy.reset();
        drop(st);

        let pushed = ckpt_control.lock().unwrap().store_blob(window.as_deref(), &pane, &paint);
        match pushed {
            Ok(()) => {
                let mut g = cell.st.lock().unwrap();
                g.last_checkpoint = Some(Instant::now());
                st = g;
            }
            Err(e) => {
                eprintln!("soksak-sidecar-terminal: checkpoint push for pane {pane} failed: {e}");
                st = cell.st.lock().unwrap();
            }
        }
    }
}

// ── 서버 ─────────────────────────────────────────────────────────────────────

/// 서비스 소켓 accept 루프 — 연결마다 hello 판정 후 요청을 처리. 블로킹, 스레드/연결.
pub fn serve(listener: Listener, ctx: SpawnCtx, token: String) {
    for conn in listener.incoming().flatten() {
        let ctx = ctx.clone();
        let token = token.clone();
        std::thread::spawn(move || {
            if let Err(e) = handle_conn(conn, &ctx, &token) {
                eprintln!("soksak-sidecar-terminal: service conn ended: {e}");
            }
        });
    }
}

fn handle_conn(conn: Stream, ctx: &SpawnCtx, token: &str) -> io::Result<()> {
    let (recv, send) = conn.split();
    let mut writer = send;
    let mut reader = BufReader::new(recv);

    // hello 선행 — version/token 판정. 불일치는 loud 거절.
    let mut hello_line = String::new();
    if reader.read_line(&mut hello_line)? == 0 {
        return Ok(());
    }
    let hello: Value = serde_json::from_str(hello_line.trim())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("hello parse: {e}")))?;
    if let Err(reply) = judge_hello(&hello, token) {
        writeln!(writer, "{reply}")?;
        return Ok(());
    }
    writeln!(writer, "{}", ok(json!({ "version": proto::PTYD_PROTOCOL_VERSION })))?;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(line.trim()) {
            Ok(req) => dispatch(ctx, &req),
            Err(e) => err("INVALID_PARAMS", &format!("request parse: {e}")),
        };
        writeln!(writer, "{reply}")?;
    }
    Ok(())
}

/// 요청 디스패치 — ensureSession(구독 부작용)은 여기서, 나머지 순수 읽기는 handle_request.
/// ensureSession{window, pane, cols?, rows?}: 이 pane 을 아직 미러하지 않으면 데몬 세션을
/// 찾아 구독한다(플러그인이 터미널 스폰 직후 호출 → 그 세션의 tee 를 근접-birth 에 잡는다).
fn dispatch(ctx: &SpawnCtx, req: &Value) -> Value {
    if req.get("op").and_then(|v| v.as_str()) == Some("ensureSession") {
        let (window, pane) = match wp(req) {
            Some(x) => x,
            None => return err("INVALID_PARAMS", "ensureSession requires window and pane"),
        };
        let cols = req.get("cols").and_then(|v| v.as_u64()).unwrap_or(80).max(1) as u16;
        let rows = req.get("rows").and_then(|v| v.as_u64()).unwrap_or(24).max(1) as u16;
        // 이미 미러 중이면 격자만 최신화(spawn_session_consumer 가 Ok(false)).
        let info = {
            let mut ctrl = ctx.ckpt_control.lock().unwrap();
            match ctrl.list_sessions() {
                Ok(list) => list
                    .into_iter()
                    .find(|s| s.window_label == window && s.pane_id == pane),
                Err(e) => return err("DAEMON", &format!("listSessions failed: {e}")),
            }
        };
        return match info {
            Some(info) => match spawn_session_consumer(ctx, info, cols, rows) {
                Ok(fresh) => ok(json!({ "pane": pane, "subscribed": fresh })),
                Err(e) => err("SUBSCRIBE_FAILED", &format!("{e}")),
            },
            None => err("NOT_FOUND", &format!("no live daemon session for pane {pane}")),
        };
    }
    handle_request(&ctx.reg, req)
}

fn judge_hello(hello: &Value, token: &str) -> Result<(), Value> {
    let version = hello.get("version").and_then(|v| v.as_u64());
    if version != Some(proto::PTYD_PROTOCOL_VERSION as u64) {
        return Err(err(
            "VERSION_SKEW",
            &format!("service speaks protocol {}, client sent {version:?}", proto::PTYD_PROTOCOL_VERSION),
        ));
    }
    if hello.get("token").and_then(|v| v.as_str()) != Some(token) {
        return Err(err("UNAUTHORIZED", "bad token"));
    }
    Ok(())
}

/// hello 이후 요청 하나를 처리한다. 순수(레지스트리 읽기·미러 직렬화)라 하니스로
/// 직접 호출해 단언할 수 있다.
pub fn handle_request(reg: &Registry, req: &Value) -> Value {
    let op = req.get("op").and_then(|v| v.as_str()).unwrap_or("");
    match op {
        "rehydrate" => {
            let (window, pane) = match wp(req) {
                Some(x) => x,
                None => return err("INVALID_PARAMS", "rehydrate requires window and pane"),
            };
            match lookup(reg, window.as_deref(), &pane) {
                None => err("NOT_FOUND", &format!("no live mirror for pane {pane}")),
                Some(cell) => {
                    let st = cell.st.lock().unwrap();
                    ok(json!({
                        "paint": B64.encode(st.mirror.rehydrate()),
                        "uptoSeq": st.consumed_seq,
                        "altActive": st.mirror.alt_active(),
                    }))
                }
            }
        }
        "coldPaint" => {
            let (window, pane) = match wp(req) {
                Some(x) => x,
                None => return err("INVALID_PARAMS", "coldPaint requires window and pane"),
            };
            match lookup(reg, window.as_deref(), &pane) {
                None => err("NOT_FOUND", &format!("no live mirror for pane {pane}")),
                Some(cell) => {
                    let st = cell.st.lock().unwrap();
                    ok(json!({
                        "paint": B64.encode(st.mirror.cold_paint()),
                        "altActive": st.mirror.alt_active(),
                    }))
                }
            }
        }
        "resize" => {
            let (window, pane) = match wp(req) {
                Some(x) => x,
                None => return err("INVALID_PARAMS", "resize requires window and pane"),
            };
            let cols = req.get("cols").and_then(|v| v.as_u64());
            let rows = req.get("rows").and_then(|v| v.as_u64());
            let (cols, rows) = match (cols, rows) {
                (Some(c), Some(r)) if c > 0 && r > 0 => (c as u16, r as u16),
                _ => return err("INVALID_PARAMS", "resize requires positive cols and rows"),
            };
            match lookup(reg, window.as_deref(), &pane) {
                None => err("NOT_FOUND", &format!("no live mirror for pane {pane}")),
                Some(cell) => {
                    cell.st.lock().unwrap().mirror.resize(cols, rows);
                    ok(json!({ "cols": cols, "rows": rows }))
                }
            }
        }
        "status" => {
            let map = reg.lock().unwrap();
            let now = Instant::now();
            let sessions: Vec<Value> = map
                .values()
                .map(|cell| {
                    let st = cell.st.lock().unwrap();
                    json!({
                        "session": st.session_id,
                        "window": st.window,
                        "pane": st.pane,
                        "altActive": st.mirror.alt_active(),
                        "suppressedReplies": st.mirror.suppressed_replies(),
                        "teeGaps": st.tee_gaps,
                        "consumedSeq": st.consumed_seq,
                        "checkpointAgeMs": st.last_checkpoint.map(|t| now.duration_since(t).as_millis() as u64),
                    })
                })
                .collect();
            let tee_gaps: u64 = map.values().map(|c| c.st.lock().unwrap().tee_gaps).sum();
            let suppressed: u64 =
                map.values().map(|c| c.st.lock().unwrap().mirror.suppressed_replies()).sum();
            ok(json!({
                "sessions": sessions,
                "checkpointAges": sessions.iter().filter_map(|s| s.get("checkpointAgeMs").cloned()).collect::<Vec<_>>(),
                "suppressedReplies": suppressed,
                "teeGaps": tee_gaps,
            }))
        }
        other => err("UNKNOWN_OP", &format!("unknown op {other}")),
    }
}

fn wp(req: &Value) -> Option<(Option<String>, String)> {
    let pane = req.get("pane").and_then(|v| v.as_str())?.to_string();
    let window = req.get("window").and_then(|v| v.as_str()).map(String::from);
    Some((window, pane))
}

fn ok(data: Value) -> Value {
    json!({ "ok": true, "code": "OK", "data": data })
}

fn err(code: &str, message: &str) -> Value {
    json!({ "ok": false, "code": code, "message": message })
}

// ── 싱글턴 프로브 ─────────────────────────────────────────────────────────────

/// 서비스 소켓에 살아 있는 응답자가 있는지 프로브. 있으면 true(호출자는 물러난다) —
/// 계약당 엔진 유닛 하나만 돈다(데몬 싱글턴 동형). 죽은 소켓 파일은 재바인드 위해 제거.
pub fn singleton_taken(home: &Path) -> bool {
    let path = proto::service_socket_path(home);
    let Ok(name) = path.as_os_str().to_fs_name::<GenericFilePath>() else {
        return false;
    };
    if Stream::connect(name).is_ok() {
        return true;
    }
    let _ = std::fs::remove_file(&path);
    false
}

/// 서비스 소켓 바인드(싱글턴 프로브 후). run 디렉토리는 데몬이 이미 만든다 —
/// 없으면 만든다(멱등).
pub fn bind_service(home: &Path) -> io::Result<Listener> {
    let dir = proto::run_dir(home);
    std::fs::create_dir_all(&dir)?;
    let path = proto::service_socket_path(home);
    let name = path.as_os_str().to_fs_name::<GenericFilePath>()?;
    ListenerOptions::new().name(name).create_sync()
}

// ── 클라이언트(플러그인/하니스 소비면) ───────────────────────────────────────

/// 서비스 소켓 클라이언트 — hello 후 rehydrate/coldPaint/status. 사이드카 미가동이면
/// connect 가 명시 에러(무음·행 아님) — RED 의 대상.
pub struct ServiceClient {
    writer: SendHalf,
    reader: BufReader<RecvHalf>,
}

impl ServiceClient {
    pub fn connect(home: &Path, token: &str) -> io::Result<Self> {
        let path = proto::service_socket_path(home);
        let name = path.as_os_str().to_fs_name::<GenericFilePath>()?;
        let stream = Stream::connect(name).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("no terminal sidecar at {}: {e}", path.display()),
            )
        })?;
        let (recv, send) = stream.split();
        let mut c = ServiceClient { writer: send, reader: BufReader::new(recv) };
        c.send(&json!({ "version": proto::PTYD_PROTOCOL_VERSION, "token": token }))?;
        let reply = c.recv()?;
        proto::require_ok(&reply)
            .map_err(|m| io::Error::new(io::ErrorKind::PermissionDenied, m))?;
        Ok(c)
    }

    fn send(&mut self, v: &Value) -> io::Result<()> {
        let mut line = serde_json::to_vec(v)?;
        line.push(b'\n');
        self.writer.write_all(&line)?;
        self.writer.flush()
    }

    fn recv(&mut self) -> io::Result<Value> {
        let mut line = String::new();
        if self.reader.read_line(&mut line)? == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "sidecar closed the connection"));
        }
        serde_json::from_str(line.trim())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("reply parse: {e}")))
    }

    pub fn request(&mut self, req: Value) -> io::Result<Value> {
        self.send(&req)?;
        self.recv()
    }

    pub fn status(&mut self) -> io::Result<Value> {
        self.request(json!({ "op": "status" }))
    }

    pub fn rehydrate(&mut self, window: Option<&str>, pane: &str) -> io::Result<Value> {
        self.request(json!({ "op": "rehydrate", "window": window, "pane": pane }))
    }

    pub fn cold_paint(&mut self, window: Option<&str>, pane: &str) -> io::Result<Value> {
        self.request(json!({ "op": "coldPaint", "window": window, "pane": pane }))
    }

    pub fn resize(&mut self, window: Option<&str>, pane: &str, cols: u16, rows: u16) -> io::Result<Value> {
        self.request(json!({ "op": "resize", "window": window, "pane": pane, "cols": cols, "rows": rows }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_rejects_bad_token_and_version_loudly() {
        assert!(judge_hello(&json!({ "version": 1, "token": "good" }), "good").is_ok());
        assert!(judge_hello(&json!({ "version": 1, "token": "bad" }), "good").is_err());
        assert!(judge_hello(&json!({ "version": 999, "token": "good" }), "good").is_err());
        assert!(judge_hello(&json!({ "token": "good" }), "good").is_err());
    }

    #[test]
    fn rehydrate_and_cold_paint_on_a_missing_pane_are_not_found() {
        let reg = new_registry();
        let r = handle_request(&reg, &json!({ "op": "rehydrate", "pane": "v2" }));
        assert_eq!(r["ok"], false);
        assert_eq!(r["code"], "NOT_FOUND");
        let c = handle_request(&reg, &json!({ "op": "coldPaint", "window": "w-1", "pane": "v2" }));
        assert_eq!(c["code"], "NOT_FOUND");
    }

    #[test]
    fn status_over_an_empty_registry_reports_zeroed_totals() {
        let reg = new_registry();
        let s = handle_request(&reg, &json!({ "op": "status" }));
        assert_eq!(s["ok"], true);
        assert_eq!(s["data"]["teeGaps"], 0);
        assert_eq!(s["data"]["suppressedReplies"], 0);
        assert_eq!(s["data"]["sessions"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn a_registered_mirror_rehydrates_over_the_request_handler() {
        let reg = new_registry();
        let cell = Arc::new(SessionCell {
            st: Mutex::new(SessionState::new(1, Some("w-1".into()), "v2".into(), 80, 24)),
            cv: Condvar::new(),
        });
        cell.st.lock().unwrap().mirror.feed(b"HELLO-STATUS\r\n");
        cell.st.lock().unwrap().consumed_seq = 14;
        reg.lock().unwrap().insert((Some("w-1".into()), "v2".into()), cell);

        let r = handle_request(&reg, &json!({ "op": "rehydrate", "window": "w-1", "pane": "v2" }));
        assert_eq!(r["ok"], true);
        assert_eq!(r["data"]["uptoSeq"], 14);
        let paint = B64.decode(r["data"]["paint"].as_str().unwrap()).unwrap();
        assert!(
            String::from_utf8_lossy(&paint).contains("HELLO-STATUS"),
            "rehydrate paint carries the fed marker"
        );
    }
}
