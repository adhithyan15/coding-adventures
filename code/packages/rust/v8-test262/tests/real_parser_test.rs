//! The parse level driven by the real `javascript-parser`, on the crate's
//! hand-written fixtures. The unit tests pin the runner's rules with a stub;
//! this checks the real parser is wired in and judged the same way.

use std::path::Path;

use v8_test262::{javascript_parse, run_parse_level, Outcome, Suite};

#[test]
fn the_real_parser_accepts_valid_programs_and_rejects_invalid_ones() {
    let suite = Suite::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"));
    let paths = suite.test_paths(&["language/parse/accepts.js".to_string(), "language/parse/negative-rejected.js".to_string(), "language/parse/valid-rejected.js".to_string()]).unwrap();
    let results = run_parse_level(&suite, &paths, &javascript_parse);

    for id in ["language/parse/accepts.js#sloppy", "language/parse/accepts.js#strict"] {
        assert_eq!(results.outcomes[id], Outcome::Pass, "{id}");
    }
    // `@@reject@@` is not JavaScript: the negative test passes because the
    // parser rejects it, and the "valid" one fails for the same reason.
    assert_eq!(results.outcomes["language/parse/negative-rejected.js#sloppy"], Outcome::Pass);
    assert!(matches!(
        results.outcomes["language/parse/valid-rejected.js#strict"],
        Outcome::Fail(_)
    ));
}
