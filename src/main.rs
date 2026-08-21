use soksak_kit_sidecar_terminal::runtime::run_service;
use soksak_sidecar_terminal_wezterm::Mirror;

fn main() {
    if let Err(error) = run_service(
        "soksak-sidecar-terminal-wezterm",
        |cols, rows| Box::new(Mirror::new(cols, rows)),
        std::env::args().skip(1),
    ) {
        eprintln!("soksak-sidecar-terminal-wezterm: {error}");
        std::process::exit(1);
    }
}
