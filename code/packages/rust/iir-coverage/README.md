# iir-coverage

**LANG dev-tools D-4** — IIR-level coverage projection.

First consumer of the `IIRFunction.source_map` populated by D-1
([PR #1834](https://github.com/adhithyan15/coding-adventures/pull/1834)).
Mirrors LANG18's Python `tetrad-runtime.coverage` layer, but for the
Rust LANG-VM stack.

---

## What it does

Given:
- an `IIRModule` (with each `IIRFunction` carrying its
  `source_map: Vec<SourceLoc>` lockstep with `instructions`), and
- an **execution trace** (`HashMap<String, HashSet<usize>>` —
  per-function set of IIR instruction indices that were reached
  during execution),

…this crate projects the trace back to **source lines** and returns
a `LineCoverageReport`.

It answers "which source lines of my program were executed?" without
any knowledge of the interpreter, the JIT, or the original source
text — just the IR + the trace.

---

## Position in the stack

```
                          ┌──────────────────────┐
                          │ IIRModule            │
                          │   ├─ source_map      │ ← populated by D-1
                          │   └─ instructions    │
                          └──────────┬───────────┘
                                     │
       (vm-core / lispy-runtime / twig-vm executes,
        records HashMap<fn, HashSet<ip>> trace)
                                     │
                                     ▼
                          ┌──────────────────────┐
                          │ iir-coverage         │ ← this crate
                          │   build_report(…)    │
                          └──────────┬───────────┘
                                     │
                                     ▼
                       ┌──────────────────────────┐
                       │ LineCoverageReport       │
                       │   covered_lines : list   │
                       │   lines_for_file(path)   │
                       │   total_lines_covered()  │
                       │   total_iir_hits()       │
                       └──────────────────────────┘
                                     │
            ┌────────────────────────┼────────────────────────┐
            ▼                        ▼                        ▼
       JSON exporter         terminal formatter        LSP code-lens
                                                         provider
```

The dispatcher (`vm-core` etc.) doesn't need to know about coverage
*data structures* — it just records an opaque trace.  The report is
computed off the hot path, on demand, after execution completes.
Multiple presentation/transport consumers all build on the same
projection.

---

## Public API

| Item | Description |
|------|-------------|
| `CoveredLine { file, line, iir_hit_count }` | One source line that was reached |
| `LineCoverageReport` | The projection result |
| `LineCoverageReport::covered_lines()` | Borrow the line slice |
| `LineCoverageReport::lines_for_file(path)` | Sorted line numbers for one file |
| `LineCoverageReport::total_lines_covered()` | Distinct `(file, line)` count |
| `LineCoverageReport::files()` | Distinct file paths in the report |
| `LineCoverageReport::total_iir_hits()` | Sum of `iir_hit_count` |
| `ExecutionTrace = HashMap<String, HashSet<usize>>` | The trace shape dispatchers produce |
| `build_report(module, trace, source_file)` | The projection function |
| `CoverageError` | `UnknownFunction`, `IpOutOfBounds`, `SourceMapDriftedFromInstructions` |

---

## Example

```rust
use iir_coverage::{build_report, ExecutionTrace};
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, SourceLoc};
use std::collections::{HashMap, HashSet};

// Hand-build a tiny IIR.
let mut module = IIRModule::new("demo", "twig");
let mut f = IIRFunction::new("main", vec![], "any", vec![
    IIRInstr::new("nop", None, vec![], "any"),
    IIRInstr::new("nop", None, vec![], "any"),
    IIRInstr::new("nop", None, vec![], "any"),
]);
f.source_map = vec![
    SourceLoc::new(1, 1),
    SourceLoc::new(2, 1),
    SourceLoc::new(3, 1),
];
module.functions.push(f);

// Pretend the dispatcher reached IPs 0 and 2.
let mut trace: ExecutionTrace = HashMap::new();
trace.insert("main".to_string(), HashSet::from([0_usize, 2_usize]));

let report = build_report(&module, &trace, "demo.twig").unwrap();
assert_eq!(report.lines_for_file("demo.twig"), vec![1, 3]);
assert_eq!(report.total_lines_covered(), 2);
```

---

## Granularity (per LANG18 §"D1 IIR-level granularity")

- The trace is a **set** of IIR instruction indices, not a count.
  "Was this IIR step reached at least once" — *not* "how many
  times".  Loop-iteration counts belong to LANG17's `BranchStats`,
  a separate layer.
- `iir_hit_count` reports the number of *distinct* IIR
  instructions at that source line that ran.  A single source line
  typically lowers to several IIR ops (e.g. `y := x + 1` →
  `load x`, `add 1`, `store y`).  If all three IIR instructions
  for that line ran, `iir_hit_count == 3`.  This is **not** an
  execution frequency.
- Saturates at `u32::MAX` if a single source line had more than
  4 billion distinct IIR instructions reach it.  Pathologically
  unreachable in any real program.

## Synthetic source positions

Compiler-synthesised IIR instructions carry `SourceLoc::SYNTHETIC`
(line `0`).  The projection drops these — synthetic instructions
correspond to no source line, so they neither contribute to coverage
totals nor appear in the report.

## Multi-file source attribution

`IIRFunction.source_map` carries `(line, column)` per instruction
but not the source-file name.  In the LANG-VM v1 pipeline, every
function in an `IIRModule` belongs to a single source unit; callers
pass the source-file path to `build_report` and the report tags
every covered line with that path.

Multi-file modules (e.g. once Twig modules lower to a single
`IIRModule`) will need a richer mapping; we'll add a
`build_report_multi_file(module, trace, file_for_function)`
overload when the first such consumer appears.

---

## Errors

| Error | When |
|-------|------|
| `UnknownFunction { name }` | Trace mentions a function not in the module — usually trace/module skew (different build than was traced) |
| `IpOutOfBounds { function, ip, instruction_count }` | Trace mentions an IP past the end of a function's instruction list — usually trace/module skew |
| `SourceMapDriftedFromInstructions { function, instructions_len, source_map_len }` | A function's `source_map` length disagrees with `instructions` — should never happen for IR built via `twig_ir_compiler::FnCtx::emit` (lockstep enforced by construction); fires only if someone builds an `IIRFunction` manually with mismatched vectors |

---

## Dependencies

- `interpreter-ir` (path) — `IIRModule`, `IIRFunction`, `SourceLoc`.

That's it.  No I/O, no FFI, no unsafe.  See `required_capabilities.json`.

---

## Tests

16 unit tests covering empty cases, basic projection, sorting,
synthetic-position dropping, `iir_hit_count` aggregation,
multi-function lines, query helpers (`lines_for_file`, `files`),
all three `CoverageError` variants, error display, and a realistic
two-function trace.

```sh
cargo test -p iir-coverage
```

---

## Roadmap

- **`vm-core` / `lispy-runtime` / `twig-vm` coverage hook** — populate
  the `ExecutionTrace` from the dispatcher (matches LANG18 §"vm-core
  coverage API").
- **`coverage-json`** — JSON exporter for tooling interop (cobertura,
  lcov, etc.).
- **`coverage-terminal`** — pretty per-file ANSI report for the CLI.
- **LSP code-lens provider** — show per-line coverage in the editor
  (consumes this crate via `code-coverage-lsp`).
- **Multi-file overload** — `build_report_multi_file(module, trace,
  file_for_function)` once Twig modules lower to a multi-source
  `IIRModule`.
- **Memory follow-up** — replace the `BTreeSet<(String, usize)>`
  per-line dedup key with a non-cloning encoding (function index)
  if traces ever grow large enough to matter.
