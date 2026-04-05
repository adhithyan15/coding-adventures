package CodingAdventures::WasmExecution;

# ============================================================================
# CodingAdventures::WasmExecution — WebAssembly 1.0 Execution Engine
# ============================================================================
#
# This module implements the complete WASM execution pipeline: typed values,
# linear memory, tables, bytecode decoding, constant expression evaluation,
# all instruction handlers, and the execution engine that ties them together.
#
# ## Architecture
#
#   WasmExecutionEngine.call_function(func_index, args)
#     |
#     +-- 1. Look up function body + type
#     +-- 2. Decode bytecodes into instructions
#     +-- 3. Build control flow map (block -> end/else)
#     +-- 4. Initialize locals (args + zero-init declared locals)
#     +-- 5. Create WasmExecutionContext
#     +-- 6. Run GenericVM.execute_with_context(code, context)
#     |       |
#     |       +-- For each instruction:
#     |           +-- Dispatch to registered handler
#     |           +-- Handler reads/modifies context (stack, memory, etc.)
#     |
#     +-- 7. Collect return values from typed stack
#
# ## Perl-Specific Notes
#
# - Bitwise NOT (~) on 64-bit Perl produces 64-bit results. We mask with
#   & 0xFFFFFFFF to get 32-bit results.
# - Perl's >> on negative integers does NOT do arithmetic shift on 64-bit
#   platforms. We use POSIX::floor($val / 2**$shift) for i32.shr_s.
# - f32 values: pack('f', $v) / unpack('f', pack('f', $v)) for rounding.
# - Each module using siblings must add use lib for sibling package paths.
#
# ============================================================================

use strict;
use warnings;
use Carp qw(croak);
use POSIX qw(floor);

our $VERSION = '0.01';

# Load sibling packages via relative lib paths
use lib '../wasm-leb128/lib';
use lib '../wasm-types/lib';
use lib '../wasm-opcodes/lib';
use lib '../wasm-module-parser/lib';
use lib '../virtual-machine/lib';

use CodingAdventures::WasmLeb128 qw(decode_unsigned decode_signed);
use CodingAdventures::WasmTypes;
use CodingAdventures::WasmOpcodes qw(get_opcode_info);
use CodingAdventures::VirtualMachine;

use Exporter 'import';
our @EXPORT_OK = qw(
    i32 i64 f32 f64 default_value
    evaluate_const_expr
);

# ============================================================================
# TrapError — unrecoverable WASM runtime error
# ============================================================================
#
# In WASM, a "trap" is a fatal runtime error (e.g., division by zero,
# out-of-bounds memory access). Traps immediately halt execution and
# propagate to the host.

package CodingAdventures::WasmExecution::TrapError;

sub new {
    my ($class, $message) = @_;
    return bless { message => $message }, $class;
}
sub message { return $_[0]->{message} }
sub throw {
    my ($class, $message) = @_;
    die $class->new($message);
}

# ============================================================================
# WasmValue Constructors
# ============================================================================
#
# Every value in WASM carries a type tag. In Perl, we represent typed values
# as hashrefs: { type => $type_code, value => $raw_value }.
#
# WASM 1.0 has four value types:
#   i32 (0x7F) — 32-bit integer
#   i64 (0x7E) — 64-bit integer (Perl IV on 64-bit)
#   f32 (0x7D) — 32-bit IEEE 754 float
#   f64 (0x7C) — 64-bit IEEE 754 double

package CodingAdventures::WasmExecution;

# i32($value) — create a 32-bit integer WASM value.
# Wraps to signed 32-bit range using the pack/unpack trick.
sub i32 {
    my ($v) = @_;
    # Wrap to signed 32-bit: pack as unsigned 32-bit, unpack as signed
    $v = unpack('l', pack('L', $v & 0xFFFFFFFF));
    return { type => 0x7F, value => $v };
}

# i64($value) — create a 64-bit integer WASM value.
# Perl uses 64-bit IVs on 64-bit platforms, so we just store the value.
sub i64 {
    my ($v) = @_;
    return { type => 0x7E, value => $v };
}

# f32($value) — create a 32-bit float WASM value.
# Rounds to single precision using pack('f')/unpack('f').
sub f32 {
    my ($v) = @_;
    $v = unpack('f', pack('f', $v));
    return { type => 0x7D, value => $v };
}

# f64($value) — create a 64-bit float WASM value.
# Perl numbers are already IEEE 754 doubles, so no conversion needed.
sub f64 {
    my ($v) = @_;
    return { type => 0x7C, value => $v };
}

# default_value($type_code) — create a zero-initialized value for a type.
sub default_value {
    my ($type_code) = @_;
    if ($type_code == 0x7F) { return i32(0); }
    if ($type_code == 0x7E) { return i64(0); }
    if ($type_code == 0x7D) { return f32(0); }
    if ($type_code == 0x7C) { return f64(0); }
    CodingAdventures::WasmExecution::TrapError->throw(
        sprintf("Unknown value type: 0x%02x", $type_code)
    );
}

# Type extraction helpers with type-safety assertions.
sub _as_i32 {
    my ($v) = @_;
    CodingAdventures::WasmExecution::TrapError->throw(
        sprintf("Type mismatch: expected i32, got 0x%02x", $v->{type})
    ) unless $v->{type} == 0x7F;
    return $v->{value};
}

sub _as_i64 {
    my ($v) = @_;
    CodingAdventures::WasmExecution::TrapError->throw(
        sprintf("Type mismatch: expected i64, got 0x%02x", $v->{type})
    ) unless $v->{type} == 0x7E;
    return $v->{value};
}

sub _as_f32 {
    my ($v) = @_;
    CodingAdventures::WasmExecution::TrapError->throw(
        sprintf("Type mismatch: expected f32, got 0x%02x", $v->{type})
    ) unless $v->{type} == 0x7D;
    return $v->{value};
}

sub _as_f64 {
    my ($v) = @_;
    CodingAdventures::WasmExecution::TrapError->throw(
        sprintf("Type mismatch: expected f64, got 0x%02x", $v->{type})
    ) unless $v->{type} == 0x7C;
    return $v->{value};
}

# ============================================================================
# LinearMemory — byte-addressable WASM heap
# ============================================================================
#
# WASM linear memory is a contiguous array of bytes, allocated in 64 KiB
# pages. All memory accesses are bounds-checked; out-of-bounds traps.
# WASM uses little-endian byte ordering.
#
# In Perl, we represent memory as a binary string and use pack/unpack for
# typed reads and writes.

package CodingAdventures::WasmExecution::LinearMemory;

use constant PAGE_SIZE => 65536;

sub new {
    my ($class, $initial_pages, $max_pages) = @_;
    return bless {
        data          => "\0" x ($initial_pages * PAGE_SIZE),
        current_pages => $initial_pages,
        max_pages     => $max_pages,  # may be undef
    }, $class;
}

# bounds_check($offset, $width) — trap if access is out of bounds.
sub _bounds_check {
    my ($self, $offset, $width) = @_;
    if ($offset < 0 || $offset + $width > length($self->{data})) {
        CodingAdventures::WasmExecution::TrapError->throw(
            "Out of bounds memory access: offset=$offset, size=$width, "
            . "memory size=" . length($self->{data})
        );
    }
}

# --- Full-width loads (little-endian) ---

sub load_i32 {
    my ($self, $offset) = @_;
    $self->_bounds_check($offset, 4);
    return unpack('l<', substr($self->{data}, $offset, 4));
}

sub load_i64 {
    my ($self, $offset) = @_;
    $self->_bounds_check($offset, 8);
    return unpack('q<', substr($self->{data}, $offset, 8));
}

sub load_f32 {
    my ($self, $offset) = @_;
    $self->_bounds_check($offset, 4);
    # Unpack as little-endian float
    my $bytes = substr($self->{data}, $offset, 4);
    return unpack('f<', $bytes);
}

sub load_f64 {
    my ($self, $offset) = @_;
    $self->_bounds_check($offset, 8);
    my $bytes = substr($self->{data}, $offset, 8);
    return unpack('d<', $bytes);
}

# --- Narrow loads for i32 ---

sub load_i32_8s {
    my ($self, $offset) = @_;
    $self->_bounds_check($offset, 1);
    return unpack('c', substr($self->{data}, $offset, 1));  # signed byte
}

sub load_i32_8u {
    my ($self, $offset) = @_;
    $self->_bounds_check($offset, 1);
    return unpack('C', substr($self->{data}, $offset, 1));  # unsigned byte
}

sub load_i32_16s {
    my ($self, $offset) = @_;
    $self->_bounds_check($offset, 2);
    return unpack('s<', substr($self->{data}, $offset, 2));  # signed 16-bit LE
}

sub load_i32_16u {
    my ($self, $offset) = @_;
    $self->_bounds_check($offset, 2);
    return unpack('v', substr($self->{data}, $offset, 2));  # unsigned 16-bit LE
}

# --- Narrow loads for i64 ---

sub load_i64_8s {
    my ($self, $offset) = @_;
    $self->_bounds_check($offset, 1);
    return unpack('c', substr($self->{data}, $offset, 1));
}

sub load_i64_8u {
    my ($self, $offset) = @_;
    $self->_bounds_check($offset, 1);
    return unpack('C', substr($self->{data}, $offset, 1));
}

# --- Full-width stores (little-endian) ---

sub store_i32 {
    my ($self, $offset, $value) = @_;
    $self->_bounds_check($offset, 4);
    substr($self->{data}, $offset, 4) = pack('l<', $value);
}

sub store_i64 {
    my ($self, $offset, $value) = @_;
    $self->_bounds_check($offset, 8);
    substr($self->{data}, $offset, 8) = pack('q<', $value);
}

sub store_f32 {
    my ($self, $offset, $value) = @_;
    $self->_bounds_check($offset, 4);
    substr($self->{data}, $offset, 4) = pack('f<', $value);
}

sub store_f64 {
    my ($self, $offset, $value) = @_;
    $self->_bounds_check($offset, 8);
    substr($self->{data}, $offset, 8) = pack('d<', $value);
}

# --- Narrow stores ---

sub store_i32_8 {
    my ($self, $offset, $value) = @_;
    $self->_bounds_check($offset, 1);
    substr($self->{data}, $offset, 1) = pack('c', $value & 0xFF);
}

sub store_i32_16 {
    my ($self, $offset, $value) = @_;
    $self->_bounds_check($offset, 2);
    substr($self->{data}, $offset, 2) = pack('s<', $value & 0xFFFF);
}

# --- Memory growth ---

sub grow {
    my ($self, $delta_pages) = @_;
    my $old_pages = $self->{current_pages};
    my $new_pages = $old_pages + $delta_pages;

    if (defined($self->{max_pages}) && $new_pages > $self->{max_pages}) {
        return -1;
    }
    if ($new_pages > 65536) {
        return -1;
    }

    $self->{data} .= "\0" x ($delta_pages * PAGE_SIZE);
    $self->{current_pages} = $new_pages;
    return $old_pages;
}

sub size   { return $_[0]->{current_pages} }
sub byte_length { return length($_[0]->{data}) }

# write_bytes($offset, \@bytes) — write raw bytes into memory.
sub write_bytes {
    my ($self, $offset, $bytes_ref) = @_;
    my $data;
    if (ref($bytes_ref) eq 'ARRAY') {
        $data = pack('C*', @$bytes_ref);
    } else {
        # Assume it's already a binary string
        $data = $bytes_ref;
    }
    $self->_bounds_check($offset, length($data));
    substr($self->{data}, $offset, length($data)) = $data;
}

# ============================================================================
# Table — array of nullable function references
# ============================================================================

package CodingAdventures::WasmExecution::Table;

sub new {
    my ($class, $initial_size, $max_size) = @_;
    my @elements = (undef) x $initial_size;
    return bless {
        elements => \@elements,
        max_size => $max_size,  # may be undef
    }, $class;
}

sub get {
    my ($self, $index) = @_;
    if ($index < 0 || $index >= scalar(@{ $self->{elements} })) {
        CodingAdventures::WasmExecution::TrapError->throw(
            "Out of bounds table access: index=$index, table size="
            . scalar(@{ $self->{elements} })
        );
    }
    return $self->{elements}[$index];
}

sub set {
    my ($self, $index, $func_index) = @_;
    if ($index < 0 || $index >= scalar(@{ $self->{elements} })) {
        CodingAdventures::WasmExecution::TrapError->throw(
            "Out of bounds table access: index=$index, table size="
            . scalar(@{ $self->{elements} })
        );
    }
    $self->{elements}[$index] = $func_index;
}

sub size { return scalar(@{ $_[0]->{elements} }) }

sub grow {
    my ($self, $delta) = @_;
    my $old_size = scalar(@{ $self->{elements} });
    my $new_size = $old_size + $delta;
    if (defined($self->{max_size}) && $new_size > $self->{max_size}) {
        return -1;
    }
    push @{ $self->{elements} }, (undef) x $delta;
    return $old_size;
}

# ============================================================================
# Bytecode Decoder — convert variable-length WASM bytecodes to instructions
# ============================================================================

package CodingAdventures::WasmExecution;

# decode_function_body($body) — decode a function body's bytecodes into
# an array of instruction hashrefs [{opcode => N, operand => V}, ...].
#
# $body is a hashref with:
#   code   => \@bytes (the raw bytecodes)
#   locals => \@type_codes (declared local types)

sub decode_function_body {
    my ($body) = @_;
    my $code = $body->{code};
    my @instructions;
    my $offset = 0;

    while ($offset < scalar(@$code)) {
        my $opcode = $code->[$offset];
        $offset++;

        my $operand = undef;

        # Decode immediates based on opcode
        if ($opcode == 0x02 || $opcode == 0x03 || $opcode == 0x04) {
            # block/loop/if: blocktype immediate
            $operand = $code->[$offset];
            $offset++;
        }
        elsif ($opcode == 0x0C || $opcode == 0x0D) {
            # br/br_if: label index (unsigned LEB128)
            my ($val, $consumed) = decode_unsigned($code, $offset);
            $operand = $val;
            $offset += $consumed;
        }
        elsif ($opcode == 0x0E) {
            # br_table: vec(label) + default label
            my ($count, $cc) = decode_unsigned($code, $offset);
            $offset += $cc;
            my @labels;
            for (1 .. $count) {
                my ($lbl, $lc) = decode_unsigned($code, $offset);
                push @labels, $lbl;
                $offset += $lc;
            }
            my ($def, $dc) = decode_unsigned($code, $offset);
            $offset += $dc;
            $operand = { labels => \@labels, default_label => $def };
        }
        elsif ($opcode == 0x10) {
            # call: function index
            my ($val, $consumed) = decode_unsigned($code, $offset);
            $operand = $val;
            $offset += $consumed;
        }
        elsif ($opcode == 0x11) {
            # call_indirect: type_idx + table_idx
            my ($type_idx, $tc) = decode_unsigned($code, $offset);
            $offset += $tc;
            my ($table_idx, $tbc) = decode_unsigned($code, $offset);
            $offset += $tbc;
            $operand = { type_idx => $type_idx, table_idx => $table_idx };
        }
        elsif ($opcode >= 0x20 && $opcode <= 0x24) {
            # local.get/set/tee, global.get/set: index (unsigned LEB128)
            my ($val, $consumed) = decode_unsigned($code, $offset);
            $operand = $val;
            $offset += $consumed;
        }
        elsif ($opcode >= 0x28 && $opcode <= 0x3B) {
            # Memory load/store: memarg (align + offset, both unsigned LEB128)
            my ($align, $ac) = decode_unsigned($code, $offset);
            $offset += $ac;
            my ($mem_offset, $oc) = decode_unsigned($code, $offset);
            $offset += $oc;
            $operand = { align => $align, offset => $mem_offset };
        }
        elsif ($opcode == 0x3F || $opcode == 0x40) {
            # memory.size/memory.grow: reserved byte (always 0x00)
            $offset++;  # skip reserved byte
        }
        elsif ($opcode == 0x41) {
            # i32.const: signed LEB128
            my ($val, $consumed) = decode_signed($code, $offset);
            $operand = $val;
            $offset += $consumed;
        }
        elsif ($opcode == 0x42) {
            # i64.const: signed LEB128 (64-bit)
            my ($val, $consumed) = decode_signed($code, $offset);
            $operand = $val;
            $offset += $consumed;
        }
        elsif ($opcode == 0x43) {
            # f32.const: 4 bytes IEEE 754 LE
            my $bytes = pack('C4', @{$code}[$offset .. $offset + 3]);
            $operand = unpack('f<', $bytes);
            $offset += 4;
        }
        elsif ($opcode == 0x44) {
            # f64.const: 8 bytes IEEE 754 LE
            my $bytes = pack('C8', @{$code}[$offset .. $offset + 7]);
            $operand = unpack('d<', $bytes);
            $offset += 8;
        }
        # All other opcodes have no immediates (numeric ops, parametric, etc.)

        push @instructions, { opcode => $opcode, operand => $operand };
    }

    return \@instructions;
}

# ============================================================================
# Control Flow Map — pre-scan block/loop/if -> end/else mappings
# ============================================================================

sub build_control_flow_map {
    my ($instructions) = @_;
    my %map;
    my @stack;

    for (my $i = 0; $i < scalar(@$instructions); $i++) {
        my $opcode = $instructions->[$i]{opcode};

        if ($opcode == 0x02 || $opcode == 0x03 || $opcode == 0x04) {
            # block/loop/if
            push @stack, { index => $i, opcode => $opcode, else_pc => undef };
        }
        elsif ($opcode == 0x05) {
            # else
            if (@stack) {
                $stack[-1]{else_pc} = $i;
            }
        }
        elsif ($opcode == 0x0B) {
            # end
            if (@stack) {
                my $opener = pop @stack;
                $map{ $opener->{index} } = {
                    end_pc  => $i,
                    else_pc => $opener->{else_pc},
                };
            }
        }
    }

    return \%map;
}

# ============================================================================
# Constant Expression Evaluator
# ============================================================================
#
# WASM constant expressions are tiny programs (i32.const, i64.const,
# f32.const, f64.const, global.get, end) used for global initializers
# and data/element segment offsets.

sub evaluate_const_expr {
    my ($expr, $globals) = @_;
    $globals //= [];

    # $expr is an arrayref of bytes
    my $pos = 0;
    my $result = undef;

    while ($pos < scalar(@$expr)) {
        my $opcode = $expr->[$pos];
        $pos++;

        if ($opcode == 0x41) {
            # i32.const
            my ($val, $consumed) = decode_signed($expr, $pos);
            $pos += $consumed;
            $result = i32($val);
        }
        elsif ($opcode == 0x42) {
            # i64.const
            my ($val, $consumed) = decode_signed($expr, $pos);
            $pos += $consumed;
            $result = i64($val);
        }
        elsif ($opcode == 0x43) {
            # f32.const (4 bytes LE)
            my $bytes = pack('C4', @{$expr}[$pos .. $pos + 3]);
            my $val = unpack('f<', $bytes);
            $pos += 4;
            $result = f32($val);
        }
        elsif ($opcode == 0x44) {
            # f64.const (8 bytes LE)
            my $bytes = pack('C8', @{$expr}[$pos .. $pos + 7]);
            my $val = unpack('d<', $bytes);
            $pos += 8;
            $result = f64($val);
        }
        elsif ($opcode == 0x23) {
            # global.get
            my ($idx, $consumed) = decode_unsigned($expr, $pos);
            $pos += $consumed;
            CodingAdventures::WasmExecution::TrapError->throw(
                "global.get: index $idx out of bounds"
            ) if $idx >= scalar(@$globals);
            $result = $globals->[$idx];
        }
        elsif ($opcode == 0x0B) {
            # end
            CodingAdventures::WasmExecution::TrapError->throw(
                "Constant expression produced no value"
            ) unless defined $result;
            return $result;
        }
        else {
            CodingAdventures::WasmExecution::TrapError->throw(
                sprintf("Illegal opcode 0x%02x in constant expression", $opcode)
            );
        }
    }

    CodingAdventures::WasmExecution::TrapError->throw(
        "Constant expression missing end opcode"
    );
}

# ============================================================================
# i32 Arithmetic Helpers
# ============================================================================
#
# These helpers handle the tricky parts of 32-bit integer arithmetic in
# Perl (which uses 64-bit IVs on 64-bit platforms).

# Wrap a value to signed 32-bit.
sub _wrap_i32 {
    my ($v) = @_;
    return unpack('l', pack('L', $v & 0xFFFFFFFF));
}

# Arithmetic right shift for i32 (sign-extending).
# Perl's >> on negative values is NOT guaranteed to be arithmetic on 64-bit.
sub _i32_shr_s {
    my ($value, $shift) = @_;
    $shift = $shift & 31;  # modulo 32
    if ($value >= 0) {
        return $value >> $shift;
    }
    # For negative values, use floor division to simulate arithmetic shift
    return POSIX::floor($value / (2 ** $shift));
}

# ============================================================================
# WasmExecutionEngine — the core interpreter
# ============================================================================

package CodingAdventures::WasmExecution::Engine;

use POSIX qw(floor);

sub new {
    my ($class, %config) = @_;
    my $vm = CodingAdventures::VirtualMachine->new();

    my $self = bless {
        vm             => $vm,
        memory         => $config{memory},
        tables         => $config{tables} || [],
        globals        => $config{globals} || [],
        global_types   => $config{global_types} || [],
        func_types     => $config{func_types} || [],
        func_bodies    => $config{func_bodies} || [],
        host_functions => $config{host_functions} || [],
        decoded_cache  => {},
    }, $class;

    # Register all WASM instruction handlers on the VM
    $self->_register_all_handlers();

    return $self;
}

# call_function($func_index, \@args) — call a WASM function by index.
# Returns an arrayref of WasmValue result hashrefs.
sub call_function {
    my ($self, $func_index, $args) = @_;
    $args //= [];

    my $func_type = $self->{func_types}[$func_index];
    CodingAdventures::WasmExecution::TrapError->throw(
        "undefined function index $func_index"
    ) unless $func_type;

    # Check argument count
    if (scalar(@$args) != scalar(@{ $func_type->{params} })) {
        CodingAdventures::WasmExecution::TrapError->throw(
            "function $func_index expects " . scalar(@{ $func_type->{params} })
            . " arguments, got " . scalar(@$args)
        );
    }

    # Check if this is a host (imported) function
    my $host_func = $self->{host_functions}[$func_index];
    if ($host_func) {
        return $host_func->($args);
    }

    # Module-defined function
    my $body = $self->{func_bodies}[$func_index];
    CodingAdventures::WasmExecution::TrapError->throw(
        "no body for function $func_index"
    ) unless $body;

    # Decode the function body (cached)
    my $decoded = $self->{decoded_cache}{$func_index};
    if (!$decoded) {
        $decoded = CodingAdventures::WasmExecution::decode_function_body($body);
        $self->{decoded_cache}{$func_index} = $decoded;
    }

    # Build control flow map
    my $cf_map = CodingAdventures::WasmExecution::build_control_flow_map($decoded);

    # Initialize locals: args + zero-initialized declared locals
    my @typed_locals = @$args;
    for my $local_type (@{ $body->{locals} || [] }) {
        push @typed_locals, CodingAdventures::WasmExecution::default_value($local_type);
    }

    # Build execution context
    my $ctx = {
        memory           => $self->{memory},
        tables           => $self->{tables},
        globals          => $self->{globals},
        global_types     => $self->{global_types},
        func_types       => $self->{func_types},
        func_bodies      => $self->{func_bodies},
        host_functions   => $self->{host_functions},
        typed_locals     => \@typed_locals,
        label_stack      => [],
        control_flow_map => $cf_map,
        saved_frames     => [],
        returned         => 0,
        return_values    => [],
    };

    # Build CodeObject
    my $code = CodingAdventures::VirtualMachine::CodeObject->new(
        instructions => $decoded,
        constants    => [],
        names        => [],
    );

    # Reset VM and execute
    $self->{vm}->reset();
    $self->{vm}->execute_with_context($code, $ctx);

    # Collect return values from typed stack
    my $result_count = scalar(@{ $func_type->{results} });
    my @results;
    for (1 .. $result_count) {
        if (scalar(@{ $self->{vm}->typed_stack }) > 0) {
            unshift @results, $self->{vm}->pop_typed();
        }
    }

    return \@results;
}

# ============================================================================
# Instruction Handler Registration
# ============================================================================
#
# Each WASM opcode gets a handler sub that receives ($vm, $instr, $code, $ctx).
# The handler implements the opcode's semantics: popping operands from the
# typed stack, performing the operation, pushing results, advancing PC.

sub _register_all_handlers {
    my ($self) = @_;
    my $vm = $self->{vm};
    my $engine = $self;

    # --- Control flow ---

    # unreachable (0x00): always trap
    $vm->register_context_opcode(0x00, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        CodingAdventures::WasmExecution::TrapError->throw("unreachable executed");
    });

    # nop (0x01): do nothing
    $vm->register_context_opcode(0x01, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        $vm->{pc}++;
    });

    # block (0x02): structured block
    $vm->register_context_opcode(0x02, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $block_type = $instr->{operand};
        my $arity = _block_type_arity($block_type);
        my $cf = $ctx->{control_flow_map}{ $vm->{pc} };
        my $end_pc = $cf ? $cf->{end_pc} : scalar(@{ $code->instructions() }) - 1;

        push @{ $ctx->{label_stack} }, {
            arity        => $arity,
            target_pc    => $end_pc,
            stack_height => scalar(@{ $vm->typed_stack }),
            is_loop      => 0,
        };
        $vm->{pc}++;
    });

    # loop (0x03): structured loop
    $vm->register_context_opcode(0x03, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $block_type = $instr->{operand};
        my $arity = _block_type_arity($block_type);

        push @{ $ctx->{label_stack} }, {
            arity        => 0,  # loop labels have 0 arity for br
            target_pc    => $vm->{pc},  # loops branch back to start
            stack_height => scalar(@{ $vm->typed_stack }),
            is_loop      => 1,
        };
        $vm->{pc}++;
    });

    # if (0x04): conditional block
    $vm->register_context_opcode(0x04, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $block_type = $instr->{operand};
        my $arity = _block_type_arity($block_type);
        my $condition = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $cf = $ctx->{control_flow_map}{ $vm->{pc} };
        my $end_pc = $cf ? $cf->{end_pc} : scalar(@{ $code->instructions() }) - 1;

        push @{ $ctx->{label_stack} }, {
            arity        => $arity,
            target_pc    => $end_pc,
            stack_height => scalar(@{ $vm->typed_stack }),
            is_loop      => 0,
        };

        if ($condition != 0) {
            # Execute then-branch
            $vm->{pc}++;
        } else {
            # Jump to else or end
            if ($cf && defined $cf->{else_pc}) {
                $vm->{pc} = $cf->{else_pc} + 1;
            } else {
                $vm->{pc} = $end_pc;
            }
        }
    });

    # else (0x05): jump to end of if block
    $vm->register_context_opcode(0x05, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        # When we hit else during then-branch execution, jump to end
        my $label = $ctx->{label_stack}[-1];
        $vm->{pc} = $label->{target_pc};
    });

    # end (0x0B): end of block/loop/if or function
    $vm->register_context_opcode(0x0B, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        if (scalar(@{ $ctx->{label_stack} }) > 0) {
            my $label = pop @{ $ctx->{label_stack} };
            # Collect result values
            my @results;
            for (1 .. $label->{arity}) {
                unshift @results, $vm->pop_typed()
                    if scalar(@{ $vm->typed_stack }) > $label->{stack_height};
            }
            # Trim stack to label height
            while (scalar(@{ $vm->typed_stack }) > $label->{stack_height}) {
                $vm->pop_typed();
            }
            # Push results back
            for my $r (@results) {
                $vm->push_typed($r);
            }
            $vm->{pc}++;
        } else {
            # Function end — check if we need to return from a call
            if (scalar(@{ $ctx->{saved_frames} }) > 0) {
                my $frame = pop @{ $ctx->{saved_frames} };
                # Collect return values
                my @ret_vals;
                my $ret_arity = $frame->{return_arity};
                for (1 .. $ret_arity) {
                    unshift @ret_vals, $vm->pop_typed()
                        if scalar(@{ $vm->typed_stack }) > 0;
                }
                # Trim stack to caller height
                while (scalar(@{ $vm->typed_stack }) > $frame->{stack_height}) {
                    $vm->pop_typed();
                }
                # Push return values
                for my $rv (@ret_vals) {
                    $vm->push_typed($rv);
                }
                # Restore caller state
                $ctx->{typed_locals}     = $frame->{locals};
                $ctx->{label_stack}      = $frame->{label_stack};
                $ctx->{control_flow_map} = $frame->{control_flow_map};
                $vm->{pc}                = $frame->{return_pc};
                $vm->{_program}          = $frame->{code};
            } else {
                # Top-level function end — halt
                $vm->{halted} = 1;
            }
        }
    });

    # br (0x0C): unconditional branch
    $vm->register_context_opcode(0x0C, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $label_idx = $instr->{operand};
        _do_branch($vm, $ctx, $label_idx);
    });

    # br_if (0x0D): conditional branch
    $vm->register_context_opcode(0x0D, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $condition = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        if ($condition != 0) {
            my $label_idx = $instr->{operand};
            _do_branch($vm, $ctx, $label_idx);
        } else {
            $vm->{pc}++;
        }
    });

    # br_table (0x0E): indexed branch
    $vm->register_context_opcode(0x0E, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $index = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $op = $instr->{operand};
        my $labels = $op->{labels};
        my $default = $op->{default_label};
        # Use unsigned interpretation for index
        $index = $index & 0xFFFFFFFF;
        my $target = ($index < scalar(@$labels)) ? $labels->[$index] : $default;
        _do_branch($vm, $ctx, $target);
    });

    # return (0x0F): return from function
    $vm->register_context_opcode(0x0F, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        if (scalar(@{ $ctx->{saved_frames} }) > 0) {
            my $frame = pop @{ $ctx->{saved_frames} };
            my @ret_vals;
            my $ret_arity = $frame->{return_arity};
            for (1 .. $ret_arity) {
                unshift @ret_vals, $vm->pop_typed()
                    if scalar(@{ $vm->typed_stack }) > 0;
            }
            while (scalar(@{ $vm->typed_stack }) > $frame->{stack_height}) {
                $vm->pop_typed();
            }
            for my $rv (@ret_vals) {
                $vm->push_typed($rv);
            }
            $ctx->{typed_locals}     = $frame->{locals};
            $ctx->{label_stack}      = $frame->{label_stack};
            $ctx->{control_flow_map} = $frame->{control_flow_map};
            $vm->{pc}                = $frame->{return_pc};
            $vm->{_program}          = $frame->{code};
        } else {
            $vm->{halted} = 1;
        }
    });

    # call (0x10): direct function call
    $vm->register_context_opcode(0x10, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $func_idx = $instr->{operand};
        _do_call($engine, $vm, $code, $ctx, $func_idx);
    });

    # call_indirect (0x11): indirect function call through table
    $vm->register_context_opcode(0x11, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $op = $instr->{operand};
        my $type_idx = $op->{type_idx};
        my $table_idx = $op->{table_idx} || 0;

        my $table = $ctx->{tables}[$table_idx];
        CodingAdventures::WasmExecution::TrapError->throw("no table at index $table_idx")
            unless $table;

        my $elem_idx = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $func_idx = $table->get($elem_idx);
        CodingAdventures::WasmExecution::TrapError->throw(
            "uninitialized table element at index $elem_idx"
        ) unless defined $func_idx;

        # Type check: the function's type must match the expected type
        my $expected_type = $ctx->{func_types}[$type_idx];
        my $actual_type = $ctx->{func_types}[$func_idx];
        CodingAdventures::WasmExecution::TrapError->throw(
            "call_indirect type mismatch"
        ) unless _types_match($expected_type, $actual_type);

        _do_call($engine, $vm, $code, $ctx, $func_idx);
    });

    # --- Parametric instructions ---

    # drop (0x1A): discard top of typed stack
    $vm->register_context_opcode(0x1A, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        $vm->pop_typed();
        $vm->{pc}++;
    });

    # select (0x1B): conditional selection
    $vm->register_context_opcode(0x1B, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $cond = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $val2 = $vm->pop_typed();
        my $val1 = $vm->pop_typed();
        $vm->push_typed($cond != 0 ? $val1 : $val2);
        $vm->{pc}++;
    });

    # --- Variable instructions ---

    # local.get (0x20)
    $vm->register_context_opcode(0x20, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $idx = $instr->{operand};
        $vm->push_typed($ctx->{typed_locals}[$idx]);
        $vm->{pc}++;
    });

    # local.set (0x21)
    $vm->register_context_opcode(0x21, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $idx = $instr->{operand};
        $ctx->{typed_locals}[$idx] = $vm->pop_typed();
        $vm->{pc}++;
    });

    # local.tee (0x22)
    $vm->register_context_opcode(0x22, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $idx = $instr->{operand};
        my $val = $vm->peek_typed();
        $ctx->{typed_locals}[$idx] = $val;
        $vm->{pc}++;
    });

    # global.get (0x23)
    $vm->register_context_opcode(0x23, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $idx = $instr->{operand};
        $vm->push_typed($ctx->{globals}[$idx]);
        $vm->{pc}++;
    });

    # global.set (0x24)
    $vm->register_context_opcode(0x24, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $idx = $instr->{operand};
        $ctx->{globals}[$idx] = $vm->pop_typed();
        $vm->{pc}++;
    });

    # --- Memory instructions ---

    # i32.load (0x28)
    $vm->register_context_opcode(0x28, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $base = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $addr = ($base & 0xFFFFFFFF) + $instr->{operand}{offset};
        $vm->push_typed(CodingAdventures::WasmExecution::i32(
            $ctx->{memory}->load_i32($addr)
        ));
        $vm->{pc}++;
    });

    # i64.load (0x29)
    $vm->register_context_opcode(0x29, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $base = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $addr = ($base & 0xFFFFFFFF) + $instr->{operand}{offset};
        $vm->push_typed(CodingAdventures::WasmExecution::i64(
            $ctx->{memory}->load_i64($addr)
        ));
        $vm->{pc}++;
    });

    # f32.load (0x2A)
    $vm->register_context_opcode(0x2A, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $base = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $addr = ($base & 0xFFFFFFFF) + $instr->{operand}{offset};
        $vm->push_typed(CodingAdventures::WasmExecution::f32(
            $ctx->{memory}->load_f32($addr)
        ));
        $vm->{pc}++;
    });

    # f64.load (0x2B)
    $vm->register_context_opcode(0x2B, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $base = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $addr = ($base & 0xFFFFFFFF) + $instr->{operand}{offset};
        $vm->push_typed(CodingAdventures::WasmExecution::f64(
            $ctx->{memory}->load_f64($addr)
        ));
        $vm->{pc}++;
    });

    # i32.load8_s (0x2C)
    $vm->register_context_opcode(0x2C, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $base = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $addr = ($base & 0xFFFFFFFF) + $instr->{operand}{offset};
        $vm->push_typed(CodingAdventures::WasmExecution::i32(
            $ctx->{memory}->load_i32_8s($addr)
        ));
        $vm->{pc}++;
    });

    # i32.load8_u (0x2D)
    $vm->register_context_opcode(0x2D, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $base = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $addr = ($base & 0xFFFFFFFF) + $instr->{operand}{offset};
        $vm->push_typed(CodingAdventures::WasmExecution::i32(
            $ctx->{memory}->load_i32_8u($addr)
        ));
        $vm->{pc}++;
    });

    # i32.load16_s (0x2E)
    $vm->register_context_opcode(0x2E, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $base = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $addr = ($base & 0xFFFFFFFF) + $instr->{operand}{offset};
        $vm->push_typed(CodingAdventures::WasmExecution::i32(
            $ctx->{memory}->load_i32_16s($addr)
        ));
        $vm->{pc}++;
    });

    # i32.load16_u (0x2F)
    $vm->register_context_opcode(0x2F, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $base = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $addr = ($base & 0xFFFFFFFF) + $instr->{operand}{offset};
        $vm->push_typed(CodingAdventures::WasmExecution::i32(
            $ctx->{memory}->load_i32_16u($addr)
        ));
        $vm->{pc}++;
    });

    # i64.load8_s (0x30)
    $vm->register_context_opcode(0x30, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $base = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $addr = ($base & 0xFFFFFFFF) + $instr->{operand}{offset};
        $vm->push_typed(CodingAdventures::WasmExecution::i64(
            $ctx->{memory}->load_i64_8s($addr)
        ));
        $vm->{pc}++;
    });

    # i64.load8_u (0x31)
    $vm->register_context_opcode(0x31, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $base = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $addr = ($base & 0xFFFFFFFF) + $instr->{operand}{offset};
        $vm->push_typed(CodingAdventures::WasmExecution::i64(
            $ctx->{memory}->load_i64_8u($addr)
        ));
        $vm->{pc}++;
    });

    # i32.store (0x36)
    $vm->register_context_opcode(0x36, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $val  = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $base = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $addr = ($base & 0xFFFFFFFF) + $instr->{operand}{offset};
        $ctx->{memory}->store_i32($addr, $val);
        $vm->{pc}++;
    });

    # i32.store8 (0x3A)
    $vm->register_context_opcode(0x3A, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $val  = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $base = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $addr = ($base & 0xFFFFFFFF) + $instr->{operand}{offset};
        $ctx->{memory}->store_i32_8($addr, $val);
        $vm->{pc}++;
    });

    # i32.store16 (0x3B)
    $vm->register_context_opcode(0x3B, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $val  = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $base = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $addr = ($base & 0xFFFFFFFF) + $instr->{operand}{offset};
        $ctx->{memory}->store_i32_16($addr, $val);
        $vm->{pc}++;
    });

    # memory.size (0x3F)
    $vm->register_context_opcode(0x3F, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $pages = $ctx->{memory} ? $ctx->{memory}->size() : 0;
        $vm->push_typed(CodingAdventures::WasmExecution::i32($pages));
        $vm->{pc}++;
    });

    # memory.grow (0x40)
    $vm->register_context_opcode(0x40, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $delta = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $result = $ctx->{memory} ? $ctx->{memory}->grow($delta) : -1;
        $vm->push_typed(CodingAdventures::WasmExecution::i32($result));
        $vm->{pc}++;
    });

    # --- Numeric: i32 const ---

    # i32.const (0x41)
    $vm->register_context_opcode(0x41, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        $vm->push_typed(CodingAdventures::WasmExecution::i32($instr->{operand}));
        $vm->{pc}++;
    });

    # i64.const (0x42)
    $vm->register_context_opcode(0x42, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        $vm->push_typed(CodingAdventures::WasmExecution::i64($instr->{operand}));
        $vm->{pc}++;
    });

    # f32.const (0x43)
    $vm->register_context_opcode(0x43, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        $vm->push_typed(CodingAdventures::WasmExecution::f32($instr->{operand}));
        $vm->{pc}++;
    });

    # f64.const (0x44)
    $vm->register_context_opcode(0x44, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        $vm->push_typed(CodingAdventures::WasmExecution::f64($instr->{operand}));
        $vm->{pc}++;
    });

    # --- i32 comparison ops ---

    # i32.eqz (0x45)
    $vm->register_context_opcode(0x45, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $a = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i32($a == 0 ? 1 : 0));
        $vm->{pc}++;
    });

    # i32.eq (0x46)
    $vm->register_context_opcode(0x46, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i32($a == $b ? 1 : 0));
        $vm->{pc}++;
    });

    # i32.ne (0x47)
    $vm->register_context_opcode(0x47, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i32($a != $b ? 1 : 0));
        $vm->{pc}++;
    });

    # i32.lt_s (0x48)
    $vm->register_context_opcode(0x48, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i32($a < $b ? 1 : 0));
        $vm->{pc}++;
    });

    # --- i32 arithmetic ops ---

    # i32.add (0x6A)
    $vm->register_context_opcode(0x6A, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i32($a + $b));
        $vm->{pc}++;
    });

    # i32.sub (0x6B)
    $vm->register_context_opcode(0x6B, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i32($a - $b));
        $vm->{pc}++;
    });

    # i32.mul (0x6C)
    $vm->register_context_opcode(0x6C, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i32($a * $b));
        $vm->{pc}++;
    });

    # i32.div_s (0x6D)
    $vm->register_context_opcode(0x6D, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        CodingAdventures::WasmExecution::TrapError->throw("integer divide by zero")
            if $b == 0;
        CodingAdventures::WasmExecution::TrapError->throw("integer overflow")
            if $a == -2147483648 && $b == -1;
        $vm->push_typed(CodingAdventures::WasmExecution::i32(int($a / $b)));
        $vm->{pc}++;
    });

    # i32.and (0x71)
    $vm->register_context_opcode(0x71, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i32($a & $b));
        $vm->{pc}++;
    });

    # i32.or (0x72)
    $vm->register_context_opcode(0x72, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i32($a | $b));
        $vm->{pc}++;
    });

    # i32.xor (0x73)
    $vm->register_context_opcode(0x73, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i32($a ^ $b));
        $vm->{pc}++;
    });

    # i32.shl (0x74)
    $vm->register_context_opcode(0x74, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i32($a << ($b & 31)));
        $vm->{pc}++;
    });

    # i32.shr_s (0x75) — arithmetic right shift
    $vm->register_context_opcode(0x75, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i32(
            CodingAdventures::WasmExecution::_i32_shr_s($a, $b)
        ));
        $vm->{pc}++;
    });

    # --- i64 arithmetic ---

    # i64.add (0x7C)
    $vm->register_context_opcode(0x7C, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_i64($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_i64($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i64($a + $b));
        $vm->{pc}++;
    });

    # i64.sub (0x7D)
    $vm->register_context_opcode(0x7D, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_i64($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_i64($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i64($a - $b));
        $vm->{pc}++;
    });

    # i64.mul (0x7E)
    $vm->register_context_opcode(0x7E, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_i64($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_i64($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i64($a * $b));
        $vm->{pc}++;
    });

    # --- f32 arithmetic ---

    # f32.add (0x92)
    $vm->register_context_opcode(0x92, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_f32($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_f32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::f32($a + $b));
        $vm->{pc}++;
    });

    # f32.sub (0x93)
    $vm->register_context_opcode(0x93, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_f32($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_f32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::f32($a - $b));
        $vm->{pc}++;
    });

    # f32.mul (0x94)
    $vm->register_context_opcode(0x94, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_f32($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_f32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::f32($a * $b));
        $vm->{pc}++;
    });

    # --- f64 arithmetic ---

    # f64.add (0xA0)
    $vm->register_context_opcode(0xA0, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_f64($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_f64($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::f64($a + $b));
        $vm->{pc}++;
    });

    # f64.sub (0xA1)
    $vm->register_context_opcode(0xA1, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_f64($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_f64($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::f64($a - $b));
        $vm->{pc}++;
    });

    # f64.mul (0xA2)
    $vm->register_context_opcode(0xA2, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $b = CodingAdventures::WasmExecution::_as_f64($vm->pop_typed());
        my $a = CodingAdventures::WasmExecution::_as_f64($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::f64($a * $b));
        $vm->{pc}++;
    });

    # --- Conversion instructions ---

    # i32.wrap_i64 (0xA7)
    $vm->register_context_opcode(0xA7, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $val = CodingAdventures::WasmExecution::_as_i64($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i32($val & 0xFFFFFFFF));
        $vm->{pc}++;
    });

    # i32.trunc_f32_s (0xA8)
    $vm->register_context_opcode(0xA8, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $val = CodingAdventures::WasmExecution::_as_f32($vm->pop_typed());
        CodingAdventures::WasmExecution::TrapError->throw("invalid conversion to integer")
            if $val != $val;  # NaN check
        $vm->push_typed(CodingAdventures::WasmExecution::i32(int($val)));
        $vm->{pc}++;
    });

    # i64.extend_i32_s (0xAC)
    $vm->register_context_opcode(0xAC, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $val = CodingAdventures::WasmExecution::_as_i32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::i64($val));  # sign-extends naturally
        $vm->{pc}++;
    });

    # f32.demote_f64 (0xB6)
    $vm->register_context_opcode(0xB6, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $val = CodingAdventures::WasmExecution::_as_f64($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::f32($val));
        $vm->{pc}++;
    });

    # f64.promote_f32 (0xBB)
    $vm->register_context_opcode(0xBB, sub {
        my ($vm, $instr, $code, $ctx) = @_;
        my $val = CodingAdventures::WasmExecution::_as_f32($vm->pop_typed());
        $vm->push_typed(CodingAdventures::WasmExecution::f64($val));
        $vm->{pc}++;
    });
}

# ============================================================================
# Branch helper
# ============================================================================

sub _do_branch {
    my ($vm, $ctx, $label_idx) = @_;

    CodingAdventures::WasmExecution::TrapError->throw(
        "branch depth $label_idx exceeds label stack size "
        . scalar(@{ $ctx->{label_stack} })
    ) if $label_idx >= scalar(@{ $ctx->{label_stack} });

    my $label = $ctx->{label_stack}[ -1 - $label_idx ];

    # Collect arity values from the stack
    my @results;
    for (1 .. $label->{arity}) {
        unshift @results, $vm->pop_typed()
            if scalar(@{ $vm->typed_stack }) > $label->{stack_height};
    }

    # Unwind stack to label height
    while (scalar(@{ $vm->typed_stack }) > $label->{stack_height}) {
        $vm->pop_typed();
    }

    # Push results back
    for my $r (@results) {
        $vm->push_typed($r);
    }

    # Pop labels up to and including the target
    for (0 .. $label_idx) {
        pop @{ $ctx->{label_stack} } if @{ $ctx->{label_stack} };
    }

    if ($label->{is_loop}) {
        # Loop: branch to start, re-push label
        push @{ $ctx->{label_stack} }, $label;
        $vm->{pc} = $label->{target_pc} + 1;  # skip loop opcode
    } else {
        # Block/if: branch to end
        $vm->{pc} = $label->{target_pc} + 1;
    }
}

# ============================================================================
# Call helper
# ============================================================================

sub _do_call {
    my ($engine, $vm, $code, $ctx, $func_idx) = @_;

    my $func_type = $ctx->{func_types}[$func_idx];
    CodingAdventures::WasmExecution::TrapError->throw(
        "undefined function index $func_idx"
    ) unless $func_type;

    # Pop arguments from the typed stack
    my @args;
    for (1 .. scalar(@{ $func_type->{params} })) {
        unshift @args, $vm->pop_typed();
    }

    # Host function?
    my $host_func = $ctx->{host_functions}[$func_idx];
    if ($host_func) {
        my $results = $host_func->(\@args);
        for my $r (@{ $results || [] }) {
            $vm->push_typed($r);
        }
        $vm->{pc}++;
        return;
    }

    # Module-defined function: save caller state and set up callee
    my $body = $ctx->{func_bodies}[$func_idx];
    CodingAdventures::WasmExecution::TrapError->throw(
        "no body for function $func_idx"
    ) unless $body;

    # Decode callee
    my $decoded = $engine->{decoded_cache}{$func_idx};
    if (!$decoded) {
        $decoded = CodingAdventures::WasmExecution::decode_function_body($body);
        $engine->{decoded_cache}{$func_idx} = $decoded;
    }

    my $callee_cf_map = CodingAdventures::WasmExecution::build_control_flow_map($decoded);

    # Initialize callee locals
    my @callee_locals = @args;
    for my $lt (@{ $body->{locals} || [] }) {
        push @callee_locals, CodingAdventures::WasmExecution::default_value($lt);
    }

    # Save caller state
    my $return_arity = scalar(@{ $func_type->{results} });
    push @{ $ctx->{saved_frames} }, {
        locals           => $ctx->{typed_locals},
        label_stack      => $ctx->{label_stack},
        control_flow_map => $ctx->{control_flow_map},
        stack_height     => scalar(@{ $vm->typed_stack }),
        return_pc        => $vm->{pc} + 1,
        return_arity     => $return_arity,
        code             => $vm->{_program},
    };

    # Set up callee state
    $ctx->{typed_locals}     = \@callee_locals;
    $ctx->{label_stack}      = [];
    $ctx->{control_flow_map} = $callee_cf_map;

    my $callee_code = CodingAdventures::VirtualMachine::CodeObject->new(
        instructions => $decoded,
        constants    => [],
        names        => [],
    );
    $vm->{_program} = $callee_code;
    $vm->{pc} = 0;
}

# ============================================================================
# Helper: block type arity
# ============================================================================

sub _block_type_arity {
    my ($block_type) = @_;
    return 0 if !defined($block_type) || $block_type == 0x40;  # empty
    return 1;  # single value type
}

# ============================================================================
# Helper: function type comparison
# ============================================================================

sub _types_match {
    my ($a, $b) = @_;
    return 0 unless $a && $b;
    return 0 unless scalar(@{ $a->{params} }) == scalar(@{ $b->{params} });
    return 0 unless scalar(@{ $a->{results} }) == scalar(@{ $b->{results} });
    for my $i (0 .. $#{ $a->{params} }) {
        return 0 unless $a->{params}[$i] == $b->{params}[$i];
    }
    for my $i (0 .. $#{ $a->{results} }) {
        return 0 unless $a->{results}[$i] == $b->{results}[$i];
    }
    return 1;
}

1;

__END__

=head1 NAME

CodingAdventures::WasmExecution - WebAssembly 1.0 execution engine

=head1 SYNOPSIS

    use CodingAdventures::WasmExecution qw(i32 i64 f32 f64 evaluate_const_expr);

    # Create execution engine
    my $engine = CodingAdventures::WasmExecution::Engine->new(
        memory         => $memory,
        tables         => [$table],
        globals        => [i32(0)],
        global_types   => [{ value_type => 0x7F, mutable => 1 }],
        func_types     => [{ params => [0x7F], results => [0x7F] }],
        func_bodies    => [$body],
        host_functions => [undef],
    );

    # Call a function
    my $results = $engine->call_function(0, [i32(5)]);

=head1 DESCRIPTION

Complete WebAssembly 1.0 execution engine including typed values,
linear memory, tables, bytecode decoder, constant expression evaluator,
and all instruction handlers for control flow, variable access, memory
operations, and numeric operations.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
