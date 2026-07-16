//! soksak-sidecar-terminal-wezterm — 서비스 런타임(SPEC.md §2).
//!
//! 서비스 부팅은 엔진-불가지다(미러가 어느 엔진을 쓰는지 이 진입점은 모른다).
//!
//! 생존 서비스: 셸을 검사포인트하는 사이드카는 셸보다 오래 살아야 하므로 stdio 가 아니라
//! identity-home 소켓에 결속한다(데몬과 동형). 부팅:
//!   1. SOKSAK_HOME 해석(데몬이 스폰 시 명시 — 추측 없음).
//!   2. 서비스 소켓 싱글턴 프로브 — 살아 있으면 물러난다(exit 0).
//!   3. 데몬 control 연결(토큰) → 현재 세션 목록.
//!   4. 세션마다 tee 구독 소비자 + 체크포인트 스레드 기동.
//!   5. 서비스 소켓 바인드 후 rehydrate/coldPaint/resize/status 를 서빙.
//!
//! 실패 격리(SPEC §7): 사이드카 사망은 셸·라이브에 무영향(데몬이 바이트 생존 소유) —
//! 복원 충실도만 degraded 하고, 소비자 쪽이 그것을 loud 하게 고지·리스폰한다.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use soksak_sidecar_terminal_wezterm::daemon::ControlClient;
use soksak_sidecar_terminal_wezterm::service::{self, new_registry, SpawnCtx};

// 소비자 미러 기본 격자 — tee 는 크기를 나르지 않는다(resize 는 제어 op). 플러그인이
// service ensureSession/resize 로 실 격자를 밀 때까지의 기본값(SPEC §5).
const DEFAULT_COLS: u16 = 80;
const DEFAULT_ROWS: u16 = 24;

fn main() {
    let home = match std::env::var("SOKSAK_HOME") {
        Ok(h) if !h.is_empty() => PathBuf::from(h),
        _ => {
            eprintln!("soksak-sidecar-terminal: SOKSAK_HOME required (the spawner supplies it)");
            std::process::exit(2);
        }
    };

    // stderr → home 로그(데몬과 동형). 플러그인이 stdio Channel 로 스폰해 stderr 를 버리므로,
    // 이 리다이렉트 없이는 체크포인트 실패 같은 loud 진단이 사라진다. 실패는 삼키지 않는다(무음
    // 금지) — 열기가 실패해도 원래 stderr 로 계속 간다(치명 아님).
    redirect_stderr_to_log(&home);

    // 싱글턴 — 계약당 엔진 유닛 하나(데몬 동형).
    if service::singleton_taken(&home) {
        eprintln!("soksak-sidecar-terminal: another instance serves this identity; exiting");
        std::process::exit(0);
    }

    // 데몬 control — 토큰 교환 + 현재 세션.
    //
    // 데몬은 앱의 첫 pty 스폰에 뜨는데, 플러그인은 사이드카를 그 전(activate)에 스폰할 수 있다
    // (부팅 순서라는 우연). 즉시 exit 하면 사이드카가 영영 안 서고, 체크포인트를 쓸 소비자가
    // 없어져 cold 복원이 무너진다(warm 은 데몬 재부착이 커버해 안 보인다). 데몬이 뜰 때까지
    // 유계 백오프로 재시도한다 — 부팅 핸드셰이크(총 수 초 상한), 메인라인 폴링 아님(종료 조건 =
    // 연결 성공 or 데드라인). 소진 후에만 물러난다(degraded loud). 소비자 플러그인의 rehydrate
    // 재시도와 동형.
    let (control, sessions) = {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        let mut delay = std::time::Duration::from_millis(100);
        loop {
            match ControlClient::connect(&home).and_then(|mut c| c.list_sessions().map(|s| (c, s))) {
                Ok(pair) => break pair,
                Err(e) => {
                    if std::time::Instant::now() >= deadline {
                        eprintln!("soksak-sidecar-terminal: cannot peer with the daemon after retries: {e}");
                        std::process::exit(3);
                    }
                    std::thread::sleep(delay);
                    delay = (delay * 2).min(std::time::Duration::from_secs(1));
                }
            }
        }
    };

    let token = match std::fs::read_to_string(
        soksak_sidecar_terminal_wezterm::proto::token_path(&home),
    ) {
        Ok(t) => t.trim().to_string(),
        Err(e) => {
            eprintln!("soksak-sidecar-terminal: token unreadable: {e}");
            std::process::exit(3);
        }
    };

    let reg = new_registry();
    // StoreBlob·listSessions·getSnapshot 은 이 control 하나를 공유해 보낸다(디바운스·드묾 —
    // mutex 직렬화로 충분). 리스트에 쓴 연결을 그대로 재사용한다.
    let ckpt_control = Arc::new(Mutex::new(control));
    let ctx = SpawnCtx { home: home.clone(), reg: reg.clone(), ckpt_control };
    // 부팅 시 이미 있는 세션을 구독한다. 부팅 후 태어난 세션은 플러그인의 ensureSession 이
    // 근접-birth 에 잡는다(서비스 면 — 부팅 목록은 스냅샷일 뿐, tee 는 데몬이 나열해 주지 않는다).
    for info in sessions {
        if let Err(e) = service::spawn_session_consumer(&ctx, info, DEFAULT_COLS, DEFAULT_ROWS) {
            eprintln!("soksak-sidecar-terminal: startup subscribe failed: {e}");
        }
    }

    let listener = match service::bind_service(&home) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("soksak-sidecar-terminal: bind service socket failed: {e}");
            std::process::exit(2);
        }
    };

    // 데몬 사망 감지 — 데몬이 죽으면(재부팅 모사 등) 이 사이드카는 죽은 피어를 붙든 좀비다.
    // 전용 control 연결이 EOF 로 풀리면 exit(0) 해 서비스 소켓을 놓는다(앱의 다음 ensureSidecar
    // 스폰이 새 데몬에 붙는 신선 인스턴스를 띄운다). 폴링 0(블로킹 read).
    {
        let home = home.clone();
        std::thread::spawn(move || {
            let _ = soksak_sidecar_terminal_wezterm::daemon::block_until_daemon_dies(&home);
            eprintln!("soksak-sidecar-terminal: daemon peer is gone — exiting to yield the singleton");
            std::process::exit(0);
        });
    }
    eprintln!(
        "soksak-sidecar-terminal: protocol {} pid {} serving {}",
        soksak_sidecar_terminal_wezterm::proto::PTYD_PROTOCOL_VERSION,
        std::process::id(),
        soksak_sidecar_terminal_wezterm::proto::service_socket_path(&home).display(),
    );
    service::serve(listener, ctx, token);
}

// 이 프로세스의 stderr(fd 2)를 home 서비스 로그로 물린다(append). 플러그인이 사이드카를
// stdio Channel 로 스폰해 stderr 를 버리므로, 이게 없으면 이후 모든 eprintln(체크포인트 실패
// 포함)이 사라진다. 데몬(코어가 stderr→ptyd-p<N>.log 로 리다이렉트)과 동형의 관측면. 열기
// 실패는 치명 아님 — 원래 stderr 로 계속 간다(무음 대신 최선).
fn redirect_stderr_to_log(home: &std::path::Path) {
    let path = soksak_sidecar_terminal_wezterm::proto::service_log_path(home);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let Ok(f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    // Replace the process's stderr (fd 2 / STD_ERROR_HANDLE) with the log. The handle is
    // leaked so it lives for the process lifetime. The syscall is platform-specific.
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        unsafe {
            libc::dup2(f.as_raw_fd(), 2);
        }
        std::mem::forget(f);
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::Console::{SetStdHandle, STD_ERROR_HANDLE};
        unsafe {
            SetStdHandle(STD_ERROR_HANDLE, f.as_raw_handle() as _);
        }
        std::mem::forget(f);
    }
}
