# HOS-D05-GO4 — Route the captured Go pipeline fixture

## Purpose

The Go `riscv-simulator` package BUILD replays the captured Go D05 pipeline
fixture through an external adapter. The Go build tool does not infer
repository-wide fixture reads from that command, so register the precise
external inputs for affected-package selection and fallback package hashing.
The package-local library and unit tests retain their empty capability
contract; only the fixture adapter reads repository files.

## Exact input route

Only these paths select and hash `go/riscv-simulator` for this fixture:

- `code/specs/fixtures/go-riscv-d05-v1/input.json`
- `code/specs/fixtures/go-riscv-d05-v1/expected_trace.json`
- `code/specs/fixtures/go-riscv-d05-v1/test-adapters/go/main.go`

Each path is matched literally. A change, deletion, or rename of one of these
files selects the Go RISC-V simulator; a neighboring README, schema, directory
name, backup extension, or unrelated language adapter does not. The package's
hash input list is the same exact three-file set. The source adapter is
included because the package BUILD runs it outside its package directory.

## BUILD integration

Both `BUILD` and `BUILD_windows` run the existing package tests and then invoke
the external adapter as a separate command line, from the package directory:

```text
go run ../../../specs/fixtures/go-riscv-d05-v1/test-adapters/go/main.go
```

This uses the package's existing Go module replacements and requires no new
dependencies. The external command uses only Go and public simulator APIs. The
Windows package front has the same portable Go command. The adapter's optional
`-update` is a manual recapture workflow and is not part of BUILD.

## Evidence

Build-tool tests pin the three exact path selections, package hash input set,
language filter behavior, nearby lookalike exclusion, and content-sensitive
hash changes for every declared path. The package BUILD then executes fixture
replay in the same CI package job as component tests. No directory-prefix
selector, broad fixture scan, capability grant, or cross-language consumer is
introduced.
