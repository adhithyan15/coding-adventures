# CodingAdventures::WasmModuleParser

WebAssembly binary module parser for Perl.

Parses `.wasm` binary files into structured Perl hashrefs following the
[WebAssembly binary format specification](https://webassembly.github.io/spec/core/binary/modules.html).

## Part of the coding-adventures stack

This library sits at the WebAssembly tooling layer. It depends on:
- `CodingAdventures::WasmLeb128` — LEB128 variable-length integer decoding
- `CodingAdventures::WasmTypes` — WebAssembly value type definitions

## Installation

```bash
cpanm --notest .
```

Or install dependencies first:

```bash
cpanm --notest CodingAdventures::WasmLeb128 CodingAdventures::WasmTypes
cpanm --notest .
```

## Usage

```perl
use CodingAdventures::WasmModuleParser qw(parse get_section);
use CodingAdventures::WasmModuleParser qw(SECTION_TYPE SECTION_EXPORT);

# Read a .wasm file (must use :raw binmode)
open my $fh, '<:raw', 'module.wasm' or die "Cannot open: $!";
my $bytes = do { local $/; <$fh> };
close $fh;

# Parse the module
my $module = parse($bytes);

# Inspect the structure
printf "Version: %d\n",     $module->{version};
printf "Types: %d\n",       scalar @{ $module->{types} };
printf "Exports: %d\n",     scalar @{ $module->{exports} };

# List all exports
for my $exp (@{ $module->{exports} }) {
    printf "  %s: %s %d\n",
        $exp->{name}, $exp->{desc}{kind}, $exp->{desc}{idx};
}

# List function type signatures
for my $i (0 .. $#{ $module->{types} }) {
    my $t = $module->{types}[$i];
    printf "Type %d: (%s) -> (%s)\n",
        $i,
        join(', ', @{ $t->{params} }),
        join(', ', @{ $t->{results} });
}

# Get a section by ID
my $types = get_section($module, SECTION_TYPE);
```

## Module Structure

The hashref returned by `parse()` has these fields:

| Field       | Type      | Description |
|-------------|-----------|-------------|
| `magic`     | string    | Always `"\x00asm"` |
| `version`   | integer   | Always `1` |
| `types`     | arrayref  | Function type signatures `{params, results}` |
| `imports`   | arrayref  | Imported symbols `{mod, name, desc}` |
| `functions` | arrayref  | Type indices for local functions |
| `tables`    | arrayref  | Table definitions `{ref_type, limits}` |
| `memories`  | arrayref  | Memory definitions `{limits}` |
| `globals`   | arrayref  | Global variable definitions |
| `exports`   | arrayref  | Exported symbols `{name, desc}` |
| `start`     | integer   | Start function index (or undef) |
| `elements`  | arrayref  | Element segment raw bytes |
| `codes`     | arrayref  | Function bodies `{locals, body}` |
| `data`      | arrayref  | Data segment raw bytes |
| `custom`    | arrayref  | Custom sections `{name, data}` |

## Section ID Constants

```perl
use CodingAdventures::WasmModuleParser qw(
    SECTION_CUSTOM   SECTION_TYPE    SECTION_IMPORT
    SECTION_FUNCTION SECTION_TABLE   SECTION_MEMORY
    SECTION_GLOBAL   SECTION_EXPORT  SECTION_START
    SECTION_ELEMENT  SECTION_CODE    SECTION_DATA
);
```

## Running Tests

```bash
prove -l -v t/
```

## License

MIT
