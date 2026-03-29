package CodingAdventures::WasmModuleParser;

# ============================================================================
# CodingAdventures::WasmModuleParser — WebAssembly binary module parser
# ============================================================================
#
# WebAssembly (Wasm) is a binary instruction format designed as a portable,
# sandboxed compilation target. This module parses the BINARY FORMAT of a
# .wasm file into a structured Perl data structure.
#
# The binary format specification is at:
#   https://webassembly.github.io/spec/core/binary/modules.html
#
# ## The Wasm Binary Format — Overview
#
# Every valid .wasm file starts with an 8-byte header:
#
#   Offset  Bytes             Description
#   ──────  ─────────────     ─────────────────────────────────────
#        0  00 61 73 6D       Magic: "\x00asm"
#        4  01 00 00 00       Version: 1 (little-endian uint32)
#
# After the header, the file contains zero or more SECTIONS. Each section has:
#
#   ┌─────────────┬──────────────────────┬────────────────────────────────────┐
#   │  Section ID │  Content Length      │  Content bytes                     │
#   │  1 byte     │  unsigned LEB128     │  (length bytes)                    │
#   └─────────────┴──────────────────────┴────────────────────────────────────┘
#
# Section IDs:
#   0  Custom     — name + arbitrary extension data
#   1  Type       — function type signatures
#   2  Import     — external symbol imports
#   3  Function   — type indices for local functions
#   4  Table      — reference table definitions
#   5  Memory     — linear memory definitions
#   6  Global     — global variable definitions
#   7  Export     — exported symbols
#   8  Start      — start function index
#   9  Element    — table initialization data
#  10  Code       — function bodies (bytecode)
#  11  Data       — memory initialization data
#
# ## LEB128 integers
#
# Most integer values in the Wasm binary format are encoded as LEB128
# (Little-Endian Base 128) — a variable-length scheme where small values
# use 1 byte and larger values use more bytes.
#
# This module delegates LEB128 decoding to CodingAdventures::WasmLeb128.
#
# ## Usage
#
#   use CodingAdventures::WasmModuleParser qw(parse);
#
#   # Read a .wasm file
#   open my $fh, '<:raw', 'module.wasm' or die $!;
#   my $bytes = do { local $/; <$fh> };
#   close $fh;
#
#   my $module = parse($bytes);
#   print "Version: $module->{version}\n";           # 1
#   print "Types: ", scalar @{$module->{types}}, "\n";
#   print "Exports: ", scalar @{$module->{exports}}, "\n";
#
#   for my $exp (@{ $module->{exports} }) {
#       printf "Export: %s (%s %d)\n",
#           $exp->{name}, $exp->{desc}{kind}, $exp->{desc}{idx};
#   }
#
# ============================================================================

use strict;
use warnings;
use Carp qw(croak);
use CodingAdventures::WasmLeb128 qw(decode_unsigned decode_signed);
use CodingAdventures::WasmTypes  qw(decode_limits);

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(
    parse parse_header parse_section get_section
    SECTION_CUSTOM SECTION_TYPE SECTION_IMPORT SECTION_FUNCTION
    SECTION_TABLE SECTION_MEMORY SECTION_GLOBAL SECTION_EXPORT
    SECTION_START SECTION_ELEMENT SECTION_CODE SECTION_DATA
    MODULE_MAGIC MODULE_VERSION
);

# ============================================================================
# Constants — Section IDs
#
# These numeric codes identify what type of data a section contains.
# They appear as the first byte of each section.
# ============================================================================

use constant {
    SECTION_CUSTOM   => 0,   # Custom section: name + arbitrary data
    SECTION_TYPE     => 1,   # Function type signatures
    SECTION_IMPORT   => 2,   # External symbol imports
    SECTION_FUNCTION => 3,   # Type indices for local functions
    SECTION_TABLE    => 4,   # Reference table definitions
    SECTION_MEMORY   => 5,   # Linear memory definitions
    SECTION_GLOBAL   => 6,   # Global variable definitions
    SECTION_EXPORT   => 7,   # Exported symbols
    SECTION_START    => 8,   # Start function index
    SECTION_ELEMENT  => 9,   # Table initialization data
    SECTION_CODE     => 10,  # Function bodies
    SECTION_DATA     => 11,  # Memory initialization data
};

# Module magic number and version
use constant MODULE_MAGIC   => "\x00asm";
use constant MODULE_VERSION => 1;

# Human-readable section names (for error messages)
my %SECTION_NAMES = (
    0  => 'custom',
    1  => 'type',
    2  => 'import',
    3  => 'function',
    4  => 'table',
    5  => 'memory',
    6  => 'global',
    7  => 'export',
    8  => 'start',
    9  => 'element',
    10 => 'code',
    11 => 'data',
);

# Export descriptor kind names
my %EXPORT_KINDS = (
    0 => 'func',
    1 => 'table',
    2 => 'mem',
    3 => 'global',
);

# ============================================================================
# _str_to_bytes($str) — convert binary string to arrayref of byte integers
#
# Perl's unpack() function can extract bytes from a binary string. We use
# unpack("C*", $str) to convert the entire string to a flat list of integers
# (each in the range 0–255), then store them in an arrayref.
#
# This byte array is then used for LEB128 decoding and direct byte access.
#
# Example:
#   _str_to_bytes("\x00\x61\x73\x6D")
#   --> [0, 97, 115, 109]
# ============================================================================
sub _str_to_bytes {
    my ($str) = @_;
    return [ unpack('C*', $str) ];
}

# ============================================================================
# _read_string(\@bytes, $offset) → ($string, $bytes_consumed)
#
# In the WebAssembly binary format, names are encoded as:
#   - A length prefix (unsigned LEB128)
#   - That many bytes of UTF-8 text
#
# This function reads such a name from the byte array starting at $offset
# (0-based). Returns the decoded string and the total number of bytes read.
#
# Example: the name "add" is encoded as:
#   0x03 0x61 0x64 0x64   → length=3, 'a', 'd', 'd'
# ============================================================================
sub _read_string {
    my ($bytes, $offset) = @_;

    my ($length, $lc) = decode_unsigned($bytes, $offset);
    my $consumed = $lc;

    my $str = pack('C*', @{$bytes}[$offset + $consumed .. $offset + $consumed + $length - 1]);
    $consumed += $length;

    return ($str, $consumed);
}

# ============================================================================
# _read_bytes(\@bytes, $offset, $count) → (\@slice, $count)
#
# Extract $count bytes starting at $offset into a new arrayref.
# Also returns $count (for convenience in the caller).
# ============================================================================
sub _read_bytes {
    my ($bytes, $offset, $count) = @_;
    my @slice = @{$bytes}[$offset .. $offset + $count - 1];
    return (\@slice, $count);
}

# ============================================================================
# _parse_init_expr(\@bytes, $offset) → (\@expr_bytes, $bytes_consumed)
#
# WebAssembly constant expressions are short instruction sequences that end
# with the "end" opcode (0x0B). They are used for global initializers and
# element/data segment offsets.
#
# Common forms:
#   i32.const N  → 0x41, signed_LEB128(N), 0x0B
#   i64.const N  → 0x42, signed_LEB128(N), 0x0B
#   f32.const V  → 0x43, 4 bytes, 0x0B
#   f64.const V  → 0x44, 8 bytes, 0x0B
#   global.get I → 0x23, unsigned_LEB128(I), 0x0B
#
# We collect bytes until we see 0x0B and return them as a raw byte array.
# This is a simplified parse adequate for structural module inspection.
# ============================================================================
sub _parse_init_expr {
    my ($bytes, $offset) = @_;

    my @expr;
    my $pos = $offset;

    while (1) {
        my $b = $bytes->[$pos];
        push @expr, $b;
        $pos++;
        last if $b == 0x0B;  # "end" opcode terminates the expression
    }

    return (\@expr, $pos - $offset);
}

# ============================================================================
# _parse_type_section(\@bytes, $offset, $size) → \@types
#
# The Type section lists all function signatures used in the module.
# Other sections refer to these signatures by index (0-based).
#
# Binary layout:
#   count (unsigned LEB128)             — number of type entries
#   For each type entry:
#     0x60                              — function type marker
#     param_count (unsigned LEB128)
#     param_type_1, …                   — each a 1-byte ValType
#     result_count (unsigned LEB128)
#     result_type_1, …                  — each a 1-byte ValType
#
# WHY 0x60?
#   This byte serves as a sentinel to distinguish function types from other
#   composite type forms that may be added by future proposals. All ValType
#   bytes are >= 0x6F, so 0x60 cannot be confused with a value type.
#
# Returns: arrayref of hashrefs {params => [...], results => [...]}
# ============================================================================
sub _parse_type_section {
    my ($bytes, $offset) = @_;

    my ($count, $cc) = decode_unsigned($bytes, $offset);
    my $pos = $offset + $cc;

    my @types;
    for my $i (1 .. $count) {
        my $marker = $bytes->[$pos];
        croak sprintf(
            'WasmModuleParser: expected 0x60 (func type) at offset %d, got 0x%02x',
            $pos, $marker
        ) unless $marker == 0x60;
        $pos++;

        # Parameters
        my ($param_count, $pc) = decode_unsigned($bytes, $pos);
        $pos += $pc;
        my @params = @{$bytes}[$pos .. $pos + $param_count - 1];
        $pos += $param_count;

        # Results
        my ($result_count, $rc) = decode_unsigned($bytes, $pos);
        $pos += $rc;
        my @results = @{$bytes}[$pos .. $pos + $result_count - 1];
        $pos += $result_count;

        push @types, { params => \@params, results => \@results };
    }

    return \@types;
}

# ============================================================================
# _parse_import_section(\@bytes, $offset, $size) → \@imports
#
# The Import section lists external symbols the module depends on.
# Imports can be functions, tables, memories, or globals.
#
# Binary layout:
#   count (unsigned LEB128)
#   For each import:
#     module_name (length-prefixed UTF-8)
#     field_name  (length-prefixed UTF-8)
#     import descriptor:
#       0x00 func   → type_index (unsigned LEB128)
#       0x01 table  → ref_type (1 byte), limits
#       0x02 mem    → limits
#       0x03 global → val_type (1 byte), mutability (1 byte)
#
# Returns: arrayref of {mod, name, desc} hashrefs.
# ============================================================================
sub _parse_import_section {
    my ($bytes, $offset) = @_;

    my ($count, $cc) = decode_unsigned($bytes, $offset);
    my $pos = $offset + $cc;

    my @imports;
    for my $i (1 .. $count) {
        my ($mod_name, $mc) = _read_string($bytes, $pos);
        $pos += $mc;

        my ($field_name, $fc) = _read_string($bytes, $pos);
        $pos += $fc;

        my $tag = $bytes->[$pos++];
        my $desc;

        if ($tag == 0x00) {
            # Function import: type index in the Type section
            my ($type_idx, $tc) = decode_unsigned($bytes, $pos);
            $pos += $tc;
            $desc = { kind => 'func', type_idx => $type_idx };
        }
        elsif ($tag == 0x01) {
            # Table import: reference type + limits
            my $ref_type = $bytes->[$pos++];
            my ($lim, $lc) = decode_limits($bytes, $pos);
            $pos += $lc;
            $desc = { kind => 'table', ref_type => $ref_type, limits => $lim };
        }
        elsif ($tag == 0x02) {
            # Memory import: limits only
            my ($lim, $lc) = decode_limits($bytes, $pos);
            $pos += $lc;
            $desc = { kind => 'mem', limits => $lim };
        }
        elsif ($tag == 0x03) {
            # Global import: value type + mutability flag
            my $val_type = $bytes->[$pos++];
            my $mut      = $bytes->[$pos++];
            $desc = { kind => 'global', val_type => $val_type, mutable => ($mut == 1 ? 1 : 0) };
        }
        else {
            croak sprintf(
                'WasmModuleParser: unknown import descriptor tag 0x%02x at offset %d',
                $tag, $pos - 1
            );
        }

        push @imports, { mod => $mod_name, name => $field_name, desc => $desc };
    }

    return \@imports;
}

# ============================================================================
# _parse_function_section(\@bytes, $offset) → \@functions
#
# The Function section maps each locally-defined function to its type
# signature (by index into the Type section).
#
# This section contains ONLY the type indices — no bytecode. The actual
# function bodies are in the Code section.
#
# Binary layout:
#   count (unsigned LEB128)
#   For each function:
#     type_index (unsigned LEB128)
#
# Returns: arrayref of type index integers.
# ============================================================================
sub _parse_function_section {
    my ($bytes, $offset) = @_;

    my ($count, $cc) = decode_unsigned($bytes, $offset);
    my $pos = $offset + $cc;

    my @functions;
    for my $i (1 .. $count) {
        my ($type_idx, $tc) = decode_unsigned($bytes, $pos);
        $pos += $tc;
        push @functions, $type_idx;
    }

    return \@functions;
}

# ============================================================================
# _parse_table_section(\@bytes, $offset) → \@tables
#
# Tables hold references (funcref or externref). They are used for indirect
# function calls — the key mechanism for implementing C function pointers,
# vtables, and dynamic dispatch in Wasm.
#
# Binary layout:
#   count (unsigned LEB128)
#   For each table:
#     ref_type (1 byte)  — 0x70=funcref, 0x6F=externref
#     limits             — min and optional max element count
#
# Returns: arrayref of {ref_type, limits} hashrefs.
# ============================================================================
sub _parse_table_section {
    my ($bytes, $offset) = @_;

    my ($count, $cc) = decode_unsigned($bytes, $offset);
    my $pos = $offset + $cc;

    my @tables;
    for my $i (1 .. $count) {
        my $ref_type = $bytes->[$pos++];
        my ($lim, $lc) = decode_limits($bytes, $pos);
        $pos += $lc;
        push @tables, { ref_type => $ref_type, limits => $lim };
    }

    return \@tables;
}

# ============================================================================
# _parse_memory_section(\@bytes, $offset) → \@memories
#
# WebAssembly's linear memory is a flat, byte-addressable array. The Memory
# section specifies its size in 64 KiB pages.
#
#   1 page = 65,536 bytes
#
# Size limits use the Limits encoding:
#   0x00 min   — no maximum (can grow until host runs out of memory)
#   0x01 min max — bounded growth
#
# Returns: arrayref of {limits} hashrefs.
# ============================================================================
sub _parse_memory_section {
    my ($bytes, $offset) = @_;

    my ($count, $cc) = decode_unsigned($bytes, $offset);
    my $pos = $offset + $cc;

    my @memories;
    for my $i (1 .. $count) {
        my ($lim, $lc) = decode_limits($bytes, $pos);
        $pos += $lc;
        push @memories, { limits => $lim };
    }

    return \@memories;
}

# ============================================================================
# _parse_global_section(\@bytes, $offset) → \@globals
#
# Global variables are module-level values accessible to all functions.
# Each global has a type, a mutability flag, and an initial value expressed
# as a constant expression.
#
# Binary layout:
#   count (unsigned LEB128)
#   For each global:
#     val_type   (1 byte)   — the type (i32, i64, f32, f64, funcref, externref)
#     mutability (1 byte)   — 0=const (immutable), 1=var (mutable)
#     init_expr             — constant expression terminated by 0x0B
#
# Returns: arrayref of {val_type, mutable, init_expr} hashrefs.
# ============================================================================
sub _parse_global_section {
    my ($bytes, $offset) = @_;

    my ($count, $cc) = decode_unsigned($bytes, $offset);
    my $pos = $offset + $cc;

    my @globals;
    for my $i (1 .. $count) {
        my $val_type = $bytes->[$pos++];
        my $mut      = $bytes->[$pos++];

        my ($init_bytes, $ic) = _parse_init_expr($bytes, $pos);
        $pos += $ic;

        push @globals, {
            val_type  => $val_type,
            mutable   => ($mut == 1 ? 1 : 0),
            init_expr => $init_bytes,
        };
    }

    return \@globals;
}

# ============================================================================
# _parse_export_section(\@bytes, $offset) → \@exports
#
# The Export section defines the module's public API — what it makes
# available to the host environment.
#
# Binary layout:
#   count (unsigned LEB128)
#   For each export:
#     name (length-prefixed UTF-8)
#     descriptor:
#       tag (1 byte):
#         0x00 → function index (unsigned LEB128)
#         0x01 → table index (unsigned LEB128)
#         0x02 → memory index (unsigned LEB128)
#         0x03 → global index (unsigned LEB128)
#
# Returns: arrayref of {name, desc} hashrefs.
# ============================================================================
sub _parse_export_section {
    my ($bytes, $offset) = @_;

    my ($count, $cc) = decode_unsigned($bytes, $offset);
    my $pos = $offset + $cc;

    my @exports;
    for my $i (1 .. $count) {
        my ($name, $nc) = _read_string($bytes, $pos);
        $pos += $nc;

        my $tag = $bytes->[$pos++];
        my ($idx, $ic) = decode_unsigned($bytes, $pos);
        $pos += $ic;

        my $kind = $EXPORT_KINDS{$tag} // sprintf('unknown_%d', $tag);
        push @exports, { name => $name, desc => { kind => $kind, idx => $idx } };
    }

    return \@exports;
}

# ============================================================================
# _parse_start_section(\@bytes, $offset) → $func_idx
#
# The optional Start section names a single function to be called when the
# module is instantiated. This function must have type () → ().
#
# Binary layout: function_index (unsigned LEB128)
#
# Returns: integer function index.
# ============================================================================
sub _parse_start_section {
    my ($bytes, $offset) = @_;
    my ($func_idx, $fc) = decode_unsigned($bytes, $offset);
    return $func_idx;
}

# ============================================================================
# _parse_code_section(\@bytes, $offset) → \@codes
#
# The Code section contains the bytecode bodies for each locally-defined
# function. The number of entries must match the Function section.
#
# Binary layout:
#   count (unsigned LEB128)              — number of function bodies
#   For each function body:
#     body_size (unsigned LEB128)        — total bytes in this body
#     local_decl_count (unsigned LEB128) — number of local variable groups
#     For each local group:
#       n    (unsigned LEB128)   — count of locals in this group
#       type (1 byte)            — ValType for these locals
#     [instruction bytes + 0x0B]  — the function bytecode
#
# LOCAL VARIABLE GROUPS
# Grouping locals by type allows compact encoding. Instead of declaring
# 5 i32 locals individually, you say "5 locals of type i32" in one entry.
#
# Returns: arrayref of {locals => [...], body => [...]} hashrefs.
#   locals: arrayref of {count, type} hashrefs
#   body:   arrayref of raw instruction bytes (including final 0x0B)
# ============================================================================
sub _parse_code_section {
    my ($bytes, $offset) = @_;

    my ($count, $cc) = decode_unsigned($bytes, $offset);
    my $pos = $offset + $cc;

    my @codes;
    for my $i (1 .. $count) {
        my ($body_size, $bc) = decode_unsigned($bytes, $pos);
        $pos += $bc;
        my $body_start = $pos;

        # Read local variable group declarations
        my ($local_count, $lcc) = decode_unsigned($bytes, $pos);
        $pos += $lcc;

        my @locals;
        for my $j (1 .. $local_count) {
            my ($n, $nc) = decode_unsigned($bytes, $pos);
            $pos += $nc;
            my $type = $bytes->[$pos++];
            push @locals, { count => $n, type => $type };
        }

        # The remaining bytes (up to body_start + body_size) are instructions
        my $instr_size = $body_size - ($pos - $body_start);
        my ($instr_bytes, $ic) = _read_bytes($bytes, $pos, $instr_size);
        $pos += $instr_size;

        push @codes, { locals => \@locals, body => $instr_bytes };
    }

    return \@codes;
}

# ============================================================================
# _parse_custom_section(\@bytes, $offset, $section_end) → \%custom
#
# Custom sections are extension points with arbitrary content. They have:
#   - A name (length-prefixed UTF-8)
#   - Data (all remaining bytes in the section)
#
# Well-known custom section names include:
#   "name"     — debug names for functions, locals, etc.
#   "producers" — information about the compiler toolchain
#   ".debug_info" — DWARF debug information
#
# Returns: hashref {name => $str, data => \@bytes}.
# ============================================================================
sub _parse_custom_section {
    my ($bytes, $offset, $section_end) = @_;

    my ($name, $nc) = _read_string($bytes, $offset);
    my $pos = $offset + $nc;

    # All remaining bytes in the section are the custom data
    my $data_size = $section_end - $pos;
    my @data = $data_size > 0
        ? @{$bytes}[$pos .. $pos + $data_size - 1]
        : ();

    return { name => $name, data => \@data };
}

# ============================================================================
# parse_header($bytes_str) → $new_offset
#
# Validate the 8-byte Wasm module header. The input is a binary string
# (use open with :raw or :bytes binmode).
#
# Checks:
#   1. Bytes 0-3 are 0x00 0x61 0x73 0x6D ("\x00asm")
#   2. Bytes 4-7 are 0x01 0x00 0x00 0x00 (version 1, little-endian)
#
# Returns: 8 (the offset after the header)
# Dies:    with a descriptive message if magic or version are wrong
# ============================================================================
sub parse_header {
    my ($bytes_ref, $offset) = @_;
    $offset //= 0;

    # Check magic bytes
    my @expected_magic = (0x00, 0x61, 0x73, 0x6D);
    for my $i (0 .. 3) {
        my $got = $bytes_ref->[$offset + $i] // 0;
        croak sprintf(
            'WasmModuleParser: invalid magic byte at offset %d: expected 0x%02x, got 0x%02x',
            $offset + $i, $expected_magic[$i], $got
        ) unless $got == $expected_magic[$i];
    }

    # Check version bytes (little-endian uint32 = 1)
    my @expected_version = (0x01, 0x00, 0x00, 0x00);
    for my $i (0 .. 3) {
        my $got = $bytes_ref->[$offset + 4 + $i] // 0;
        croak sprintf(
            'WasmModuleParser: invalid version byte at offset %d: expected 0x%02x, got 0x%02x',
            $offset + 4 + $i, $expected_version[$i], $got
        ) unless $got == $expected_version[$i];
    }

    return $offset + 8;
}

# ============================================================================
# parse_section(\@bytes, $offset) → (\%section_info, $content_start_offset)
#
# Reads one section header: a 1-byte section ID and an unsigned LEB128 length.
# Does NOT parse the section content — only the envelope.
#
# Returns (undef, $offset) if $offset is past the end of the byte array.
#
# Returns:
#   section_info — hashref {
#       id            => <section ID integer 0-11>,
#       name          => <human-readable name string>,
#       size          => <content byte count>,
#       content_start => <0-based offset of first content byte>,
#       content_end   => <0-based offset of last content byte>,
#   }
#   content_start_offset — same as section_info->{content_start}
# ============================================================================
sub parse_section {
    my ($bytes, $offset) = @_;

    return (undef, $offset) if $offset >= scalar(@$bytes);

    my $id  = $bytes->[$offset];
    my $pos = $offset + 1;

    my ($size, $sc) = decode_unsigned($bytes, $pos);
    $pos += $sc;

    my $content_start = $pos;
    my $content_end   = $pos + $size - 1;

    my $info = {
        id            => $id,
        name          => ($SECTION_NAMES{$id} // sprintf('unknown_%d', $id)),
        size          => $size,
        content_start => $content_start,
        content_end   => $content_end,
    };

    return ($info, $content_start);
}

# ============================================================================
# get_section(\%module, $section_id) → section_data or undef
#
# Convenience accessor for retrieving a parsed section from a module hashref.
# Returns the appropriate field from the module structure, or undef if the
# section was not present in the binary.
#
# For custom sections (id=0), returns the full custom sections arrayref.
# ============================================================================
sub get_section {
    my ($module, $section_id) = @_;

    return $module->{types}     if $section_id == SECTION_TYPE;
    return $module->{imports}   if $section_id == SECTION_IMPORT;
    return $module->{functions} if $section_id == SECTION_FUNCTION;
    return $module->{tables}    if $section_id == SECTION_TABLE;
    return $module->{memories}  if $section_id == SECTION_MEMORY;
    return $module->{globals}   if $section_id == SECTION_GLOBAL;
    return $module->{exports}   if $section_id == SECTION_EXPORT;
    return $module->{start}     if $section_id == SECTION_START;
    return $module->{elements}  if $section_id == SECTION_ELEMENT;
    return $module->{codes}     if $section_id == SECTION_CODE;
    return $module->{data}      if $section_id == SECTION_DATA;
    return $module->{custom}    if $section_id == SECTION_CUSTOM;
    return undef;
}

# ============================================================================
# parse($bytes_str) → \%module
#
# Parse a complete WebAssembly binary module from a binary string.
#
# The binary string should be read from a .wasm file with :raw binmode:
#   open my $fh, '<:raw', 'module.wasm' or die $!;
#   my $bytes_str = do { local $/; <$fh> };
#   close $fh;
#
# Returns a hashref:
#   {
#     magic     => "\x00asm",         # always
#     version   => 1,                  # always
#     types     => [...],              # array of {params, results}
#     imports   => [...],              # array of {mod, name, desc}
#     functions => [...],              # array of type indices
#     tables    => [...],              # array of {ref_type, limits}
#     memories  => [...],              # array of {limits}
#     globals   => [...],              # array of {val_type, mutable, init_expr}
#     exports   => [...],              # array of {name, desc}
#     start     => undef or N,         # start function index or undef
#     elements  => [...],              # array of raw byte arrays
#     codes     => [...],              # array of {locals, body}
#     data      => [...],              # array of raw byte arrays
#     custom    => [...],              # array of {name, data}
#   }
#
# Dies if the magic number or version bytes are invalid.
# ============================================================================
sub parse {
    my ($bytes_str) = @_;

    # Convert binary string to integer array for byte-level access
    my $bytes = _str_to_bytes($bytes_str);

    # Initialize module with empty defaults
    my $module = {
        magic     => MODULE_MAGIC,
        version   => MODULE_VERSION,
        types     => [],
        imports   => [],
        functions => [],
        tables    => [],
        memories  => [],
        globals   => [],
        exports   => [],
        start     => undef,
        elements  => [],
        codes     => [],
        data      => [],
        custom    => [],
    };

    # Validate and skip the 8-byte header
    my $pos = parse_header($bytes, 0);

    # Parse sections until we run out of bytes
    while ($pos < scalar(@$bytes)) {
        my ($section_info, $content_start) = parse_section($bytes, $pos);
        last unless defined $section_info;

        my $id     = $section_info->{id};
        my $cstart = $section_info->{content_start};
        my $cend   = $section_info->{content_end};
        my $csize  = $section_info->{size};

        if ($id == SECTION_TYPE) {
            $module->{types} = _parse_type_section($bytes, $cstart);
        }
        elsif ($id == SECTION_IMPORT) {
            $module->{imports} = _parse_import_section($bytes, $cstart);
        }
        elsif ($id == SECTION_FUNCTION) {
            $module->{functions} = _parse_function_section($bytes, $cstart);
        }
        elsif ($id == SECTION_TABLE) {
            $module->{tables} = _parse_table_section($bytes, $cstart);
        }
        elsif ($id == SECTION_MEMORY) {
            $module->{memories} = _parse_memory_section($bytes, $cstart);
        }
        elsif ($id == SECTION_GLOBAL) {
            $module->{globals} = _parse_global_section($bytes, $cstart);
        }
        elsif ($id == SECTION_EXPORT) {
            $module->{exports} = _parse_export_section($bytes, $cstart);
        }
        elsif ($id == SECTION_START) {
            $module->{start} = _parse_start_section($bytes, $cstart);
        }
        elsif ($id == SECTION_ELEMENT) {
            my ($raw, $rc) = _read_bytes($bytes, $cstart, $csize);
            push @{ $module->{elements} }, $raw;
        }
        elsif ($id == SECTION_CODE) {
            $module->{codes} = _parse_code_section($bytes, $cstart);
        }
        elsif ($id == SECTION_DATA) {
            my ($raw, $rc) = _read_bytes($bytes, $cstart, $csize);
            push @{ $module->{data} }, $raw;
        }
        elsif ($id == SECTION_CUSTOM) {
            my $custom = _parse_custom_section($bytes, $cstart, $cend);
            push @{ $module->{custom} }, $custom;
        }
        # Unknown section IDs are silently ignored for forward compatibility.

        # Advance to the next section
        $pos = $cend + 1;
    }

    return $module;
}

1;

__END__

=head1 NAME

CodingAdventures::WasmModuleParser - WebAssembly binary module parser

=head1 SYNOPSIS

    use CodingAdventures::WasmModuleParser qw(parse get_section);
    use CodingAdventures::WasmModuleParser qw(
        SECTION_TYPE SECTION_IMPORT SECTION_EXPORT SECTION_CODE
    );

    # Read a .wasm file
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

    # Get a section by ID
    my $types = get_section($module, SECTION_TYPE);

=head1 DESCRIPTION

Parses WebAssembly binary modules (.wasm files) into structured Perl data
following the WebAssembly binary format specification
(L<https://webassembly.github.io/spec/core/binary/modules.html>).

=head1 FUNCTIONS

=over 4

=item C<parse($bytes_str)>

Parse a Wasm binary string into a module hashref. Dies if the magic number
or version bytes are invalid.

=item C<parse_header(\@bytes, $offset)>

Validate the 8-byte Wasm header. Returns the offset after the header (8).

=item C<parse_section(\@bytes, $offset)>

Parse one section envelope (ID + LEB128 length). Returns (undef, $offset)
at end of input, otherwise (\%section_info, $content_start).

=item C<get_section(\%module, $section_id)>

Retrieve a parsed section from a module hashref by section ID constant.

=back

=head1 CONSTANTS

    SECTION_CUSTOM    0    SECTION_TYPE      1
    SECTION_IMPORT    2    SECTION_FUNCTION  3
    SECTION_TABLE     4    SECTION_MEMORY    5
    SECTION_GLOBAL    6    SECTION_EXPORT    7
    SECTION_START     8    SECTION_ELEMENT   9
    SECTION_CODE     10    SECTION_DATA     11

=head1 MODULE STRUCTURE

The hashref returned by C<parse()> has these keys:

    magic     => "\x00asm"
    version   => 1
    types     => [ {params=>[...], results=>[...]}, ... ]
    imports   => [ {mod=>'env', name=>'log', desc=>{kind=>'func',type_idx=>0}}, ... ]
    functions => [ 0, 1, 0, ... ]      # type indices
    tables    => [ {ref_type=>0x70, limits=>{min=>1, max=>undef}}, ... ]
    memories  => [ {limits=>{min=>1, max=>undef}}, ... ]
    globals   => [ {val_type=>0x7F, mutable=>0, init_expr=>[...]}, ... ]
    exports   => [ {name=>'add', desc=>{kind=>'func', idx=>0}}, ... ]
    start     => undef or N
    elements  => [ \@raw_bytes, ... ]
    codes     => [ {locals=>[{count=>N,type=>T},...], body=>\@bytes}, ... ]
    data      => [ \@raw_bytes, ... ]
    custom    => [ {name=>'name', data=>\@bytes}, ... ]

=head1 DEPENDENCIES

L<CodingAdventures::WasmLeb128>, L<CodingAdventures::WasmTypes>

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
