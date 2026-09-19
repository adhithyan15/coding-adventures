# Closure Compiler oracle manifest

`manifest.json` is the offline provenance ledger for every immediate fixture
directory under `tests/diff/`. It separates four claims that must not be
collapsed into one another:

1. the released upstream source tag and commit;
2. the exact Maven artifact used as the executable oracle;
3. the later upstream source commit inspected by the parity audit; and
4. the provenance of the bytes or assertions currently checked into each
   fixture.

The manifest currently pins Google Closure Compiler `v20260915`. Its released
JAR is not committed to this repository and CI never downloads or executes it.
The Rust verifiers only parse checked-in data and resolve checked-in paths.

## Obtain and verify the oracle

Download the artifact from the manifest's `upstream.release.artifact.url` into
a directory outside the repository. Before executing it, verify both its byte
length and SHA-256 against the manifest. For example, on a Unix host:

```sh
curl --fail --location --output closure-compiler-v20260915.jar \
  https://repo1.maven.org/maven2/com/google/javascript/closure-compiler/v20260915/closure-compiler-v20260915.jar
test "$(wc -c < closure-compiler-v20260915.jar)" -eq 14976538
printf '%s  %s\n' \
  9c8af06056aa06f968b5a457540a85869c7ba2861c211c56d8d4ef6c35ddf36d \
  closure-compiler-v20260915.jar | sha256sum --check
```

On PowerShell:

```powershell
$jar = Resolve-Path .\closure-compiler-v20260915.jar
if ((Get-Item -LiteralPath $jar).Length -ne 14976538) { throw 'wrong byte length' }
$hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $jar).Hash.ToLowerInvariant()
if ($hash -ne '9c8af06056aa06f968b5a457540a85869c7ba2861c211c56d8d4ef6c35ddf36d') {
    throw 'wrong SHA-256'
}
```

The published POM and the JAR's embedded `META-INF/LICENSE*` and
`META-INF/NOTICE.txt` identify the artifact as Apache-2.0. The embedded
manifest records `Build-Jdk-Spec: 21`; the reviewed captures used Java
21.0.12.

## Expand a command template

Run commands from `code/programs/rust/closurec` with locale `C.UTF-8`, timezone
`UTC`, and UTF-8 encoding. For `closure-flags-file-v1`, replace
`{oracle_jar}` with the verified artifact path and replace `{fixture_flags}`
with one argument per nonblank, noncomment line in the fixture's `flags.txt`.
Capture exit status, stdout, stderr, and any declared output files separately.

The typed-pipeline matrix is explicit in the manifest because its four probes
do not use `flags.txt`. Local-only correlation-vector and transitional
print-tree contracts have no upstream command: inventing one would turn a
classification ledger into false provenance.

## Refresh the minify cohort

With the independently downloaded JAR and Java 21.0.12, run from this package:

```sh
cargo run --example oracle_refresh -- \
  --oracle-jar /absolute/path/to/closure-compiler-v20260915.jar \
  --java /absolute/path/to/java-21.0.12
```

`--java` must be an absolute executable path. The tool checks its canonical
launcher bytes and exact version both before and after capture. Before running
anything, it checks the manifest against independent hard-coded release,
artifact, Java, and exact-argv trust pins; stream-verifies the JAR byte length
and SHA-256; and preflights all 462 `minify_*` fixtures. Each case permits only
one `WHITESPACE_ONLY` pair and one fixture-contained `--js` path.

Java receives only private snapshots of the verified JAR/input bytes. The tool
removes JVM option and classpath injection variables, caps workers, input,
aggregate memory, process time, and process output, and kills/reaps failed
children. It rejects report symlinks and replaces the report atomically from a
same-directory, create-new temporary file only after the entire capture and
post-run Java identity checks succeed. It never downloads an artifact.

The checked-in `v20260915` run reports 462 equal stdout captures, zero changed,
and zero declined. Three equal legacy-octal cases have non-empty upstream
warning stderr; the report intentionally preserves those bytes because stdout
identity does not imply diagnostic parity.

If a later capture reports `changed` or `declined`, review every result before
commit. A changed golden must be updated to the captured bytes and carry an
`accepted_expected_update` review with a unique reason and issue link. A
decline keeps the prior golden and requires a `documented_decline` review.
Unreviewed non-equal results fail the offline verifier.

## Change the ledger

When adding or renaming a `tests/diff/` directory:

1. add it to exactly one fixture set;
2. identify its Rust harness;
3. choose an honest disposition and current-provenance status;
4. attach a command only when an upstream comparison exists; and
5. run the verifier and the full closurec suite.

```sh
cargo test --manifest-path code/programs/rust/closurec/Cargo.toml \
  --test oracle_manifest --test oracle_refresh_report --no-fail-fast
```

The verifiers reject unknown schema fields, unsafe or stale paths, missing and
duplicate classifications, incomplete command matrices, false upstream claims,
malformed lowercase SHA-256 text, altered report bytes or hashes, malformed
review/issue identifiers, unreviewed changes/declines, and drift between the
explicit 462-fixture minify cohort and runtime discovery.
