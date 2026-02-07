//! CLI integration tests for strata run / strata replay (Issue 011a Phase 5).
//!
//! These tests invoke the compiled binary to verify end-to-end behavior.

use std::process::Command;

fn strata_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_strata-cli"))
}

#[test]
fn cli_run_no_trace() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file = dir.path().join("simple.strata");
    std::fs::write(
        &file,
        r#"
        fn main() -> Int {
            1 + 2
        }
        "#,
    )
    .expect("write source");

    let output = strata_bin()
        .args(["run", file.to_str().unwrap()])
        .output()
        .expect("run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "strata run should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("main() = 3"),
        "stdout should contain result: {}",
        stdout
    );
}

#[test]
fn cli_run_with_trace() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let source = dir.path().join("traced.strata");
    let input = dir.path().join("input.txt");
    let trace = dir.path().join("trace.jsonl");
    std::fs::write(&input, "hello trace cli").expect("write input");

    let src = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            read_file(&fs, "{}")
        }}
        "#,
        input.to_str().unwrap()
    );
    std::fs::write(&source, &src).expect("write source");

    let output = strata_bin()
        .args([
            "run",
            source.to_str().unwrap(),
            "--trace",
            trace.to_str().unwrap(),
        ])
        .output()
        .expect("run binary");

    assert!(
        output.status.success(),
        "strata run --trace should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify trace file was created with valid JSONL
    let trace_content = std::fs::read_to_string(&trace).expect("read trace file");
    let lines: Vec<&str> = trace_content.lines().filter(|l| !l.is_empty()).collect();
    assert!(!lines.is_empty(), "trace should have at least one entry");

    let entry: serde_json::Value = serde_json::from_str(lines[0]).expect("parse JSONL line");
    assert_eq!(entry["operation"], "read_file");
    assert_eq!(entry["output"]["status"], "ok");
}

#[test]
fn cli_replay_success() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let source = dir.path().join("replay_ok.strata");
    let input = dir.path().join("data.txt");
    let trace = dir.path().join("trace.jsonl");
    std::fs::write(&input, "replay data").expect("write input");

    let src = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            read_file(&fs, "{}")
        }}
        "#,
        input.to_str().unwrap()
    );
    std::fs::write(&source, &src).expect("write source");

    // Run with trace-full (replay-capable)
    let run_output = strata_bin()
        .args([
            "run",
            source.to_str().unwrap(),
            "--trace-full",
            trace.to_str().unwrap(),
        ])
        .output()
        .expect("run binary");
    assert!(
        run_output.status.success(),
        "run should succeed: {}",
        String::from_utf8_lossy(&run_output.stderr)
    );

    // Replay
    let replay_output = strata_bin()
        .args([
            "replay",
            trace.to_str().unwrap(),
            source.to_str().unwrap(),
        ])
        .output()
        .expect("replay binary");

    let stdout = String::from_utf8_lossy(&replay_output.stdout);
    assert!(
        replay_output.status.success(),
        "replay should succeed, stderr: {}",
        String::from_utf8_lossy(&replay_output.stderr)
    );
    assert!(
        stdout.contains("Replay successful"),
        "stdout should report success: {}",
        stdout
    );
}

#[test]
fn cli_replay_mismatch() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let source_v1 = dir.path().join("v1.strata");
    let source_v2 = dir.path().join("v2.strata");
    let input = dir.path().join("file.txt");
    let trace = dir.path().join("trace.jsonl");
    std::fs::write(&input, "original").expect("write input");

    let src_v1 = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            read_file(&fs, "{}")
        }}
        "#,
        input.to_str().unwrap()
    );
    std::fs::write(&source_v1, &src_v1).expect("write v1 source");

    // Record trace from v1
    let run_output = strata_bin()
        .args([
            "run",
            source_v1.to_str().unwrap(),
            "--trace-full",
            trace.to_str().unwrap(),
        ])
        .output()
        .expect("run binary");
    assert!(run_output.status.success());

    // v2 reads a different path
    let src_v2 = r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {Fs};

        fn main(fs: FsCap) -> String & {Fs} {
            read_file(&fs, "/different/path.txt")
        }
    "#;
    std::fs::write(&source_v2, src_v2).expect("write v2 source");

    // Replay v2 against v1's trace — should fail
    let replay_output = strata_bin()
        .args([
            "replay",
            trace.to_str().unwrap(),
            source_v2.to_str().unwrap(),
        ])
        .output()
        .expect("replay binary");

    assert!(
        !replay_output.status.success(),
        "replay should fail on mismatch"
    );

    let stderr = String::from_utf8_lossy(&replay_output.stderr);
    assert!(
        stderr.contains("input mismatch") || stderr.contains("mismatch"),
        "stderr should mention mismatch: {}",
        stderr
    );
}
