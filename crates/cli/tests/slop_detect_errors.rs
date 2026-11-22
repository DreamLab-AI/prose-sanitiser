//! `slop-detect` must not report an unreadable or oversized input as a clean scan.

use std::process::Command;

fn slop_detect() -> Command {
    Command::new(env!("CARGO_BIN_EXE_slop-detect"))
}

#[test]
fn an_oversized_file_exits_with_the_tool_error_code_and_names_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let big = dir.path().join("big.css");
    std::fs::File::create(&big)
        .unwrap()
        .set_len(prose_sanitiser::slop::prose::MAX_FILE_BYTES + 1)
        .unwrap();
    let output = slop_detect().arg(dir.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("big.css"), "{stderr}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.starts_with("clean"), "{stdout}");

    let output = slop_detect()
        .arg("--json")
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["errors"].as_array().unwrap().len(), 1);
}

#[test]
fn a_readable_clean_file_still_exits_zero_with_no_errors_key() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ok.css");
    std::fs::write(&path, "a { margin: 0 }\n").unwrap();
    let output = slop_detect().arg("--json").arg(&path).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.get("errors").is_none());
}
