//! Integration tests for host function dispatch (Issue 011a Phase 2 & 3).
//!
//! These tests verify that Strata programs can call extern functions
//! that dispatch to real Rust host implementations, with capability
//! injection at the main() entry point and structured trace emission.

use strata_cli::eval::{run_module, run_module_traced, Value};

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
    assert!(err.contains("read_file"), "error should mention read_file: {}", err);
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
        .map(|l| serde_json::from_str(l).expect("invalid JSONL line"))
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
        .map(|l| serde_json::from_str(l).expect("invalid JSONL line"))
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
    assert_eq!(entries.len(), 1, "expected 1 trace entry, got {}", entries.len());

    let entry = &entries[0];
    assert_eq!(entry["seq"], 0);
    assert_eq!(entry["effect"], "Fs");
    assert_eq!(entry["operation"], "read_file");
    assert_eq!(entry["capability"]["kind"], "FsCap");
    assert_eq!(entry["capability"]["access"], "borrow");
    assert_eq!(entry["inputs"]["path"], path_str);
    assert_eq!(entry["output"]["status"], "ok");
    assert_eq!(entry["output"]["value"], "traced content");
    assert!(entry["output"]["value_hash"].as_str().unwrap().starts_with("sha256:"));
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
    assert_eq!(entry["inputs"]["path"], path_str);
    assert_eq!(entry["inputs"]["content"], "hello trace");
    assert_eq!(entry["output"]["status"], "ok");
    assert_eq!(entry["output"]["value"], "()");
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
    assert!(entry["output"]["value"].is_null(), "large output should omit value");
    assert!(entry["output"]["value_hash"].as_str().unwrap().starts_with("sha256:"));
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
    assert_eq!(entry["output"]["value"], "small");
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
    let err_value = entry["output"]["value"].as_str().unwrap();
    assert!(err_value.contains("read_file"), "error should mention read_file: {}", err_value);
    assert!(entry["output"]["value_hash"].as_str().unwrap().starts_with("sha256:"));
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
