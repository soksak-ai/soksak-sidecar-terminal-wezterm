//! 실 ptyd 통합(GREEN) — 코어 데몬 바이너리를 실제로 기동해 tee 소비·체크포인트·서비스
//! 런타임을 끝단까지 검증한다. 데몬 바이너리는 SOKSAK_PTYD_BIN 으로 주입한다(하니스
//! scripts/e2e/ptyd-integration.sh 가 코어 워크트리에서 빌드해 세팅). env 부재 시(맨
//! `cargo test`)는 무엇이 없는지 loud 하게 알리고 물러난다 — 게이트는 하니스다.
//!
//! 데몬 wire 는 엔진-불가지라 wezterm 미러도 같은 tee→feed→rehydrate/StoreBlob 왕복을 탄다.
//!
//! 두 결(둘 다 실 데몬):
//!   1. library_tee_consumption_and_seal — DaemonClient/TeeStream/Mirror/store_blob 를
//!      직접 구동: 세션 스폰 → tee 구독 → 바이트 흘림 → rehydrate 페인트 정합 →
//!      StoreBlob 후 봉인 파일 존재 + 평문 마커 부재.
//!   2. binary_runtime_serves_and_singleton — 사이드카 바이너리를 띄워 서비스 소켓으로
//!      rehydrate/status 를 받고, 싱글턴 프로브(둘째 인스턴스 exit 0)를 확인.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use serde_json::json;

use soksak_sidecar_terminal_wezterm::daemon::{ControlClient, TeeFrame, TeeStream};
use soksak_sidecar_terminal_wezterm::mirror::Mirror;
use soksak_sidecar_terminal_wezterm::proto;
use soksak_sidecar_terminal_wezterm::service::ServiceClient;

const MARKER: &str = "TEE-MARKER-9137";

// ── 하니스 프리앰블 ───────────────────────────────────────────────────────────

fn ptyd_bin() -> Option<PathBuf> {
    match std::env::var("SOKSAK_PTYD_BIN") {
        Ok(p) if !p.is_empty() && Path::new(&p).exists() => Some(PathBuf::from(p)),
        _ => {
            eprintln!(
                "SKIP: set SOKSAK_PTYD_BIN to the built soksak-ptyd binary to run the real-daemon \
                 integration (scripts/e2e/ptyd-integration.sh builds it in the core worktree)."
            );
            None
        }
    }
}

fn fresh_home(tag: &str) -> PathBuf {
    // 짧은 홈 접두 — 서비스 소켓 `soksak-sidecar-terminal-p1.sock`(31자)를 매단 전체 경로가
    // macOS Unix 소켓 SUN_LEN(~104B)을 넘지 않아야 bind_service 가 성공한다. pid+nanos 가
    // 이미 유일성을 주므로 엔진 접미는 붙이지 않는다(붙이면 예산 초과 — 실측).
    let nanos = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    let home = PathBuf::from(std::env::var("HOME").unwrap())
        .join(".soksak-e2e")
        .join(format!("wz-int-{tag}-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(home.join("run")).unwrap();
    home
}

fn rand_pk_b64() -> String {
    // 32바이트 임의값 — X25519 는 모든 32바이트를 유효 공개키(u-좌표)로 받는다. 통합은
    // 봉인 성공 + 평문 부재만 단언하므로 개봉 키는 불필요.
    let mut f = std::fs::File::open("/dev/urandom").unwrap();
    let mut buf = [0u8; 32];
    f.read_exact(&mut buf).unwrap();
    B64.encode(buf)
}

// 데몬을 기동하고 control 소켓이 응답할 때까지 대기(폴링은 하니스 동기화 — 프로덕션
// 폴링 아님).
struct Daemon {
    child: Child,
    home: PathBuf,
}

impl Daemon {
    fn start(bin: &Path, home: &Path) -> Self {
        let child = Command::new(bin)
            .env("SOKSAK_HOME", home)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn soksak-ptyd");
        let ctrl = proto::control_socket_path(home);
        wait_until(Duration::from_secs(5), || {
            std::os::unix::net::UnixStream::connect(&ctrl).is_ok()
        })
        .expect("ptyd control socket did not come up");
        Daemon { child, home: home.to_path_buf() }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

fn wait_until(timeout: Duration, mut f: impl FnMut() -> bool) -> Result<(), ()> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if f() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    if f() {
        Ok(())
    } else {
        Err(())
    }
}

fn read_token(home: &Path) -> String {
    std::fs::read_to_string(proto::token_path(home)).unwrap().trim().to_string()
}

// CreateOrAttach 로 셸 세션을 띄운다(체크포인트 키 동반). 반환 = session id.
fn spawn_shell(control: &mut ControlClient, window: &str, pane: &str) -> u64 {
    let reply = control
        .request(json!({
            "op": "createOrAttach",
            "paneId": pane,
            "cols": 80,
            "rows": 24,
            "cwd": null,
            "shell": "/bin/sh",
            "env": [["TERM", "xterm-256color"]],
            "envRemove": [],
            "windowLabel": window,
            "checkpointPk": rand_pk_b64(),
            "checkpointKeyId": "ptyk-test",
        }))
        .expect("createOrAttach");
    assert_eq!(reply["ok"], true, "createOrAttach failed: {reply}");
    reply["data"]["session"].as_u64().expect("session id")
}

fn write_marker_command(control: &mut ControlClient, session: u64) {
    let data = format!("printf '{MARKER}\\n'\n");
    let reply = control
        .request(json!({ "op": "write", "session": session, "dataB64": B64.encode(data.as_bytes()) }))
        .expect("write");
    assert_eq!(reply["ok"], true, "write failed: {reply}");
}

// ── 결 1: 라이브러리 소비 + 봉인 ─────────────────────────────────────────────

#[test]
fn library_tee_consumption_and_seal() {
    let Some(bin) = ptyd_bin() else { return };
    let home = fresh_home("lib");
    let daemon = Daemon::start(&bin, &home);
    let window = "w-test";
    let pane = "v2";

    let mut control = ControlClient::connect(&home).expect("control connect");
    let session = spawn_shell(&mut control, window, pane);

    // tee 구독을 write 보다 먼저 — 마커 출력이 구독 이후에 흐르도록.
    let tee = TeeStream::subscribe(&home, session).expect("subscribe");

    // 미러와 누적 바이트를 한 락 아래 둔다 — rehydrate 정합 비교를 원자로.
    struct Consumed {
        mirror: Mirror,
        bytes: Vec<u8>,
    }
    let consumed = Arc::new(Mutex::new(Consumed { mirror: Mirror::new(80, 24), bytes: Vec::new() }));
    let stop = Arc::new(AtomicBool::new(false));
    let drain = {
        let consumed = consumed.clone();
        let stop = stop.clone();
        std::thread::spawn(move || {
            let mut tee = tee;
            loop {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                match tee.next_frame() {
                    Ok(Some(TeeFrame::Data(b))) => {
                        let mut c = consumed.lock().unwrap();
                        c.mirror.feed(&b);
                        c.bytes.extend_from_slice(&b);
                    }
                    Ok(Some(TeeFrame::Gap { .. })) => {}
                    Ok(None) | Err(_) => break,
                }
            }
        })
    };

    write_marker_command(&mut control, session);

    // tee 가 마커를 배달할 때까지 대기.
    let seen = wait_until(Duration::from_secs(8), || {
        let c = consumed.lock().unwrap();
        c.bytes.windows(MARKER.len()).any(|w| w == MARKER.as_bytes())
    });
    assert!(seen.is_ok(), "the tee never delivered the marker");

    // 원자 스냅샷: rehydrate + 누적 바이트 + cold_paint 를 한 락에서.
    let (rehydrated, judge_bytes, cold) = {
        let c = consumed.lock().unwrap();
        (c.mirror.rehydrate(), c.bytes.clone(), c.mirror.cold_paint())
    };

    // 정합: 복원 페인트를 먹인 신선 미러 == 같은 tee 바이트를 먹인 신선 미러. 여기는 실 데몬
    // 왕복(tee→미러→페인트→봉인)이 살아 있는지 보는 통합 스모크다 — 계약의 합격 판정은 선언된
    // 골든이 하고(tests/conformance.rs), 이 파일은 그 판정을 흉내 내지 않는다.
    let mut restored = Mirror::new(80, 24);
    restored.feed(&rehydrated);
    let mut direct = Mirror::new(80, 24);
    direct.feed(&judge_bytes);
    let restored_text = screen_text(&restored);
    let direct_text = screen_text(&direct);
    assert!(
        restored_text.iter().any(|l| l.contains(MARKER)),
        "rehydrated paint must carry the marker; got {restored_text:?}"
    );
    assert_eq!(
        restored_text,
        direct_text,
        "rehydrated screen must match a fresh mirror fed identical bytes"
    );

    // StoreBlob(cold_paint) → 데몬 봉인 → 체크포인트 경로에 봉인 파일.
    control.store_blob(Some(window), pane, &cold).expect("store_blob");
    let ckpt = proto::checkpoint_path(&home, window, pane);
    assert!(ckpt.exists(), "sealed checkpoint must exist at {}", ckpt.display());
    let sealed = std::fs::read(&ckpt).unwrap();
    assert!(
        !sealed.windows(MARKER.len()).any(|w| w == MARKER.as_bytes()),
        "the on-disk checkpoint must be sealed — no plaintext marker"
    );

    // drain 스레드는 tee 의 블로킹 read 에 갇혀 있다 — stop 는 최선노력이고, 실제 종료는
    // daemon 이 drop 되며 ptyd 를 죽여 tee 가 EOF 될 때다. 여기서 join 하면 세션이 살아
    // 있어 데드락이므로 detach 한다(함수 종료 시 daemon drop → ptyd kill → 스레드 종료).
    stop.store(true, Ordering::SeqCst);
    drop(control);
    drop(daemon); // ptyd 를 죽여 tee 를 EOF → drain 스레드가 풀린다
    drop(drain); // join 하지 않는다(detach)
}

// ── 결 2: 실행 바이너리 런타임 + 싱글턴 ──────────────────────────────────────

struct Sidecar(Child);
impl Drop for Sidecar {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

// The sidecar binary under test — SOKSAK_SIDECAR_BIN overrides (to drive a staged dist
// artifact through the same round trip), else the cargo-built binary.
fn sidecar_bin() -> String {
    std::env::var("SOKSAK_SIDECAR_BIN")
        .ok()
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_soksak-sidecar-terminal-wezterm").to_string())
}

fn spawn_sidecar(home: &Path) -> Sidecar {
    Sidecar(
        Command::new(sidecar_bin())
            .env("SOKSAK_HOME", home)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn sidecar"),
    )
}

#[test]
fn binary_runtime_serves_and_singleton() {
    let Some(bin) = ptyd_bin() else { return };
    let home = fresh_home("bin");
    let daemon = Daemon::start(&bin, &home);
    let window = "w-bin";
    let pane = "v3";

    let mut control = ControlClient::connect(&home).expect("control connect");
    let session = spawn_shell(&mut control, window, pane);
    let token = read_token(&home);

    // 사이드카 기동 — 부팅 시 현재 세션을 tee 구독한다.
    let _sidecar = spawn_sidecar(&home);

    // 서비스 소켓이 뜨고 status 가 이 세션을 (구독 성공 후) 보고할 때까지 대기.
    wait_until(Duration::from_secs(8), || {
        let mut c = match ServiceClient::connect(&home, &token) {
            Ok(c) => c,
            Err(_) => return false,
        };
        match c.status() {
            Ok(s) => s["data"]["sessions"]
                .as_array()
                .map(|a| a.iter().any(|x| x["pane"] == pane))
                .unwrap_or(false),
            Err(_) => false,
        }
    })
    .expect("sidecar never reported the subscribed session over status");

    // 구독 후 마커를 흘린다 — 바이너리의 소비 스레드가 미러에 반영.
    write_marker_command(&mut control, session);

    // 서비스 rehydrate 가 마커를 담을 때까지 대기(tee 비동기 배달).
    let mut client = ServiceClient::connect(&home, &token).expect("service connect");
    let saw_marker = wait_until(Duration::from_secs(8), || {
        let reply = match client.rehydrate(Some(window), pane) {
            Ok(r) => r,
            Err(_) => return false,
        };
        reply["data"]["paint"]
            .as_str()
            .and_then(|p| B64.decode(p).ok())
            .map(|paint| paint.windows(MARKER.len()).any(|w| w == MARKER.as_bytes()))
            .unwrap_or(false)
    });
    assert!(saw_marker.is_ok(), "service rehydrate never carried the marker");

    // status: 이 세션이 소비 seq 를 전진시켰다.
    let status = client.status().expect("status");
    let sess = status["data"]["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["pane"] == pane)
        .expect("session in status");
    assert!(sess["consumedSeq"].as_u64().unwrap() > 0, "consumedSeq must advance from the tee");

    // 실행 바이너리의 체크포인트 경로: 디바운스 후 봉인 파일이 나타난다(+ 평문 부재).
    let ckpt = proto::checkpoint_path(&home, window, pane);
    wait_until(Duration::from_secs(5), || ckpt.exists())
        .expect("a sealed checkpoint never appeared for the running sidecar");
    let sealed = std::fs::read(&ckpt).unwrap();
    assert!(
        !sealed.windows(MARKER.len()).any(|w| w == MARKER.as_bytes()),
        "the on-disk checkpoint must be sealed — no plaintext marker"
    );

    // 싱글턴: 둘째 인스턴스는 소켓이 물려 있음을 프로브하고 exit 0.
    let second = Command::new(sidecar_bin())
        .env("SOKSAK_HOME", &home)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn second sidecar");
    let status = wait_child(second, Duration::from_secs(5)).expect("second instance must exit promptly");
    assert!(status.success(), "the second instance must yield to the singleton (exit 0)");

    drop(control);
    let _ = daemon;
}

// ── 결 3: ensureSession — 부팅 후 태어난 세션의 동적 구독 ─────────────────────
// tee 를 데몬이 나열해 주지 않으므로, 사이드카가 뜬 뒤 생긴 세션은 자동 구독 대상이
// 아니다. 플러그인이 터미널 스폰 직후 ensureSession 을 부르면 그 세션을 근접-birth 에
// 잡는다. RED: ensureSession 없이는 rehydrate 가 NOT_FOUND. GREEN: 부른 뒤 마커가 담긴다.

#[test]
fn ensure_session_subscribes_a_session_born_after_the_sidecar_started() {
    let Some(bin) = ptyd_bin() else { return };
    let home = fresh_home("ensure");
    let daemon = Daemon::start(&bin, &home);
    let window = "w-ens";
    let pane = "v-ens";
    let token = read_token(&home);

    // 사이드카를 먼저 띄운다 — 이 시점엔 세션이 없다(부팅 목록 비어 있음).
    let _sidecar = spawn_sidecar(&home);
    wait_until(Duration::from_secs(8), || ServiceClient::connect(&home, &token).is_ok())
        .expect("service socket never came up");

    // 사이드카 기동 '후' 세션을 만든다 — 부팅 목록엔 없으니 자동 구독 대상이 아니다.
    let mut control = ControlClient::connect(&home).expect("control connect");
    let session = spawn_shell(&mut control, window, pane);

    let mut client = ServiceClient::connect(&home, &token).expect("service connect");
    // ensureSession 전: 아직 미러 안 함 → rehydrate NOT_FOUND(RED 경계).
    let before =
        client.request(json!({ "op": "rehydrate", "window": window, "pane": pane })).expect("rehydrate");
    assert_eq!(before["ok"], false, "a session born after startup must not be mirrored yet: {before}");
    assert_eq!(before["code"], "NOT_FOUND", "{before}");

    // ensureSession → 사이드카가 이 pane 의 라이브 세션을 찾아 구독한다.
    let ensured = client
        .request(json!({ "op": "ensureSession", "window": window, "pane": pane, "cols": 80, "rows": 24 }))
        .expect("ensureSession");
    assert_eq!(ensured["ok"], true, "ensureSession must subscribe the live session: {ensured}");

    // 이제 마커를 흘리면 rehydrate 가 담는다.
    write_marker_command(&mut control, session);
    let saw = wait_until(Duration::from_secs(8), || match client.rehydrate(Some(window), pane) {
        Ok(r) => r["data"]["paint"]
            .as_str()
            .and_then(|p| B64.decode(p).ok())
            .map(|paint| paint.windows(MARKER.len()).any(|w| w == MARKER.as_bytes()))
            .unwrap_or(false),
        Err(_) => false,
    });
    assert!(saw.is_ok(), "after ensureSession the mirror must carry the marker");

    drop(control);
    let _ = daemon;
}

// ── 결 4: 근접-birth 씨앗 — 구독 '전' 출력도 warm 복원에 담긴다 ────────────────
// 사이드카 구독이 초기 출력보다 늦으면(스폰 레이스), tee 는 start_seq(마커 이후)부터라
// 미러가 구독 전 출력을 못 본다 → warm 복원에서 그 화면이 사라진다(간헐 RED). subscribe
// ack 이 구독 순간 데몬 링에 원자 캡처한 backlog(seedB64)를 실어 주고, 소비자가 이를 미러에
// 선주입해 메운다(getSnapshot 미러 직렬화 불의존 — 링 씨앗). RED: 씨앗 없으면 구독 전
// 마커가 rehydrate 에 부재. GREEN: 링 씨앗으로 담긴다. 추가 출력 없이 검증한다.
#[test]
fn a_late_subscribe_seeds_output_written_before_the_subscription() {
    let Some(bin) = ptyd_bin() else { return };
    let home = fresh_home("seed");
    let daemon = Daemon::start(&bin, &home);
    let window = "w-seed";
    let pane = "v-seed";
    let token = read_token(&home);

    let _sidecar = spawn_sidecar(&home);
    wait_until(Duration::from_secs(8), || ServiceClient::connect(&home, &token).is_ok())
        .expect("service socket never came up");

    let mut control = ControlClient::connect(&home).expect("control connect");
    let session = spawn_shell(&mut control, window, pane);

    // 마커를 '구독 전'에 흘린다 — 데몬 화면엔 오르나 사이드카는 아직 구독 안 함.
    // 마커가 출력에만 나타나게 분할한다: cooked 모드가 입력 라인을 에코하므로 리터럴을
    // printf 인자로 그대로 쓰면 에코된 입력에도 뜬다 → 구독 후 tee 가 그 사본을 실어
    // 씨앗을 우회해버린다. '%s%s' 로 두 조각을 이어붙이면 에코엔 분리(공백)돼 뜨고
    // 출력에만 연속 마커가 나온다 → 유일 출처가 씨앗임을 보장.
    let (a, b) = MARKER.split_at(MARKER.len() / 2);
    let data = format!("printf '%s%s\\n' '{a}' '{b}'\n");
    let reply = control
        .request(json!({ "op": "write", "session": session, "dataB64": B64.encode(data.as_bytes()) }))
        .expect("write");
    assert_eq!(reply["ok"], true, "write failed: {reply}");
    // 마커 '출력'이 데몬 링(씨앗 원천)에 올랐는지 확인하고 나서 구독한다. 씨앗은 이제
    // subscribe ack 의 backlog(seedB64)다 — throwaway 구독의 seed 로 링 반영을 확인한다
    // (구독은 즉시 드롭돼 데몬이 reap 한다; getSnapshot 미러 프로브 불의존).
    wait_until(Duration::from_secs(8), || {
        TeeStream::subscribe(&home, session)
            .map(|t| t.seed().windows(MARKER.len()).any(|w| w == MARKER.as_bytes()))
            .unwrap_or(false)
    })
    .expect("daemon ring never carried the pre-subscribe marker");

    // 구독한다 — start_seq 는 마커 이후. 씨앗이 없으면 마커는 tee 로 오지 않는다.
    let mut client = ServiceClient::connect(&home, &token).expect("service connect");
    let ensured = client
        .request(json!({ "op": "ensureSession", "window": window, "pane": pane, "cols": 80, "rows": 24 }))
        .expect("ensureSession");
    assert_eq!(ensured["ok"], true, "ensureSession must subscribe the live session: {ensured}");

    // 추가 출력 없이도 rehydrate 가 구독 전 마커를 담아야 한다 — 씨앗 경로가 유일한 출처.
    let saw = wait_until(Duration::from_secs(5), || match client.rehydrate(Some(window), pane) {
        Ok(r) => r["data"]["paint"]
            .as_str()
            .and_then(|p| B64.decode(p).ok())
            .map(|paint| paint.windows(MARKER.len()).any(|w| w == MARKER.as_bytes()))
            .unwrap_or(false),
        Err(_) => false,
    });
    assert!(saw.is_ok(), "the seed must carry output written before the subscription");

    drop(control);
    let _ = daemon;
}

// ── 결 5: 부팅 순서 — 데몬보다 먼저 뜬 서비스도 결국 봉인-블롭을 쓴다 ──────────────
// 플러그인은 사이드카를 앱의 첫 pty 스폰(=데몬 기동) '전'에 스폰할 수 있다(activate). 데몬
// 미도달 시 즉시 exit 하면 서비스가 영영 안 서고 봉인-블롭이 안 써져 cold 복원이 무너진다 —
// warm 은 데몬 재부착이 가려 안 보이므로 이 갭이 '크러치-가림'(코어 auto-checkpoint)으로
// 빠져나갔었다. 데몬 control 을 유계 재시도해 뜰 때까지 버틴 뒤, 출력→디바운스 창 내 봉인-블롭을
// 단언한다. 격리 StoreBlob 테스트는 역학만 봤고 이 순서를 안 봤다 — 그 구멍을 막는다.
#[test]
fn a_service_started_before_the_daemon_still_seals_a_checkpoint() {
    let Some(bin) = ptyd_bin() else { return };
    // 짧은 태그 — 서비스 소켓 경로가 Unix SUN_LEN(~104B)을 넘지 않게(프로덕션 홈은 짧다).
    let home = fresh_home("bd");

    // 데몬 '전에' 사이드카를 스폰한다 — 즉시 exit(재시도 없음)면 서비스 소켓이 안 뜬다.
    let _sidecar = spawn_sidecar(&home);
    std::thread::sleep(Duration::from_millis(600)); // 데몬 없이 재시도 중이어야 한다

    // 이제 데몬을 띄운다 — 재시도가 이걸 잡아야 한다(부팅 핸드셰이크).
    let daemon = Daemon::start(&bin, &home);
    let token = read_token(&home);

    // 서비스가 떴다 = 사이드카가 exit 하지 않고 데몬을 재시도로 잡았다.
    wait_until(Duration::from_secs(10), || ServiceClient::connect(&home, &token).is_ok())
        .expect("service never came up — a service started before the daemon must retry, not exit");

    // 세션 + ensureSession 구독 + 출력 → 디바운스 창(idle 300ms / cap 5s) 안에 봉인-블롭.
    let mut control = ControlClient::connect(&home).expect("control connect");
    let window = "w-bd";
    let pane = "v-bd";
    let session = spawn_shell(&mut control, window, pane);
    let mut client = ServiceClient::connect(&home, &token).expect("service connect");
    let ensured = client
        .request(json!({ "op": "ensureSession", "window": window, "pane": pane, "cols": 80, "rows": 24 }))
        .expect("ensureSession");
    assert_eq!(ensured["ok"], true, "ensureSession must subscribe the live session: {ensured}");
    write_marker_command(&mut control, session);

    // 사이드카 checkpoint_loop → store_blob 산출물. 크러치 없이 사이드카만으로 검증한다.
    let ckpt = proto::checkpoint_path(&home, window, pane);
    wait_until(Duration::from_secs(8), || ckpt.exists())
        .expect("no sealed checkpoint — the sidecar checkpoint chain broke when it started before the daemon");
    let sealed = std::fs::read(&ckpt).unwrap();
    assert!(
        !sealed.windows(MARKER.len()).any(|w| w == MARKER.as_bytes()),
        "the on-disk checkpoint must be sealed — no plaintext marker"
    );

    drop(control);
    let _ = daemon;
}

fn wait_child(mut child: Child, timeout: Duration) -> Option<std::process::ExitStatus> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => return None,
        }
    }
    let _ = child.kill();
    None
}

// 미러 화면을 평문 행으로(스크롤백 → 보이는 화면). 통합 스모크가 마커 생존을 보는 창.
fn screen_text(m: &Mirror) -> Vec<String> {
    let hist = m.history_size() as i32;
    (-hist..m.rows() as i32)
        .map(|l| m.line_cells(l).iter().filter(|c| !c.spacer).map(|c| c.ch).collect())
        .collect()
}
