package CodingAdventures::WasmTypes;

# ============================================================================
# CodingAdventures::WasmTypes — WebAssembly value types and type definitions
# ============================================================================
#
# WebAssembly (Wasm) is a statically typed binary instruction format. This
# module defines the fundamental types from the WebAssembly specification and
# provides functions to encode/decode their binary representations.
#
# ## The Wasm Type System
#
# WebAssembly's type system is minimal and precise. Every value in a running
# Wasm program has one of these types:
#
#   INTEGER TYPES:
#     i32 — 32-bit integer.  Used for both signed and unsigned arithmetic.
#     i64 — 64-bit integer.  For 64-bit arithmetic and memory addresses.
#
#   FLOATING-POINT TYPES:
#     f32 — 32-bit IEEE 754 single-precision float.
#     f64 — 64-bit IEEE 754 double-precision float.
#
#   VECTOR TYPE:
#     v128 — 128-bit SIMD vector (added by the SIMD proposal).
#
#   REFERENCE TYPES:
#     funcref   — an opaque reference to a function; may be null.
#     externref — an opaque reference to a host/external value; may be null.
#
# ## Binary Encoding of Types
#
# In the WebAssembly binary format, each type is encoded as a single byte.
# The values were chosen to be in the range -1 to -64 when interpreted as a
# signed byte, which is the range that encodes as a single-byte signed LEB128.
#
#   i32      = 0x7F = 127  (-1  as signed)
#   i64      = 0x7E = 126  (-2  as signed)
#   f32      = 0x7D = 125  (-3  as signed)
#   f64      = 0x7C = 124  (-4  as signed)
#   v128     = 0x7B = 123  (-5  as signed)
#   funcref  = 0x70 = 112  (-16 as signed)
#   externref = 0x6F = 111 (-17 as signed)
#
# ## Composite Types
#
#   FuncType:  {params => [\@param_types], results => [\@result_types]}
#              Encoded as: 0x60, param_count (LEB128), params...,
#                          result_count (LEB128), results...
#
#   Limits:    {min => N, max => M|undef}
#              Encoded as: 0x00 min_leb128       (no maximum)
#                       or 0x01 min_leb128 max_leb128 (has maximum)
#
# ## Usage
#
#   use CodingAdventures::WasmTypes qw(
#       encode_val_type decode_val_type
#       encode_limits   decode_limits
#       encode_func_type decode_func_type
#       is_val_type is_ref_type val_type_name
#   );
#
#   is_val_type(0x7F)        # true  (i32)
#   val_type_name(0x7E)      # "i64"
#   encode_val_type(0x7F)    # (0x7F)   — list of one byte
#   decode_val_type([0x7F])  # (0x7F, 1)  — type byte, bytes_consumed
#
# ============================================================================

use strict;
use warnings;
use Carp qw(croak);
use CodingAdventures::WasmLeb128 qw(encode_unsigned decode_unsigned);

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(
    is_val_type    is_ref_type    val_type_name
    encode_val_type decode_val_type
    encode_limits   decode_limits
    encode_func_type decode_func_type
);

# ============================================================================
# ValType constants
# ============================================================================
#
# These are the byte codes used in the binary format to identify value types.
# They appear in function signatures, global types, table element types,
# and block type annotations.

use constant {
    VALTYPE_I32      => 0x7F,  # 32-bit integer
    VALTYPE_I64      => 0x7E,  # 64-bit integer
    VALTYPE_F32      => 0x7D,  # 32-bit IEEE 754 float
    VALTYPE_F64      => 0x7C,  # 64-bit IEEE 754 float
    VALTYPE_V128     => 0x7B,  # 128-bit SIMD vector
    VALTYPE_FUNCREF  => 0x70,  # function reference (opaque, nullable)
    VALTYPE_EXTERNREF => 0x6F, # external/host reference (opaque, nullable)
};

# Export these constants so callers don't need to hardcode hex values.
our %ValType = (
    i32      => VALTYPE_I32,
    i64      => VALTYPE_I64,
    f32      => VALTYPE_F32,
    f64      => VALTYPE_F64,
    v128     => VALTYPE_V128,
    funcref  => VALTYPE_FUNCREF,
    externref => VALTYPE_EXTERNREF,
);

# ============================================================================
# RefType constants
# ============================================================================
#
# Reference types are the subset of value types that hold opaque references.
# They are used as element types in tables and in reference instructions.

our %RefType = (
    funcref  => VALTYPE_FUNCREF,
    externref => VALTYPE_EXTERNREF,
);

# ============================================================================
# BlockType constant
# ============================================================================
#
# Block type 0x40 means "no values" (void/epsilon). It is used in block, loop,
# and if instructions that neither consume nor produce values on the stack.

use constant BLOCK_TYPE_EMPTY => 0x40;
our %BlockType = ( empty => BLOCK_TYPE_EMPTY );

# ============================================================================
# ExternType constants
# ============================================================================
#
# These codes appear in the import/export sections to identify what kind of
# entity is being imported or exported.

our %ExternType = (
    func   => 0,
    table  => 1,
    mem    => 2,
    global => 3,
);

# Internal lookup set for fast is_val_type checks
my %_VALID_VAL_TYPES = map { $_ => 1 } (
    VALTYPE_I32, VALTYPE_I64, VALTYPE_F32, VALTYPE_F64,
    VALTYPE_V128, VALTYPE_FUNCREF, VALTYPE_EXTERNREF,
);

# Internal name map for val_type_name
my %_VAL_TYPE_NAMES = (
    VALTYPE_I32()      => 'i32',
    VALTYPE_I64()      => 'i64',
    VALTYPE_F32()      => 'f32',
    VALTYPE_F64()      => 'f64',
    VALTYPE_V128()     => 'v128',
    VALTYPE_FUNCREF()  => 'funcref',
    VALTYPE_EXTERNREF() => 'externref',
);

# ============================================================================
# is_val_type($byte) — predicate: is this a valid ValType byte?
# ============================================================================
#
# Returns true (1) if $byte is one of the seven WebAssembly value type codes.
# Returns false ('') otherwise.
#
# @param  $byte   Integer to test.
# @return         1 or ''.

sub is_val_type {
    my $byte = ( @_ == 2 ) ? $_[1] : $_[0];
    return exists $_VALID_VAL_TYPES{$byte} ? 1 : '';
}

# ============================================================================
# is_ref_type($byte) — predicate: is this a reference type byte?
# ============================================================================
#
# Reference types are the two types that hold opaque references:
#   funcref  (0x70) and externref (0x6F).
#
# @param  $byte   Integer to test.
# @return         1 or ''.

sub is_ref_type {
    my $byte = ( @_ == 2 ) ? $_[1] : $_[0];
    return ($byte == VALTYPE_FUNCREF || $byte == VALTYPE_EXTERNREF) ? 1 : '';
}

# ============================================================================
# val_type_name($byte) — human-readable name for a ValType byte
# ============================================================================
#
# Returns the standard WebAssembly mnemonic for the given byte, or a fallback
# string "unknown_0xXX" for unrecognized bytes.
#
# EXAMPLES
#   val_type_name(0x7F) => "i32"
#   val_type_name(0x70) => "funcref"
#   val_type_name(0x42) => "unknown_0x42"
#
# @param  $byte   The value type byte code.
# @return         String name.

sub val_type_name {
    my $byte = ( @_ == 2 ) ? $_[1] : $_[0];
    return $_VAL_TYPE_NAMES{$byte} // sprintf('unknown_0x%02x', $byte);
}

# ============================================================================
# encode_val_type($val_type) — encode as a one-element byte list
# ============================================================================
#
# In the WebAssembly binary format, a value type is always exactly one byte.
# This function validates the input and returns a list containing that byte.
#
# @param  $val_type   A valid ValType byte (e.g., 0x7F for i32).
# @return             List of one integer: ($val_type).
# @dies               If $val_type is not a recognized value type.

sub encode_val_type {
    my $val_type = ( @_ == 2 ) ? $_[1] : $_[0];
    croak sprintf(
        'CodingAdventures::WasmTypes::encode_val_type: invalid val_type 0x%02x',
        $val_type
    ) unless is_val_type($val_type);
    return ($val_type);
}

# ============================================================================
# decode_val_type($bytes_ref, $offset) — decode from byte array
# ============================================================================
#
# Reads one byte at $offset (0-based) from $bytes_ref, validates it as a
# value type, and returns (type_byte, bytes_consumed).
#
# @param  $bytes_ref  Array reference to byte values.
# @param  $offset     0-based starting index (defaults to 0).
# @return             List: (type_byte, 1).
# @dies               If offset is out of range or byte is not a valid ValType.

sub decode_val_type {
    my ($bytes_ref, $offset) = @_;
    $offset //= 0;
    croak 'CodingAdventures::WasmTypes::decode_val_type: offset out of range'
        if $offset >= scalar @$bytes_ref;
    my $byte = $bytes_ref->[$offset];
    croak sprintf(
        'CodingAdventures::WasmTypes::decode_val_type: invalid val_type 0x%02x at offset %d',
        $byte, $offset
    ) unless is_val_type($byte);
    return ($byte, 1);
}

# ============================================================================
# encode_limits(\%limits) — encode a Limits structure as a byte list
# ============================================================================
#
# A Limits structure specifies the minimum (and optionally maximum) size of a
# memory region or table. In the binary format:
#
#   Flag byte 0x00: no maximum → [0x00, leb128(min)]
#   Flag byte 0x01: has maximum → [0x01, leb128(min), leb128(max)]
#
# EXAMPLES
#   encode_limits({min=>0})         → (0x00, 0x00)
#   encode_limits({min=>1, max=>4}) → (0x01, 0x01, 0x04)
#
# @param  $limits_ref  Hash reference with keys 'min' and optionally 'max'.
# @return              Flat list of byte values.

sub encode_limits {
    my ($limits_ref) = @_;
    my @result;

    if ( !defined $limits_ref->{max} ) {
        # No maximum: flag = 0x00, then min as unsigned LEB128
        push @result, 0x00;
        push @result, encode_unsigned( $limits_ref->{min} );
    }
    else {
        # Has maximum: flag = 0x01, then min, then max (both unsigned LEB128)
        push @result, 0x01;
        push @result, encode_unsigned( $limits_ref->{min} );
        push @result, encode_unsigned( $limits_ref->{max} );
    }

    return @result;
}

# ============================================================================
# decode_limits($bytes_ref, $offset) — decode a Limits structure
# ============================================================================
#
# Reads a flag byte at $offset, then reads min (and max if flag=0x01) as
# unsigned LEB128 integers.
#
# @param  $bytes_ref  Array reference to byte values.
# @param  $offset     0-based starting index (defaults to 0).
# @return             List: (hash_ref, bytes_consumed)
#                     where hash_ref = {min=>N, max=>M_or_undef}
# @dies               If the flag byte is neither 0x00 nor 0x01.

sub decode_limits {
    my ($bytes_ref, $offset) = @_;
    $offset //= 0;
    my $consumed = 0;

    # Read the flag byte
    my $flag = $bytes_ref->[$offset + $consumed];
    $consumed++;

    if ( $flag == 0x00 ) {
        # No maximum
        my ($min_val, $min_count) = decode_unsigned($bytes_ref, $offset + $consumed);
        $consumed += $min_count;
        return ( { min => $min_val, max => undef }, $consumed );
    }
    elsif ( $flag == 0x01 ) {
        # Has maximum
        my ($min_val, $min_count) = decode_unsigned($bytes_ref, $offset + $consumed);
        $consumed += $min_count;
        my ($max_val, $max_count) = decode_unsigned($bytes_ref, $offset + $consumed);
        $consumed += $max_count;
        return ( { min => $min_val, max => $max_val }, $consumed );
    }
    else {
        croak sprintf(
            'CodingAdventures::WasmTypes::decode_limits: invalid flag byte 0x%02x', $flag
        );
    }
}

# ============================================================================
# encode_func_type(\%func_type) — encode a function type signature
# ============================================================================
#
# A function type maps a list of parameter types to a list of result types.
# Binary encoding:
#
#   0x60                          — magic "function type" prefix
#   param_count (unsigned LEB128)
#   param_type_1, …               — each one byte (a ValType)
#   result_count (unsigned LEB128)
#   result_type_1, …              — each one byte (a ValType)
#
# EXAMPLE: (i32, i32) → i64
#   {params=>[0x7F, 0x7F], results=>[0x7E]}
#   → (0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7E)
#
# WHY 0x60?
#   This byte was chosen because it falls outside the ValType range.  It acts
#   as a sentinel telling the decoder "this is a function type entry".
#
# @param  $ft_ref  Hash reference with keys 'params' and 'results'
#                  (each an array ref of ValType bytes).
# @return          Flat list of bytes.

sub encode_func_type {
    my ($ft_ref) = @_;
    my @result = (0x60);  # function type magic byte

    # Parameter count + each param type
    push @result, encode_unsigned( scalar @{ $ft_ref->{params} } );
    push @result, @{ $ft_ref->{params} };

    # Result count + each result type
    push @result, encode_unsigned( scalar @{ $ft_ref->{results} } );
    push @result, @{ $ft_ref->{results} };

    return @result;
}

# ============================================================================
# decode_func_type($bytes_ref, $offset) — decode a function type signature
# ============================================================================
#
# Expects the magic byte 0x60 at $offset, followed by param count (LEB128),
# param types, result count (LEB128), result types.
#
# @param  $bytes_ref  Array reference to byte values.
# @param  $offset     0-based starting index (defaults to 0).
# @return             List: (hash_ref, bytes_consumed)
#                     where hash_ref = {params=>[...], results=>[...]}
# @dies               If the first byte is not 0x60 or if invalid ValType.

sub decode_func_type {
    my ($bytes_ref, $offset) = @_;
    $offset //= 0;
    my $consumed = 0;

    # Check magic byte
    my $magic = $bytes_ref->[$offset + $consumed];
    $consumed++;
    croak sprintf(
        'CodingAdventures::WasmTypes::decode_func_type: expected 0x60, got 0x%02x', $magic
    ) unless $magic == 0x60;

    # Read params
    my ($param_count, $pc) = decode_unsigned($bytes_ref, $offset + $consumed);
    $consumed += $pc;
    my @params;
    for ( 1 .. $param_count ) {
        my $vt = $bytes_ref->[$offset + $consumed];
        $consumed++;
        croak sprintf(
            'CodingAdventures::WasmTypes::decode_func_type: invalid param type 0x%02x', $vt
        ) unless is_val_type($vt);
        push @params, $vt;
    }

    # Read results
    my ($result_count, $rc) = decode_unsigned($bytes_ref, $offset + $consumed);
    $consumed += $rc;
    my @results;
    for ( 1 .. $result_count ) {
        my $vt = $bytes_ref->[$offset + $consumed];
        $consumed++;
        croak sprintf(
            'CodingAdventures::WasmTypes::decode_func_type: invalid result type 0x%02x', $vt
        ) unless is_val_type($vt);
        push @results, $vt;
    }

    return ( { params => \@params, results => \@results }, $consumed );
}

1;

__END__

=head1 NAME

CodingAdventures::WasmTypes - WebAssembly value types and binary type encoding

=head1 SYNOPSIS

    use CodingAdventures::WasmTypes qw(
        is_val_type is_ref_type val_type_name
        encode_val_type decode_val_type
        encode_limits   decode_limits
        encode_func_type decode_func_type
    );

    is_val_type(0x7F)           # 1  (i32)
    val_type_name(0x7E)         # "i64"
    my @b = encode_val_type(0x7F);     # (0x7F)
    my ($t, $n) = decode_val_type([0x7F]);  # (0x7F, 1)

    my @lb = encode_limits({min=>1, max=>4});   # (0x01, 0x01, 0x04)
    my ($lim, $lc) = decode_limits(\@lb);       # ({min=>1, max=>4}, 3)

=head1 DESCRIPTION

Provides all WebAssembly value types (ValType, RefType, BlockType, ExternType)
as Perl constants and hashes, plus encode/decode functions for binary format
representation.

=head1 CONSTANTS / HASHES

=over 4

=item C<%CodingAdventures::WasmTypes::ValType>

Keys: i32, i64, f32, f64, v128, funcref, externref

=item C<%CodingAdventures::WasmTypes::RefType>

Keys: funcref, externref

=item C<%CodingAdventures::WasmTypes::BlockType>

Keys: empty (0x40)

=item C<%CodingAdventures::WasmTypes::ExternType>

Keys: func (0), table (1), mem (2), global (3)

=back

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
