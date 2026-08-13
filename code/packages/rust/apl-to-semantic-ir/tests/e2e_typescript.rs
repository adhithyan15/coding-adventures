//! End-to-end round-trip: APL → SIR → TypeScript → a REAL `tsc`/`tsx`
//! toolchain, against the REAL `@coding-adventures/sir-runtime-core` and
//! `@coding-adventures/sir-runtime-array` npm packages.
//!
//! This is deliberately NOT built like `tests/e2e_node.rs` (which targets
//! the JavaScript backend's self-contained output) or like
//! `semantic-ir-to-typescript/tests/run_with_node.rs` (which strips the TS
//! backend's type annotations and hand-stubs its runtime imports into small
//! inline JS, to avoid a TypeScript-toolchain test dependency for its
//! smaller KW3/exceptions surface). Neither approach can prove anything
//! about this crate's array/matrix codegen: `sir-runtime-array` is a real,
//! non-trivial N-D array runtime (`ndarray`/`reduce`/`scan`/`matmul`/
//! `reshape`/`ravel`/`catenate`, ~10 modules) that nobody had ever hand-
//! ported into a faithful stub, and doing so would risk testing a SECOND,
//! possibly-divergent implementation instead of the real one. So this file
//! spins up a real scratch npm project with `file:` dependencies on the
//! actual `code/packages/typescript/sir-runtime-core`/`sir-runtime-array`
//! packages, type-checks the emitted TypeScript with real `tsc --strict`,
//! and executes it with `tsx` (real esbuild-based TS execution, not a
//! stub) — the first place in this repo a Rust test drives a real
//! TypeScript toolchain rather than a bespoke shim.
//!
//! ## Two real bugs this proof found and this PR fixes
//!
//! Before this file's fixes landed, EVERY one of the programs below either
//! printed the wrong thing or failed to type-check:
//!
//! 1. **`toDisplay` never recognised an NDArray value.** `sir-runtime-core`
//!    (which every TS-emitted program imports) has no dependency on
//!    `sir-runtime-array` (a non-array-sourced program should pull in zero
//!    array code) and its `toDisplay` fell straight through to `String(v)`
//!    for any object it didn't recognise — so `+/1 2 3` printed
//!    `[object Object]`, not `6`. Fixed the same way the JS backend's own
//!    self-contained `formatSeen` already solves this (duck-type the
//!    `{shape, data}` shape rather than importing the type): `values.ts`
//!    gained an `"apl"` `setDisplayConvention` (mirroring the existing
//!    `"ruby"` one), `NDArrayLike`/`isNdArrayLike`/`fmtNumApl`/
//!    `displayNdArrayApl` (a 1:1 port of `apl_runtime::value::display`/
//!    `fmt_num`, already ported once before into `semantic-ir-to-
//!    javascript`'s `ArrayRt`), and `NDArrayLike` joined the public `Val`
//!    union (the same way SIR16's `Val[]`/`Map<Val,Val>` already did) so
//!    `__Sir.write(...)`'s call sites type-check. `emit.rs` now emits
//!    `__Sir.setDisplayConvention("apl")` for an APL-sourced module,
//!    exactly mirroring the existing `"ruby"` branch just above it.
//! 2. **`indexGenerator` required a pre-wrapped `NDArray`, but the emitter
//!    hands it a bare number.** `⍳6`'s `count` operand is a plain SIR
//!    `Expr` the emitter renders as-is (`__SirArray.indexGenerator(6)`) —
//!    `tsc --strict` catches this statically (`TS2345: Argument of type
//!    'number' is not assignable to parameter of type 'NDArray'`), and
//!    without `--strict` it throws at runtime instead (`isScalar` reads
//!    `.data.length` off a bare number). `sir-runtime-array` already has
//!    the exact same "bare-scalar-operand" problem solved for `elementwise`
//!    (`.* ./ .\` and `* /` can receive an unwrapped literal operand) via a
//!    `toArrayValue(v: number | NDArray): NDArray` normaliser — exported
//!    and reused in `iota.ts` rather than re-solved.
//!
//! Both were real, previously-unexercised gaps in code that has declared
//! `Feature::NDArrays`/`MatrixOps`/`ArrayColumnMajor` support since this
//! backend's array/matrix codegen was written — this is exactly the kind
//! of bug a hand-stubbed shim (which stubs away the real runtime, or skips
//! type-checking) cannot catch.
//!
//! ## A known CI-detection gap
//!
//! CI installs Node.js (`actions/setup-node`) only when the build-tool's
//! git-diff-based language detector sees a changed file under a
//! TypeScript-tagged package. A Rust-only diff touching just this test
//! file's own crate does not trip that detector (the detector has no
//! notion of "this Rust test shells out to `npm`/`npx` at runtime against
//! an unrelated TypeScript package directory" — that dependency is
//! invisible to it, unlike a normal `Cargo.toml`/`package.json` edge).
//! Practically: this test's own introducing PR will likely SKIP in CI
//! (`npm` unavailable in that job), but any future PR that also touches
//! `code/packages/typescript/sir-runtime-*` or `semantic-ir-to-typescript`
//! WILL exercise it for real, since those changes do trip `needs_typescript`.
//! Verified locally (Node 22, npm 11) before this PR was pushed. Teaching
//! the build tool this cross-language edge is tracked as separate,
//! follow-up work — out of scope here.

use std::fs;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use apl_to_semantic_ir::compile_source;
use semantic_ir_to_typescript::compile;

fn npm_available() -> bool {
    Command::new("npm")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Absolute path to a real `code/packages/typescript/<name>` package,
/// resolved from this crate's own `CARGO_MANIFEST_DIR`
/// (`.../code/packages/rust/apl-to-semantic-ir`) rather than a relative
/// path from the scratch project — the scratch project's `package.json`
/// needs an absolute `file:` target regardless of where the scratch
/// directory itself lives.
fn ts_package_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../typescript")
        .join(name)
        .canonicalize()
        .unwrap_or_else(|e| panic!("resolve real path of typescript/{name}: {e}"))
}

/// A FIXED (not per-run/PID-suffixed) scratch npm project directory, so
/// `npm install` is idempotent and cached across repeated `cargo test`
/// invocations within one worktree — unlike `tests/e2e_node.rs`'s
/// `run_via_node`, which writes one unique throwaway `.js` file per test and
/// needs no such caching. Caching across DIFFERENT worktrees was never
/// possible anyway: the `file:` dependency paths below are absolute and
/// worktree-specific.
fn project_dir() -> PathBuf {
    std::env::temp_dir().join("sir_apl_ts_e2e_project")
}

/// Materialise the scratch project's `package.json`/`tsconfig.json` and run
/// `npm install` against the REAL runtime packages (plus their own
/// transitive `file:` dependencies, which npm does not auto-hoist for a
/// nested local package the way it does for registry packages — each is
/// listed explicitly at the top level here so Node's module resolution
/// finds them). Returns `false` (skip, don't fail) only when `npm` itself
/// is unavailable, matching every other `node_available()`-guarded test in
/// this repo; any failure of `npm install` itself is a hard test failure.
fn ensure_project_ready() -> bool {
    if !npm_available() {
        return false;
    }
    let dir = project_dir();
    fs::create_dir_all(dir.join("src")).expect("create scratch project dir");

    let package_json = format!(
        r#"{{
  "name": "apl-ts-e2e-scratch",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "dependencies": {{
    "@coding-adventures/sir-runtime-core": "file:{core}",
    "@coding-adventures/sir-runtime-array": "file:{array}",
    "@coding-adventures/sir-runtime-exceptions": "file:{exceptions}",
    "@coding-adventures/sir-runtime-pairs": "file:{pairs}"
  }},
  "devDependencies": {{
    "typescript": "^5.0.0",
    "tsx": "^4.19.0",
    "@types/node": "^22.0.0"
  }}
}}
"#,
        core = ts_package_path("sir-runtime-core").display(),
        array = ts_package_path("sir-runtime-array").display(),
        exceptions = ts_package_path("sir-runtime-exceptions").display(),
        pairs = ts_package_path("sir-runtime-pairs").display(),
    );
    fs::write(dir.join("package.json"), package_json).expect("write package.json");

    // `preserveSymlinks`: npm installs a `file:` dependency as a symlink
    // into `node_modules`, and Node's default ESM resolution walks up from
    // a symlink's REAL (target) path, not its `node_modules` location —
    // so, unpatched, resolving `sir-runtime-core`'s OWN `file:` dependency
    // on `sir-runtime-pairs` from inside the symlinked `sir-runtime-core`
    // source tree fails to find this scratch project's `node_modules` at
    // all. `preserveSymlinks` (both here for `tsc`'s type resolution and
    // as a `node` CLI flag below for `tsx`'s runtime resolution) keeps
    // resolution anchored at the symlink's apparent location instead.
    let tsconfig = r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "Node16",
    "moduleResolution": "Node16",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "preserveSymlinks": true,
    "noEmit": true
  },
  "include": ["src"]
}
"#;
    fs::write(dir.join("tsconfig.json"), tsconfig).expect("write tsconfig.json");

    let install = Command::new("npm")
        .arg("install")
        .current_dir(&dir)
        .output()
        .expect("spawn npm install");
    assert!(
        install.status.success(),
        "npm install failed:\n{}",
        String::from_utf8_lossy(&install.stderr)
    );
    true
}

struct Case {
    name: &'static str,
    source: &'static str,
    expected: &'static str,
}

/// A representative pilot slice of `tests/e2e_node.rs`'s own (already
/// vetted, JS-backend-proven) corpus — reduce/scan/outer-product/reshape/
/// ravel/index-generator — plus two cases `e2e_node.rs` doesn't need
/// dedicated coverage for: a bare negative scalar (exercises `toDisplay`'s
/// plain-`number` branch, not the NDArray branch) and a directly-printed
/// 2D matrix (exercises `displayNdArrayApl`'s rank-2/column-major-to-
/// row-major-display path, which none of the ported cases hit on their
/// own since they all reduce/ravel/shape down to a scalar or vector
/// first). Full corpus parity with `e2e_node.rs` is a natural follow-up,
/// not required to prove the pipeline itself is sound.
const CORPUS: &[Case] = &[
    Case {
        name: "reduce_add",
        source: "+/1 2 3 4\n",
        expected: "10",
    },
    Case {
        name: "reduce_max",
        source: "⌈/3 1 4 1 5\n",
        expected: "5",
    },
    Case {
        name: "scan_then_reduce",
        source: "-/+\\1 2 3\n",
        expected: "¯8",
    },
    Case {
        name: "index_generator",
        source: "⍳5\n",
        expected: "1 2 3 4 5",
    },
    Case {
        name: "outer_product",
        source: "+/,(⍳2)∘.×(⍳3)\n",
        expected: "18",
    },
    Case {
        name: "ravel_of_reshape",
        source: ",(,2 3)⍴⍳6\n",
        expected: "1 2 3 4 5 6",
    },
    Case {
        name: "negative_scalar",
        source: "-3\n",
        expected: "¯3",
    },
    Case {
        name: "matrix_print",
        source: "2 3⍴⍳6\n",
        expected: "1 2 3\n4 5 6",
    },
];

#[test]
fn apl_array_matrix_programs_execute_correctly_under_real_typescript_toolchain() {
    if !ensure_project_ready() {
        eprintln!(
            "note: `npm` unavailable — skipping the real TypeScript toolchain \
             execution proof (see this file's module doc comment's \"known \
             CI-detection gap\" section)"
        );
        return;
    }
    let dir = project_dir();

    // Emit every corpus program's TypeScript first, so `tsc` type-checks
    // the WHOLE batch in one invocation below — much faster than spawning
    // a fresh `tsc` process per case, and it's the batch-wide result (not
    // any one file) that answers "does this backend's array/matrix codegen
    // type-check against the real runtime packages at all".
    for case in CORPUS {
        let module = compile_source(case.source, case.name)
            .unwrap_or_else(|e| panic!("lowering {}: {e}", case.name));
        let report = semantic_ir::validate(&module);
        assert!(
            report.is_ok(),
            "SIR validation failed for {}: {:?}",
            case.name,
            report.issues
        );
        let artifact = compile(&module).expect("backend emit should succeed");

        let path = dir.join("src").join(format!("{}.ts", case.name));
        // Remove any stale entry from a prior run first, then create with
        // `create_new` — refuses to follow an existing symlink at this
        // path (mirrors `tests/e2e_node.rs`'s `run_via_node` discipline).
        // Unlike that file's PID-suffixed unique names, this project
        // directory is deliberately fixed/reused across runs (see
        // `project_dir`'s doc comment), so a stale file is expected on a
        // second run and simply cleared first rather than treated as a
        // collision.
        let _ = fs::remove_file(&path);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap_or_else(|e| panic!("create {}: {e}", path.display()));
        file.write_all(artifact.source.as_bytes())
            .expect("write ts source");
    }

    // Real `tsc --strict`, against the REAL runtime packages — this is
    // what actually caught both bugs this PR fixes (see the module doc
    // comment): a hand-written stub (as `run_with_node.rs` uses for its
    // smaller surface) strips type annotations rather than checking them,
    // so it could never have caught the `indexGenerator` type error.
    let tsc = Command::new("npx")
        .args(["tsc", "-p", "tsconfig.json"])
        .current_dir(&dir)
        .output()
        .expect("spawn tsc");
    assert!(
        tsc.status.success(),
        "tsc type-check failed:\n{}",
        String::from_utf8_lossy(&tsc.stdout)
    );

    // Execute each program and check its actual output. `tsx` (real
    // esbuild-based TS-to-JS transformation, not a stub) resolves the
    // emitted `import`s through the SAME real npm packages `tsc` just
    // type-checked. `--preserve-symlinks` is required here too, for the
    // same reason as `tsconfig.json`'s own setting above — this is
    // `node`'s equivalent CLI flag for `tsx`'s runtime resolution.
    for case in CORPUS {
        let path = dir.join("src").join(format!("{}.ts", case.name));
        let output = Command::new("node")
            .args(["--preserve-symlinks", "--import", "tsx"])
            .arg(&path)
            .current_dir(&dir)
            .output()
            .unwrap_or_else(|e| panic!("spawn node for {}: {e}", case.name));
        assert!(
            output.status.success(),
            "node execution failed for {}: stderr=\n{}",
            case.name,
            String::from_utf8_lossy(&output.stderr)
        );
        let got = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            got.trim(),
            case.expected,
            "case {} produced the wrong output",
            case.name
        );
    }
}
