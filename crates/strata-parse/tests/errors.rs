use strata_parse::parse_str;

#[test]
fn missing_semicolon_is_error() {
    let err = parse_str("<mem>", "let a = 1").unwrap_err().to_string();
    assert!(err.contains("expected Semicolon"));
}

#[test]
fn unexpected_token_top_level() {
    let err = parse_str("<mem>", "42;").unwrap_err().to_string();
    assert!(err.contains("unexpected token at top level"));
}
