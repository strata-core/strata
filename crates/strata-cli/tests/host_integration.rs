//! Integration tests for host function dispatch (Issue 011a Phase 2 & 3).
//!
//! These tests verify that Strata programs can call extern functions
//! that dispatch to real Rust host implementations, with capability
//! injection at the main() entry point and structured trace emission.

use strata_cli::eval::{
    run_module, run_module_replay, run_module_traced, run_module_traced_full, Value,
};

use std::sync::{Arc, Mutex};

fn run_ok(src: &str) -> Value {
    let module = strata_parse::parse_str("<test>", src).expect("parse failed");
    let mut tc = strata_types::TypeChecker::new();
    tc.check_module(&module).expect("type check failed");
    run_module(&module).expect("run_module failed")
}

fn run_err(src: &str) -> String {
    let module = strata_parse::parse_str("<test>", src).expect("parse failed");
    let mut tc = strata_types::TypeChecker::new();
    tc.check_module(&module).expect("type check failed");
    run_module(&module).unwrap_err().to_string()
}

#[test]
fn host_read_file_ok() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("test_read.txt");
    std::fs::write(&file_path, "hello strata").expect("write test file");

    let path_str = file_path.to_str().unwrap();
    let src = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            read_file(&fs, "{}")
        }}
        "#,
        path_str
    );

    let result = run_ok(&src);
    match result {
        Value::Str(s) => assert_eq!(s, "hello strata"),
        other => panic!("expected Str, got: {}", other),
    }
}

#[test]
fn host_write_file_ok() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("test_write.txt");
    let path_str = file_path.to_str().unwrap();

    let src = format!(
        r#"
        extern fn write_file(fs: &FsCap, path: String, content: String) -> () & {{Fs}};

        fn main(fs: FsCap) -> () & {{Fs}} {{
            write_file(&fs, "{}", "written by strata")
        }}
        "#,
        path_str
    );

    let result = run_ok(&src);
    assert!(matches!(result, Value::Unit));

    let content = std::fs::read_to_string(&file_path).expect("read back file");
    assert_eq!(content, "written by strata");
}

#[test]
fn host_now_returns_string() {
    let src = r#"
        extern fn now(t: &TimeCap) -> String & {Time};

        fn main(t: TimeCap) -> String & {Time} {
            now(&t)
        }
    "#;

    let result = run_ok(src);
    match result {
        Value::Str(s) => {
            // Should be epoch format like "1234567890.123"
            assert!(s.contains('.'), "expected epoch format with dot: {}", s);
        }
        other => panic!("expected Str, got: {}", other),
    }
}

#[test]
fn host_random_int_returns_int() {
    let src = r#"
        extern fn random_int(r: &RandCap) -> Int & {Rand};

        fn main(r: RandCap) -> Int & {Rand} {
            random_int(&r)
        }
    "#;

    let result = run_ok(src);
    match result {
        Value::Int(n) => {
            assert!((0..1000).contains(&n), "expected 0..999, got: {}", n);
        }
        other => panic!("expected Int, got: {}", other),
    }
}

#[test]
fn main_receives_caps() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("cap_test.txt");
    std::fs::write(&file_path, "cap_content").expect("write test file");
    let path_str = file_path.to_str().unwrap();

    let src = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            read_file(&fs, "{}")
        }}
        "#,
        path_str
    );

    let result = run_ok(&src);
    match result {
        Value::Str(s) => assert_eq!(s, "cap_content"),
        other => panic!("expected Str, got: {}", other),
    }
}

#[test]
fn multiple_caps() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("multi_cap.txt");
    std::fs::write(&file_path, "multi").expect("write test file");
    let path_str = file_path.to_str().unwrap();

    let src = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};
        extern fn now(t: &TimeCap) -> String & {{Time}};

        fn main(fs: FsCap, t: TimeCap) -> String & {{Fs, Time}} {{
            let _time = now(&t);
            read_file(&fs, "{}")
        }}
        "#,
        path_str
    );

    let result = run_ok(&src);
    match result {
        Value::Str(s) => assert_eq!(s, "multi"),
        other => panic!("expected Str, got: {}", other),
    }
}

#[test]
fn host_read_nonexistent_file_error() {
    let src = r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {Fs};

        fn main(fs: FsCap) -> String & {Fs} {
            read_file(&fs, "/nonexistent/path/file.txt")
        }
    "#;

    let err = run_err(src);
    assert!(
        err.contains("read_file"),
        "error should mention read_file: {}",
        err
    );
}

#[test]
fn host_write_then_read() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("roundtrip.txt");
    let path_str = file_path.to_str().unwrap();

    let src = format!(
        r#"
        extern fn write_file(fs: &FsCap, path: String, content: String) -> () & {{Fs}};
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            write_file(&fs, "{path}", "roundtrip data");
            read_file(&fs, "{path}")
        }}
        "#,
        path = path_str
    );

    let result = run_ok(&src);
    match result {
        Value::Str(s) => assert_eq!(s, "roundtrip data"),
        other => panic!("expected Str, got: {}", other),
    }
}

// =========================================================================
// Phase 3: Trace emission tests
// =========================================================================

/// Shared buffer that implements Write, allowing us to capture trace output
/// even though run_module_traced takes ownership of Box<dyn Write>.
#[derive(Clone)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl SharedBuf {
    fn new() -> Self {
        SharedBuf(Arc::new(Mutex::new(Vec::new())))
    }

    fn contents(&self) -> String {
        let bytes = self.0.lock().unwrap();
        String::from_utf8(bytes.clone()).expect("trace output should be UTF-8")
    }
}

impl std::io::Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn run_traced(src: &str) -> (Value, Vec<serde_json::Value>) {
    let module = strata_parse::parse_str("<test>", src).expect("parse failed");
    let mut tc = strata_types::TypeChecker::new();
    tc.check_module(&module).expect("type check failed");
    let buf = SharedBuf::new();
    let writer = buf.clone();
    let result = run_module_traced(&module, Box::new(writer)).expect("run_module_traced failed");
    let output = buf.contents();
    let entries: Vec<serde_json::Value> = output
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str::<serde_json::Value>(l).expect("invalid JSONL line"))
        // Filter to only effect records (skip header/footer)
        .filter(|v| v.get("record").and_then(|r| r.as_str()) == Some("effect"))
        .collect();
    (result, entries)
}

fn run_traced_err(src: &str) -> Vec<serde_json::Value> {
    let module = strata_parse::parse_str("<test>", src).expect("parse failed");
    let mut tc = strata_types::TypeChecker::new();
    tc.check_module(&module).expect("type check failed");
    let buf = SharedBuf::new();
    let writer = buf.clone();
    let _ = run_module_traced(&module, Box::new(writer));
    let output = buf.contents();
    output
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str::<serde_json::Value>(l).expect("invalid JSONL line"))
        .filter(|v| v.get("record").and_then(|r| r.as_str()) == Some("effect"))
        .collect()
}

#[test]
fn trace_records_read_file() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("trace_read.txt");
    std::fs::write(&file_path, "traced content").expect("write test file");
    let path_str = file_path.to_str().unwrap();

    let src = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            read_file(&fs, "{}")
        }}
        "#,
        path_str
    );

    let (result, entries) = run_traced(&src);
    assert!(matches!(result, Value::Str(_)));
    assert_eq!(
        entries.len(),
        1,
        "expected 1 trace entry, got {}",
        entries.len()
    );

    let entry = &entries[0];
    assert_eq!(entry["seq"], 0);
    assert_eq!(entry["effect"], "Fs");
    assert_eq!(entry["operation"], "read_file");
    assert_eq!(entry["capability"]["kind"], "FsCap");
    assert_eq!(entry["capability"]["access"], "borrow");
    // Inputs are now tagged TraceValue objects: {"t":"Str","v":"..."}
    assert_eq!(entry["inputs"]["path"]["t"], "Str");
    assert_eq!(entry["inputs"]["path"]["v"], path_str);
    assert_eq!(entry["output"]["status"], "ok");
    // Output value is now a tagged TraceValue
    assert_eq!(entry["output"]["value"]["t"], "Str");
    assert_eq!(entry["output"]["value"]["v"], "traced content");
    assert!(entry["output"]["value_hash"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert!(entry["output"]["value_size"].as_u64().unwrap() > 0);
    assert!(entry["duration_ms"].is_u64());
    assert!(entry["timestamp"].as_str().unwrap().contains('T'));
}

#[test]
fn trace_records_write_file() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("trace_write.txt");
    let path_str = file_path.to_str().unwrap();

    let src = format!(
        r#"
        extern fn write_file(fs: &FsCap, path: String, content: String) -> () & {{Fs}};

        fn main(fs: FsCap) -> () & {{Fs}} {{
            write_file(&fs, "{}", "hello trace")
        }}
        "#,
        path_str
    );

    let (_result, entries) = run_traced(&src);
    assert_eq!(entries.len(), 1);

    let entry = &entries[0];
    assert_eq!(entry["effect"], "Fs");
    assert_eq!(entry["operation"], "write_file");
    assert_eq!(entry["capability"]["kind"], "FsCap");
    assert_eq!(entry["capability"]["access"], "borrow");
    assert_eq!(entry["inputs"]["path"]["t"], "Str");
    assert_eq!(entry["inputs"]["path"]["v"], path_str);
    assert_eq!(entry["inputs"]["content"]["t"], "Str");
    assert_eq!(entry["inputs"]["content"]["v"], "hello trace");
    assert_eq!(entry["output"]["status"], "ok");
    assert_eq!(entry["output"]["value"]["t"], "Unit");
}

#[test]
fn trace_hashes_large_output() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("trace_large.txt");
    // Write >1024 bytes so value field is omitted
    let large_content = "x".repeat(2000);
    std::fs::write(&file_path, &large_content).expect("write test file");
    let path_str = file_path.to_str().unwrap();

    let src = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            read_file(&fs, "{}")
        }}
        "#,
        path_str
    );

    let (result, entries) = run_traced(&src);
    match &result {
        Value::Str(s) => assert_eq!(s.len(), 2000),
        other => panic!("expected Str, got: {}", other),
    }
    assert_eq!(entries.len(), 1);

    let entry = &entries[0];
    assert_eq!(entry["output"]["status"], "ok");
    // value should be absent (null in JSON) because output > 1024 bytes
    assert!(
        entry["output"]["value"].is_null(),
        "large output should omit value"
    );
    assert!(entry["output"]["value_hash"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert_eq!(entry["output"]["value_size"], 2000);
}

#[test]
fn trace_includes_small_output() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("trace_small.txt");
    std::fs::write(&file_path, "small").expect("write test file");
    let path_str = file_path.to_str().unwrap();

    let src = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            read_file(&fs, "{}")
        }}
        "#,
        path_str
    );

    let (_result, entries) = run_traced(&src);
    assert_eq!(entries.len(), 1);

    let entry = &entries[0];
    assert_eq!(entry["output"]["status"], "ok");
    assert_eq!(entry["output"]["value"]["t"], "Str");
    assert_eq!(entry["output"]["value"]["v"], "small");
    assert_eq!(entry["output"]["value_size"], 5);
}

#[test]
fn trace_records_error() {
    let src = r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {Fs};

        fn main(fs: FsCap) -> String & {Fs} {
            read_file(&fs, "/nonexistent/trace_error_test.txt")
        }
    "#;

    let entries = run_traced_err(src);
    assert_eq!(entries.len(), 1);

    let entry = &entries[0];
    assert_eq!(entry["output"]["status"], "error");
    assert_eq!(entry["output"]["value"]["t"], "Str");
    let err_value = entry["output"]["value"]["v"].as_str().unwrap();
    assert!(
        err_value.contains("read_file"),
        "error should mention read_file: {}",
        err_value
    );
    assert!(entry["output"]["value_hash"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
}

#[test]
fn trace_sequential_seq_numbers() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file1 = dir.path().join("seq1.txt");
    let file2 = dir.path().join("seq2.txt");
    std::fs::write(&file1, "one").expect("write");
    std::fs::write(&file2, "two").expect("write");
    let path1 = file1.to_str().unwrap();
    let path2 = file2.to_str().unwrap();

    let src = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            let _a = read_file(&fs, "{p1}");
            read_file(&fs, "{p2}")
        }}
        "#,
        p1 = path1,
        p2 = path2
    );

    let (_result, entries) = run_traced(&src);
    assert_eq!(entries.len(), 2, "expected 2 trace entries");
    assert_eq!(entries[0]["seq"], 0);
    assert_eq!(entries[1]["seq"], 1);
}

#[test]
fn trace_disabled_no_output() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("no_trace.txt");
    std::fs::write(&file_path, "no trace").expect("write test file");
    let path_str = file_path.to_str().unwrap();

    let src = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            read_file(&fs, "{}")
        }}
        "#,
        path_str
    );

    // run_module (not traced) should produce no trace output
    let result = run_ok(&src);
    assert!(matches!(result, Value::Str(_)));
    // No way to check "no output" here beyond verifying it didn't crash.
    // The real check: run_module doesn't use a tracer at all.
}

// =========================================================================
// Phase 4: Deterministic replay tests
// =========================================================================

/// Helper: run a program with full tracing, then replay the trace.
/// Uses run_module_traced_full so the trace is replay-capable.
fn trace_and_replay(src: &str) -> (Value, Value) {
    let module = strata_parse::parse_str("<test>", src).expect("parse failed");
    let mut tc = strata_types::TypeChecker::new();
    tc.check_module(&module).expect("type check failed");

    let buf = SharedBuf::new();
    let writer = buf.clone();
    let live_result = run_module_traced_full(&module, Box::new(writer)).expect("live run failed");

    let trace = buf.contents();
    let replay_result = run_module_replay(&module, &trace).expect("replay failed");

    (live_result, replay_result)
}

#[test]
fn replay_produces_same_result() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("replay_ok.txt");
    std::fs::write(&file_path, "replay content").expect("write test file");
    let path_str = file_path.to_str().unwrap();

    let src = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            read_file(&fs, "{}")
        }}
        "#,
        path_str
    );

    let (live, replay) = trace_and_replay(&src);
    match (&live, &replay) {
        (Value::Str(a), Value::Str(b)) => assert_eq!(a, b),
        _ => panic!(
            "expected matching Str values, got live={}, replay={}",
            live, replay
        ),
    }
}

#[test]
fn replay_detects_input_mismatch() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file1 = dir.path().join("input_match.txt");
    std::fs::write(&file1, "data").expect("write");
    let path1 = file1.to_str().unwrap();

    // Record a trace reading file1
    let src_live = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            read_file(&fs, "{}")
        }}
        "#,
        path1
    );

    let module_live = strata_parse::parse_str("<test>", &src_live).expect("parse");
    let mut tc = strata_types::TypeChecker::new();
    tc.check_module(&module_live).expect("typecheck");
    let buf = SharedBuf::new();
    let writer = buf.clone();
    let _ = run_module_traced_full(&module_live, Box::new(writer)).expect("live run");
    let trace = buf.contents();

    // Replay with a different path
    let src_replay = r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {Fs};

        fn main(fs: FsCap) -> String & {Fs} {
            read_file(&fs, "/different/path.txt")
        }
    "#;

    let module_replay = strata_parse::parse_str("<test>", src_replay).expect("parse");
    let mut tc2 = strata_types::TypeChecker::new();
    tc2.check_module(&module_replay).expect("typecheck");
    let err = run_module_replay(&module_replay, &trace)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("input mismatch"),
        "expected input mismatch error, got: {}",
        err
    );
}

#[test]
fn replay_detects_operation_mismatch() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file1 = dir.path().join("op_match.txt");
    std::fs::write(&file1, "data").expect("write");
    let path1 = file1.to_str().unwrap();

    // Record a trace with read_file
    let src_live = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            read_file(&fs, "{}")
        }}
        "#,
        path1
    );

    let module_live = strata_parse::parse_str("<test>", &src_live).expect("parse");
    let mut tc = strata_types::TypeChecker::new();
    tc.check_module(&module_live).expect("typecheck");
    let buf = SharedBuf::new();
    let writer = buf.clone();
    let _ = run_module_traced_full(&module_live, Box::new(writer)).expect("live run");
    let trace = buf.contents();

    // Replay with write_file instead
    let src_replay = format!(
        r#"
        extern fn write_file(fs: &FsCap, path: String, content: String) -> () & {{Fs}};

        fn main(fs: FsCap) -> () & {{Fs}} {{
            write_file(&fs, "{}", "new content")
        }}
        "#,
        path1
    );

    let module_replay = strata_parse::parse_str("<test>", &src_replay).expect("parse");
    let mut tc2 = strata_types::TypeChecker::new();
    tc2.check_module(&module_replay).expect("typecheck");
    let err = run_module_replay(&module_replay, &trace)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("operation mismatch"),
        "expected operation mismatch error, got: {}",
        err
    );
}

#[test]
fn replay_detects_extra_effects() {
    // Trace has 2 calls, program makes 1 → UnreplayedEffects
    let dir = tempfile::tempdir().expect("create tempdir");
    let file1 = dir.path().join("extra1.txt");
    let file2 = dir.path().join("extra2.txt");
    std::fs::write(&file1, "one").expect("write");
    std::fs::write(&file2, "two").expect("write");
    let path1 = file1.to_str().unwrap();
    let path2 = file2.to_str().unwrap();

    // Record trace with 2 reads
    let src_live = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            let _a = read_file(&fs, "{p1}");
            read_file(&fs, "{p2}")
        }}
        "#,
        p1 = path1,
        p2 = path2
    );

    let module_live = strata_parse::parse_str("<test>", &src_live).expect("parse");
    let mut tc = strata_types::TypeChecker::new();
    tc.check_module(&module_live).expect("typecheck");
    let buf = SharedBuf::new();
    let writer = buf.clone();
    let _ = run_module_traced_full(&module_live, Box::new(writer)).expect("live run");
    let trace = buf.contents();

    // Replay with only 1 read
    let src_replay = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            read_file(&fs, "{p1}")
        }}
        "#,
        p1 = path1
    );

    let module_replay = strata_parse::parse_str("<test>", &src_replay).expect("parse");
    let mut tc2 = strata_types::TypeChecker::new();
    tc2.check_module(&module_replay).expect("typecheck");
    let err = run_module_replay(&module_replay, &trace)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("unreplayed"),
        "expected unreplayed effects error, got: {}",
        err
    );
}

#[test]
fn replay_detects_missing_effects() {
    // Trace has 1 call, program makes 2 → UnexpectedEffect
    let dir = tempfile::tempdir().expect("create tempdir");
    let file1 = dir.path().join("missing1.txt");
    std::fs::write(&file1, "only").expect("write");
    let path1 = file1.to_str().unwrap();

    // Record trace with 1 read
    let src_live = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            read_file(&fs, "{p1}")
        }}
        "#,
        p1 = path1
    );

    let module_live = strata_parse::parse_str("<test>", &src_live).expect("parse");
    let mut tc = strata_types::TypeChecker::new();
    tc.check_module(&module_live).expect("typecheck");
    let buf = SharedBuf::new();
    let writer = buf.clone();
    let _ = run_module_traced_full(&module_live, Box::new(writer)).expect("live run");
    let trace = buf.contents();

    // Replay with 2 reads
    let src_replay = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            let _a = read_file(&fs, "{p1}");
            read_file(&fs, "{p1}")
        }}
        "#,
        p1 = path1
    );

    let module_replay = strata_parse::parse_str("<test>", &src_replay).expect("parse");
    let mut tc2 = strata_types::TypeChecker::new();
    tc2.check_module(&module_replay).expect("typecheck");
    let err = run_module_replay(&module_replay, &trace)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("unexpected extern call") || err.contains("not in trace"),
        "expected unexpected effect error, got: {}",
        err
    );
}

#[test]
fn replay_handles_errors() {
    // Record a trace that contains an error, then replay and verify same error
    let src = r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {Fs};

        fn main(fs: FsCap) -> String & {Fs} {
            read_file(&fs, "/nonexistent/replay_error.txt")
        }
    "#;

    let module = strata_parse::parse_str("<test>", src).expect("parse");
    let mut tc = strata_types::TypeChecker::new();
    tc.check_module(&module).expect("typecheck");

    // Live run — capture trace (program errors)
    let buf = SharedBuf::new();
    let writer = buf.clone();
    let live_err = run_module_traced_full(&module, Box::new(writer))
        .unwrap_err()
        .to_string();
    let trace = buf.contents();

    // Replay — should produce same error
    let replay_err = run_module_replay(&module, &trace).unwrap_err().to_string();

    assert!(
        replay_err.contains("read_file"),
        "replayed error should mention read_file: {}",
        replay_err
    );
    // Both should contain the same host error message
    assert!(
        live_err.contains("read_file") && replay_err.contains("read_file"),
        "live='{}', replay='{}'",
        live_err,
        replay_err
    );
}

#[test]
fn trace_write_failure_aborts_execution() {
    // A writer that fails on the second write (header succeeds, first effect fails).
    struct FailWriter {
        writes: std::sync::atomic::AtomicU32,
    }
    impl std::io::Write for FailWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let n = self
                .writes
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n >= 1 {
                // Fail on second write (first effect entry)
                Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "disk full",
                ))
            } else {
                Ok(buf.len())
            }
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("fail_trace.txt");
    std::fs::write(&file_path, "fail data").expect("write test file");
    let path_str = file_path.to_str().unwrap();

    let src = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            read_file(&fs, "{}")
        }}
        "#,
        path_str
    );

    let module = strata_parse::parse_str("<test>", &src).expect("parse");
    let mut tc = strata_types::TypeChecker::new();
    tc.check_module(&module).expect("typecheck");

    let writer = FailWriter {
        writes: std::sync::atomic::AtomicU32::new(0),
    };
    let err = run_module_traced_full(&module, Box::new(writer))
        .unwrap_err()
        .to_string();

    assert!(
        err.contains("trace write error") || err.contains("disk full"),
        "expected trace write error, got: {}",
        err
    );
}

#[test]
fn replay_rejects_audit_trace() {
    // Traces recorded with --trace (audit mode) cannot be replayed
    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("audit_trace.txt");
    std::fs::write(&file_path, "audit data").expect("write test file");
    let path_str = file_path.to_str().unwrap();

    let src = format!(
        r#"
        extern fn read_file(fs: &FsCap, path: String) -> String & {{Fs}};

        fn main(fs: FsCap) -> String & {{Fs}} {{
            read_file(&fs, "{}")
        }}
        "#,
        path_str
    );

    let module = strata_parse::parse_str("<test>", &src).expect("parse");
    let mut tc = strata_types::TypeChecker::new();
    tc.check_module(&module).expect("typecheck");

    // Record audit trace (not replay-capable)
    let buf = SharedBuf::new();
    let writer = buf.clone();
    let _ = run_module_traced(&module, Box::new(writer)).expect("live run");
    let trace = buf.contents();

    // Replay should fail with NotReplayable
    let err = run_module_replay(&module, &trace).unwrap_err().to_string();
    assert!(
        err.contains("not replayable") || err.contains("--trace-full"),
        "expected not-replayable error, got: {}",
        err
    );
}

// =========================================================================
// Fix 5 security tests: TraceValue rejects non-data types
// =========================================================================

#[test]
fn replay_rejects_cap_in_trace_output() {
    // TraceValue only has Int/Float/Str/Bool/Unit — a crafted "Cap" tag must fail.
    use strata_cli::host::TraceValue;
    let json = r#"{"t":"Cap","v":"FsCap"}"#;
    let result = serde_json::from_str::<TraceValue>(json);
    assert!(
        result.is_err(),
        "TraceValue should reject Cap tag: {:?}",
        result
    );
}

#[test]
fn replay_rejects_closure_in_trace_output() {
    use strata_cli::host::TraceValue;
    let json = r#"{"t":"Closure","v":{"params":["x"],"body":"x+1"}}"#;
    let result = serde_json::from_str::<TraceValue>(json);
    assert!(
        result.is_err(),
        "TraceValue should reject Closure tag: {:?}",
        result
    );
}

#[test]
fn replay_rejects_tuple_in_trace_output() {
    use strata_cli::host::TraceValue;
    let json = r#"{"t":"Tuple","v":[1,2,3]}"#;
    let result = serde_json::from_str::<TraceValue>(json);
    assert!(
        result.is_err(),
        "TraceValue should reject Tuple tag: {:?}",
        result
    );
}

// =========================================================================
// Fix 6 security tests: schema version and footer completeness
// =========================================================================

#[test]
fn replay_rejects_unknown_version() {
    use strata_cli::host::TraceReplayer;
    // Craft a trace with schema_version "99.0" — must be rejected.
    let trace = r#"{"record":"header","schema_version":"99.0","timestamp":"2026-01-01T00:00:00.000Z","full_values":true}
{"record":"effect","seq":0,"timestamp":"2026-01-01T00:00:00.001Z","effect":"Fs","operation":"read_file","capability":{"kind":"FsCap","access":"borrow"},"inputs":{"path":{"t":"Str","v":"/tmp/x"}},"output":{"status":"ok","value":{"t":"Str","v":"data"},"value_hash":"sha256:abc","value_size":4},"duration_ms":1,"full_values":true}
{"record":"footer","timestamp":"2026-01-01T00:00:00.002Z","effect_count":1,"trace_status":"complete","program_status":"success"}"#;

    let err = TraceReplayer::from_jsonl(trace).unwrap_err().to_string();
    assert!(
        err.contains("unsupported trace schema version") || err.contains("99.0"),
        "expected version rejection, got: {}",
        err
    );
}

#[test]
fn replay_detects_missing_footer() {
    use strata_cli::host::TraceReplayer;
    // Truncated trace: header + effect but no footer.
    let trace = r#"{"record":"header","schema_version":"0.1","timestamp":"2026-01-01T00:00:00.000Z","full_values":true}
{"record":"effect","seq":0,"timestamp":"2026-01-01T00:00:00.001Z","effect":"Fs","operation":"read_file","capability":{"kind":"FsCap","access":"borrow"},"inputs":{"path":{"t":"Str","v":"/tmp/x"}},"output":{"status":"ok","value":{"t":"Str","v":"data"},"value_hash":"sha256:abc","value_size":4},"duration_ms":1,"full_values":true}"#;

    let replayer = TraceReplayer::from_jsonl(trace).expect("should parse truncated trace");
    assert!(
        !replayer.is_trace_complete(),
        "truncated trace should not be marked complete"
    );
}

// =========================================================================
// Fix 2 security test: tagged value type confusion
// =========================================================================

#[test]
fn trace_tagged_str_not_confused_with_int() {
    use strata_cli::host::TraceValue;

    let str_val = TraceValue::Str("42".to_string());
    let int_val = TraceValue::Int(42);

    // They must serialize to different JSON
    let str_json = serde_json::to_string(&str_val).unwrap();
    let int_json = serde_json::to_string(&int_val).unwrap();
    assert_ne!(
        str_json, int_json,
        "Str(\"42\") and Int(42) must serialize differently"
    );

    // Round-trip preserves the distinction
    let str_back: TraceValue = serde_json::from_str(&str_json).unwrap();
    let int_back: TraceValue = serde_json::from_str(&int_json).unwrap();
    assert_eq!(str_back, TraceValue::Str("42".to_string()));
    assert_eq!(int_back, TraceValue::Int(42));
    assert_ne!(str_back, int_back);
}
