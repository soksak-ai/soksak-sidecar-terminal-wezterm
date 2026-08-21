#[test]
fn absent_service_fails() {
    let home = std::path::Path::new("/tmp/soksak-absent-terminal-service");
    soksak_kit_sidecar_terminal::integration::assert_absent_service_fails(
        home,
        "soksak-sidecar-terminal-wezterm",
    );
}
