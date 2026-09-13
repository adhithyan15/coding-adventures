# A CI gate keyed on a package name string can go stale silently when the classifier improves (dependabot/CodeQL alert fix PR)

**Context:** PR #11876 (5x extract-zip, 5x js-yaml, 2x CodeQL alert fixes). The
diff touched exactly one file outside `code/programs/go/unix-tools`: a JS test
under `code/programs/mosaic/venture-browser/host/web/`. CI's
`build (ubuntu-latest)` job failed with:

    error: failed to run custom build command for `cairo-sys-rs v0.20.10`
    Package cairo was not found in the pkg-config search path.

**What happened:** the "Build and test affected packages" step is driven
directly by the go build-tool's dependency graph, which correctly marked the
whole `mosaic/programs/venture-browser` package (a single Starlark BUILD unit
covering `host/web/*.js` alongside its Rust workspace) as affected and ran
`cargo test` on it — which needs `libcairo2-dev`. But the *separate* workflow
step that installs `libcairo2-dev` doesn't key off the build plan directly; it
keys off a `needs_venture_windows` flag computed by
`code/scripts/venture_windows_ci_acceptance.py`, which checks the affected
package list against a hardcoded `ACCEPTANCE_PACKAGES` set containing
`"unknown/programs/venture-browser"`. The build-tool, at some point, learned to
classify `.mil/.mll/.msl`-based Mosaic packages under `language: "mosaic"`
instead of the generic `"unknown"` fallback — so the real affected-package name
had become `"mosaic/programs/venture-browser"`. The acceptance script's set was
never updated, silently stopped matching, and the cairo install step got
skipped every time — undetected because no PR since the reclassification had
touched *only* venture-browser's non-Rust files without also touching Rust
elsewhere (which would have triggered `needs_rust` independently and installed
cairo anyway, masking the gap).

**Diagnosis path that worked:** don't trust the coarse `needs_*` boolean in
isolation — reproduce the build plan locally (`build-tool -diff-base
origin/main -detect-languages -emit-plan plan.json`) and grep it for the
package in question to see its *actual* current name, then diff that against
whatever string the gating script hardcodes.

    python3 -c "
    import json
    d = json.load(open('plan.json'))
    for p in d['packages']:
        if 'venture' in p['name'].lower():
            print(p['name'], p['language'])
    "
    # -> mosaic/programs/venture-browser | mosaic   (script expected "unknown/programs/venture-browser")

**Generalisation:** any CI gate that pattern-matches a package-classifier's
output by a hardcoded string (not by re-deriving it from the current
classifier) will silently rot when the classifier gets more precise. Five
sibling scripts (`mosaic_swift_runtime_ci_acceptance.py`,
`mosaic_compose_runtime_ci_acceptance.py`,
`mosaic_flutter_runtime_ci_acceptance.py`,
`mosaic_xaml_windows_ci_acceptance.py`, `mosaic_qt_runtime_ci_acceptance.py`)
hardcode the same stale `"unknown/programs/task-app"` and are presumed to have
the identical latent gap — flagged as a follow-up rather than bundled into an
unrelated security-alert PR, since fixing them isn't required to unblock this
one and touches five extra files with their own test fixtures.
