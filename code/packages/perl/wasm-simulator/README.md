# CodingAdventures::WasmSimulator

WebAssembly interpreter / simulator for Perl.

Executes WebAssembly modules by interpreting bytecode on a software-emulated
stack machine. Accepts parsed modules from
[CodingAdventures::WasmModuleParser](../wasm-module-parser/) and runs them.

## Part of the coding-adventures stack

```
WasmLeb128          — LEB128 variable-length integer encoding
WasmTypes           — Value type constants
WasmOpcodes         — Opcode name/metadata table
WasmModuleParser    — Binary .wasm parser → structured Perl hashref
WasmSimulator       — Bytecode executor (this package)
```

## Installation

```bash
cpanm --notest .
```

## Usage

```perl
use CodingAdventures::WasmModuleParser qw(parse);
use CodingAdventures::WasmSimulator;

# Load and parse a .wasm file
open my $fh, '<:raw', 'add.wasm' or die $!;
my $wasm = do { local $/; <$fh> };
close $fh;

my $mod  = parse($wasm);
my $inst = CodingAdventures::WasmSimulator->new($mod);

# Call an exported function
my @results = $inst->call('add', 3, 4);
print $results[0];  # 7

# Read/write linear memory
$inst->memory_write(0, [0xFF, 0x00, 0x00, 0x00]);
my @bytes = $inst->memory_read(0, 4);

# Access global variables
my $val = $inst->get_global('my_global');
$inst->set_global('my_global', 42);
```

## Execution Model

WebAssembly is a **stack machine**. Every instruction pops operands and pushes
results on an implicit value stack. For example, `i32.add` pops two i32 values
and pushes their sum.

**Linear memory** is a flat byte array in 64 KiB pages. `memory.grow` can
expand it; out-of-bounds access causes a trap (die).

**Globals** persist across calls; they're initialized from constant
expressions at instantiation time.

**Control flow** uses a label stack for structured `block`/`loop`/`if`
regions. `br N` branches to the Nth enclosing label's target.

## Supported Instructions

| Category   | Instructions |
|------------|-------------|
| Numeric    | `i32.const`, `i64.const` |
| Arithmetic | `i32.add`, `i32.sub`, `i32.mul`, `i32.div_s`, `i32.div_u`, `i32.rem_s`, `i32.rem_u` |
| Bitwise    | `i32.and`, `i32.or`, `i32.xor`, `i32.shl`, `i32.shr_s`, `i32.shr_u` |
| Comparison | `i32.eq`, `i32.ne`, `i32.lt_s`, `i32.lt_u`, `i32.le_s`, `i32.le_u`, `i32.gt_s`, `i32.gt_u`, `i32.ge_s`, `i32.ge_u`, `i32.eqz` |
| Memory     | `i32.load`, `i32.store`, `memory.size`, `memory.grow` |
| Control    | `nop`, `unreachable`, `block`, `loop`, `if`, `else`, `end`, `br`, `br_if`, `return`, `call` |
| Variable   | `local.get`, `local.set`, `local.tee`, `global.get`, `global.set` |
| Stack      | `drop`, `select` |

## API Reference

### `CodingAdventures::WasmSimulator->new($module)`

Create a new instance from a parsed module hashref (from `WasmModuleParser::parse()`).

### `$inst->call($name, @args)`

Call an exported function by name. Returns a list of results.

### `$inst->call_by_index($idx, @args)`

Call a function by 0-based module index.

### `$inst->get_global($name)`

Get the current value of an exported global variable.

### `$inst->set_global($name, $value)`

Set the value of an exported mutable global. Dies if immutable.

### `$inst->memory_read($offset, $length)`

Read bytes from linear memory. Returns a list of byte values (0–255).

### `$inst->memory_write($offset, \@bytes)`

Write an arrayref of byte values into linear memory.

## License

MIT
