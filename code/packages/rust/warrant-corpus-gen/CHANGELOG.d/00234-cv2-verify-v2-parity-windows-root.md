# Unreleased: the verify-v2 parity test strips the Windows verbatim prefix from `--repo-root`

`the_oracles_verify_v2_accepts_a_generator_written_corpus` passed `--repo-root` as `root.canonicalize()`, which on Windows is a
verbatim path (`\\?\D:\...`). The oracle's `expected_spec_ref` does `warrant.resolve().relative_to(root.resolve())`, and
Python's `resolve()` of `--warrant` yields a plain `D:\...`, so it refused the corpus ("--warrant is not inside the repository
root") on `build (windows-2025)`.

## Fixed

- `tests/data_eval_parity.rs`: a `plain_path` helper strips the `\\?\` prefix (keeping `\\?\UNC\...` network paths verbatim);
  the identity on other platforms. Test-only: no generator or oracle behaviour changes.
