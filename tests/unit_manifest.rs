use std::{fs, path::PathBuf};

#[test]
fn unit_manifest_declares_the_provider_boundary() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let value: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("soksak-unit.json")).expect("read unit manifest"),
    ).expect("parse unit manifest");
    assert_eq!(value["spec"], "soksak-spec-unit@0.0.1");
    assert_eq!(value["kind"], "sidecar");
    assert_eq!(value["id"], "soksak-sidecar-terminal-wezterm");
    assert_eq!(value["version"], "0.0.1");
    assert_eq!(value["dependencies"][0]["id"], "soksak-sidecar-pty");
    assert_eq!(value["implements"][0]["id"], "soksak-spec-sidecar-terminal");
    assert_eq!(value["entrypoints"][0]["path"], "dist/soksak-sidecar-terminal-wezterm");
}
