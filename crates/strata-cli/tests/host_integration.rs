//! Integration tests for host function dispatch (Issue 011a Phase 2).
//!
//! These tests verify that Strata programs can call extern functions
//! that dispatch to real Rust host implementations, with capability
//! injection at the main() entry point.

use strata_cli::eval::{run_module, Value};

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
