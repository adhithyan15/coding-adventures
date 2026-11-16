# go/warrant-corpus

The **Go lane runner** for the language-neutral Warrant evidence corpus
(`warrant-corpus-v1`, [VER04i](../../../specs/VER04i-polyglot-warrant-evidence.md),
section 5.3). It is the third non-Rust lane, written to match the
[Python](../../python/warrant-corpus) and [TypeScript](../../typescript/warrant-corpus)
runners check for check.

A package's `.warrant` file is compiled once, by the Rust compiler
(`warrant-grammar`), into a corpus directory under
`code/specs/fixtures/<module>-warrant-v1/`. This package runs that corpus against
a **Go** library and writes the lane's `warrant-lane-evidence-v1` manifest. It
uses only the Go standard library and depends on no library it tests.

## Where it fits

```text
code/specs/<pkg>.warrant ──(Rust: parse, check, compile)──► corpus.json + cases.tfv
                                                              │
   go/<pkg>/evidence.bindings + cmd/warrant-evidence ──►  warrant-corpus (this package)
   (op -> export, sentinel -> Err, adapters)                  │  enumerate domains in Go
                                                              │  interpret the model IR (no eval)
                                                              │  hash inputs + expected, compare with the corpus
                                                              │  call the REAL export, compare
                                                              │  break one op at a time (negative controls)
                                                              ▼
                                       go/<pkg>/evidence/test-manifest.json
```

The corpus does not ship its ~570,000 exhaustive inputs: it ships a descriptor
(`bytes_d1`, `u64_edge`, ...) plus two SHA-256 digests per suite, computed by Rust
over the inputs and over the *model's* answers. This runner regenerates both
streams in Go; a skipped case, a reordered domain or a different model evaluation
changes a digest and fails the suite (`stream-mismatch`).

## Using it from a leaf package

Go cannot import a package by name at run time, so the leaf package carries a
small entry program (for `ct-compare`: `cmd/warrant-evidence`) that compiles its
library in and calls `warrantcorpus.Main`:

```go
registry := &warrantcorpus.Registry{
    Exports: map[string]warrantcorpus.Export{
        "CTEq": {Call: adapter, Impl: ctcompare.CTEq}, // Impl is only inspected, never called
    },
    Errors: map[string]error{"ErrLengthMismatch": ErrLengthMismatch},
}
os.Exit(warrantcorpus.Main(args, warrantcorpus.ProcessIO(), registry))
```

An **adapter** translates values and errors and nothing else (no computed answer,
retry or fallback). The runner checks that `Impl` really is the function the
binding names: `runtime.FuncForPC` must give `<import>.<export>` and the
function's source file must lie under the binding's `source` root, so the
manifest's `source_sha256` describes code that was compiled into the binary.

```sh
go run ./cmd/warrant-evidence run --out evidence/test-manifest.json   # regenerate
go run ./cmd/warrant-evidence check evidence/test-manifest.json       # CI drift check
go run ./cmd/warrant-evidence sabotage                                # prove it can fail
```

| Command | Meaning |
|---|---|
| `run [--out F]` | examples, exhaustive suites, negative controls; writes the manifest and `domain-labels.json` only on a clean full pass |
| `check F` | the same run; exit 1 if the regenerated manifest or labels differ from the committed files (ignoring `toolchain.version`) |
| `sabotage` | for every control, a child process (this binary) with `WARRANT_EVIDENCE_FAULT=<fault>:<op>` must print `not ok` and exit 1 |
| `list-faults` | `<fault>:<op>` for each control |
| `source-digest D` | `warrant-src-v1` digest of a source tree (`*.go` except tests) |
| `validate-manifest F [--registry R] [--require-active]` | structural validation against the corpus (it accepts a manifest from any lane) |

Exit status: `0` all passed, `1` a verdict failed, `2` the run was refused. Output
is TAP-style (`ok <id>` / `not ok <id> <failure-id>`), naming a failure by suite
and case *index*, never the input bytes. Environment: `WARRANT_EVIDENCE_FAULT`,
`WARRANT_EVIDENCE_FILTER`, `WARRANT_EVIDENCE_OUT`; a filter without a real fault
is refused and a faulted or filtered run never writes a manifest.

## What differs from the other lanes, and why

- **Errors map by sentinel identity.** `errors=VARIANT:ErrName` names an `error`
  value in the registry; a returned error maps only if it *is* that value. A
  wrapped error, an equal-looking one and any panic that escapes an adapter are
  `unexpected-error`.
- **`U64` is a native `uint64`**, `Bytes` is exactly `[]byte` (a named byte-slice
  type fails), and a `fresh` result is checked by mutation: flipping the result
  must not change an argument and vice versa.
- **JSON is read by the runner's own strict reader.** `encoding/json` keeps the
  last duplicate key and replaces invalid UTF-8 and lone surrogate escapes with
  U+FFFD. Output is canonical (sorted keys, two-space indent, LF, ASCII-only),
  byte-identical to the other lanes.
- **Containment.** Corpus, binding and package paths must be strictly inside
  `--repo-root`. `filepath.Rel` fails across Windows drives and returns `..foo`
  for a directory of that name, so the check handles the error, refuses an
  absolute result and tests `..` or `..<sep>`; the Windows shapes are unit-tested
  from any host.
- **The label digest** binds to the manifest's own `toolchain.name`, so this
  runner validates the Python and TypeScript manifests and labels too (a test
  does exactly that).
- **`runner.src_sha256`** digests this package's own source as embedded with
  `go:embed`, so it describes the bytes that were compiled. Build without
  `-trimpath`: the provenance check needs real source paths and fails closed
  otherwise.

## Honest limits

A passing run proves the enumerated domain only (for `ct-compare`:
`Bytes[347] x Bytes[347]`, `U64[377] x U64[377]`, ...), never "all inputs". The
Warrant model, the corpus compiler, the binding file **and the adapters** are
trusted at the boundary (the adapters are digested with the leaf package's
source). Nothing here proves constant-time behaviour of Go code (VER04i section
11, item 5). The runner is not yet run under the VER08b sandbox (backlog
CAP-RT-1xx).

## Development

```sh
go vet ./...
go test ./... -cover      # about 99% statement coverage
```

The suite runs the whole real corpus (572,109 cases) through a reference library
defined in the tests, plus a deliberately wrong library per defect; the plumbing
tests use a two-suite slice of it. Fixtures are copied as bytes, never read and
rewritten as text.

## Shared conformance kit

`conformance_test.go` runs every case of `code/specs/fixtures/warrant-corpus-conformance-v1`
(VER04i 9.1, WD-100) through this lane's own functions: `parseJSON` then `renderJSON`
(json), `parseType` + `decodeJSON` + the stream encoder (tagged), `relativeInside` with the
case's `posix` or `win32` flavour (containment) and `manifestLabelDigest` (label-digest).
All four categories run, and the per-category counts are compared with `index.json`.
Production calls the same containment helper with `hostFlavor`, so the device-path and `//`
rules apply there too. Symlinks are resolved by `resolveInside` before the lexical check.
