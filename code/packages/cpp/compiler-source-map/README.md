# compiler-source-map (C++)

The source-mapping sidecar for an AOT compiler pipeline, in pure ISO C++17,
header-only, in namespace `ca::csm`. A faithful port of the Rust
`compiler-source-map` crate.

As a program is lowered `source → AST → IR → (optimiser passes) → machine code`,
this sidecar records at each stage which IDs map to which, so any machine-code
location can be traced back to its source position and vice-versa. Four segments
(SourceToAst, AstToIr, IrToIr per pass, IrToMachineCode) plus a `SourceMapChain`
that composes them.

## API

```cpp
#include "compiler_source_map.hpp"
namespace csm = ca::csm;

csm::SourceMapChain chain;
csm::SourcePosition pos{"test.bf", 1, 1, 1};
chain.source_to_ast.add(pos, 0);
chain.ast_to_ir.add(0, {7, 8, 9, 10});
csm::IrToMachineCode mc;
mc.add(7, 0, 4);
chain.ir_to_machine_code = std::move(mc);

auto entries = chain.source_to_mc(pos);       // std::optional<vector<…>>
const csm::SourcePosition* p = chain.mc_to_source(0);
```

Value semantics throughout (mirroring the Rust structs' public fields). `Option`
becomes `std::optional` or a borrowed pointer (nullptr when absent); lookups are
linear scans ("first match wins").

## Portability

Pure ISO C++17 — standard library only. Compiles clean under GCC, Clang, and MSVC
with `-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).

## Development

```bash
sh BUILD
```
