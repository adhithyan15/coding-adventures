//! # v8-test262 — the conformance gate for the V8C JavaScript engine
//!
//! [test262](https://github.com/tc39/test262) is TC39's conformance suite for
//! ECMAScript. This crate runs it (spec: `code/specs/V8C09-test262-runner.md`).
//! It exists *before* the engine does: its first level needs only the parser.
//!
//! ```text
//!   test262/test/**/*.js
//!        │  header::parse_header        what the test expects
//!        ▼
//!   variants::variants                  sloppy + strict, or one of them
//!        │
//!        ▼
//!   judge (per level)                   parse: accept / reject; run: later
//!        │
//!        ▼
//!   expected::compare                   the grow-only expected-pass list
//! ```
//!
//! ## The one rule
//!
//! A test passes because the engine did the right thing, never because the
//! runner recognised the test. Nothing here looks at a test's path,
//! description or text to decide its outcome (V8C01 revision, point 4). A test
//! the current level cannot judge is a `Skip` with its reason, never a `Pass`.

pub mod expected;
pub mod header;
pub mod variants;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use coding_adventures_javascript_parser::parse_javascript_typed;
use coding_adventures_javascript_tokens::EsVersion;

use header::{parse_header, Metadata, Phase};
use variants::{harness_files, result_id, variants, Variant};

/// The edition every test is parsed as. test262 tracks the living standard;
/// the newest grammar is the right one.
pub const ES_VERSION: EsVersion = EsVersion::Es2025;

/// The result of one run of one test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    /// Did the wrong thing; the message says what.
    Fail(String),
    /// This level cannot judge it; the reason says why. Never counted as a pass.
    Skip(String),
}

/// How to parse: a function from source text to accept (`Ok`) or reject
/// (`Err(message)`). The real parser in production; a stub in unit tests.
pub type ParseFn = dyn Fn(&str) -> Result<(), String> + Sync;

/// The largest test or harness file the runner reads. test262 files are a few
/// kilobytes; anything far larger is not a test262 file and is failed, not read.
pub const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;

/// The real parser, run on its own thread with a generous stack. A parser
/// **panic** is caught and reported as a rejected parse. A stack **overflow**
/// is not catchable in Rust -- it aborts the process -- so the large stack
/// makes it unlikely rather than impossible; `MAX_FILE_BYTES` bounds the input.
pub fn javascript_parse(source: &str) -> Result<(), String> {
    let source = source.to_string();
    let worker = std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(move || parse_javascript_typed(&source, ES_VERSION).map(|_| ()))
        .map_err(|error| format!("could not start the parser thread: {error}"))?;
    match worker.join() {
        Ok(result) => result,
        Err(_) => Err("the parser panicked".to_string()),
    }
}

/// A test262 checkout: `test/` holds the tests, `harness/` the includes.
pub struct Suite {
    root: PathBuf,
}

impl Suite {
    pub fn new(root: impl Into<PathBuf>) -> Suite {
        Suite { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Every test file under `test/`, as paths relative to `test/` with `/`
    /// separators, sorted. `*_FIXTURE.js` files are imported by module tests,
    /// not tests themselves, and are left out. `filters` keeps only paths that
    /// start with one of them (all when empty).
    pub fn test_paths(&self, filters: &[String]) -> std::io::Result<Vec<String>> {
        let test_root = self.root.join("test");
        let mut paths = Vec::new();
        collect_js(&test_root, &test_root, &mut paths)?;
        paths.retain(|path| !path.ends_with("_FIXTURE.js"));
        if !filters.is_empty() {
            paths.retain(|path| filters.iter().any(|filter| path.starts_with(filter.as_str())));
        }
        paths.sort();
        Ok(paths)
    }
}

fn collect_js(root: &Path, dir: &Path, out: &mut Vec<String>) -> std::io::Result<()> {
    let mut entries = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect_js(root, &path, out)?;
        } else if kind.is_file()
            && path.extension().is_some_and(|ext| ext == "js")
            && !entry.file_name().to_string_lossy().chars().any(char::is_control)
        {
            let relative = path.strip_prefix(root).expect("walked under root");
            out.push(
                relative
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/"),
            );
        }
    }
    Ok(())
}

/// Every judged result of a run, by id.
#[derive(Debug, Default)]
pub struct Results {
    pub outcomes: BTreeMap<String, Outcome>,
}

impl Results {
    pub fn passes(&self) -> BTreeSet<String> {
        self.ids_where(|outcome| *outcome == Outcome::Pass)
    }

    /// Ids this run judged, pass or fail. Skips are not judged.
    pub fn judged(&self) -> BTreeSet<String> {
        self.ids_where(|outcome| !matches!(outcome, Outcome::Skip(_)))
    }

    fn ids_where(&self, keep: impl Fn(&Outcome) -> bool) -> BTreeSet<String> {
        self.outcomes
            .iter()
            .filter(|(_, outcome)| keep(outcome))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Pass / fail / skip counts per area: the first two path segments
    /// (`language/expressions`, `built-ins/Array`).
    pub fn summary(&self) -> BTreeMap<String, [usize; 3]> {
        let mut summary: BTreeMap<String, [usize; 3]> = BTreeMap::new();
        for (id, outcome) in &self.outcomes {
            let area = id.split('/').take(2).collect::<Vec<_>>().join("/");
            let slot = match outcome {
                Outcome::Pass => 0,
                Outcome::Fail(_) => 1,
                Outcome::Skip(_) => 2,
            };
            summary.entry(area).or_default()[slot] += 1;
        }
        summary
    }

    /// Skip reasons and how often each occurred.
    pub fn skip_reasons(&self) -> BTreeMap<String, usize> {
        let mut reasons = BTreeMap::new();
        for outcome in self.outcomes.values() {
            if let Outcome::Skip(reason) = outcome {
                *reasons.entry(reason.clone()).or_default() += 1;
            }
        }
        reasons
    }
}

/// Run the **parse level** (V8C09 §3) over `paths`.
///
/// No code runs. A test whose `negative.phase` is `parse` passes when the
/// parser rejects it; every other test passes when the parser accepts it and
/// every harness file it includes. Runtime-negative tests must still parse.
pub fn run_parse_level(suite: &Suite, paths: &[String], parse: &ParseFn) -> Results {
    let mut results = Results::default();
    let mut harness_cache: HashMap<String, Result<(), String>> = HashMap::new();

    for path in paths {
        let full = suite.root.join("test").join(path);
        let source = match read_bounded(&full) {
            Ok(source) => source,
            Err(error) => {
                results
                    .outcomes
                    .insert(path.clone(), Outcome::Fail(format!("unreadable: {error}")));
                continue;
            }
        };
        let metadata = match parse_header(&source) {
            Ok(metadata) => metadata,
            Err(error) => {
                results.outcomes.insert(path.clone(), Outcome::Fail(format!("header: {error}")));
                continue;
            }
        };
        let runs = variants(&metadata);
        if runs.is_empty() {
            results.outcomes.insert(
                path.clone(),
                Outcome::Fail("header asks for both onlyStrict and noStrict".to_string()),
            );
            continue;
        }
        for variant in runs {
            let outcome = judge_parse(suite, &metadata, variant, &source, parse, &mut harness_cache);
            results.outcomes.insert(result_id(path, variant), outcome);
        }
    }
    results
}

fn judge_parse(
    suite: &Suite,
    metadata: &Metadata,
    variant: Variant,
    source: &str,
    parse: &ParseFn,
    harness_cache: &mut HashMap<String, Result<(), String>>,
) -> Outcome {
    if variant == Variant::Module {
        return Outcome::Skip("module goal: javascript-parser parses scripts only".to_string());
    }
    let text = match variant {
        Variant::Strict => format!("\"use strict\";\n{source}"),
        _ => source.to_string(),
    };

    if let Some(negative) = &metadata.negative {
        match negative.phase {
            Phase::Parse => {
                return match parse(&text) {
                    Err(_) => Outcome::Pass,
                    Ok(()) => Outcome::Fail(format!(
                        "accepted a program that must be a parse-time {}",
                        negative.error_type
                    )),
                };
            }
            Phase::Resolution => {
                return Outcome::Skip("resolution phase: module linking not implemented".to_string());
            }
            // A runtime-negative test is a valid program: it must parse.
            Phase::Runtime => {}
        }
    }

    if let Err(error) = parse(&text) {
        return Outcome::Fail(format!("rejected a valid program: {}", first_line(&error)));
    }
    for file in harness_files(metadata, variant) {
        // Every test262 include is a plain file name at the top of `harness/`.
        // Anything else -- a path, `..`, an absolute path -- could name a file
        // outside the suite (or a device such as /dev/zero), so it fails the
        // test instead of being read.
        if !is_bare_file_name(&file) {
            return Outcome::Fail(format!("harness {file}: an include must be a bare file name"));
        }
        let verdict = harness_cache.entry(file.clone()).or_insert_with(|| {
            match read_bounded(&suite.root.join("harness").join(&file)) {
                Ok(harness) => parse(&harness),
                Err(error) => Err(format!("missing: {error}")),
            }
        });
        if let Err(error) = verdict {
            return Outcome::Fail(format!("harness {file}: {}", first_line(error)));
        }
    }
    Outcome::Pass
}

fn is_bare_file_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.contains(['/', '\\'])
        && !Path::new(name).is_absolute()
}

/// Read a regular file of at most `MAX_FILE_BYTES` as UTF-8. A directory,
/// device or pipe is refused before it is opened for reading.
fn read_bounded(path: &Path) -> std::io::Result<String> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(std::io::Error::other("not a regular file"));
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(std::io::Error::other(format!(
            "{} bytes is larger than the {MAX_FILE_BYTES}-byte limit",
            metadata.len()
        )));
    }
    fs::read_to_string(path)
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("")
}

/// The commit a checkout is at, read from `.git` without running git: a
/// detached `HEAD` holds the SHA; a branch `HEAD` names a ref file or a
/// `packed-refs` line. `None` when it cannot be determined.
pub fn checkout_revision(root: &Path) -> Option<String> {
    let git = root.join(".git");
    let head = fs::read_to_string(git.join("HEAD")).ok()?;
    let head = head.trim();
    let Some(reference) = head.strip_prefix("ref: ") else {
        return is_sha(head).then(|| head.to_string());
    };
    if let Ok(sha) = fs::read_to_string(git.join(reference)) {
        let sha = sha.trim();
        return is_sha(sha).then(|| sha.to_string());
    }
    let packed = fs::read_to_string(git.join("packed-refs")).ok()?;
    packed.lines().find_map(|line| {
        let (sha, name) = line.split_once(' ')?;
        (name == reference && is_sha(sha)).then(|| sha.to_string())
    })
}

fn is_sha(text: &str) -> bool {
    text.len() == 40 && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_suite() -> Suite {
        Suite::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
    }

    /// A stand-in parser for judging logic: it rejects any source containing
    /// the marker `@@reject@@`. The real parser is exercised in the
    /// integration test; this keeps these tests about the runner's rules.
    fn stub(source: &str) -> Result<(), String> {
        if source.contains("@@reject@@") {
            Err("stub rejected".to_string())
        } else {
            Ok(())
        }
    }

    fn outcome(results: &Results, id: &str) -> Outcome {
        results.outcomes.get(id).unwrap_or_else(|| panic!("no result for {id}")).clone()
    }

    #[test]
    fn walks_tests_sorted_and_skips_fixtures() {
        let paths = fixture_suite().test_paths(&[]).unwrap();
        assert!(paths.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(paths.iter().all(|path| !path.ends_with("_FIXTURE.js")));
        assert!(paths.contains(&"language/parse/accepts.js".to_string()));
    }

    #[test]
    fn filters_keep_only_matching_prefixes() {
        let suite = fixture_suite();
        let paths = suite.test_paths(&["language/parse/neg".to_string()]).unwrap();
        assert!(!paths.is_empty());
        assert!(paths.iter().all(|path| path.starts_with("language/parse/neg")));
    }

    #[test]
    fn judges_each_rule_of_the_parse_level() {
        let suite = fixture_suite();
        let paths = suite.test_paths(&[]).unwrap();
        let results = run_parse_level(&suite, &paths, &stub);

        // A valid program runs twice and passes both ways.
        assert_eq!(outcome(&results, "language/parse/accepts.js#sloppy"), Outcome::Pass);
        assert_eq!(outcome(&results, "language/parse/accepts.js#strict"), Outcome::Pass);
        // A parse-negative test passes only when rejected.
        assert_eq!(outcome(&results, "language/parse/negative-rejected.js#strict"), Outcome::Pass);
        assert!(matches!(
            outcome(&results, "language/parse/negative-accepted.js#sloppy"),
            Outcome::Fail(_)
        ));
        // A runtime-negative test must still parse.
        assert_eq!(outcome(&results, "language/parse/negative-runtime.js#sloppy"), Outcome::Pass);
        // A valid program the parser rejects fails.
        assert!(matches!(outcome(&results, "language/parse/valid-rejected.js#sloppy"), Outcome::Fail(_)));
        // Modules and resolution are skipped with reasons, never passed.
        assert!(matches!(outcome(&results, "language/parse/module.js"), Outcome::Skip(_)));
        assert!(matches!(outcome(&results, "language/parse/resolution.js#strict"), Outcome::Skip(_)));
        // onlyStrict runs once, as strict.
        assert!(results.outcomes.contains_key("language/parse/only-strict.js#strict"));
        assert!(!results.outcomes.contains_key("language/parse/only-strict.js#sloppy"));
        // A missing include fails the test that needs it.
        assert!(matches!(outcome(&results, "language/parse/missing-include.js#sloppy"), Outcome::Fail(_)));
        // A file with no header is a failure, reported, not a pass.
        assert!(matches!(outcome(&results, "language/parse/no-header.js"), Outcome::Fail(_)));
    }

    #[test]
    fn summary_and_skip_reasons_count_by_area() {
        let suite = fixture_suite();
        let paths = suite.test_paths(&[]).unwrap();
        let results = run_parse_level(&suite, &paths, &stub);
        // 11 files: seven run twice (sloppy + strict), three run once
        // (only-strict, resolution, module), and no-header fails once = 18.
        assert_eq!(results.outcomes.len(), 18);
        let [pass, fail, skip] = results.summary()["language/parse"];
        assert_eq!((pass, fail, skip), (7, 9, 2));
        assert_eq!(results.skip_reasons().values().sum::<usize>(), 2);
        assert_eq!(results.judged().len(), 16);
        assert_eq!(results.passes().len(), 7);
    }

    #[test]
    fn an_include_must_be_a_bare_file_name() {
        for bad in ["../x.js", "/etc/passwd", "sub/x.js", ".hidden.js", "", "a\\b.js"] {
            assert!(!is_bare_file_name(bad), "{bad:?}");
        }
        assert!(is_bare_file_name("compareArray.js"));

        let suite = fixture_suite();
        let paths = suite.test_paths(&["language/parse/escaping-include.js".to_string()]).unwrap();
        let results = run_parse_level(&suite, &paths, &stub);
        match outcome(&results, "language/parse/escaping-include.js#sloppy") {
            Outcome::Fail(message) => assert!(message.contains("bare file name"), "{message}"),
            other => panic!("expected a failure, got {other:?}"),
        }
    }

    #[test]
    fn reads_a_detached_and_a_branch_head() {
        let temp = std::env::temp_dir().join(format!("v8-test262-rev-{}", std::process::id()));
        let git = temp.join(".git");
        fs::create_dir_all(git.join("refs/heads")).unwrap();
        let sha = "0123456789abcdef0123456789abcdef01234567";
        fs::write(git.join("HEAD"), format!("{sha}\n")).unwrap();
        assert_eq!(checkout_revision(&temp).as_deref(), Some(sha));
        fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(git.join("refs/heads/main"), format!("{sha}\n")).unwrap();
        assert_eq!(checkout_revision(&temp).as_deref(), Some(sha));
        fs::remove_file(git.join("refs/heads/main")).unwrap();
        fs::write(git.join("packed-refs"), format!("# pack\n{sha} refs/heads/main\n")).unwrap();
        assert_eq!(checkout_revision(&temp).as_deref(), Some(sha));
        fs::write(git.join("HEAD"), "not a sha\n").unwrap();
        assert_eq!(checkout_revision(&temp), None);
        fs::remove_dir_all(&temp).unwrap();
    }
}
