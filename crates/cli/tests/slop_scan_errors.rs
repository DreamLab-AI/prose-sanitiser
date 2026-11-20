//! COV-A-F4: `slop-scan` must not report an unreadable input as a clean scan.

use std::process::Command;

fn slop_scan() -> Command {
    Command::new(env!("CARGO_BIN_EXE_slop-scan"))
}

#[cfg(unix)]
#[test]
fn an_unreadable_file_exits_with_the_tool_error_code_and_names_the_file() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let locked = dir.path().join("locked.md");
    std::fs::write(&locked, "We leverage a robust approach.\n").unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::File::open(&locked).is_ok() {
        return; // root: permissions do not bite
    }
    let output = slop_scan().arg(dir.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("locked.md"), "{stderr}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("Nothing flagged"), "{stdout}");

    let output = slop_scan().arg("--json").arg(dir.path()).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["files_scanned"], 0);
    assert_eq!(json["errors"].as_array().unwrap().len(), 1);
}

#[test]
fn a_clean_readable_file_still_exits_zero_with_no_errors_key() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("note.md");
    std::fs::write(&path, "A short note about hills.\n").unwrap();
    let output = slop_scan().arg("--json").arg(&path).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.get("errors").is_none());
}
