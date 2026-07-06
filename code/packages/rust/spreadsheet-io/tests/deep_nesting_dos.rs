//! DoS guard: a deeply-nested JSON document (adversarial untrusted input) must
//! return an error, never overflow the native thread stack.
//!
//! `load_json` feeds untrusted bytes into the recursive-descent grammar parser,
//! which recurses once per nesting level. Without a depth cap, an input like
//! `[[[…]]]` a few hundred levels deep overflows the stack — an *uncatchable*
//! SIGSEGV that aborts the whole process, defeating the panic-free contract.
//! `json-parser` caps recursion at `DEFAULT_MAX_RULE_DEPTH` (128), so such input
//! is refused as an ordinary `IoError::Json`. This test pins that at the codec
//! level: 100 000 levels deep resolves to an error in milliseconds, no crash.

#[test]
fn deeply_nested_json_is_rejected_not_crash() {
    let depth = 100_000;
    let mut s = String::with_capacity(depth * 2);
    for _ in 0..depth {
        s.push('[');
    }
    for _ in 0..depth {
        s.push(']');
    }
    match spreadsheet_io::load_json(s.as_bytes()) {
        Ok(_) => panic!("a 100k-deep array must not parse as a valid workbook"),
        Err(e) => assert!(matches!(e, spreadsheet_io::IoError::Json(_))),
    }
}
