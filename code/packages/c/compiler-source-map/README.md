# compiler-source-map (C)

The source-mapping sidecar for an AOT compiler pipeline, in pure ISO C17. A
faithful port of the Rust `compiler-source-map` crate.

As a program is lowered `source → AST → IR → (optimiser passes) → machine code`,
this sidecar records, at each stage, which IDs map to which — so any error,
breakpoint, or profiling sample on the machine code can be traced back to its
original source position, and vice-versa. Four segments plus a chain:

1. **SourceToAst** — source position → AST node ID
2. **AstToIr** — AST node ID → IR instruction IDs (one-to-many)
3. **IrToIr** (one per pass) — original IR ID → optimised IR IDs (or deleted)
4. **IrToMachineCode** — IR ID → machine-code byte range

## API

```c
#include "compiler_source_map.h"

SmapChain *c = smap_chain_new();
SmapPosition p = {"test.bf", 1, 1, 1};
smap_s2a_add(smap_chain_source_to_ast(c), &p, 0);
int64_t ir[] = {7, 8, 9, 10};
smap_a2i_add(smap_chain_ast_to_ir(c), 0, ir, 4);
SmapIrToMc *mc = smap_i2mc_new();
smap_i2mc_add(mc, 7, 0, 4);
smap_chain_set_machine_code(c, mc);      /* chain takes ownership */

SmapMcEntry *res; size_t n;
smap_chain_source_to_mc(c, &p, &res, &n);   /* forward: pos → MC entries */
free(res);
smap_chain_mc_to_source(c, 0);              /* reverse: MC offset → pos */
smap_chain_free(c);
```

Each segment is a malloc'd handle (`*_new` / `*_free`). The chain owns its
SourceToAst/AstToIr segments (get borrowed pointers to fill) and **takes
ownership** of the passes and backend segment handed to it. `Option` becomes a
status/NULL; lookups are linear scans ("first match wins"), and every dynamic
array guards its growth against `size_t` overflow.

## Portability

Pure ISO C17 — no POSIX `strdup`, no extensions. Compiles clean under GCC, Clang,
and MSVC with `-pedantic-errors` / `/permissive-` and warnings-as-errors, via the
shared [`iso-harness`](../iso-harness).

## Development

```bash
sh BUILD
```
