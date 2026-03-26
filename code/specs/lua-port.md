# Lua Port

## Overview

This spec describes the plan to port all existing packages in the coding-adventures
monorepo to Lua 5.4. Lua is the fifth implementation language alongside Python,
Ruby, Go, and Rust.

Lua fills a niche none of the existing languages cover: **embeddability and
minimalism as design goals**. The entire language is intentionally small — 8 types,
one compound data structure (the table), and a minimal standard library. Every
feature earns its place. This constraint produces genuinely interesting teaching
moments throughout the computing stack.

## Why Lua

| Property | Teaching value |
|----------|---------------|
| Tables as everything | Arrays, dictionaries, objects, modules, and namespaces are all the same structure. Forces a rethink of how data structures emerge from primitives. |
| Metatables and metamethods | Operator overloading and OOP without classes. A fascinating contrast to Ruby's class system or Go's interfaces. |
| Coroutines (first-class) | Cooperative multitasking baked into the language. Could model pipeline stages or VM eval loops as suspendable iterators. |
| Embeddability | Lua is designed to live *inside* other programs (Nginx/OpenResty, Neovim, Redis, Roblox). A completely different relationship with a host environment. |
| Minimal syntax | Fewer keywords than Python. The literate programming style benefits from a language where the explanations can dominate the syntax. |

## Language Version

**Lua 5.4** (released June 2020, latest stable).

Why 5.4 specifically:

- **Native integers**: Lua 5.4 has separate integer and float subtypes (both 64-bit).
  Earlier versions used doubles for everything, which complicates bit manipulation.
- **Bitwise operators**: Native `&`, `|`, `~`, `>>`, `<<`, `~` operators (introduced
  in 5.3). Essential for logic-gates, arithmetic, fp-arithmetic, assembler, and all
  ISA simulator packages.
- **`string.pack` / `string.unpack`**: Binary data encoding/decoding (introduced in
  5.3). Needed for fp-arithmetic IEEE 754 bit extraction and assembler binary output.
- **Generalized `for` with `<close>`**: Deterministic resource cleanup via to-be-closed
  variables. Useful for file I/O in html-renderer and pipeline.

Lua 5.4 is available on all three CI platforms (Linux, macOS, Windows) via standard
package managers and GitHub Actions.

## Packages to Port

All 27 packages currently implemented in Go and Python will be ported to Lua.
The table below lists every package, its Lua directory name, and any Lua-specific
notes on the implementation approach.

### Layer 10 — Logic Gates (Foundation)

| Package | Lua directory | Notes |
|---------|--------------|-------|
| logic-gates | `logic_gates` | Metatables with `__call` metamethod make gate objects callable. Multi-input variants use variadic `...` args. NAND-derived gates demonstrate functional completeness. Sequential logic (latches, flip-flops, counters, shift registers) use tables for state. |

### Layer 9 — Arithmetic

| Package | Lua directory | Notes |
|---------|--------------|-------|
| arithmetic | `arithmetic` | Half/full adder, ripple-carry adder, ALU. Native `&`, `|`, `~` operators for bit manipulation. Two's complement subtraction via `~ (complement) + 1`. Status flags (zero, carry, negative, overflow) returned as a table. |
| fp-arithmetic | `fp_arithmetic` | IEEE 754 FP32/FP16/BF16. Use `string.pack("f", n)` and `string.unpack("I4", packed)` to extract raw bits. Barrel shifter, leading-one detector built from logic gates. FMA with single rounding. |

### Layer 8 — CPU

| Package | Lua directory | Notes |
|---------|--------------|-------|
| clock | `clock` | Cycle counter as a simple table with metatable for `:tick()`, `:reset()`, `:current()`. |
| cpu-simulator | `cpu_simulator` | Registers as a table (indexed 0..N). Memory as a table with `__index`/`__newindex` metamethods for bounds checking. Fetch-decode-execute as a coroutine — yields after each cycle for debuggability. |

### Layer 7 — ISA Simulators

| Package | Lua directory | Notes |
|---------|--------------|-------|
| arm-simulator | `arm_simulator` | 16 registers (table indexed 0..15). Condition code evaluation via lookup table. Operand2 barrel shifter. 32-bit instruction decoding with `>>` and `&` masking. |
| riscv-simulator | `riscv_simulator` | 32 registers (table, index 0 hardwired to 0 via `__newindex` metamethod). R/I/S/B/U/J format decoding. Sign extension via arithmetic shift. |
| wasm-simulator | `wasm_simulator` | Stack machine — operand stack as a table with `table.insert`/`table.remove`. Local variables as a table. Linear memory as a table. Block/loop control flow with label stack. |
| intel4004-simulator | `intel4004_simulator` | 4-bit data path. All values masked with `& 0xF`. Accumulator + 16 registers (4-bit each). Historical: world's first commercial microprocessor. |

### Layers 5-6 — Assembler and Compiler

| Package | Lua directory | Notes |
|---------|--------------|-------|
| assembler | `assembler` | Two-pass assembly. Pass 1: build symbol table (label → address) as a table. Pass 2: encode instructions to binary using `string.pack`. Output: binary string + symbol table. |
| bytecode-compiler | `bytecode_compiler` | AST visitor pattern via table dispatch (`node_handlers[node.type](node)`). Constant pool and name pool as tables. Jump offset patching. Emits CodeObject table (instructions + constants + names). |

### Layers 2-4 — Lexer, Parser, VM

| Package | Lua directory | Notes |
|---------|--------------|-------|
| lexer | `lexer` | Character-by-character scanning with `string.byte`. Token positions tracked as `{line, column}`. Keyword set as a hash table for O(1) lookup. Error recovery via `:skip_to_next_token()`. |
| parser | `parser` | Recursive descent. AST nodes as tables: `{type="BinaryOp", op="+", left=..., right=...}`. Operator precedence encoded via grammar rule nesting. Error messages include token position. |
| virtual-machine | `virtual_machine` | Stack-based eval loop. Value stack as a table. Environment (variable bindings) as a table. Instruction dispatch via table lookup (`handlers[opcode](vm)`). Trace mode: captures per-step snapshots as tables. The eval loop could optionally be a coroutine that yields after each instruction for step-through debugging. |
| jvm-simulator | `jvm_simulator` | Typed opcodes (iadd, ladd, fadd, dadd). Local variable slots as a table. Operand stack as a table. Method frame management. |
| clr-simulator | `clr_simulator` | Type-inferred opcodes (add works on any type). Multi-byte opcode decoding (0xFE prefix). Stack + locals as tables. |
| jit-compiler | `jit_compiler` | Future package — spec exists but not yet implemented in any language. Shell only. |

### Language-Specific Lexers/Parsers

| Package | Lua directory | Notes |
|---------|--------------|-------|
| python-lexer | `python_lexer` | INDENT/DEDENT generation from indentation tracking. Python keyword recognition. String prefix handling (f-strings, r-strings, b-strings). |
| python-parser | `python_parser` | Python grammar: decorators, comprehensions, function definitions. AST output as tables. |
| ruby-lexer | `ruby_lexer` | Symbol literals, string interpolation, block syntax. Ruby keyword recognition. |
| ruby-parser | `ruby_parser` | Ruby grammar: blocks, method definitions, Ruby-specific syntax. |
| grammar-tools | `grammar_tools` | Grammar definitions as tables of rules. Future: parser/lexer generator from grammar. |

### Cross-Cutting Packages

| Package | Lua directory | Notes |
|---------|--------------|-------|
| directed-graph | `directed_graph` | Adjacency list as `{node = {neighbor1, neighbor2, ...}}`. Topological sort, cycle detection, reachability, affected-node computation. All graph algorithms use tables + recursion. |
| pipeline | `pipeline` | Orchestrator chaining all stages. Each stage's output is a table (PipelineReport). Coroutines are a natural fit here — each stage could be a coroutine that processes data and yields its report. |
| html-renderer | `html_renderer` | Generates self-contained HTML from PipelineReport tables. String building via `table.concat` (Lua's idiomatic string builder — accumulate fragments in a table, join once). Color-coded tokens, SVG AST trees, bytecode tables, execution traces. |
| cache | `cache` | Set-associative cache hierarchy (L1I/L1D/L2/L3). Cache lines as tables `{valid, dirty, tag, data}`. LRU replacement via ordered list. Address decomposition with `>>` and `&`. Hit/miss/eviction statistics. |
| branch-predictor | `branch_predictor` | Static, 1-bit, 2-bit saturating counter, BTB. Prediction tables as tables indexed by branch address. State machine transitions as table lookups. |
| hazard-detection | `hazard_detection` | RAW/WAR/WAW dependency detection. Pipeline stage tracking as tables. Data forwarding logic. |

## Packages NOT Ported

None. All 27 packages will be ported. The jit-compiler package will be created as
a shell (matching its status in other languages).

## Lua Project Layout

Each Lua package follows LuaRocks conventions — the standard Lua package manager
(analogous to PyPI for Python, RubyGems for Ruby).

```
code/packages/lua/logic_gates/
├── coding-adventures-logic-gates-0.1.0-1.rockspec   # Package metadata + dependencies
├── src/
│   └── coding_adventures/
│       └── logic_gates/
│           ├── init.lua          # Module entry point (require "coding_adventures.logic_gates")
│           ├── gates.lua         # Combinational gates (AND, OR, XOR, NOT, NAND, NOR, XNOR)
│           └── sequential.lua    # Sequential logic (latches, flip-flops, counters)
├── tests/
│   ├── test_gates.lua            # Gate truth table tests
│   └── test_sequential.lua       # Sequential logic tests
├── BUILD                         # Build/test command for CI
├── README.md
└── CHANGELOG.md
```

### Why this layout

- **`src/coding_adventures/logic_gates/`**: Mirrors the Python `src/` layout. The
  nested `coding_adventures/logic_gates/` path means the module is imported as
  `require("coding_adventures.logic_gates")`, which is the idiomatic Lua module path.
  This avoids name collisions with other Lua packages.

- **`.rockspec` file**: The LuaRocks equivalent of `pyproject.toml` or `.gemspec`.
  Declares the package name, version, dependencies, and build instructions. Named
  `coding-adventures-{package}-{version}-{revision}.rockspec` following LuaRocks
  conventions.

- **`init.lua`**: The module entry point. When you `require("coding_adventures.logic_gates")`,
  Lua looks for `coding_adventures/logic_gates/init.lua`. This file re-exports the
  public API.

### Naming Conventions

| Aspect | Convention | Example |
|--------|-----------|---------|
| Directory name | `snake_case` | `logic_gates`, `cpu_simulator` |
| Rockspec name | `coding-adventures-{name}` with hyphens | `coding-adventures-logic-gates` |
| Module path | `coding_adventures.{name}` | `coding_adventures.logic_gates` |
| Source files | `snake_case.lua` | `gates.lua`, `sequential.lua` |
| Test files | `test_{name}.lua` | `test_gates.lua` |
| Functions | `snake_case` | `half_adder()`, `ripple_carry_adder()` |
| Constants | `UPPER_CASE` | `OPCODES`, `MAX_REGISTERS` |
| "Classes" (metatables) | `PascalCase` | `Gate`, `CPU`, `Cache` |
| Private functions | `_prefixed` | `_decode_instruction()` |

These conventions match Lua community standards and parallel the Ruby naming approach
already used in this repo.

## Rockspec Format

Each package has a `.rockspec` file declaring metadata and dependencies:

```lua
package = "coding-adventures-logic-gates"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Logic gates — the foundation of all digital circuits",
    detailed = [[
        Implements AND, OR, XOR, NOT, NAND, NOR, XNOR gates plus sequential
        logic (latches, flip-flops, counters, shift registers). Every gate is
        tested against its complete truth table. Layer 10 of the computing stack.
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.logic_gates"] = "src/coding_adventures/logic_gates/init.lua",
        ["coding_adventures.logic_gates.gates"] = "src/coding_adventures/logic_gates/gates.lua",
        ["coding_adventures.logic_gates.sequential"] = "src/coding_adventures/logic_gates/sequential.lua",
    },
}
```

Packages with internal dependencies declare them:

```lua
-- In arithmetic's rockspec:
dependencies = {
    "lua >= 5.4",
    "coding-adventures-logic-gates >= 0.1.0",
}
```

## Testing Framework

**busted** — the standard Lua testing framework (analogous to pytest for Python,
minitest for Ruby).

```lua
-- tests/test_gates.lua
local gates = require("coding_adventures.logic_gates")

describe("AND gate", function()
    it("returns 1 only when both inputs are 1", function()
        assert.are.equal(0, gates.AND(0, 0))
        assert.are.equal(0, gates.AND(0, 1))
        assert.are.equal(0, gates.AND(1, 0))
        assert.are.equal(1, gates.AND(1, 1))
    end)
end)

describe("NAND-derived gates", function()
    it("match their direct implementations", function()
        for _, a in ipairs({0, 1}) do
            for _, b in ipairs({0, 1}) do
                assert.are.equal(gates.AND(a, b), gates.nand_and(a, b))
                assert.are.equal(gates.OR(a, b), gates.nand_or(a, b))
            end
        end
    end)
end)
```

### Test Coverage

**luacov** — the standard Lua coverage tool. Works with busted out of the box.

Coverage targets match repo standards:
- 95%+ for library packages (logic-gates, arithmetic, etc.)
- 80%+ for program packages (pipeline, html-renderer)

## Linting

**luacheck** — the standard Lua static analyzer (analogous to ruff for Python,
standardrb for Ruby, go vet for Go).

Catches:
- Unused variables and unreachable code
- Undefined global variables (critical in Lua since globals are implicit)
- Shadowed variables
- Line length and style issues

A `.luacheckrc` file at `code/packages/lua/.luacheckrc` will configure repo-wide
settings:

```lua
std = "lua54"
max_line_length = 120
globals = {}  -- No implicit globals allowed
```

## BUILD File

Each Lua package's BUILD file:

```
luarocks make --local --lua-version 5.4
busted tests/ --verbose --coverage
```

This installs the package locally (so dependencies resolve correctly) and runs
tests with coverage.

## Build System Changes

The Go build tool needs three modifications to support Lua:

### 1. Discovery: Add "lua" to language inference

In `internal/discovery/discovery.go`, the `inferLanguage` function checks path
components against a list of known languages. Add `"lua"` to that list:

```go
// Before:
for _, lang := range []string{"python", "ruby", "go"} {

// After:
for _, lang := range []string{"python", "ruby", "go", "lua"} {
```

### 2. Resolver: Parse .rockspec dependencies

In `internal/resolver/resolver.go`, add a `parseLuaDeps` function that reads
`.rockspec` files and extracts internal dependencies.

Rockspec dependency format:
```lua
dependencies = {
    "lua >= 5.4",
    "coding-adventures-logic-gates >= 0.1.0",
}
```

Parsing strategy: find lines matching `"coding-adventures-*"` inside the
dependencies block, strip version specifiers, and map to internal names.

Naming convention for the Rosetta Stone mapping:
- Rockspec: `"coding-adventures-logic-gates"` maps to `"lua/logic_gates"`

Add a case to `buildKnownNames`:
```go
case "lua":
    // Convert dir name to rockspec name: "logic_gates" → "coding-adventures-logic-gates"
    // Note: rockspec names use hyphens, dir names use underscores
    rockspecName := "coding-adventures-" + strings.ReplaceAll(
        strings.ToLower(filepath.Base(pkg.Path)), "_", "-")
    known[rockspecName] = pkg.Name
```

Add a case to `ResolveDependencies`:
```go
case "lua":
    deps = parseLuaDeps(pkg, knownNames)
```

### 3. DIRS files: Add lua directory

- Add `lua` to `code/packages/DIRS`
- Create `code/packages/lua/DIRS` listing all 27 package directories

## CI Changes

The GitHub Actions workflow needs Lua toolchain setup:

```yaml
- name: Set up Lua
  uses: leafo/gh-actions-lua@v11
  with:
    luaVersion: "5.4"

- name: Set up LuaRocks
  uses: leafo/gh-actions-luarocks@v4

- name: Install Lua test tools
  run: |
    luarocks install busted
    luarocks install luacov
    luarocks install luacheck
```

These are the standard GitHub Actions for Lua, maintained by the creator of
LuaRocks (leafo). They support Linux and macOS — the two OS platforms already in
our CI matrix.

### Windows Support

The user runs Windows locally. Lua 5.4 is available on Windows via:

- **LuaBinaries**: Precompiled Lua 5.4 binaries for Windows (lua.org)
- **LuaRocks**: Has Windows support via MSVC or MinGW toolchain
- **scoop**: `scoop install lua luarocks` (if using scoop package manager)

For local development on Windows:
- BUILD files use shell syntax (bash), which works under Git Bash or WSL
- Alternatively, `BUILD_windows` could be added for Windows-native commands, though
  the current build tool would need a minor extension to check `runtime.GOOS == "windows"`

The build tool currently checks `BUILD_mac` (darwin) and `BUILD_linux` (linux) but
not `BUILD_windows`. Since the user develops on Windows, we should add this:

```go
// In getBuildFile and GetBuildFileForPlatform:
if system == "windows" {
    platformBuild := filepath.Join(directory, "BUILD_windows")
    if fileExists(platformBuild) {
        return platformBuild
    }
}
```

## Port Order

Packages will be ported bottom-up following the dependency graph. Each layer is
independent within itself, so packages at the same layer can be ported in parallel.

```
Phase 1: Foundation (no dependencies)
  ├── logic_gates
  ├── clock
  ├── directed_graph
  ├── grammar_tools
  └── lexer

Phase 2: Core computation (depends on Phase 1)
  ├── arithmetic          (← logic_gates)
  ├── parser              (← lexer)
  ├── cache               (← clock)
  ├── branch_predictor    (← clock)
  └── hazard_detection    (← clock)

Phase 3: Upper layers (depends on Phase 2)
  ├── fp_arithmetic       (← arithmetic, logic_gates)
  ├── cpu_simulator        (← arithmetic)
  ├── bytecode_compiler    (← parser)
  └── python_lexer, ruby_lexer (← lexer patterns)

Phase 4: Simulators (depends on Phase 3)
  ├── arm_simulator        (← cpu_simulator)
  ├── riscv_simulator      (← cpu_simulator)
  ├── wasm_simulator       (← cpu_simulator)
  ├── intel4004_simulator  (← cpu_simulator)
  ├── virtual_machine      (← bytecode_compiler)
  ├── jvm_simulator        (← standalone)
  ├── clr_simulator        (← standalone)
  └── python_parser, ruby_parser (← parser patterns)

Phase 5: Integration (depends on Phase 4)
  ├── assembler            (← ISA simulators)
  ├── pipeline             (← all stages)
  └── html_renderer        (← pipeline)

Phase 6: Future
  └── jit_compiler         (shell only — matches other languages)
```

## Lua-Specific Teaching Opportunities

Each package should highlight what makes Lua's approach interesting compared to the
other four implementations. These are the unique teaching angles per package:

| Package | Lua-specific angle |
|---------|-------------------|
| logic-gates | `__call` metamethod: gates as callable objects. Demonstrate functional completeness (NAND→everything) using closures. |
| arithmetic | Show how explicit masking (`& 0xFF`) replaces typed integers. Compare to Go's `uint8` and Rust's `u8`. |
| cpu-simulator | Eval loop as a coroutine — `coroutine.yield()` after each cycle. Contrast with Go's goroutines and Python's generators. |
| riscv-simulator | `__newindex` metamethod on register table to enforce x0=0 invariant. The hardware constraint becomes a language constraint. |
| virtual-machine | Instruction dispatch via table lookup vs. switch/case. Benchmark table dispatch vs. if/elseif chains. Coroutine-based step debugger. |
| cache | Metatables for cache line objects. `__tostring` for debug printing. LRU as a doubly-linked list using tables. |
| html-renderer | `table.concat` as string builder pattern. Contrast with Python's `"".join()`, Ruby's `<<`, Go's `strings.Builder`. |
| directed-graph | Pure table-based graph: `graph[node] = {neighbors}`. Show how Lua tables unify array and dictionary concepts. |
| pipeline | Coroutines as pipeline stages — each stage is a coroutine that consumes input and produces output. Demonstrate cooperative multitasking. |

## OOP Pattern

Lua has no native class system. All packages will use the standard metatable-based
OOP pattern consistently:

```lua
-- Define a "class"
local Gate = {}
Gate.__index = Gate

-- Constructor
function Gate.new(name, truth_table)
    local self = setmetatable({}, Gate)
    self.name = name
    self.truth_table = truth_table
    return self
end

-- Method
function Gate:evaluate(a, b)
    return self.truth_table[a * 2 + b + 1]
end

-- Make instances callable
function Gate:__call(a, b)
    return self:evaluate(a, b)
end
```

This pattern is used universally across the Lua ecosystem (LOVE2D, OpenResty, etc.)
and should be introduced in logic-gates (the first package), then reused everywhere.

## Error Handling

Lua uses `error()` + `pcall()`/`xpcall()` for error handling (similar to
exceptions in Python/Ruby, but without try/catch syntax).

All input validation follows the same pattern as other languages:

```lua
function AND(a, b)
    if (a ~= 0 and a ~= 1) or (b ~= 0 and b ~= 1) then
        error("inputs must be 0 or 1, got: " .. tostring(a) .. ", " .. tostring(b))
    end
    return a & b  -- Lua 5.4 native bitwise AND
end
```

## Module System

Each package's `init.lua` re-exports the public API:

```lua
-- src/coding_adventures/logic_gates/init.lua
local gates = require("coding_adventures.logic_gates.gates")
local sequential = require("coding_adventures.logic_gates.sequential")

return {
    -- Combinational gates
    AND = gates.AND,
    OR = gates.OR,
    XOR = gates.XOR,
    NOT = gates.NOT,
    NAND = gates.NAND,
    NOR = gates.NOR,
    XNOR = gates.XNOR,

    -- NAND-derived
    nand_not = gates.nand_not,
    nand_and = gates.nand_and,
    nand_or = gates.nand_or,
    nand_xor = gates.nand_xor,

    -- Multi-input
    AND_N = gates.AND_N,
    OR_N = gates.OR_N,

    -- Sequential
    SRLatch = sequential.SRLatch,
    DLatch = sequential.DLatch,
    DFlipFlop = sequential.DFlipFlop,
    ShiftRegister = sequential.ShiftRegister,
    BinaryCounter = sequential.BinaryCounter,
}
```

## Summary

| Dimension | Decision |
|-----------|----------|
| Language version | Lua 5.4 |
| Package manager | LuaRocks |
| Test framework | busted |
| Coverage tool | luacov |
| Linter | luacheck |
| OOP pattern | Metatables with `__index` |
| Naming | `snake_case` dirs, `coding-adventures-*` rockspecs |
| Total packages | 27 (matching Go and Python) |
| CI platforms | Linux + macOS (via leafo/gh-actions-lua) |
| Local dev | Windows (via LuaBinaries or scoop) |
| Build tool changes | 3 modifications (discovery, resolver, DIRS) |
