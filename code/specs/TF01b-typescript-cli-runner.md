# TF01b — TypeScript Test Bootstrap CLI Runner via cli-builder

**Parent:** [TF00](TF00-portable-test-framework.md), [TF01](TF01-bootstrap-harness.md), [TF01a](TF01a-typescript-test-bootstrap.md), [VER00 §7.1](VER00-verified-shipping-program.md).

This specification defines the declarative command-line interface for `@coding-adventures/test-bootstrap`, powered by the internal `@coding-adventures/cli-builder` engine.

It delivers **VER-P030**: a robust, drop-in CLI test runner supporting self-corpus execution, standalone test file execution, deterministic PRNG seeding, fault-injected sabotage verification, regex filtering, and fail-closed exit codes with zero external third-party dependencies.

---

## 1. Architecture and Design

The CLI runner bridges the pure verification harness (`@coding-adventures/test-bootstrap`) and declarative CLI argument parsing (`@coding-adventures/cli-builder`):

```text
┌────────────────────────────────────────────────────────┐
│             CLI Entry Point (bin/cli.mjs)              │
└──────────────────────────┬─────────────────────────────┘
                           │
       ┌───────────────────┴───────────────────┐
       ▼                                       ▼
┌─────────────────────────────┐ ┌─────────────────────────────┐
│  @coding-adventures/        │ │  @coding-adventures/        │
│  cli-builder                │ │  test-bootstrap             │
│  - Parser                   │ │  - testBootstrapCliSpec    │
│  - HelpGenerator            │ │  - parseFile / TFV v1       │
│  - TokenClassifier          │ │  - buildSelfCorpusSuite     │
│  - PositionalResolver       │ │  - Matchers & Assertions    │
└──────────────┬──────────────┘ └──────────────┬──────────────┘
               │                               │
               └───────────────┬───────────────┘
                               ▼
            ┌────────────────────────────────────┐
            │ Execution Target:                  │
            │ 1. Self-corpus (cases.tfv)         │
            │ 2. Test files (**/*.test.ts)       │
            └────────────────────────────────────┘
```

---

## 2. Declarative CLI Specification

The CLI is defined declaratively using `CliSpec`:

- **Name**: `test-bootstrap`
- **Version**: `0.1.0`
- **Parsing Mode**: `gnu` (standard GNU-style option processing with `--` operand delimiter)
- **Builtin Flags**:
  - `help`: `true` (auto-generates standard help text via `HelpGenerator`)
  - `version`: `true` (auto-generates `--version` output)

### 2.1 Flags

| Flag ID | Short | Long | Type | Default | Description |
|---|---|---|---|---|---|
| `seed` | `-s` | `--seed` | `integer` | `0` | Deterministic PRNG seed for execution order and generation |
| `sabotage` | | `--sabotage` | `enum` | `"none"` | Fault-injection sabotage mode (`none`, `always-pass`, `never-pass`, `invert`, `timeout`, `crash`) |
| `reporter` | `-r` | `--reporter` | `enum` | `"tap"` | Output reporter format (`tap`, `spec`, `summary`) |
| `filter` | `-f` | `--filter` | `string` | `null` | Regular expression pattern to filter test cases by name |
| `bail` | `-b` | `--bail` | `boolean` | `false` | Stop execution immediately on the first test failure |
| `timeout` | `-t` | `--timeout` | `integer` | `5000` | Per-test execution timeout in milliseconds |
| `self-corpus` | | `--self-corpus` | `boolean` | `false` | Force execution of canonical `test-bootstrap-v1` self-corpus |

### 2.2 Positional Arguments

| Argument ID | Type | Variadic | Description |
|---|---|---|---|
| `files` | `string` | `true` (0 or more) | Paths or glob patterns of test files to execute |

---

## 3. Execution Semantics

1. **Argument Parsing**:
   - `Parser` from `@coding-adventures/cli-builder` parses `process.argv`.
   - If a `HelpResult` is returned: prints help text to `stdout` and exits with code `0`.
   - If a `VersionResult` is returned: prints version to `stdout` and exits with code `0`.
   - If `ParseErrors` is thrown: prints error details to `stderr` and exits with code `2` (`USAGE_EXIT`).

2. **Target Dispatch**:
   - If `--self-corpus` is specified, or if no `files` are passed and `cases.tfv` is located in standard paths: runs the TFV v1 self-corpus runner (`buildSelfCorpusSuite`).
   - If `files` are provided: executes the specified test files using Node 22 native test runner with `--experimental-strip-types --test`.

3. **Closed Exit Codes**:
   - `0`: All tests passed.
   - `1`: One or more tests failed, or sabotage assertion triggered.
   - `2`: Invalid CLI invocation, unparseable flags, or missing required arguments.

---

## 4. Zero External Dependencies & Build-Tool Contracts

- Zero external runtime npm dependencies (`"dependencies": { "@coding-adventures/cli-builder": "file:../cli-builder" }`).
- Declared build-tool dependency: `# build-tool: deps=typescript/cli-builder` in both `BUILD` and `BUILD_windows`.
- Strict compiler portability: `src/` files do not reference Node.js APIs or `@types/node`. All Node I/O is isolated to `bin/cli.mjs`.
