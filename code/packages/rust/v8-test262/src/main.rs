//! `v8-test262` — run test262 at one level and gate on the expected-pass list.
//!
//! ```text
//! v8-test262 --test262 DIR [--level parse] [--expected FILE] [--update]
//!            [--json] [--allow-revision-mismatch] [PATH-PREFIX …]
//! ```
//!
//! - `--test262 DIR` (or `TEST262_DIR`): a test262 checkout.
//! - `--level parse`: the only level so far (V8C09 §3).
//! - `--expected FILE`: defaults to this crate's `expected/<level>.txt`.
//! - `--update`: add new passes to the list (never removes anything).
//! - `--json`: the summary as JSON on stdout.
//! - `--allow-revision-mismatch`: run a checkout that is not at the pinned
//!   `TEST262_REVISION` (the list is then not comparable; for exploration).
//! - `PATH-PREFIX`: only tests under these paths (relative to `test/`).
//!
//! Exit status: 0 no regressions; 1 regressions; 2 usage or input error.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::ExitCode;

use v8_test262::expected::{compare, parse_expected, render_expected};
use v8_test262::{checkout_revision, javascript_parse, run_parse_level, Outcome, Suite};

const CRATE_DIR: &str = env!("CARGO_MANIFEST_DIR");

struct Options {
    test262: PathBuf,
    level: String,
    expected: PathBuf,
    update: bool,
    json: bool,
    allow_revision_mismatch: bool,
    filters: Vec<String>,
}

fn usage(problem: &str) -> ExitCode {
    eprintln!("v8-test262: {problem}");
    eprintln!(
        "usage: v8-test262 --test262 DIR [--level parse] [--expected FILE] [--update] [--json] [--allow-revision-mismatch] [PATH-PREFIX ...]"
    );
    ExitCode::from(2)
}

fn parse_options(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut test262 = std::env::var_os("TEST262_DIR").map(PathBuf::from);
    let mut level = "parse".to_string();
    let mut expected = None;
    let mut update = false;
    let mut json = false;
    let mut allow_revision_mismatch = false;
    let mut filters = Vec::new();
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--test262" => test262 = Some(PathBuf::from(value("--test262")?)),
            "--level" => level = value("--level")?,
            "--expected" => expected = Some(PathBuf::from(value("--expected")?)),
            "--update" => update = true,
            "--json" => json = true,
            "--allow-revision-mismatch" => allow_revision_mismatch = true,
            flag if flag.starts_with("--") => return Err(format!("unknown option {flag}")),
            prefix => filters.push(prefix.trim_start_matches("test/").to_string()),
        }
    }
    if level != "parse" {
        return Err(format!("unknown level {level:?}; only `parse` exists so far"));
    }
    let test262 = test262.ok_or("no test262 checkout: pass --test262 DIR or set TEST262_DIR")?;
    let expected =
        expected.unwrap_or_else(|| PathBuf::from(CRATE_DIR).join("expected").join(format!("{level}.txt")));
    Ok(Options {
        test262,
        level,
        expected,
        update,
        json,
        allow_revision_mismatch,
        filters,
    })
}

fn main() -> ExitCode {
    let options = match parse_options(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(problem) => return usage(&problem),
    };

    // The list is only meaningful against the revision it was measured at.
    let pinned = std::fs::read_to_string(PathBuf::from(CRATE_DIR).join("TEST262_REVISION"))
        .map(|text| text.trim().to_string())
        .unwrap_or_default();
    let actual = checkout_revision(&options.test262);
    if actual.as_deref() != Some(pinned.as_str()) && !options.allow_revision_mismatch {
        return usage(&format!(
            "checkout is at {}, but the lists were measured at {pinned}; fetch that commit or pass --allow-revision-mismatch",
            actual.as_deref().unwrap_or("an unknown revision")
        ));
    }

    let suite = Suite::new(&options.test262);
    let paths = match suite.test_paths(&options.filters) {
        Ok(paths) => paths,
        Err(error) => return usage(&format!("cannot read {}/test: {error}", options.test262.display())),
    };
    let results = run_parse_level(&suite, &paths, &javascript_parse);

    let expected = std::fs::read_to_string(&options.expected)
        .map(|text| parse_expected(&text))
        .unwrap_or_default();
    let passes = results.passes();
    let comparison = compare(&expected, &passes, &results.judged());

    if options.json {
        let summary = serde_json::json!({
            "level": options.level,
            "revision": pinned,
            "results": results.outcomes.len(),
            "passes": passes.len(),
            "regressions": comparison.regressions,
            "newPasses": comparison.new_passes.len(),
            "areas": results.summary().into_iter().map(|(area, [pass, fail, skip])| {
                (area, serde_json::json!({ "pass": pass, "fail": fail, "skip": skip }))
            }).collect::<serde_json::Map<_, _>>(),
            "skipReasons": results.skip_reasons(),
        });
        println!("{}", serde_json::to_string_pretty(&summary).expect("json"));
    } else {
        for (area, [pass, fail, skip]) in results.summary() {
            println!("{area:<48} pass {pass:>6}  fail {fail:>6}  skip {skip:>6}");
        }
        for (reason, count) in results.skip_reasons() {
            println!("skipped {count:>6}: {reason}");
        }
        println!(
            "{} results, {} pass; {} listed; {} new passes; {} regressions",
            results.outcomes.len(),
            passes.len(),
            expected.len(),
            comparison.new_passes.len(),
            comparison.regressions.len()
        );
        for id in &comparison.regressions {
            let why = match results.outcomes.get(id) {
                Some(Outcome::Fail(message)) => message.as_str(),
                _ => "",
            };
            eprintln!("REGRESSION {id}: {why}");
        }
    }

    if options.update {
        // Only ever add: the union of the list and this run's passes.
        let grown: BTreeSet<String> = expected.union(&passes).cloned().collect();
        if let Err(error) = std::fs::write(&options.expected, render_expected(&options.level, &pinned, &grown)) {
            return usage(&format!("cannot write {}: {error}", options.expected.display()));
        }
    }

    if comparison.is_regression_free() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
