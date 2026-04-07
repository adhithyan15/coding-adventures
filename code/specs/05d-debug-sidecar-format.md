# 05d — Debug Sidecar Format

## Overview

When a debugger pauses execution on a line of source code, it needs to answer a question that sounds simple but requires careful bookkeeping: "The VM just stopped at bytecode offset 0x1D. What line of source code is that?"

The answer lives in the **debug sidecar** — a companion file emitted alongside every bytecode file that maps the bytecode world back to the source world.

This spec defines the binary format of that sidecar, the writer API (called by the compiler), and the reader API (called by the debug adapter). The format is **language-agnostic**: it makes no assumptions about BASIC, Python, JavaScript, or any other language. It only knows about offsets, source locations, named storage slots, and lexical scopes.

## Layer Position

```
Lexer → Parser → Bytecode Compiler → [EMITS .dbg ALONGSIDE .bytecode]
                                              ↓
                                       Debug Adapter reads .dbg
                                              ↓
                                       VS Code gets breakpoints, step, inspect
```

**Input from:** Bytecode compiler (during compilation, the compiler calls the sidecar writer at each emit).
**Output to:** Debug adapter (reads the sidecar to translate between bytecode offsets and source locations).

## Concepts

### Why one source line becomes many bytecode instructions

Consider this BASIC line:

```basic
10 LET X = A + B * C
```

The compiler must respect operator precedence (multiply before add), so it emits:

```
offset 0x1A:  LOAD  A      ← load variable A
offset 0x1B:  LOAD  B      ← load variable B
offset 0x1C:  LOAD  C      ← load variable C
offset 0x1D:  MUL          ← B * C
offset 0x1E:  ADD          ← A + (B*C)
offset 0x1F:  STORE X      ← X = result
```

Six instructions, one source line. The debugger needs to know that all of offsets 0x1A through 0x1F belong to line 10 of the source file.

The sidecar stores exactly this mapping, plus richer information needed for variable inspection, call stack display, and step operations.

### The sidecar is separate from bytecode

The sidecar is kept in a separate file (`program.dbg` alongside `program.bytecode`) for two reasons:

1. **Zero cost in production.** The VM never loads `.dbg` unless a debugger attaches. The execution hot path is untouched.
2. **Strippable.** You can ship a release build without the sidecar and lose no runtime behaviour — only debuggability.

This is the same design used by CLR (`.pdb` files), native binaries (DWARF sections stripped with `strip -g`), and JavaScript source maps (`.map` files referenced by a comment).

### Five things the sidecar tracks

| Section | Question it answers |
|---|---|
| **Source file table** | Which source files contributed to this bytecode? |
| **Line number table** | For offset N, what file/line/column is it from? |
| **Execution unit table** | What are the functions/subroutines/closures? Where do they start and end? |
| **Variable table** | What is the human name of slot 3 in frame 0 while at offset 0x2A? |
| **Scope table** | How are lexical scopes nested inside each execution unit? |

## Binary Format

The file begins with a fixed header followed by a directory of sections. Each section is self-describing and optional — a reader that does not understand a section ID can skip it.

### Magic and Header

```
Offset  Size  Field
──────  ────  ─────────────────────────────────────────────
0x00    4     magic: 0x44 0x42 0x47 0x00  (ASCII "DBG\0")
0x04    2     format_version: u16         (currently 1)
0x06    1     vm_hint: u8                 (0=stack, 1=register, 0xFF=unspecified)
0x07    1     section_count: u8
0x08    N*6   section_directory           (section_count entries, see below)
```

`vm_hint` is advisory. The sidecar format is valid for both VM types; the hint helps readers pre-allocate data structures appropriately.

**Section directory entry (6 bytes each):**

```
Offset  Size  Field
──────  ────  ─────────────────────────────────────
0       1     section_id: u8
1       1     reserved: u8  (must be 0)
2       4     section_offset: u32  (byte offset from start of file)
```

Sections may appear in any order. A reader locates a section by scanning the directory for its ID.

### Section IDs

| ID | Name | Required |
|---|---|---|
| 0x01 | String Table | Yes |
| 0x02 | Source File Table | Yes |
| 0x03 | Line Number Table | Yes |
| 0x04 | Execution Unit Table | Yes |
| 0x05 | Variable Table | No |
| 0x06 | Scope Table | No |

Future section IDs may be added. Readers must skip unknown sections.

### Section 0x01 — String Table

All human-readable strings (file paths, variable names, type hints, unit names) are stored once in a deduplicated string table. Every other section references strings by their byte offset within this section.

```
[null-terminated string] [null-terminated string] ... [0x00]
```

The section ends with an extra null byte. Offset 0 in the string table is always the empty string `""`.

Example:
```
Offset  Content
──────  ───────
0       \0          (empty string — the canonical "unknown/unnamed")
1       hello.bas\0
10      <main>\0
17      x\0
19      integer\0
```

A name field in another section containing the value `1` means `hello.bas`. A value of `17` means `x`. This deduplication keeps the file compact when the same variable name appears in hundreds of entries.

### Section 0x02 — Source File Table

Lists all source files that contributed to this bytecode.

```
[count: u16]
per entry (44 bytes):
  path_strtab_offset: u32    (offset into string table)
  checksum: bytes[32]        (SHA-256 of the source file at compile time)
  reserved: bytes[8]         (must be zero)
```

The checksum allows the debug adapter to warn the user if the source file has changed since the bytecode was compiled — the classic "the breakpoint won't land correctly" problem.

File IDs are implicit: the first entry is ID 0, the second is ID 1, and so on.

### Section 0x03 — Line Number Table

Maps bytecode offsets to source locations. This is the most-queried section at runtime.

The table is stored as a sorted sequence of **delta-encoded rows**. Delta encoding exploits the fact that adjacent instructions almost always come from the same or adjacent source lines — storing "line changed by +1" instead of the full line number keeps each row very small.

**Row format (8 bytes):**
```
d_offset: u16    delta from previous row's offset (first row: absolute offset)
d_line:   i16    delta from previous row's line   (first row: absolute line)
column:   u16    absolute column number (1-based)
file_id:  u8     index into source file table
flags:    u8     reserved (0)
```

The table is terminated by a sentinel row of all zeros.

**Example** — compiling the BASIC program:

```basic
10 PRINT "hello"
20 LET X = 1 + 2
30 END
```

Produces bytecode:
```
0x00  LOAD_CONST  "hello"     ← line 10
0x02  PRINT                   ← line 10
0x03  LOAD_CONST  1           ← line 20
0x05  LOAD_CONST  2           ← line 20
0x07  ADD                     ← line 20
0x08  STORE  X                ← line 20
0x0A  HALT                    ← line 30
```

Line number table (as decoded values, not on-disk deltas):
```
offset  line  col  file
──────  ────  ───  ────
0x00    10    1    0
0x02    10    7    0       ← PRINT keyword starts at col 7
0x03    20    1    0
0x05    20    11   0       ← literal "2" is at column 11
0x07    20    9    0       ← "+" operator
0x08    20    5    0       ← "X =" target
0x0A    30    1    0
[sentinel]
```

**Delta encoding of the same rows (what's on disk):**
```
d_offset  d_line  col   file_id
────────  ──────  ────  ───────
0x0000    10      1     0       ← first row: absolute values
0x0002    0       7     0       ← same line (delta=0), col 7
0x0001    10      1     0       ← new line (delta=+10)
0x0002    0       11    0
0x0002    0       9     0
0x0001    0       5     0
0x0002    10      1     0
0x0000    0       0     0       ← sentinel
```

**Lookup algorithm (offset → source location):**

To find the source location for a given bytecode offset, do a linear scan accumulating the deltas until you find the last row whose accumulated offset is ≤ the query offset:

```python
def lookup_offset(line_table, query_offset):
    best = None
    current_offset = 0
    current_line = 0
    for row in line_table:
        current_offset += row.d_offset
        current_line   += row.d_line
        if current_offset > query_offset:
            break
        best = SourceLocation(
            file_id = row.file_id,
            line    = current_line,
            column  = row.column,
        )
    return best
```

For production use, pre-decode the table into a sorted array and binary-search by accumulated offset.

**Reverse lookup (source location → offset):**

To set a breakpoint on line 20, scan the decoded table for the first row whose `(file_id, line)` matches:

```python
def lookup_line(line_table, file_id, line):
    current_offset = 0
    current_line = 0
    for row in line_table:
        current_offset += row.d_offset
        current_line   += row.d_line
        if row.file_id == file_id and current_line == line:
            return current_offset
    return None  # line not reachable
```

### Section 0x04 — Execution Unit Table

An *execution unit* is any named, callable unit of code: a function, subroutine, lambda, method, or the top-level program. The language decides what counts as an execution unit; the sidecar just records them.

```
[count: u16]
per entry (20 bytes):
  name_strtab_offset:   u32    (human name, e.g. "main", "add", "GOSUB_100")
  start_offset:         u32    (first bytecode offset in this unit)
  end_offset:           u32    (one-past-last bytecode offset)
  param_count:          u8     (number of parameters, 0 for top-level)
  parent_unit_id:       i16    (-1 if no parent; index into this table otherwise)
  reserved:             bytes[5]
```

Unit IDs are implicit: first entry is ID 0, and so on.

The `parent_unit_id` enables the debug adapter to reconstruct the call tree (which function defined which nested function), distinct from the runtime call stack.

For BASIC's top-level program, there is one execution unit named `<main>` with `param_count=0` and `parent_unit_id=-1`.

### Section 0x05 — Variable Table

Maps storage slots (for stack VMs) or register indices (for register VMs) to human-readable names, valid over a specific range of bytecode offsets.

```
[count: u32]
per entry (24 bytes):
  unit_id:          u16    (which execution unit owns this variable)
  slot_or_reg:      u16    (stack slot index or register number)
  name_strtab:      u32    (human name)
  type_hint_strtab: u32    (type name as string, e.g. "integer", "string", "")
  live_start:       u32    (first bytecode offset where this binding is valid)
  live_end:         u32    (one-past-last offset where valid)
```

`type_hint` is a string, not an enum, so it works for any language's type system. For dynamically typed languages, it can be empty or record the inferred type.

**Live ranges** are what allow the debugger to say "variable `x` goes out of scope here" and to handle the common case where a slot is reused for different variables in different parts of a function:

```
offset 0x00–0x1F:  slot 2 is "i"    (loop counter)
offset 0x20–0x3F:  slot 2 is "tmp"  (scratch variable in second block)
```

Without live ranges, inspecting slot 2 after 0x1F would show stale data and the wrong name.

### Section 0x06 — Scope Table

Lexical scopes form a tree inside each execution unit. The scope table stores this tree so the debug adapter can answer "what variables are in scope here?" and implement step-over correctly (step-over means "advance until we're at a different line in the same or shallower scope").

```
[count: u16]
per entry (14 bytes):
  unit_id:          u16    (which execution unit)
  parent_scope_id:  i16    (-1 for the root scope of a unit)
  start_offset:     u32
  end_offset:       u32
  reserved:         bytes[2]
```

Scope IDs are implicit. Variables in the variable table can optionally reference a scope ID if multiple variables in the same unit share the same slot over disjoint live ranges.

## Writer API

The bytecode compiler calls the sidecar writer as it emits instructions. The writer accumulates state and serialises to disk when compilation completes.

```elixir
# Create a new writer for a compilation unit
writer = SidecarWriter.new(vm_hint: :stack)

# Register a source file; returns a file_id
{file_id, writer} = SidecarWriter.add_source_file(writer, "hello.bas", checksum)

# Before emitting each instruction, record the source location
writer = SidecarWriter.record_location(writer,
  offset:  compiler.current_offset(),
  file_id: file_id,
  line:    current_token.line,
  column:  current_token.column
)

# When entering a function/subroutine
{unit_id, writer} = SidecarWriter.begin_unit(writer,
  name:         "add",
  start_offset: compiler.current_offset(),
  param_count:  2,
  parent_unit:  parent_unit_id
)

# When a variable enters scope
writer = SidecarWriter.declare_variable(writer,
  unit_id:    unit_id,
  slot:       compiler.allocate_slot("x"),
  name:       "x",
  type_hint:  "integer",
  live_start: compiler.current_offset()
)

# When a variable goes out of scope
writer = SidecarWriter.end_variable(writer, unit_id, slot, live_end: compiler.current_offset())

# When a function/subroutine ends
writer = SidecarWriter.end_unit(writer, unit_id, end_offset: compiler.current_offset())

# Serialise to disk
:ok = SidecarWriter.write_file(writer, "hello.dbg")
```

The writer deduplicates strings automatically — calling `record_location` with the same file and line as the previous call is a no-op (the delta-encoded row would be all zeros), so the writer suppresses it.

## Reader API

The debug adapter loads the sidecar once when the debug session starts and makes two kinds of queries:

```elixir
# Load the sidecar
{:ok, sidecar} = SidecarReader.load("hello.dbg")

# --- Breakpoint path: source → offset ---
# "Set a breakpoint on line 20 of hello.bas"
{:ok, offset} = SidecarReader.source_to_offset(sidecar, file: "hello.bas", line: 20)
# => {:ok, 0x03}

# --- Stopped path: offset → source ---
# "The VM stopped at offset 0x07. Where is that?"
{:ok, loc} = SidecarReader.offset_to_source(sidecar, 0x07)
# => {:ok, %{file: "hello.bas", line: 20, column: 9}}

# --- Stack frame path ---
# "The VM's call stack has frame with return_offset 0x1A. What unit is that in?"
{:ok, unit} = SidecarReader.find_unit(sidecar, offset: 0x1A)
# => {:ok, %{id: 0, name: "<main>", start: 0x00, end: 0xFF}}

# --- Variable inspection path ---
# "In unit 0 at offset 0x1A, what are all live variables?"
vars = SidecarReader.live_variables(sidecar, unit_id: 0, at_offset: 0x1A)
# => [%{slot: 0, name: "x", type_hint: "integer"}, ...]
```

## File Extension and Naming Convention

| File | Contents |
|---|---|
| `program.bytecode` | Bytecode instructions (opaque to debugger) |
| `program.dbg` | Debug sidecar (read only by debug adapter) |

When the compiler writes `hello.bytecode`, it writes `hello.dbg` alongside it. The debug adapter locates the sidecar by replacing the bytecode extension with `.dbg`.

## Relationship to the Bytecode Compiler

The bytecode compiler (`04-bytecode-compiler.md`) emits a `CodeObject` containing instructions, constants, and a name table. The sidecar is a separate, parallel output — the compiler builds both simultaneously during its single AST walk.

Concretely: every time `GenericCompiler.emit/3` is called, the compiler also calls `SidecarWriter.record_location/4` with the current source position. This guarantees every bytecode offset has a corresponding line table entry.

```
compile(ast_node):
    before_offset = compiler.current_offset()
    compiler = emit_instructions_for(ast_node)
    writer   = record_location(writer, before_offset, ast_node.start_line, ast_node.start_column)
    return compiler, writer
```

## Converters to Standard Debug Formats

A core goal of the sidecar design is that **existing tools should work** — `gdb`, `lldb`, `WinDbg`, Chrome DevTools, and any other debugger that speaks a standard format should be able to consume debug information produced by our compiler toolchain without modification.

The sidecar is our *canonical* internal format. Converters are one-way exporters that read a `.dbg` file and write a standard format. This means:

- Our own DAP-based debugger reads `.dbg` natively (fast, zero conversion overhead)
- When a user needs to use `gdb` or `lldb`, they run a converter once and get a standard ELF+DWARF binary
- The converter is a separate tool — the compiler and VM are not coupled to any specific output format

### Standard Formats Worth Targeting

| Format | Used by | Enables |
|---|---|---|
| **DWARF** (inside ELF) | `gdb`, `lldb`, `perf`, `valgrind`, `addr2line` | Native Linux/macOS tooling, profiling, crash analysis |
| **PDB / Portable PDB** | `WinDbg`, Visual Studio, `dotnet-dump` | Windows tooling, .NET ecosystem |
| **Source Maps** (`.map`) | Chrome DevTools, Firefox DevTools, Node.js | Browser debugging of languages that compile to JS-like output |
| **JVM LineNumberTable** | `jdb`, IntelliJ, JVM profilers | JVM ecosystem interop |

### Converter Architecture

Every converter implements the same interface:

```elixir
defmodule SidecarConverter do
  @type options :: keyword()

  @callback convert(
    sidecar_path :: String.t(),
    bytecode_path :: String.t(),
    output_path :: String.t(),
    opts :: options()
  ) :: :ok | {:error, String.t()}
end
```

Converters read the sidecar via `SidecarReader` and the bytecode file, then write the target format. They are standalone CLI tools, not library dependencies of the compiler or VM:

```
# Produce an ELF binary with embedded DWARF from our bytecode + sidecar
dbg-to-dwarf hello.bytecode hello.dbg --output hello.elf

# Produce a source map alongside a JS-compiled output
dbg-to-sourcemap hello.bytecode hello.dbg --output hello.map

# Produce a .pdb sidecar for Windows tooling
dbg-to-pdb hello.bytecode hello.dbg --output hello.pdb
```

### DWARF Converter (Primary Target)

DWARF is the most important target because it unlocks the entire native toolchain ecosystem. A DWARF converter maps our sidecar sections to DWARF sections inside an ELF binary:

| Sidecar section | DWARF equivalent |
|---|---|
| Line Number Table | `.debug_line` section |
| Execution Unit Table | `.debug_info` `DW_TAG_subprogram` entries |
| Variable Table | `.debug_info` `DW_TAG_variable` / `DW_TAG_formal_parameter` entries |
| Scope Table | `.debug_info` `DW_TAG_lexical_block` entries |
| String Table | `.debug_str` section |
| Source File Table | `.debug_line` file table |

What the converter produces is an ELF binary where the `.text` section contains the raw bytecode (treated as an opaque byte sequence) and the DWARF sections describe its source mapping. `gdb` and `lldb` can then load this file, set breakpoints by source line, and inspect variables — using their native UI, not our DAP adapter.

```
# After conversion, standard tools work:
$ gdb hello.elf
(gdb) break hello.bas:20
Breakpoint 1 at 0x3: file hello.bas, line 20.
(gdb) run
Breakpoint 1, main () at hello.bas:20
20      LET X = A + B * C
(gdb) print X
$1 = 42
```

### Source Map Converter

Source maps are a JSON format originally designed for JavaScript minifiers but now used broadly wherever one text format compiles to another. A source map records the mapping from output positions back to input positions.

Our line number table maps directly to source map v3 `mappings` (VLQ-encoded offset pairs). The converter:

1. Reads the line number table from the sidecar
2. Encodes each row as a VLQ-encoded mapping segment
3. Writes a `.map` JSON file

This allows any tool that understands source maps (Chrome DevTools, Node.js `--enable-source-maps`, Jest, etc.) to display our language's source code instead of raw bytecode addresses in stack traces and profiler output.

### Keeping the Sidecar as the Source of Truth

Converters are **always lossy in reverse**: you can go from `.dbg` to DWARF, but going from DWARF back to `.dbg` loses information specific to our format. The sidecar is therefore the canonical, lossless representation. Converters are generated outputs, not inputs to our toolchain.

This means:
- The compiler always emits `.dbg` — never DWARF, PDB, or source maps directly
- Converters are run as a post-processing step, not part of the core pipeline
- Adding support for a new standard format means writing a new converter, not changing the compiler

## Size Characteristics

For a typical small program (1000 bytecode instructions, one source file):

| Section | Approximate size |
|---|---|
| Header + directory | 56 bytes |
| String table | ~200 bytes |
| Source file table | ~48 bytes |
| Line number table | ~8 KB (8 bytes/row × 1000 rows) |
| Execution unit table | ~200 bytes (10 units × 20 bytes) |
| Variable table | ~2.4 KB (100 variables × 24 bytes) |
| Scope table | ~140 bytes (10 scopes × 14 bytes) |
| **Total** | **~11 KB** |

This is negligible compared to the compiler toolchain itself. Even for large programs (100,000 instructions), the sidecar remains well under 1 MB.
