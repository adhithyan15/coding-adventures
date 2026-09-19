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
The Rust verifier only parses checked-in data and resolves checked-in paths.

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

## Change the ledger

When adding or renaming a `tests/diff/` directory:

1. add it to exactly one fixture set;
2. identify its Rust harness;
3. choose an honest disposition and current-provenance status;
4. attach a command only when an upstream comparison exists; and
5. run the verifier and the full closurec suite.

```sh
cargo test --manifest-path code/programs/rust/closurec/Cargo.toml \
  --test oracle_manifest --no-fail-fast
```

The verifier rejects unknown schema fields, unsafe or stale paths, missing and
duplicate classifications, incomplete command matrices, false upstream claims,
and drift between the explicit 462-fixture minify cohort and runtime discovery.
CCR-004 is responsible for actually rerunning those 462 goldens and reviewing
every changed byte before their current provenance advances to `v20260915`.
