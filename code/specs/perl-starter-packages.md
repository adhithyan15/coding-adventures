# Perl Starter Packages — Pure Perl Ports of the Computing Stack Base

## 1. Overview

This spec defines the first wave of Perl packages: 11 pure Perl ports
covering the computing stack from logic gates (Layer 10) through the
virtual machine (Layer 5). These packages establish Perl conventions,
prove the infrastructure from Specs 1-3 (build tool, scaffold generator),
and serve as the foundation for all future Perl work.

### Why these 11 packages?

They span the full depth of the computing stack's core path:

```
Layer 10:  logic-gates      ← boolean logic, the foundation
Layer 9:   arithmetic        ← adders, ALU, built on gates
           bitset            ← compact boolean arrays
           directed-graph    ← graph algorithms (used by build tool)
           matrix            ← 2D numeric computation
           immutable-list    ← persistent data structure
           tree              ← hierarchical data
Layer 2:   lexer             ← tokenization
Layer 3:   parser            ← syntax analysis
Layer 4:   bytecode-compiler ← code generation
Layer 5:   virtual-machine   ← execution
```

Together, these demonstrate that Perl can express everything from bit
manipulation to a working bytecode interpreter.

### Design Principle: Pure Perl, No External Deps

Every package in this starter set uses only Perl core modules plus
dependencies on other monorepo packages. The only CPAN dependency is
`Test2::V0` for testing. This mirrors the Go and Rust packages, which have
zero external dependencies.

Later, Spec 5 (FFI data structures) will add FFI::Platypus wrappers for
Rust-backed high-performance variants of the data structure packages.

---

## 2. Where It Fits

### 2.1 Dependency Diagram

```
                  virtual-machine
                       |
                  bytecode-compiler
                       |
                     parser
                       |
                     lexer         (no deps below this line)

  arithmetic          tree         immutable-list
      |                |
  logic-gates    directed-graph    matrix    bitset
```

Packages at the same level have no dependencies on each other and can be
implemented in parallel.

### 2.2 Package Locations

All packages live in `code/packages/perl/<kebab-name>/`.

---

## 3. Common Patterns

Every Perl package in the starter set follows these conventions:

### 3.1 File Structure

```
code/packages/perl/<kebab-name>/
  Makefile.PL
  cpanfile
  BUILD
  README.md
  CHANGELOG.md
  lib/
    CodingAdventures/
      <CamelCase>.pm            # Main module
      <CamelCase>/
        <SubModule>.pm          # Sub-modules (if needed)
  t/
    00-load.t
    01-<name>.t
    02-<name>.t
    ...
```

### 3.2 Module Boilerplate

Every `.pm` file begins with:

```perl
package CodingAdventures::<CamelCase>;

use strict;
use warnings;

our $VERSION = '0.01';
```

And ends with:

```perl
1;
```

### 3.3 Constructor Pattern

```perl
sub new {
    my ($class, %args) = @_;
    return bless {
        field1 => $args{field1} // default_value,
        field2 => $args{field2},
    }, $class;
}
```

### 3.4 Accessor Pattern

```perl
# Read-only accessor
sub name { return $_[0]->{name} }

# Read-write accessor
sub set_name {
    my ($self, $value) = @_;
    $self->{name} = $value;
    return $self;  # for chaining
}
```

### 3.5 Error Handling

Perl uses `die` for errors. Callers catch with `eval { }` or `try` (if
using Try::Tiny):

```perl
sub get {
    my ($self, $index) = @_;
    die "Index $index out of bounds (size: $self->{size})\n"
        if $index < 0 || $index >= $self->{size};
    return $self->{data}[$index];
}
```

For structured errors, die with a hash reference:

```perl
die { type => 'CycleError', message => "Graph contains a cycle", nodes => \@cycle };
```

### 3.6 Constants (No Enums in Perl)

Perl has no `enum` keyword. Use `use constant`:

```perl
use constant {
    ADD  => 0,
    SUB  => 1,
    MUL  => 2,
    DIV  => 3,
};
```

Or for exportable constants, use a hash:

```perl
my %OPCODES = (
    LOAD_CONST => 0,
    ADD        => 1,
    JUMP       => 2,
);
```

### 3.7 Operator Overloading

For bitset and matrix, use `use overload`:

```perl
use overload
    '&'  => \&bitwise_and,
    '|'  => \&bitwise_or,
    '^'  => \&bitwise_xor,
    '~'  => \&bitwise_not,
    '""' => \&to_string;
```

### 3.8 Porting Reference

Port from the **Python** implementation as the primary reference. Python's
imperative style maps most naturally to Perl. Use the existing specs (under
`code/specs/`) for authoritative API definitions.

---

## 4. Per-Package Specifications

### 4.1 logic-gates

**Source reference:** `code/packages/python/logic-gates/`
**Perl module:** `CodingAdventures::LogicGates`

**Module structure:**
```
lib/CodingAdventures/LogicGates.pm          # All gate functions
```

**Public API (exported functions):**

| Function | Signature | Returns | Description |
|----------|-----------|---------|-------------|
| `NOT` | `($a)` | `0 or 1` | Boolean NOT |
| `AND` | `($a, $b)` | `0 or 1` | Boolean AND |
| `OR` | `($a, $b)` | `0 or 1` | Boolean OR |
| `XOR` | `($a, $b)` | `0 or 1` | Boolean XOR |
| `NAND` | `($a, $b)` | `0 or 1` | Boolean NAND |
| `NOR` | `($a, $b)` | `0 or 1` | Boolean NOR |
| `XNOR` | `($a, $b)` | `0 or 1` | Boolean XNOR |
| `AND_N` | `(@bits)` | `0 or 1` | Multi-input AND |
| `OR_N` | `(@bits)` | `0 or 1` | Multi-input OR |
| `mux2` | `($sel, $a, $b)` | `0 or 1` | 2-input multiplexer |
| `demux` | `($sel, $input)` | `($out0, $out1)` | 1-to-2 demultiplexer |
| `decoder` | `(@sel_bits)` | `@outputs` | n-to-2^n decoder |
| `encoder` | `(@inputs)` | `@encoded` | 2^n-to-n encoder |
| `sr_latch` | `($s, $r, $q_prev)` | `($q, $q_bar)` | SR latch |
| `d_latch` | `($d, $enable, $q_prev)` | `($q, $q_bar)` | D latch |
| `d_flip_flop` | `($d, $clk, $clk_prev, $q_prev)` | `($q, $q_bar)` | D flip-flop |
| `register` | `(\@data, $load, $clk, $clk_prev, \@q_prev)` | `@q` | N-bit register |

**Perl-specific notes:**
- All gate functions operate on `0` and `1` integer values.
- Multi-output functions return lists (Perl's natural multiple return).
- Export functions via `use Exporter`: `our @EXPORT_OK = qw(NOT AND OR ...)`.
- NAND universality: implement `nand_not`, `nand_and`, `nand_or`, `nand_xor`
  using only `NAND` calls — demonstrate that NAND is a universal gate.

**Test count: 50+**
- Truth tables for each gate (4 cases per 2-input gate, 2 per NOT)
- Multi-input AND_N, OR_N with 3-8 inputs
- Mux/demux with all selector values
- Sequential logic: latch state transitions, clock edge detection

---

### 4.2 arithmetic

**Source reference:** `code/packages/python/arithmetic/`
**Perl module:** `CodingAdventures::Arithmetic`
**Depends on:** `logic-gates`

**Module structure:**
```
lib/CodingAdventures/Arithmetic.pm          # Adders, subtractor, ALU
```

**Public API:**

| Function/Class | Signature | Returns | Description |
|----------------|-----------|---------|-------------|
| `half_adder` | `($a, $b)` | `($sum, $carry)` | Half adder |
| `full_adder` | `($a, $b, $cin)` | `($sum, $cout)` | Full adder |
| `ripple_carry_adder` | `(\@a, \@b, $cin)` | `(\@sum, $cout)` | N-bit adder |
| `ALU->new` | `(width => $n)` | ALU object | Create ALU |
| `ALU->execute` | `($op, \@a, \@b)` | `ALUResult` | Execute operation |

**ALU operations (constants):**

```perl
use constant {
    ALU_ADD => 0,
    ALU_SUB => 1,
    ALU_AND => 2,
    ALU_OR  => 3,
    ALU_XOR => 4,
    ALU_NOT => 5,
    ALU_SHL => 6,
    ALU_SHR => 7,
};
```

**Test count: 50+**
- Half adder: all 4 input combinations
- Full adder: all 8 input combinations
- Ripple carry: 4-bit, 8-bit, overflow detection
- ALU: each operation, zero flag, carry flag, overflow flag

---

### 4.3 bitset

**Source reference:** `code/packages/python/bitset/`
**Perl module:** `CodingAdventures::Bitset`

**Module structure:**
```
lib/CodingAdventures/Bitset.pm
```

**Public API:**

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `new` | `($class, $size)` | Bitset | Create with capacity |
| `set` | `($self, $index)` | `$self` | Set bit to 1 |
| `clear` | `($self, $index)` | `$self` | Set bit to 0 |
| `get` | `($self, $index)` | `0 or 1` | Get bit value |
| `popcount` | `($self)` | int | Count set bits |
| `any` | `($self)` | bool | True if any bit set |
| `none` | `($self)` | bool | True if no bits set |
| `size` | `($self)` | int | Capacity |
| `iter_set_bits` | `($self)` | `@indices` | List of set bit positions |
| `bitwise_and` | `($self, $other)` | Bitset | AND of two bitsets |
| `bitwise_or` | `($self, $other)` | Bitset | OR of two bitsets |
| `bitwise_xor` | `($self, $other)` | Bitset | XOR of two bitsets |
| `bitwise_not` | `($self)` | Bitset | NOT (complement) |
| `from_binary_str` | `($class, $str)` | Bitset | Parse `"1010"` |
| `to_string` | `($self)` | string | Binary representation |

**Operator overloading:**
- `&` → `bitwise_and`
- `|` → `bitwise_or`
- `^` → `bitwise_xor`
- `~` → `bitwise_not`
- `""` → `to_string`

**Implementation notes:**
- Store bits as an array of integers (Perl scalars), packing 32 bits per
  word (Perl's safe integer bitwise range on 32-bit systems).
- Alternatively, use Perl's `vec()` function for bit-level storage — this
  is Perl's built-in bit vector facility.
- `popcount` uses the Kernighan bit-counting trick: `$n &= ($n - 1)`.

**Test count: 40+**

---

### 4.4 directed-graph

**Source reference:** `code/packages/python/directed-graph/`
**Perl module:** `CodingAdventures::DirectedGraph`

**Module structure:**
```
lib/CodingAdventures/DirectedGraph.pm
```

**Public API:**

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `new` | `($class)` | Graph | Empty graph |
| `add_node` | `($self, $name)` | `$self` | Add a node |
| `add_edge` | `($self, $from, $to)` | `$self` | Add directed edge |
| `has_node` | `($self, $name)` | bool | Check node exists |
| `has_edge` | `($self, $from, $to)` | bool | Check edge exists |
| `nodes` | `($self)` | `@names` | All node names |
| `edges` | `($self)` | `@pairs` | All edges as `[$from, $to]` |
| `neighbors` | `($self, $name)` | `@names` | Outgoing neighbors |
| `topological_sort` | `($self)` | `@names` | Sorted order (dies on cycle) |
| `independent_groups` | `($self)` | `@groups` | Groups by dependency level |
| `affected_nodes` | `($self, \@changed)` | `@names` | Transitive dependents |
| `to_dot` | `($self)` | string | Graphviz DOT format |
| `to_ascii` | `($self)` | string | ASCII table representation |

**Error cases:**
- `die` with `CycleError` message if `topological_sort` detects a cycle.
- `die` with `NodeNotFoundError` for operations on nonexistent nodes.

**This package is critical** — the Perl build tool (Spec 2) depends on it
for dependency resolution, parallel execution grouping, and change
propagation.

**Test count: 35+**

---

### 4.5 tree

**Source reference:** `code/packages/python/tree/`
**Perl module:** `CodingAdventures::Tree`
**Depends on:** `directed-graph`

**Module structure:**
```
lib/CodingAdventures/Tree.pm
```

**Public API:**

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `new` | `($class, $root_label)` | Tree | Create with root |
| `add_child` | `($self, $parent, $child)` | `$self` | Add child node |
| `parent` | `($self, $node)` | string or undef | Parent of node |
| `children` | `($self, $node)` | `@names` | Children of node |
| `siblings` | `($self, $node)` | `@names` | Siblings of node |
| `depth` | `($self, $node)` | int | Depth from root |
| `height` | `($self)` | int | Height of tree |
| `size` | `($self)` | int | Number of nodes |
| `leaves` | `($self)` | `@names` | Leaf nodes |
| `preorder` | `($self)` | `@names` | Pre-order traversal |
| `postorder` | `($self)` | `@names` | Post-order traversal |
| `level_order` | `($self)` | `@names` | Breadth-first traversal |
| `path_to` | `($self, $node)` | `@names` | Path from root to node |
| `lca` | `($self, $a, $b)` | string | Lowest common ancestor |
| `to_ascii` | `($self)` | string | ASCII tree rendering |

**Test count: 35+**

---

### 4.6 matrix

**Source reference:** `code/packages/python/matrix/`
**Perl module:** `CodingAdventures::Matrix`

**Module structure:**
```
lib/CodingAdventures/Matrix.pm
```

**Public API:**

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `new` | `($class, \@data)` | Matrix | From 2D array |
| `zeros` | `($class, $rows, $cols)` | Matrix | Zero matrix |
| `identity` | `($class, $n)` | Matrix | Identity matrix |
| `rows` | `($self)` | int | Number of rows |
| `cols` | `($self)` | int | Number of columns |
| `get` | `($self, $r, $c)` | number | Element at (r,c) |
| `set` | `($self, $r, $c, $val)` | `$self` | Set element |
| `add` | `($self, $other)` | Matrix | Element-wise addition |
| `subtract` | `($self, $other)` | Matrix | Element-wise subtraction |
| `scale` | `($self, $scalar)` | Matrix | Scalar multiplication |
| `transpose` | `($self)` | Matrix | Transposed matrix |
| `dot` | `($self, $other)` | Matrix | Matrix multiplication |
| `eq` | `($self, $other)` | bool | Element-wise equality |
| `to_string` | `($self)` | string | Formatted output |

**Operator overloading:**
- `+` → `add`
- `-` → `subtract`
- `*` → `scale` (scalar) or `dot` (matrix)
- `==` → `eq`
- `""` → `to_string`

**Perl-specific note:** All Perl numbers are floating point (double
precision). This is actually a natural fit for matrix operations — no need
for separate integer vs float matrix types.

**Test count: 35+**

---

### 4.7 immutable-list

**Source reference:** `code/packages/python/immutable-list/`
**Perl module:** `CodingAdventures::ImmutableList`

**Module structure:**
```
lib/CodingAdventures/ImmutableList.pm
```

**Public API:**

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `new` | `($class)` | ImmutableList | Empty list |
| `push` | `($self, $value)` | ImmutableList | New list with value appended |
| `pop` | `($self)` | ImmutableList | New list without last element |
| `get` | `($self, $index)` | value | Element at index |
| `set` | `($self, $index, $value)` | ImmutableList | New list with updated element |
| `len` | `($self)` | int | Number of elements |
| `to_array` | `($self)` | `@values` | All elements as Perl list |

**Key property:** `push`, `pop`, and `set` return **new** ImmutableList
objects. The original is never modified. This is the persistent data
structure pattern — structural sharing via a 32-way trie.

**Perl-specific note:** Perl arrays are mutable and resizable. The
ImmutableList is intentionally different — it demonstrates persistent data
structures in a language that doesn't have them natively.

**Test count: 30+**

---

### 4.8 lexer

**Source reference:** `code/packages/python/lexer/`
**Perl module:** `CodingAdventures::Lexer`

**Module structure:**
```
lib/CodingAdventures/Lexer.pm
lib/CodingAdventures/Lexer/Token.pm
lib/CodingAdventures/Lexer/TokenType.pm
```

**Public API:**

| Class/Function | Method | Returns | Description |
|----------------|--------|---------|-------------|
| `Lexer->new` | `(source => $text)` | Lexer | Create lexer |
| `Lexer->tokenize` | `()` | `@tokens` | Tokenize source |
| `Token->new` | `(type => $type, value => $val, line => $n, col => $n)` | Token | |
| `Token->type` | `()` | TokenType constant | |
| `Token->value` | `()` | string | |
| `Token->line` | `()` | int | |

**Token types (constants in TokenType.pm):**

```perl
use constant {
    NUMBER      => 'NUMBER',
    STRING      => 'STRING',
    IDENTIFIER  => 'IDENTIFIER',
    KEYWORD     => 'KEYWORD',
    OPERATOR    => 'OPERATOR',
    PUNCTUATION => 'PUNCTUATION',
    NEWLINE     => 'NEWLINE',
    EOF_TOKEN   => 'EOF',
    ERROR       => 'ERROR',
};
```

**Perl advantage:** Perl's regex engine makes tokenization very natural.
The lexer can use `pos()` and `\G` anchor for efficient sequential matching:

```perl
while (pos($source) < length($source)) {
    if ($source =~ /\G(\d+)/gc)     { push @tokens, Token->new(type => NUMBER, ...) }
    elsif ($source =~ /\G(".*?")/gc) { push @tokens, Token->new(type => STRING, ...) }
    ...
}
```

**Test count: 40+**

---

### 4.9 parser

**Source reference:** `code/packages/python/parser/`
**Perl module:** `CodingAdventures::Parser`
**Depends on:** `lexer`

**Module structure:**
```
lib/CodingAdventures/Parser.pm
lib/CodingAdventures/Parser/ASTNode.pm
```

**Public API:**

| Class/Function | Method | Returns | Description |
|----------------|--------|---------|-------------|
| `Parser->new` | `(tokens => \@tokens)` | Parser | Create parser |
| `Parser->parse` | `()` | ASTNode | Parse token stream into AST |
| `ASTNode->new` | `(rule => $name, children => \@nodes, token => $tok)` | ASTNode | |
| `ASTNode->rule` | `()` | string | Grammar rule name |
| `ASTNode->children` | `()` | `@nodes` | Child nodes |
| `ASTNode->token` | `()` | Token or undef | Leaf token |
| `ASTNode->to_string` | `($indent)` | string | Pretty-printed tree |

**Error handling:**
- `die` with `ParseError` message on syntax errors.
- Include line/column information from the current token.

**Test count: 40+**

---

### 4.10 bytecode-compiler

**Source reference:** `code/packages/python/bytecode-compiler/`
**Perl module:** `CodingAdventures::BytecodeCompiler`
**Depends on:** `parser`

**Module structure:**
```
lib/CodingAdventures/BytecodeCompiler.pm
lib/CodingAdventures/BytecodeCompiler/Instruction.pm
lib/CodingAdventures/BytecodeCompiler/CodeObject.pm
lib/CodingAdventures/BytecodeCompiler/OpCode.pm
```

**Public API:**

| Class | Method | Returns | Description |
|-------|--------|---------|-------------|
| `BytecodeCompiler->new` | `()` | Compiler | |
| `BytecodeCompiler->compile` | `($ast)` | CodeObject | AST to bytecode |
| `CodeObject->new` | `(...)` | CodeObject | Compiled code unit |
| `CodeObject->instructions` | `()` | `@instructions` | Bytecode instructions |
| `CodeObject->constants` | `()` | `@values` | Constant pool |
| `Instruction->new` | `(opcode => $op, operand => $val)` | Instruction | |

**OpCodes (constants):**

```perl
use constant {
    LOAD_CONST  => 0,
    LOAD_NAME   => 1,
    STORE_NAME  => 2,
    ADD         => 3,
    SUB         => 4,
    MUL         => 5,
    DIV         => 6,
    JUMP        => 7,
    JUMP_IF     => 8,
    CALL        => 9,
    RETURN      => 10,
    PRINT       => 11,
    POP         => 12,
    COMPARE     => 13,
};
```

**Test count: 35+**

---

### 4.11 virtual-machine

**Source reference:** `code/packages/python/virtual-machine/`
**Perl module:** `CodingAdventures::VirtualMachine`
**Depends on:** `bytecode-compiler`

**Module structure:**
```
lib/CodingAdventures/VirtualMachine.pm
lib/CodingAdventures/VirtualMachine/CallFrame.pm
```

**Public API:**

| Class | Method | Returns | Description |
|-------|--------|---------|-------------|
| `VirtualMachine->new` | `()` | VM | Create VM |
| `VirtualMachine->execute` | `($code_object)` | result | Run bytecode |
| `VirtualMachine->stack` | `()` | `@values` | Current stack contents |
| `VirtualMachine->globals` | `()` | `%vars` | Global variables |
| `VirtualMachine->trace` | `()` | `@snapshots` | Execution trace |

**Error handling:**
- `StackUnderflowError` — pop on empty stack
- `DivisionByZeroError` — divide by zero
- `UndefinedNameError` — reference to unbound variable
- `InvalidOpcodeError` — unknown instruction
- `MaxRecursionError` — call stack too deep

**End-to-end test:** Compile a simple program source string through the
lexer, parser, bytecode compiler, and VM — verify the output matches. This
proves the entire pipeline works.

**Test count: 35+**

---

## 5. Perl-Specific Porting Considerations

### 5.1 No Enum Type

Python has `enum.Enum`. Go has `iota`. Rust has `enum`. Perl has none.

**Solution:** `use constant` for simple cases, hash lookups for mappable
cases:

```perl
# Simple constants
use constant { ADD => 0, SUB => 1, MUL => 2 };

# Reverse lookup (name from value)
my %OPCODE_NAMES = reverse %{{ ADD => 0, SUB => 1, MUL => 2 }};
```

### 5.2 No Static Typing

Perl is dynamically typed. Where Python uses type hints and Go uses type
declarations, Perl validates at boundaries:

```perl
sub add {
    my ($self, $other) = @_;
    die "Dimension mismatch\n"
        unless $self->{rows} == $other->{rows}
            && $self->{cols} == $other->{cols};
    ...
}
```

### 5.3 Multiple Return Values

Perl naturally supports multiple return values via list context:

```perl
sub half_adder {
    my ($a, $b) = @_;
    return (XOR($a, $b), AND($a, $b));  # (sum, carry)
}

my ($sum, $carry) = half_adder(1, 1);
```

This is more natural than Python's tuples or Go's multi-return.

### 5.4 Closures for Iterators

Where Python uses generators (`yield`), Perl uses closures:

```perl
sub iter_set_bits {
    my ($self) = @_;
    my @indices;
    for my $i (0 .. $self->{size} - 1) {
        push @indices, $i if $self->get($i);
    }
    return @indices;
}
```

Or for lazy iteration, return a closure:

```perl
sub iter_set_bits {
    my ($self) = @_;
    my $i = 0;
    return sub {
        while ($i < $self->{size}) {
            return $i++ if $self->get($i++);
        }
        return undef;
    };
}
```

### 5.5 Perl's Number Type

All Perl numbers are double-precision floats internally. This means:
- Matrix operations work naturally with floating point.
- Integer operations work correctly up to 2^53.
- Bitwise operations (`&`, `|`, `^`, `~`) convert to integers first.
- The bitset must be careful with word sizes — use 32-bit words to stay
  within safe integer range for bitwise operations.

---

## 6. Implementation Sequence

The packages must be implemented in dependency order:

```
Phase 1 (parallel):  logic-gates, bitset, directed-graph, matrix, immutable-list, lexer
Phase 2 (parallel):  arithmetic (needs logic-gates), tree (needs directed-graph)
Phase 3 (serial):    parser (needs lexer)
Phase 4 (serial):    bytecode-compiler (needs parser)
Phase 5 (serial):    virtual-machine (needs bytecode-compiler)
```

Phase 1 packages have no internal dependencies and can all be implemented
concurrently. Phases 3-5 are a linear chain.

---

## 7. Test Strategy

### 7.1 Total Test Counts

| Package | Test Count | Key Test Categories |
|---------|-----------|---------------------|
| logic-gates | 50+ | Truth tables, multi-input, sequential logic |
| arithmetic | 50+ | Adder combinations, ALU operations, flags |
| bitset | 40+ | Set/get/clear, bitwise ops, popcount, iteration |
| directed-graph | 35+ | Topo sort, cycle detection, affected nodes |
| tree | 35+ | Traversals, LCA, structure queries |
| matrix | 35+ | Arithmetic, transpose, dot product, errors |
| immutable-list | 30+ | Push/pop, persistence, get/set |
| lexer | 40+ | Token types, position tracking, error tokens |
| parser | 40+ | AST construction, error recovery, grammar rules |
| bytecode-compiler | 35+ | Instruction encoding, constant pool, scoping |
| virtual-machine | 35+ | Instruction dispatch, stack ops, end-to-end |
| **Total** | **~425** | |

### 7.2 Test Framework

All tests use `Test2::V0`:

```perl
use Test2::V0;

is(AND(1, 1), 1, 'AND(1,1) = 1');
is(AND(1, 0), 0, 'AND(1,0) = 0');
like(dies { $graph->topological_sort }, qr/cycle/i, 'cycle detected');

done_testing;
```

Key `Test2::V0` functions used:
- `is($got, $expected, $name)` — equality
- `ok($bool, $name)` — boolean assertion
- `like($string, qr/pattern/, $name)` — regex match
- `dies { ... }` — capture exception
- `is_deeply(\@got, \@expected, $name)` — deep structure comparison

---

## 8. Trade-Offs

### 8.1 Port from Python vs Ruby

| | Python | Ruby |
|-|--------|------|
| Syntax similarity to Perl | High (imperative, C-like) | Medium (more OO-centric) |
| Data structure mapping | dict → hash, list → array | Hash → hash, Array → array |
| Error handling | try/except → eval{}/die | begin/rescue → eval{}/die |
| **Decision** | **Python** | — |

Python's imperative style and explicit data structures map most naturally to
Perl. Ruby's implicit `self` and method-heavy style would require more
adaptation.

### 8.2 Operator Overloading: Where to Use

| Package | Overload? | Why |
|---------|-----------|-----|
| bitset | Yes (`&`, `|`, `^`, `~`) | Bitwise operators are natural |
| matrix | Yes (`+`, `-`, `*`) | Arithmetic operators are natural |
| logic-gates | No | Functions, not objects |
| directed-graph | No | No natural operator mapping |
| tree | No | No natural operator mapping |

### 8.3 Error Handling: die vs Return Codes

| | die (exceptions) | Return codes |
|-|------------------|-------------|
| Perl convention | Idiomatic | C-style, not Perl-ish |
| Caller ergonomics | `eval { }` catches | Must check every call |
| Stack traces | Automatic with `Carp::confess` | Manual |
| **Decision** | **die** | — |

---

## 9. Future Extensions

- **Additional packages:** Port the remaining ~90 packages as the monorepo
  grows (cpu-simulator, assembler, compiler backends, etc.).
- **FFI acceleration:** Spec 5 adds FFI::Platypus wrappers for the data
  structure packages (bitset, directed-graph, matrix, tree, immutable-list).
- **Grammar-driven lexer/parser:** Port the grammar-driven variants
  (`GrammarLexer`, `GrammarParser`) that read `.grammar` files.
- **Backend compilers:** Port JVM, CLR, WASM compiler backends.
- **Benchmarks:** Compare pure Perl vs FFI performance for data structures.
