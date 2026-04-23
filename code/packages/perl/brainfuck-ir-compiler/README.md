# CodingAdventures::BrainfuckIrCompiler

Brainfuck AOT compiler frontend. Compiles a Brainfuck AST into the general-purpose IR defined by `CodingAdventures::CompilerIr`. Perl port of `code/packages/go/brainfuck-ir-compiler/`.

## Pipeline position

```
Brainfuck source
     ↓  CodingAdventures::Brainfuck::Parser
   AST
     ↓  CodingAdventures::BrainfuckIrCompiler  ← this package
   IR + SourceMapChain (Segments 1 & 2)
     ↓  codegen-riscv  (future)
RISC-V machine code
```

## Register allocation

| Register | Purpose |
|----------|---------|
| v0 | Tape base address |
| v1 | Tape pointer offset (current cell, 0-based) |
| v2 | Temporary (cell values) |
| v3 | Temporary (bounds checks) |
| v4 | Syscall argument |
| v5 | Max pointer (tape_size − 1, debug mode only) |
| v6 | Zero constant (debug mode only) |

## IR sequences per command

| Command | IR output |
|---------|-----------|
| `>` (RIGHT) | `ADD_IMM v1, v1, 1` |
| `<` (LEFT) | `ADD_IMM v1, v1, -1` |
| `+` (INC) | `LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, 1; AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1` |
| `-` (DEC) | `LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, -1; AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1` |
| `.` (OUTPUT) | `LOAD_BYTE v2, v0, v1; ADD_IMM v4, v2, 0; SYSCALL 1` |
| `,` (INPUT) | `SYSCALL 2; STORE_BYTE v4, v0, v1` |

## BuildConfig presets

| Preset | Bounds checks | Debug locs | Byte masking | Tape size |
|--------|---------------|------------|--------------|-----------|
| `debug_config` | ON | ON | ON | 30000 |
| `release_config` | OFF | OFF | ON | 30000 |

## Usage

```perl
use CodingAdventures::BrainfuckIrCompiler qw(compile);
use CodingAdventures::BrainfuckIrCompiler::BuildConfig;
use CodingAdventures::Brainfuck::Parser;
use CodingAdventures::CompilerIr qw(print_ir);

my $ast    = CodingAdventures::Brainfuck::Parser->parse('++[-].');
my $cfg    = CodingAdventures::BrainfuckIrCompiler::BuildConfig->release_config;
my $result = compile($ast, 'hello.bf', $cfg);

# $result->{program}    is an IrProgram
# $result->{source_map} is a SourceMapChain

print print_ir($result->{program});
```

## Dependencies

- `CodingAdventures::CompilerIr`
- `CodingAdventures::CompilerSourceMap`
- `CodingAdventures::Brainfuck` (for the AST type)

Test dependency: `Test2::V0`.
