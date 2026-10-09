use super::*;

fn errors(rel: &str, text: &str) -> Vec<String> {
    let mut findings = Findings::default();
    check_file(rel, text, &mut findings);
    findings.errors
}

fn lines(n: usize) -> String {
    "const C: u32 = 0;\n".repeat(n)
}

#[test]
fn file_at_limit_passes_and_over_limit_fails_with_advice() {
    assert!(errors("crates/a/src/x.rs", &lines(MAX_FILE_LINES)).is_empty());
    let errs = errors("crates/a/src/x.rs", &lines(MAX_FILE_LINES + 1));
    assert_eq!(errs.len(), 1);
    assert!(errs[0].contains("301 satır"), "{errs:?}");
    assert!(errs[0].contains("alt modüllere bölün"));
}

#[test]
fn inline_test_module_does_not_count() {
    let text = format!("{}#[cfg(test)]\n{}", lines(10), lines(400));
    assert_eq!(production_lines(&text), 10);
    assert!(errors("crates/a/src/x.rs", &text).is_empty());
}

#[test]
fn test_files_are_exempt_from_length() {
    assert!(errors("crates/a/tests/big.rs", &lines(500)).is_empty());
    assert!(errors("crates/a/src/x_tests.rs", &lines(500)).is_empty());
}

#[test]
fn allow_marker_needs_a_reason() {
    let with_reason = format!("// xtask: allow-long-file: üretilmiş tablo\n{}", lines(400));
    let mut findings = Findings::default();
    check_file("crates/a/src/x.rs", &with_reason, &mut findings);
    assert!(findings.errors.is_empty());
    assert_eq!(findings.notes.len(), 1);
    let without = format!("// xtask: allow-long-file:\n{}", lines(400));
    assert_eq!(errors("crates/a/src/x.rs", &without).len(), 1);
}

#[test]
fn long_lines_are_rejected_even_in_tests() {
    let long = format!("// {}\n", "x".repeat(MAX_LINE_WIDTH));
    let errs = errors("crates/a/tests/t.rs", &long);
    assert_eq!(errs.len(), 1);
    assert!(errs[0].starts_with("crates/a/tests/t.rs:1:"));
    let exact = format!("{}\n", "x".repeat(MAX_LINE_WIDTH));
    assert!(errors("crates/a/src/x.rs", &exact).is_empty());
}

#[test]
fn mod_rs_allows_only_reexports() {
    let ok = "//! Belge\n#[cfg(feature = \"x\")]\npub mod a;\npub use a::{\n    B,\n    C,\n};\n";
    assert!(errors("crates/a/src/m/mod.rs", ok).is_empty());
    let bad = "pub mod a;\nfn helper() {}\n";
    let errs = errors("crates/a/src/m/mod.rs", bad);
    assert_eq!(errs.len(), 1);
    assert!(errs[0].contains("fn helper()"));
    assert_eq!(errors("crates/a/src/m/mod.rs", "mod private;\n").len(), 1);
}
