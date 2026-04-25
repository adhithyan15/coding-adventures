# F05 Full-IEEE Extensions

## Overview

The existing F05 spec (`code/specs/F05-verilog-vhdl.md`) deliberately scopes Verilog and VHDL to a **synthesizable subset** — the constructs that map to physical hardware. That covers the chip-design side of the world: modules, ports, processes, assignments, FSMs. It does *not* cover the testbench side: `initial` blocks, `$display`, `wait` statements, file I/O, delays, assertions, configurations.

The user has committed to **full IEEE 1076-2008 (VHDL) and IEEE 1364-2005 (Verilog) compliance** — not the synthesizable subset. The full standards include constructs that have no synthesis meaning but are essential for verification, documentation, and modeling. A test bench *must* be able to call `$display` and `wait #10`. A VHDL design *must* be able to use `assert`, `report`, `wait until rising_edge(clk)`, and `textio` reads.

This spec defines the **additive extensions** to F05's `verilog.tokens`, `verilog.grammar`, `vhdl.tokens`, and `vhdl.grammar` files needed to cover the full standards. It is purely additive — every construct F05 currently parses must continue to parse identically. New constructs are added; none are removed; none are reinterpreted.

The downstream consumer of all this is `hdl-ir.md`. Some extensions land in HIR as first-class node types (e.g., `wait`, `delay`, `initial`); others land as auxiliary metadata (e.g., `attribute`); some are documented as "parsed but ignored by simulation" (e.g., `specify` blocks, which are timing annotations).

### Generality

These extensions are necessary for any non-trivial digital design verification — a 4-bit adder testbench needs `initial begin a = 0; ... end` and `$display` and `$finish`. A larger design (32-bit ALU, RISC-V core) needs `assert` / `report` for self-checking, `wait` for clock generation, file I/O for memory-image loading, and configurations for managing multi-architecture builds.

## Layer Position

```
Verilog/VHDL source text (FULL IEEE)
        │
        ▼
┌─────────────────────────────┐
│  F05 grammars + this spec   │
│  verilog.tokens (extended)  │
│  vhdl.tokens (extended)     │
│  verilog.grammar (extended) │
│  vhdl.grammar (extended)    │
└─────────────────────────────┘
        │
        ▼
        AST (full-IEEE shape)
        │
        ▼
hdl-ir.md elaborator
        │
   ┌────┴─────────────────┐
   ▼                      ▼
synthesis.md          hardware-vm.md
(ignores testbench   (uses everything)
 constructs)
```

## Compatibility Statement

| Requirement | Mechanism |
|---|---|
| All existing F05-valid programs must continue to parse | Extensions are pure additions. Grammar rules are added, not modified. |
| AST shape for existing constructs is unchanged | New AST node types are added in their own modules. |
| Lexer behavior for existing tokens is unchanged | New tokens are added; existing token regexes are not re-ordered or modified. |
| Synthesizable-subset programs do not need any new tokens | The "subset" remains a subset. |
| Existing tests in F05 packages must pass unmodified | Verified by re-running F05 unit tests after extensions land. |

## Verilog Extensions (IEEE 1364-2005)

### V-Ext-1: Initial blocks

```verilog
initial begin
  a = 0;
  b = 1;
  #10 a = 1;
end
```

**Grammar additions to `verilog.grammar`:**

```ebnf
module_item    = ... existing ...
               | initial_construct ;

initial_construct = "initial" statement ;
```

**AST**: `InitialConstruct(body: Statement)`. Already partially in `F05` as a deferred construct; promote to first-class.

**HIR mapping**: process with no sensitivity, runs once at time 0.

### V-Ext-2: Delay control

```verilog
#10 a = 1;
@(posedge clk) b = a;
@(*) y = a + b;
```

**Tokens (already exist)**: `HASH`, `AT`, `STAR`.

**Grammar additions:**

```ebnf
delay_or_event_control = HASH delay_value
                       | AT event_control ;

delay_value = NUMBER
            | REAL_NUMBER
            | LPAREN expression RPAREN ;

event_control = LPAREN event_expression { ("or" | COMMA) event_expression } RPAREN
              | LPAREN STAR RPAREN ;

event_expression = [ "posedge" | "negedge" ] expression ;

procedural_timing_control_statement =
    delay_or_event_control statement_or_null ;

blocking_assignment    = lvalue [delay_or_event_control] EQUALS expression ;
nonblocking_assignment = lvalue [delay_or_event_control] LESS_EQUALS expression ;
```

**AST**: `Delay(value: Expr, body: Stmt)`, `EventControl(events: list[EventExpr], body: Stmt)`.

**HIR mapping**: `wait` continuation in the simulation VM. Synthesis rejects with a clear error ("delay control is not synthesizable; use a clocked process instead").

### V-Ext-3: System tasks and functions

```verilog
$display("count = %d", count);
$monitor("%t: a=%b b=%b", $time, a, b);
$write("hello\n");
$finish;
$stop;
$random
$time, $stime, $realtime
$dumpfile("trace.vcd"); $dumpvars(0, top);
$readmemh("rom.hex", mem);
$readmemb("rom.bin", mem);
$fopen, $fclose, $fwrite, $fread, $fscanf, $sscanf
```

**Tokens (already exist)**: `SYSTEM_ID = /\$[a-zA-Z_][a-zA-Z0-9_$]*/`.

**Grammar additions**: Already covered by the existing `primary` rule's `NAME LPAREN ... RPAREN` form, generalized to accept `SYSTEM_ID` as a function name. We only need to extend the `primary` rule:

```ebnf
primary = ... existing ...
        | SYSTEM_ID
        | SYSTEM_ID LPAREN [ expression { COMMA expression } ] RPAREN ;
```

**AST**: `SystemTaskCall(name: str, args: list[Expr])` — the lexer already produces `SYSTEM_ID`.

**HIR mapping**: registered as built-in tasks. `$display` / `$monitor` produce side-effect statements that the simulator executes and synthesis ignores. `$readmemh` / `$readmemb` are first-class memory-init constructs.

### V-Ext-4: Strength specifiers

```verilog
buf (strong1, weak0) b1 (out, in);
assign (pull1, pull0) y = a;
```

**Tokens (new)**:
```
KEYWORD = "supply0", "supply1", "strong0", "strong1",
          "pull0", "pull1", "weak0", "weak1",
          "highz0", "highz1"
```
(Most are already in F05's keyword list as net types; we add the strength variants.)

**Grammar additions:**

```ebnf
strength_specifier = LPAREN strength0 COMMA strength1 RPAREN
                   | LPAREN strength1 COMMA strength0 RPAREN
                   | LPAREN strength0 RPAREN
                   | LPAREN strength1 RPAREN ;

strength0 = "supply0" | "strong0" | "pull0" | "weak0" | "highz0" ;
strength1 = "supply1" | "strong1" | "pull1" | "weak1" | "highz1" ;

continuous_assign = "assign" [strength_specifier] [delay_or_event_control]
                    assignment { COMMA assignment } SEMICOLON ;
```

**AST**: optional `strength: StrengthSpec | None` field on `Assign` and primitive instantiations.

**HIR mapping**: 9-state value propagation rules (`std_logic`-style resolution) consult strengths. Synthesis ignores strengths but issues a warning.

### V-Ext-5: User-defined primitives (UDP)

```verilog
primitive my_mux(out, sel, a, b);
  output out;
  input sel, a, b;
  table
    0 ? 0 : 0;
    0 ? 1 : 1;
    1 0 ? : 0;
    1 1 ? : 1;
  endtable
endprimitive
```

**Grammar additions:**

```ebnf
description = ... existing ...
            | udp_declaration ;

udp_declaration = "primitive" NAME LPAREN udp_port_list RPAREN SEMICOLON
                  { udp_port_declaration }
                  [ udp_initial_statement ]
                  "table" { udp_table_entry } "endtable"
                  "endprimitive" ;

udp_port_list = NAME { COMMA NAME } ;
udp_port_declaration = ("output" | "input" | "reg") NAME { COMMA NAME } SEMICOLON ;
udp_initial_statement = "initial" NAME EQUALS NUMBER SEMICOLON ;
udp_table_entry = udp_state_input { udp_state_input } [ COLON udp_level ] COLON udp_level SEMICOLON ;
udp_state_input = "0" | "1" | "x" | "X" | "?" | "*" | "(" udp_level udp_level ")" | ... ;
```

**AST**: `UDPDecl(name, ports, body, table)`.

**HIR mapping**: a special primitive cell type.

### V-Ext-6: Specify blocks

```verilog
specify
  (clk *> q) = (3, 4);  // rise/fall delay
  $setup(d, posedge clk, 1);
  $hold(posedge clk, d, 0.5);
endspecify
```

**Grammar additions:**

```ebnf
module_item = ... existing ... | specify_block ;

specify_block = "specify" { specify_item } "endspecify" ;

specify_item = path_declaration SEMICOLON
             | system_timing_check SEMICOLON
             | specparam_declaration SEMICOLON ;

path_declaration = simple_path_declaration ;
simple_path_declaration = LPAREN list_of_path_inputs path_op list_of_path_outputs RPAREN
                          EQUALS path_delay_value ;
path_op = STAR GREATER_THAN | EQUALS GREATER_THAN ;

system_timing_check = SYSTEM_ID LPAREN ... RPAREN ;
```

**AST**: `SpecifyBlock(items: list[PathDecl | TimingCheck | Specparam])`.

**HIR mapping**: stored as timing annotations on the module; consumed by `hardware-vm.md` (delay aware mode) and back-annotated SDF flows. Synthesis ignores entirely.

### V-Ext-7: Configurations

```verilog
config cfg1;
  design lib1.top;
  default liblist lib1 lib2;
  instance top.u1 use lib2.cell;
endconfig
```

**Tokens (new)**: `KEYWORD` additions: `config`, `endconfig`, `design`, `default`, `liblist`, `instance`, `use`, `cell`.

**Grammar additions**: a separate `configuration_declaration` description.

**AST**: `ConfigDecl(name, design_stmt, default_clause, instance_clauses)`.

**HIR mapping**: stored at elaboration time; affects which library cell variants are instantiated.

### V-Ext-8: Tasks and functions (full)

F05 partially specs these. Extensions:

- Default argument values.
- Functions with no arguments.
- Recursive tasks.
- `automatic` keyword (re-entrant).

**Grammar additions:**

```ebnf
function_declaration = "function" [ "automatic" ] [ range ] NAME
                       [ LPAREN function_port_list RPAREN ] SEMICOLON
                       { function_item }
                       statement
                       "endfunction" ;

task_declaration = "task" [ "automatic" ] NAME
                   [ LPAREN task_port_list RPAREN ] SEMICOLON
                   { task_item }
                   statement
                   "endtask" ;
```

### V-Ext-9: Disable statements and named blocks

```verilog
begin : my_block
  ...
  if (err) disable my_block;
  ...
end
```

**Already partially in F05**. Promote `disable` to a first-class statement:

```ebnf
sequential_statement = ... existing ...
                     | disable_statement ;
disable_statement = "disable" hierarchical_identifier SEMICOLON ;
```

### V-Ext-10: Force / release

```verilog
initial begin
  force x = 1'b0;
  #10;
  release x;
end
```

**Grammar additions:**

```ebnf
sequential_statement = ... existing ...
                     | force_statement
                     | release_statement ;
force_statement   = "force" lvalue EQUALS expression SEMICOLON ;
release_statement = "release" lvalue SEMICOLON ;
```

### V-Ext-11: Repeat, forever, while loops

F05 already covers `for`. Add `repeat`, `forever`, `while`:

```ebnf
loop_statement = ... existing ...
               | "while"   LPAREN expression RPAREN statement
               | "repeat"  LPAREN expression RPAREN statement
               | "forever" statement ;
```

### V-Ext-12: Wait statements

```verilog
wait (a == 1) begin ... end;
```

**Grammar additions:**

```ebnf
sequential_statement = ... existing ...
                     | wait_statement ;
wait_statement = "wait" LPAREN expression RPAREN statement_or_null ;
```

### V-Ext-13: Attributes

```verilog
(* synthesis, parallel_case *)
case (op)
  ...
endcase
```

**Lexer addition**: a new skip pattern that consumes `(* ... *)` and emits an `ATTRIBUTE` token carrying the contents (or attaches to the next token as metadata).

**Grammar**: attributes can attach to almost anything (modules, instances, statements, expressions, ports). Easiest implementation: parse them as an optional prefix on every grammar rule that accepts them, store as `attributes: dict[str, Expr] | None` on the affected AST node.

### V-Ext-14: Verilog 2001-2005 features beyond synth subset

- `localparam` (already in F05)
- `signed` arithmetic (already in F05)
- Generate-conditional `if/case` inside `generate` (already in F05)
- Multi-dimensional arrays of ports/regs/wires
- Implicit nets configuration (`` `default_nettype none ``)
- Unsized port and parameter declarations

## VHDL Extensions (IEEE 1076-2008)

### VHDL-Ext-1: Wait statements

```vhdl
wait;                          -- wait forever
wait until rising_edge(clk);   -- wait for condition
wait for 10 ns;                -- wait for time
wait on a, b;                  -- wait for change
wait on a until b = '1' for 100 ns;  -- combined
```

**Tokens (new)**: `KEYWORD` addition `wait`, `for` (already), `until`, `on`.

**Grammar additions:**

```ebnf
sequential_statement = ... existing ...
                     | wait_statement ;

wait_statement = [ NAME COLON ] "wait" [ sensitivity_clause ]
                 [ condition_clause ] [ timeout_clause ] SEMICOLON ;

sensitivity_clause = "on" name_list ;
condition_clause   = "until" expression ;
timeout_clause     = "for" expression ;
```

**AST**: `WaitStmt(on: list[Name], until: Expr | None, for_: Expr | None)`.

**HIR mapping**: a primary `wait` continuation node. Critical for VHDL processes — they may have either a sensitivity list *or* explicit `wait` statements (not both).

### VHDL-Ext-2: Delays in signal assignment

```vhdl
y <= '0' after 5 ns, '1' after 10 ns;
```

F05 covers `waveform_element = expression`. Extend:

```ebnf
waveform_element = expression [ "after" expression ] ;
```

### VHDL-Ext-3: File I/O and textio

```vhdl
use std.textio.all;
process
  variable l : line;
  variable v : integer;
  file f : text open read_mode is "stim.txt";
begin
  while not endfile(f) loop
    readline(f, l);
    read(l, v);
    ...
  end loop;
  wait;
end process;
```

**Tokens (new)**: `file`, `is`, `open`, `read_mode`, `write_mode`, `append_mode`, `text`, `line`, `endfile`, `readline`, `writeline`, `read`, `write`. (Some may already be keywords; verify.)

**Grammar additions:**

```ebnf
block_declarative_item = ... existing ...
                       | file_declaration ;

file_declaration = "file" name_list COLON subtype_indication
                   [ file_open_information ] SEMICOLON ;

file_open_information = [ "open" expression ] "is" expression ;
```

**AST**: `FileDecl(names, type, open_kind, name_expr)`.

**HIR mapping**: file objects are first-class in HIR. The simulation VM implements actual file I/O; synthesis warns and skips.

### VHDL-Ext-4: Assert / report / severity

```vhdl
assert reset_n = '0' for 10 ns
  report "reset must be asserted at startup"
  severity error;

report "Beginning test 1" severity note;
```

**Grammar additions** (mostly already in F05 but extend):

```ebnf
assertion_statement = [ NAME COLON ] "assert" condition
                      [ "report" expression ]
                      [ "severity" expression ] SEMICOLON ;

report_statement    = [ NAME COLON ] "report" expression
                      [ "severity" expression ] SEMICOLON ;
```

**AST**: `AssertStmt`, `ReportStmt`.

**HIR mapping**: first-class. Simulation evaluates condition, prints message, optionally halts on `failure` severity.

### VHDL-Ext-5: Configuration declarations

```vhdl
configuration cfg of top is
  for arch
    for u1 : adder use entity work.adder4(behavior); end for;
  end for;
end configuration cfg;
```

**Grammar additions:**

```ebnf
library_unit = ... existing ...
             | configuration_declaration ;

configuration_declaration = "configuration" NAME "of" NAME "is"
                            { configuration_declarative_item }
                            block_configuration
                            "end" [ "configuration" ] [ NAME ] SEMICOLON ;

block_configuration = "for" NAME { configuration_item } "end" "for" SEMICOLON ;

configuration_item = component_configuration | block_configuration ;

component_configuration = "for" component_specification
                          [ binding_indication SEMICOLON ]
                          [ block_configuration ]
                          "end" "for" SEMICOLON ;
```

**AST**: `ConfigDecl(name, of, items)`.

**HIR mapping**: resolved at elaboration; affects component → entity binding.

### VHDL-Ext-6: Attributes

```vhdl
attribute synthesis_off : boolean;
attribute synthesis_off of u_debug : label is true;

signal x'event       -- attribute on a signal: rising/falling
clock'last_value     -- previous value
data'length          -- length of an array
```

F05 partially handles attribute *access* via `TICK NAME`. Extend to declaration and full attribute sets:

```ebnf
block_declarative_item = ... existing ...
                       | attribute_declaration
                       | attribute_specification ;

attribute_declaration   = "attribute" NAME COLON subtype_indication SEMICOLON ;
attribute_specification = "attribute" NAME "of" entity_specification
                          "is" expression SEMICOLON ;

entity_specification = entity_name_list COLON entity_class ;
entity_class = "entity" | "architecture" | "package" | "configuration"
             | "procedure" | "function" | "signal" | "variable" | "constant"
             | "type" | "subtype" | "label" | "literal" | "file"
             | "component" | "all" ;
```

**Predefined attributes (must be recognized in expressions)**:
- For signals: `'event`, `'active`, `'last_event`, `'last_active`, `'last_value`, `'transaction`, `'delayed`, `'stable`, `'quiet`
- For arrays: `'left`, `'right`, `'low`, `'high`, `'range`, `'reverse_range`, `'length`, `'ascending`
- For scalars: `'left`, `'right`, `'low`, `'high`, `'image`, `'value`, `'pos`, `'val`, `'succ`, `'pred`

These need not be tokens; they're just `NAME`s after a `TICK`. The elaborator recognizes them.

### VHDL-Ext-7: Aliases

```vhdl
alias slv8 is std_logic_vector(7 downto 0);
alias top4 : std_logic_vector(3 downto 0) is data(7 downto 4);
```

**Grammar additions:**

```ebnf
block_declarative_item = ... existing ...
                       | alias_declaration ;

alias_declaration = "alias" alias_designator [ COLON subtype_indication ]
                    "is" name [ signature ] SEMICOLON ;

alias_designator = NAME | CHAR_LITERAL | OPERATOR_SYMBOL ;
```

### VHDL-Ext-8: Generate inside generate (recursive)

F05 already covers `for`/`if` generate. VHDL-2008 adds `case`-generate:

```vhdl
g1 : case width generate
  when 8 => u : adder8 port map(...);
  when 16 => u : adder16 port map(...);
  when others => ...;
end generate g1;
```

**Grammar addition:**

```ebnf
generate_statement = ... existing ...
                   | case_generate ;

case_generate = NAME COLON "case" expression "generate"
                { case_generate_alternative }
                "end" "generate" [ NAME ] SEMICOLON ;

case_generate_alternative = "when" choices ARROW { concurrent_statement } ;
```

### VHDL-Ext-9: Protected types (object-oriented VHDL)

```vhdl
type counter is protected
  procedure increment;
  impure function value return integer;
end protected counter;
```

**Grammar additions** (substantial; consult IEEE 1076-2008 §5.6).

**AST**: `ProtectedType`, `ProtectedTypeBody`.

**HIR mapping**: protected types are essentially objects with mutex; first-class but flagged as advanced.

### VHDL-Ext-10: Block / guarded statements

```vhdl
b1 : block (enable = '1')
begin
  q <= guarded d when rising_edge(clk);
end block b1;
```

Already partially in F05; promote to first-class.

### VHDL-Ext-11: Subprograms with named association in calls

```vhdl
result := my_func(a => x, b => y);
```

Grammar already covered by F05's named association rule; extend `function_call` similarly.

### VHDL-Ext-12: PSL embedded properties (IEEE 1850)

```vhdl
-- psl assert always (req -> next ack);
```

**Lexer addition**: pragma comments starting with `-- psl`. The PSL grammar is large; we **document and parse-skip** PSL content for v1, marking it as a separate future spec.

## Cross-language polish

### Standard packages

VHDL relies on standard library packages: `std.standard`, `std.textio`, `ieee.std_logic_1164`, `ieee.numeric_std`, `ieee.std_logic_arith`, etc. The parsers can already lex/parse `library` and `use` clauses. The **elaborator** (in `hdl-elaboration.md`) must provide the actual definitions of these packages.

For F05 extensions: define the *grammar* for these packages (they are themselves VHDL files) and ensure they parse. Implementation of the elaboration semantics is downstream.

Standard packages required (parsed but elaborated downstream):
- `std.standard` — built-in types (`bit`, `boolean`, `integer`, `time`, `string`)
- `std.textio` — file I/O
- `ieee.std_logic_1164` — `std_logic`, `std_ulogic`, resolution functions
- `ieee.numeric_std` — `signed`, `unsigned` types and arithmetic
- `ieee.numeric_bit` — analogous for `bit`
- `ieee.math_real` — math constants and functions

### Verilog libraries

The corresponding Verilog files are simpler — Verilog doesn't have standard packages, but it has standard `system tasks` ($display, $monitor, etc.) which the F05 lexer already recognizes. The elaborator just needs a built-in registry.

## New Tokens Summary

### Verilog new keyword tokens
```
automatic, config, endconfig, design, liblist, instance, use, cell,
specparam, primitive, endprimitive, table, endtable, force, release,
repeat, forever, while
```

### VHDL new keyword tokens
```
wait, until, on, for, file, is, open, read_mode, write_mode, append_mode,
text, line, alias
```
(many already present; this is the additive list).

## AST Schema Additions

In addition to the per-extension AST nodes mentioned above, the AST root node must support:

- `attributes: list[Attribute]` on every declaration node.
- `source_location: SourceLocation` on every node (already in F05; confirm).
- `comments: list[Comment]` optional, attached to declarations (for round-trip pretty-printing).

## Edge Cases

| Scenario | Handling |
|---|---|
| Verilog `(* ... *)` attribute on every conceivable target | Implement as an optional prefix on grammar rules; store on AST. |
| VHDL attribute names overloaded with `range` keyword (`'range`) | The lexer emits `TICK` then the keyword `RANGE`; parser handles. |
| Mixed-case keywords in VHDL (case-insensitive) | F05's `post_tokenize` lowercases — extend new keywords there. |
| Nested `specify` and `generate` blocks | Both grammars handle nesting via recursive rules. |
| `wait` with no arguments (`wait;`) | Parses as `WaitStmt(on=[], until=None, for_=None)`. Means "wait forever." |
| `assert` without `report` clause | Default message is implementation-defined; we use `"Assertion failed"`. |
| `force` outside of an initial/always block | Syntax error. |
| Empty `table` in UDP | Syntax error. |
| File `is` clause referring to a `string` expression evaluated at elaboration | Defer evaluation to elaborator. |
| PSL pragmas in non-comment positions | Reject; PSL only appears as `-- psl ...` comments in v1. |
| Aliases creating cycles | Caught at elaboration (alias resolution). |

## Test Strategy

### Lexer (target 95%+)
- Every new keyword tokenizes as `KEYWORD`.
- VHDL `'range`, `'reverse_range`, `'length` all tokenize correctly with `TICK` followed by `KEYWORD` or `NAME`.
- Verilog attribute `(* synthesis *)` tokenizes via the new skip-or-attribute rule.
- VHDL multi-line `assert ... report ... severity` lexes cleanly.

### Parser (target 80%+)
- Every new grammar rule has at least one positive and one negative test.
- The full IEEE 1364-2005 reference suite, where available, parses without error.
- The full IEEE 1076-2008 reference suite, where available, parses without error.
- Existing F05 tests pass unmodified (regression).

### Integration
- A real-world testbench (300+ lines) for a 32-bit ALU parses cleanly: `initial`, `wait`, `$display`, `assert`, `$readmemh`.
- A real-world VHDL testbench using `textio` parses cleanly.
- Pretty-print round-trip (parse → emit → parse) produces identical AST.

## IEEE Conformance Matrix

After this spec is implemented:

| Standard | Coverage | Notes |
|---|---|---|
| **IEEE 1364-2005** | Full grammar | UDP tables and configurations parsed; specify-block path delays parsed but timing semantics handled in `hardware-vm.md`. |
| **IEEE 1364-2001** | Full | Subset of 1364-2005. |
| **IEEE 1364-1995** | Full | Subset. |
| **IEEE 1076-2008** | Full grammar | Protected types, case-generate, PSL pragmas (parse-skip). |
| **IEEE 1076-2002** | Full | Subset of 2008. |
| **IEEE 1076-1993** | Full | Subset. |
| **IEEE 1800 (SystemVerilog)** | Out of scope | Distinct future spec `systemverilog-extensions.md`. |
| **IEEE 1850 (PSL)** | Skipped | PSL pragmas parse-skipped; full grammar deferred. |
| **IEEE 1481 (SDF)** | N/A | SDF is a back-annotation format consumed by the simulator, not the parser; see `hardware-vm.md`. |

## Open Questions

1. **Should attribute syntax be a lexer skip-pattern (with attribute reconstruction at parse time) or a first-class lexer pattern emitting a token sequence?**
   - Skip-pattern is faster but loses position info for individual sub-expressions inside the attribute.
   - Recommendation: emit an `ATTRIBUTE` token whose value is the textual content; downstream parses it lazily.

2. **How aggressive is PSL parsing?**
   - Recommendation: parse-skip in v1 (record it as a comment with a marker); v2 spec `psl-grammar.md` covers full PSL.

3. **Standard package definitions: ship with the parser package or provide separately?**
   - Recommendation: ship as `code/grammars/vhdl_stdlib/` directory with the canonical IEEE-licensed-equivalent text; loaded by elaborator.

4. **Should specify-block timing data round-trip through SDF?**
   - Recommendation: yes. `hdl-ir.md` carries timing as annotations; `hardware-vm.md` reads SDF for back-annotation.

5. **Strength specifiers — full 9-state propagation or 4-state for v1?**
   - Recommendation: parse fully but simulate as 4-state in v1; full 9-state semantics in `hardware-vm.md`.

## Future Work

- Full PSL grammar (`psl-grammar.md`).
- SystemVerilog extensions (`systemverilog-extensions.md`).
- VHDL-2019 features (interfaces, generics on packages).
- Extended attribute syntax for back-annotation (constraints files).
- SDF parser (`sdf-parser.md`) for back-annotated timing.
