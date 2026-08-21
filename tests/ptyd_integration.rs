use std::path::{Path, PathBuf};

#[test]
fn warm_restore_uses_the_shared_runtime() {
    let Some(pty) = std::env::var("SOKSAK_PTYD_BIN")
        .ok()
        .map(PathBuf::from)
        .filter(|path| path.is_file())
    else {
        eprintln!("SKIP: set SOKSAK_PTYD_BIN to the Go PTY binary");
        return;
    };
    let service = Path::new(env!("CARGO_BIN_EXE_soksak-sidecar-terminal-wezterm"));
    soksak_kit_sidecar_terminal::integration::assert_warm_restore(
        &pty,
        service,
        "soksak-sidecar-terminal-wezterm",
    );
}
