package CodingAdventures::WasmSimulator;

# ============================================================================
# CodingAdventures::WasmSimulator — WebAssembly interpreter / simulator
# ============================================================================
#
# This module executes WebAssembly modules. It accepts a parsed module
# (produced by CodingAdventures::WasmModuleParser) and runs its bytecode
# instructions on a software-emulated Wasm virtual machine.
#
# ## The WebAssembly Execution Model
#
# WebAssembly is a STACK MACHINE. Unlike register-based machines (x86, ARM)
# where operations read named registers, a stack machine stores all working
# values on an implicit VALUE STACK.
#
# Every instruction either PUSHES values onto the stack, POPS values off,
# or does both. For example, executing `i32.add`:
#
#   Before:              After:
#   ┌───────────┐        ┌───────────┐
#   │    7      │ ← top  │    10     │ ← top   (7 + 3 = 10)
#   ├───────────┤        └───────────┘
#   │    3      │
#   └───────────┘
#
# This is simpler to validate (the type checker works statically) and
# straightforward to interpret (just a loop over instructions).
#
# ## Linear Memory
#
# Every Wasm module has access to a single array of bytes called LINEAR MEMORY.
# This models the flat address space used by programs written in C, C++, Rust,
# etc. Key properties:
#
#   - Addressed by byte offset (0-based unsigned integers)
#   - Organized into 64 KiB pages (65536 bytes per page)
#   - Can grow at runtime via `memory.grow`; never shrinks
#   - Accesses outside allocated bounds trap (cause a runtime error)
#
# In this simulator, linear memory is a Perl array of byte values indexed 0..(N-1).
#
# ## Functions and Activation Frames
#
# When a function is called, an ACTIVATION FRAME is created:
#
#   Frame = {
#     func_idx  — which function we're executing
#     locals    — array of local variable values (includes parameters)
#   }
#
# The value stack is shared across frames (Wasm callers' values stay below
# callee's values on the stack).
#
# ## Structured Control Flow and Labels
#
# WebAssembly does NOT have arbitrary goto. All branches are STRUCTURED:
#
#   block: branch target is the END of the block (forward jump, like break)
#   loop:  branch target is the START of the loop (backward jump, like continue)
#   if:    branch target is the END of the if/else (forward jump)
#
# A LABEL STACK tracks active blocks/loops/ifs. `br N` pops N+1 labels and
# jumps to the Nth label's target.
#
# ## Module Structure
#
# This module exports one class: CodingAdventures::WasmSimulator::Instance
# (accessible as WasmSimulator::Instance or via the new() constructor on the
# top-level package for convenience).
#
# ## Usage
#
#   use CodingAdventures::WasmModuleParser qw(parse);
#   use CodingAdventures::WasmSimulator;
#
#   my $bytes = do { local $/; open my $f, '<:raw', 'add.wasm'; <$f> };
#   my $mod   = parse($bytes);
#   my $inst  = CodingAdventures::WasmSimulator->new($mod);
#
#   my @results = $inst->call("add", 3, 4);
#   print $results[0];  # 7
#
# ============================================================================

use strict;
use warnings;
use Carp qw(croak confess);

use CodingAdventures::WasmLeb128 qw(decode_unsigned decode_signed);

our $VERSION = '0.01';

# ============================================================================
# Constants
# ============================================================================

# PAGE_SIZE: WebAssembly linear memory pages are exactly 64 KiB each.
# memory.size returns the number of pages; memory.grow adds pages.
use constant PAGE_SIZE => 65536;

# Maximum pages for memory.grow (prevents runaway allocation in tests)
use constant MAX_PAGES => 64;

# ============================================================================
# to_i32($n) — Wrap a number to signed 32-bit range
# ============================================================================
#
# WebAssembly i32 arithmetic is modular (wrapping). Perl uses native integers
# or doubles, so we must simulate 32-bit wrap-around explicitly.
#
# Two's complement signed 32-bit range: -2147483648 to +2147483647
# We wrap by masking to 32 bits then sign-extending if bit 31 is set.
#
# Examples:
#   to_i32(2147483648)  →  -2147483648  (INT_MIN)
#   to_i32(4294967295)  →  -1
#   to_i32(-1)          →  -1           (already in range)

sub to_i32 {
    my ($n) = @_;
    # Mask to unsigned 32-bit range
    my $u = $n % 4294967296;
    $u += 4294967296 if $u < 0;
    # Sign-extend: if bit 31 is set, the value is negative
    $u -= 4294967296 if $u >= 2147483648;
    return $u;
}

# to_u32($n) — Interpret as unsigned 32-bit (for shift amounts, unsigned ops)
sub to_u32 {
    my ($n) = @_;
    my $u = $n % 4294967296;
    $u += 4294967296 if $u < 0;
    return $u;
}

# bool_to_i32($cond) — Convert Perl truth value to Wasm 0/1
sub bool_to_i32 {
    return $_[0] ? 1 : 0;
}

# ============================================================================
# Bitwise helpers (portable across Perl versions)
# ============================================================================
#
# Perl 5.26+ supports native 64-bit integers; bitwise ops work on integers.
# We use Perl's built-in &, |, ^, <<, >> which work on integers natively.
# We do need to mask to 32 bits afterward since Perl integers are 64-bit.

sub bit_and { return to_i32(to_u32($_[0]) & to_u32($_[1])) }
sub bit_or  { return to_i32(to_u32($_[0]) | to_u32($_[1])) }
sub bit_xor { return to_i32(to_u32($_[0]) ^ to_u32($_[1])) }

sub bit_shl {
    my ($a, $b) = @_;
    $b = to_u32($b) % 32;
    return to_i32(to_u32($a) << $b);
}

sub bit_shr_s {
    # Arithmetic right shift: fill vacated bits with sign bit
    my ($a, $b) = @_;
    $b = to_u32($b) % 32;
    my $signed = to_i32($a);
    # In Perl, >> on a signed integer is arithmetic (implementation-defined, but
    # on all common platforms it's arithmetic for negative values).
    # We explicitly convert to signed first.
    return to_i32($signed >> $b);
}

sub bit_shr_u {
    # Logical right shift: fill vacated bits with zeros
    my ($a, $b) = @_;
    $b = to_u32($b) % 32;
    return to_i32(to_u32($a) >> $b);
}

# ============================================================================
# Memory helpers
# ============================================================================

# make_memory($num_pages) — allocate a fresh zero-initialized linear memory
sub make_memory {
    my ($num_pages) = @_;
    $num_pages //= 0;
    return {
        bytes      => [],      # arrayref, indexed 0..(size_pages*PAGE_SIZE-1)
        size_pages => $num_pages,
    };
}

# memory_load($mem, $addr, $n_bytes) — read n_bytes from linear memory
# Returns an arrayref of byte values.
# Dies (traps) if the access is out of bounds.
sub memory_load {
    my ($mem, $addr, $n_bytes) = @_;
    my $limit = $mem->{size_pages} * PAGE_SIZE;
    if ($addr < 0 || $addr + $n_bytes > $limit) {
        croak sprintf(
            "WasmSimulator: memory access out of bounds: addr=%d, len=%d, memory_size=%d",
            $addr, $n_bytes, $limit
        );
    }
    my @result;
    for my $i (0 .. $n_bytes - 1) {
        push @result, ($mem->{bytes}[$addr + $i] // 0);
    }
    return \@result;
}

# memory_store($mem, $addr, $byte_arrayref) — write bytes to linear memory
# Dies (traps) if the write would go out of bounds.
sub memory_store {
    my ($mem, $addr, $bytes) = @_;
    my $limit = $mem->{size_pages} * PAGE_SIZE;
    my $n = scalar @$bytes;
    if ($addr < 0 || $addr + $n > $limit) {
        croak sprintf(
            "WasmSimulator: memory write out of bounds: addr=%d, len=%d, memory_size=%d",
            $addr, $n, $limit
        );
    }
    for my $i (0 .. $n - 1) {
        $mem->{bytes}[$addr + $i] = $bytes->[$i];
    }
}

# read_i32_le($mem, $addr) — read 4 bytes little-endian as signed i32
sub read_i32_le {
    my ($mem, $addr) = @_;
    my $bs = memory_load($mem, $addr, 4);
    my $u = $bs->[0]
          + $bs->[1] * 256
          + $bs->[2] * 65536
          + $bs->[3] * 16777216;
    return to_i32($u);
}

# write_i32_le($mem, $addr, $val) — write signed i32 as 4 bytes little-endian
sub write_i32_le {
    my ($mem, $addr, $val) = @_;
    my $u = to_u32($val);
    memory_store($mem, $addr, [
        $u % 256,
        int($u / 256) % 256,
        int($u / 65536) % 256,
        int($u / 16777216) % 256,
    ]);
}

# ============================================================================
# LEB128 reading helpers
# ============================================================================
#
# These wrap CodingAdventures::WasmLeb128 to return (value, new_pos) where
# new_pos is the byte index AFTER the consumed LEB128 bytes. The bytecode
# arrays are 0-indexed Perl arrays.

sub read_leb_u {
    my ($bytes, $pos) = @_;
    my ($val, $count) = decode_unsigned($bytes, $pos);
    return ($val, $pos + $count);
}

sub read_leb_s {
    my ($bytes, $pos) = @_;
    my ($val, $count) = decode_signed($bytes, $pos);
    return ($val, $pos + $count);
}

# ============================================================================
# eval_init_expr(\@bytes, \@globals) — evaluate a global initializer
# ============================================================================
#
# Global initializers are "constant expressions": a restricted subset of
# instructions that can only produce one constant value. Valid forms in MVP:
#
#   i32.const n; end
#   i64.const n; end
#   global.get i; end  (where i is an already-initialized import global)

sub eval_init_expr {
    my ($bytes, $globals) = @_;
    my $pos    = 0;
    my $opcode = $bytes->[$pos++];

    if ($opcode == 0x41) {
        # i32.const <signed LEB128>
        my ($val) = decode_signed($bytes, $pos);
        return to_i32($val);
    } elsif ($opcode == 0x42) {
        # i64.const <signed LEB128> — treat as Perl integer
        my ($val) = decode_signed($bytes, $pos);
        return $val;
    } elsif ($opcode == 0x23) {
        # global.get <unsigned LEB128>
        my ($idx) = decode_unsigned($bytes, $pos);
        if ($globals && $globals->[$idx]) {
            return $globals->[$idx]{value};
        }
        croak "WasmSimulator: global.get in init_expr: global $idx not found";
    } else {
        croak sprintf("WasmSimulator: unsupported init_expr opcode 0x%02x", $opcode);
    }
}

# ============================================================================
# scan_for_end(\@bytecode, $start_pos) — find matching `end` in bytecode
# ============================================================================
#
# Scan forward from $start_pos, tracking nesting depth of block/loop/if.
# Returns ($end_pos, $else_pos) where $else_pos may be undef.
# This lets block/loop/if instructions know where to branch to without
# executing instructions in between.
#
# This is necessary because block entry happens BEFORE execution of the body,
# so we need to know the end position up-front for br targets.

sub scan_for_end {
    my ($bytecode, $start_pos) = @_;
    my $depth    = 0;
    my $else_pos = undef;
    my $i        = $start_pos;

    while ($i <= $#$bytecode) {
        my $op = $bytecode->[$i];

        if ($op == 0x02 || $op == 0x03 || $op == 0x04) {
            # block, loop, if: increase nesting depth; skip blocktype
            $depth++;
            $i += 2;
        } elsif ($op == 0x05) {
            # else
            $else_pos = $i if $depth == 0;
            $i++;
        } elsif ($op == 0x0b) {
            # end
            if ($depth == 0) {
                return ($i, $else_pos);
            }
            $depth--;
            $i++;
        } elsif ($op == 0x0c || $op == 0x0d) {
            # br, br_if: skip label index LEB128
            $i++;
            my (undef, $cnt) = decode_unsigned($bytecode, $i);
            $i += $cnt;
        } elsif ($op == 0x0e) {
            # br_table: skip vec of labels + default
            $i++;
            my ($n, $cnt) = decode_unsigned($bytecode, $i);
            $i += $cnt;
            for (1 .. $n + 1) {
                my (undef, $lc) = decode_unsigned($bytecode, $i);
                $i += $lc;
            }
        } elsif ($op == 0x10) {
            # call: skip func_idx LEB128
            $i++;
            my (undef, $cnt) = decode_unsigned($bytecode, $i);
            $i += $cnt;
        } elsif ($op == 0x11) {
            # call_indirect: skip type_idx + table_idx
            $i++;
            my (undef, $c1) = decode_unsigned($bytecode, $i);
            $i += $c1;
            my (undef, $c2) = decode_unsigned($bytecode, $i);
            $i += $c2;
        } elsif ($op == 0x20 || $op == 0x21 || $op == 0x22
              || $op == 0x23 || $op == 0x24) {
            # local/global get/set/tee: skip index LEB128
            $i++;
            my (undef, $cnt) = decode_unsigned($bytecode, $i);
            $i += $cnt;
        } elsif (($op >= 0x28 && $op <= 0x3e)) {
            # memory load/store: skip align + offset (two LEB128s)
            $i++;
            my (undef, $c1) = decode_unsigned($bytecode, $i);
            $i += $c1;
            my (undef, $c2) = decode_unsigned($bytecode, $i);
            $i += $c2;
        } elsif ($op == 0x3f || $op == 0x40) {
            # memory.size, memory.grow: skip reserved byte
            $i += 2;
        } elsif ($op == 0x41 || $op == 0x42) {
            # i32.const / i64.const: skip signed LEB128
            $i++;
            my (undef, $cnt) = decode_signed($bytecode, $i);
            $i += $cnt;
        } elsif ($op == 0x43) {
            # f32.const: skip 4 bytes
            $i += 5;
        } elsif ($op == 0x44) {
            # f64.const: skip 8 bytes
            $i += 9;
        } else {
            # All other opcodes: no immediates
            $i++;
        }
    }
    croak "WasmSimulator: scan_for_end: could not find matching end";
}

# ============================================================================
# execute_expr($instance, $func_idx, \@bytecode, \@locals) → \@results
# ============================================================================
#
# The heart of the simulator. Executes a sequence of Wasm bytecode instructions
# from a function body and returns the value stack at the end.
#
# Parameters:
#   $instance  — the Instance object (provides access to globals, memory, module)
#   $func_idx  — 0-based index of the function being executed
#   $bytecode  — arrayref of byte values (0-based indexing)
#   $locals    — arrayref of local variable values (0-based, includes params)
#
# Returns an arrayref containing the final value stack contents.
#
# The instruction loop runs until `return` or falling off the end of bytecode.
# Structured control flow (block/loop/if) is handled via a LABEL STACK that
# tracks open regions and their branch targets.

sub execute_expr {
    my ($instance, $func_idx, $bytecode, $locals) = @_;

    # The value stack: arrayref used as a LIFO stack.
    # Push: push @$stack, $val;
    # Pop:  pop @$stack
    my $stack  = [];

    # The label stack: arrayref of hashrefs.
    # Each entry: { kind => "block"|"loop"|"if", end_pos => N, start_pos => N }
    my $labels = [];

    # Instruction pointer (0-based index into $bytecode)
    my $pc = 0;

    # -----------------------------------------------------------------------
    # Helper closures for stack operations
    # -----------------------------------------------------------------------
    my $push = sub { push @$stack, $_[0] };
    my $pop  = sub {
        croak "WasmSimulator: value stack underflow" unless @$stack;
        return pop @$stack;
    };
    my $peek = sub {
        croak "WasmSimulator: peek on empty stack" unless @$stack;
        return $stack->[-1];
    };

    # -----------------------------------------------------------------------
    # do_branch($depth) — perform a branch to label at depth `$depth`
    # Returns the new $pc to jump to, or undef to signal function return.
    # -----------------------------------------------------------------------
    my $do_branch = sub {
        my ($depth) = @_;
        my $n          = scalar @$labels;
        my $target_idx = $n - 1 - $depth;

        if ($target_idx < 0) {
            # Branch target is outside all blocks → function return
            return undef;
        }

        my $label = $labels->[$target_idx];

        # Remove labels from target_idx+1 to n-1 (we're leaving those blocks)
        splice @$labels, $target_idx + 1;

        # For a loop, branch goes BACK to the start (repeat the loop)
        # For block/if, branch goes FORWARD to after the end
        if ($label->{kind} eq 'loop') {
            return $label->{start_pos};
        } else {
            return $label->{end_pos} + 1;  # +1 to skip past the `end` byte
        }
    };

    # -----------------------------------------------------------------------
    # Main instruction dispatch loop
    # -----------------------------------------------------------------------
    while ($pc <= $#$bytecode) {
        my $opcode = $bytecode->[$pc++];

        # ---------------------------------------------------------------
        # CONTROL FLOW INSTRUCTIONS
        # ---------------------------------------------------------------

        if ($opcode == 0x00) {
            # unreachable: trap immediately
            croak "WasmSimulator: unreachable instruction executed";
        }

        elsif ($opcode == 0x01) {
            # nop: no operation — do nothing
        }

        elsif ($opcode == 0x02) {
            # block <blocktype>
            # Begins a "block" region. The label's end_pos is the matching `end`.
            my $blocktype = $bytecode->[$pc++];
            my ($end_pos, $else_pos) = scan_for_end($bytecode, $pc);
            push @$labels, {
                kind      => 'block',
                end_pos   => $end_pos,
                start_pos => $pc,
            };
        }

        elsif ($opcode == 0x03) {
            # loop <blocktype>
            # Like block, but br to this label goes BACK to start_pos.
            my $blocktype = $bytecode->[$pc++];
            my ($end_pos) = scan_for_end($bytecode, $pc);
            push @$labels, {
                kind      => 'loop',
                end_pos   => $end_pos,
                start_pos => $pc,  # restart point for `br 0` inside the loop
            };
        }

        elsif ($opcode == 0x04) {
            # if <blocktype>
            # Pop condition. Execute then-arm if nonzero; jump to else/end if zero.
            my $blocktype = $bytecode->[$pc++];
            my ($end_pos, $else_pos) = scan_for_end($bytecode, $pc);
            my $cond = $pop->();
            if ($cond != 0) {
                # Execute then-arm
                push @$labels, {
                    kind      => 'if',
                    end_pos   => $end_pos,
                    else_pos  => $else_pos,
                    start_pos => $pc,
                };
                # $pc is already inside the then-arm; continue executing
            } else {
                # Skip to else or end
                if (defined $else_pos) {
                    $pc = $else_pos + 1;  # skip past the `else` byte
                    push @$labels, {
                        kind      => 'if',
                        end_pos   => $end_pos,
                        else_pos  => undef,
                        start_pos => $pc,
                    };
                } else {
                    $pc = $end_pos + 1;  # jump past the `end`
                }
            }
        }

        elsif ($opcode == 0x05) {
            # else: reached from then-arm falling through
            # Jump to the end of the if
            my $label = $labels->[-1];
            if ($label && $label->{kind} eq 'if') {
                $pc = $label->{end_pos} + 1;
                pop @$labels;
            } else {
                croak "WasmSimulator: else without matching if";
            }
        }

        elsif ($opcode == 0x0b) {
            # end: close the current block/loop/if
            if (@$labels) {
                pop @$labels;
            } else {
                # Closes the function body — done!
                last;
            }
        }

        elsif ($opcode == 0x0c) {
            # br <label_depth>: unconditional branch
            my ($depth, $new_pc) = read_leb_u($bytecode, $pc);
            $pc = $new_pc;
            my $jump_pc = $do_branch->($depth);
            if (!defined $jump_pc) {
                last;  # function return
            }
            $pc = $jump_pc;
        }

        elsif ($opcode == 0x0d) {
            # br_if <label_depth>: conditional branch
            my ($depth, $new_pc) = read_leb_u($bytecode, $pc);
            $pc = $new_pc;
            my $cond = $pop->();
            if ($cond != 0) {
                my $jump_pc = $do_branch->($depth);
                if (!defined $jump_pc) {
                    last;  # function return
                }
                $pc = $jump_pc;
            }
            # If condition is zero: fall through
        }

        elsif ($opcode == 0x0f) {
            # return: explicit function return
            last;
        }

        elsif ($opcode == 0x10) {
            # call <func_idx>: direct function call
            my ($callee_idx, $new_pc) = read_leb_u($bytecode, $pc);
            $pc = $new_pc;

            # Look up the function's type to know how many args to pop
            my $mod       = $instance->{module};
            my $type_idx  = $mod->{functions}[$callee_idx];
            croak "WasmSimulator: call: function $callee_idx out of range"
                unless defined $type_idx;

            my $func_type = $mod->{types}[$type_idx];
            my $n_params  = scalar @{ $func_type->{params} };

            # Pop arguments from the stack (reverse order → restore to array)
            my @args;
            for my $i (reverse 0 .. $n_params - 1) {
                $args[$i] = $pop->();
            }

            # Execute callee
            my @results = $instance->call_by_index($callee_idx, @args);

            # Push results onto our stack
            $push->($_) for @results;
        }

        # ---------------------------------------------------------------
        # PARAMETRIC INSTRUCTIONS
        # ---------------------------------------------------------------

        elsif ($opcode == 0x1a) {
            # drop: discard top of stack
            $pop->();
        }

        elsif ($opcode == 0x1b) {
            # select: pop condition and two values; push one based on condition
            # Stack: [..., val1, val2, cond] → push val1 if cond!=0, else val2
            my $cond = $pop->();
            my $val2 = $pop->();
            my $val1 = $pop->();
            $push->($cond != 0 ? $val1 : $val2);
        }

        # ---------------------------------------------------------------
        # VARIABLE INSTRUCTIONS
        # ---------------------------------------------------------------

        elsif ($opcode == 0x20) {
            # local.get <local_idx>: push local variable value
            my ($idx, $new_pc) = read_leb_u($bytecode, $pc);
            $pc = $new_pc;
            my $val = $locals->[$idx] // 0;
            $push->($val);
        }

        elsif ($opcode == 0x21) {
            # local.set <local_idx>: pop and store in local variable
            my ($idx, $new_pc) = read_leb_u($bytecode, $pc);
            $pc = $new_pc;
            $locals->[$idx] = $pop->();
        }

        elsif ($opcode == 0x22) {
            # local.tee <local_idx>: like local.set but leaves value on stack
            my ($idx, $new_pc) = read_leb_u($bytecode, $pc);
            $pc = $new_pc;
            $locals->[$idx] = $peek->();
            # Value stays on the stack (NOT popped)
        }

        elsif ($opcode == 0x23) {
            # global.get <global_idx>: push global variable value
            my ($idx, $new_pc) = read_leb_u($bytecode, $pc);
            $pc = $new_pc;
            my $glob = $instance->{globals}[$idx];
            croak "WasmSimulator: global.get: index $idx out of range"
                unless defined $glob;
            $push->($glob->{value});
        }

        elsif ($opcode == 0x24) {
            # global.set <global_idx>: pop and store in mutable global
            my ($idx, $new_pc) = read_leb_u($bytecode, $pc);
            $pc = $new_pc;
            my $glob = $instance->{globals}[$idx];
            croak "WasmSimulator: global.set: index $idx out of range"
                unless defined $glob;
            croak "WasmSimulator: global.set: global $idx is immutable"
                unless $glob->{mutable};
            $glob->{value} = $pop->();
        }

        # ---------------------------------------------------------------
        # MEMORY INSTRUCTIONS
        # ---------------------------------------------------------------
        #
        # Memory instructions carry a "memory argument": alignment hint (ignored)
        # and static byte offset (added to the runtime address).

        elsif ($opcode == 0x28) {
            # i32.load <align> <offset>
            my ($align,  $p1) = read_leb_u($bytecode, $pc);
            my ($offset, $p2) = read_leb_u($bytecode, $p1);
            $pc = $p2;
            my $addr = $pop->();
            $push->(read_i32_le($instance->{memory}, $addr + $offset));
        }

        elsif ($opcode == 0x36) {
            # i32.store <align> <offset>
            my ($align,  $p1) = read_leb_u($bytecode, $pc);
            my ($offset, $p2) = read_leb_u($bytecode, $p1);
            $pc = $p2;
            my $val  = $pop->();
            my $addr = $pop->();
            write_i32_le($instance->{memory}, $addr + $offset, $val);
        }

        elsif ($opcode == 0x3f) {
            # memory.size: push current page count
            $pc++;  # skip reserved byte
            $push->($instance->{memory}{size_pages});
        }

        elsif ($opcode == 0x40) {
            # memory.grow: pop delta pages, push old size (or -1 on failure)
            $pc++;  # skip reserved byte
            my $delta    = $pop->();
            my $old_size = $instance->{memory}{size_pages};
            if ($old_size + $delta <= MAX_PAGES) {
                $instance->{memory}{size_pages} = $old_size + $delta;
                $push->($old_size);
            } else {
                $push->(to_i32(-1));
            }
        }

        # ---------------------------------------------------------------
        # NUMERIC — CONSTANTS
        # ---------------------------------------------------------------

        elsif ($opcode == 0x41) {
            # i32.const <signed LEB128>
            my ($val, $new_pc) = read_leb_s($bytecode, $pc);
            $pc = $new_pc;
            $push->(to_i32($val));
        }

        elsif ($opcode == 0x42) {
            # i64.const <signed LEB128>: treat as Perl integer
            my ($val, $new_pc) = read_leb_s($bytecode, $pc);
            $pc = $new_pc;
            $push->($val);
        }

        # ---------------------------------------------------------------
        # NUMERIC — i32 COMPARISONS
        # ---------------------------------------------------------------
        #
        # All comparison instructions pop two i32 values and push 1 (true)
        # or 0 (false) as an i32.

        elsif ($opcode == 0x45) {
            # i32.eqz: 1 if top is 0
            $push->(bool_to_i32($pop->() == 0));
        }

        elsif ($opcode == 0x46) {
            # i32.eq
            my $b = $pop->(); my $a = $pop->();
            $push->(bool_to_i32($a == $b));
        }

        elsif ($opcode == 0x47) {
            # i32.ne
            my $b = $pop->(); my $a = $pop->();
            $push->(bool_to_i32($a != $b));
        }

        elsif ($opcode == 0x48) {
            # i32.lt_s (signed)
            my $b = $pop->(); my $a = $pop->();
            $push->(bool_to_i32(to_i32($a) < to_i32($b)));
        }

        elsif ($opcode == 0x49) {
            # i32.lt_u (unsigned)
            my $b = to_u32($pop->()); my $a = to_u32($pop->());
            $push->(bool_to_i32($a < $b));
        }

        elsif ($opcode == 0x4a) {
            # i32.gt_s (signed)
            my $b = $pop->(); my $a = $pop->();
            $push->(bool_to_i32(to_i32($a) > to_i32($b)));
        }

        elsif ($opcode == 0x4b) {
            # i32.gt_u (unsigned)
            my $b = to_u32($pop->()); my $a = to_u32($pop->());
            $push->(bool_to_i32($a > $b));
        }

        elsif ($opcode == 0x4c) {
            # i32.le_s (signed)
            my $b = $pop->(); my $a = $pop->();
            $push->(bool_to_i32(to_i32($a) <= to_i32($b)));
        }

        elsif ($opcode == 0x4d) {
            # i32.le_u (unsigned)
            my $b = to_u32($pop->()); my $a = to_u32($pop->());
            $push->(bool_to_i32($a <= $b));
        }

        elsif ($opcode == 0x4e) {
            # i32.ge_s (signed)
            my $b = $pop->(); my $a = $pop->();
            $push->(bool_to_i32(to_i32($a) >= to_i32($b)));
        }

        elsif ($opcode == 0x4f) {
            # i32.ge_u (unsigned)
            my $b = to_u32($pop->()); my $a = to_u32($pop->());
            $push->(bool_to_i32($a >= $b));
        }

        # ---------------------------------------------------------------
        # NUMERIC — i32 ARITHMETIC
        # ---------------------------------------------------------------
        #
        # Binary ops follow: pop b (top), pop a (second), compute a OP b, push result.
        # All results are wrapped to 32-bit signed by to_i32().

        elsif ($opcode == 0x6a) {
            # i32.add (wrapping)
            my $b = $pop->(); my $a = $pop->();
            $push->(to_i32($a + $b));
        }

        elsif ($opcode == 0x6b) {
            # i32.sub (wrapping)
            my $b = $pop->(); my $a = $pop->();
            $push->(to_i32($a - $b));
        }

        elsif ($opcode == 0x6c) {
            # i32.mul (wrapping)
            my $b = $pop->(); my $a = $pop->();
            $push->(to_i32($a * $b));
        }

        elsif ($opcode == 0x6d) {
            # i32.div_s: signed division, truncate toward zero
            my $b = to_i32($pop->());
            my $a = to_i32($pop->());
            croak "WasmSimulator: i32.div_s: division by zero" if $b == 0;
            croak "WasmSimulator: i32.div_s: integer overflow (INT_MIN / -1)"
                if $a == -2147483648 && $b == -1;
            # Truncated division toward zero: POSIX::floor would go toward -inf.
            # int() in Perl truncates toward zero for positive results but not for
            # negative. Use a portable formula: sign * int(abs(a) / abs(b))
            my $sign   = (($a < 0) != ($b < 0)) ? -1 : 1;
            my $result = $sign * int(abs($a) / abs($b));
            $push->(to_i32($result));
        }

        elsif ($opcode == 0x6e) {
            # i32.div_u: unsigned division
            my $b = to_u32($pop->());
            my $a = to_u32($pop->());
            croak "WasmSimulator: i32.div_u: division by zero" if $b == 0;
            $push->(to_i32(int($a / $b)));
        }

        elsif ($opcode == 0x6f) {
            # i32.rem_s: signed remainder (truncated toward zero)
            my $b = to_i32($pop->());
            my $a = to_i32($pop->());
            croak "WasmSimulator: i32.rem_s: remainder by zero" if $b == 0;
            # Truncated remainder: a - b * trunc(a/b)
            my $sign   = (($a < 0) != ($b < 0)) ? -1 : 1;
            my $q      = $sign * int(abs($a) / abs($b));
            my $result = $a - $b * $q;
            $push->(to_i32($result));
        }

        elsif ($opcode == 0x70) {
            # i32.rem_u: unsigned remainder
            my $b = to_u32($pop->());
            my $a = to_u32($pop->());
            croak "WasmSimulator: i32.rem_u: remainder by zero" if $b == 0;
            $push->(to_i32($a % $b));
        }

        elsif ($opcode == 0x71) {
            # i32.and: bitwise AND
            my $b = $pop->(); my $a = $pop->();
            $push->(bit_and($a, $b));
        }

        elsif ($opcode == 0x72) {
            # i32.or: bitwise OR
            my $b = $pop->(); my $a = $pop->();
            $push->(bit_or($a, $b));
        }

        elsif ($opcode == 0x73) {
            # i32.xor: bitwise XOR
            my $b = $pop->(); my $a = $pop->();
            $push->(bit_xor($a, $b));
        }

        elsif ($opcode == 0x74) {
            # i32.shl: left shift (shift amount mod 32)
            my $b = $pop->(); my $a = $pop->();
            $push->(bit_shl($a, $b));
        }

        elsif ($opcode == 0x75) {
            # i32.shr_s: arithmetic (sign-extending) right shift
            my $b = $pop->(); my $a = $pop->();
            $push->(bit_shr_s($a, $b));
        }

        elsif ($opcode == 0x76) {
            # i32.shr_u: logical (zero-filling) right shift
            my $b = $pop->(); my $a = $pop->();
            $push->(bit_shr_u($a, $b));
        }

        else {
            croak sprintf(
                "WasmSimulator: unsupported opcode 0x%02x at pc=%d",
                $opcode, $pc - 1
            );
        }
    }

    return $stack;
}

# ============================================================================
# Instance class
# ============================================================================
#
# An INSTANCE is an initialized module ready for execution. Creating an
# instance involves:
#   1. Evaluating global initializers
#   2. Allocating linear memory
#   3. Applying data segments
#   4. Building the export map for name-based lookup

# new($class, $module) — create a new instance from a parsed Wasm module
#
# $module is the hashref returned by CodingAdventures::WasmModuleParser::parse().
sub new {
    my ($class, $module) = @_;

    my $self = bless {
        module   => $module,
        globals  => [],
        memory   => undef,
        export_map      => {},
        func_export_map => {},
    }, $class;

    # ------------------------------------------------------------------
    # Step 1: Initialize globals
    # ------------------------------------------------------------------
    for my $g (@{ $module->{globals} }) {
        my $value = eval_init_expr($g->{init_expr}, $self->{globals});
        push @{ $self->{globals} }, {
            value    => $value,
            mutable  => ($g->{mutable} ? 1 : 0),
            val_type => $g->{val_type},
        };
    }

    # ------------------------------------------------------------------
    # Step 2: Allocate linear memory
    # ------------------------------------------------------------------
    if (@{ $module->{memories} }) {
        my $mem_def  = $module->{memories}[0];
        # Cap initial pages to prevent resource exhaustion from malicious
        # Wasm binaries that set limits.min to the spec maximum (65535 pages
        # = 4 GiB). 64 pages (4 MiB) is sufficient for all simulator tests.
        my $initial = $mem_def->{limits}{min};
        $initial = MAX_PAGES if $initial > MAX_PAGES;
        $self->{memory} = make_memory($initial);
    } else {
        $self->{memory} = make_memory(0);
    }

    # ------------------------------------------------------------------
    # Step 3: Apply data segments
    # ------------------------------------------------------------------
    for my $seg (@{ $module->{data} // [] }) {
        my $offset = eval_init_expr(
            $seg->{offset_expr} // [0x41, 0x00, 0x0b],
            $self->{globals}
        );
        memory_store($self->{memory}, $offset, $seg->{bytes} // []);
    }

    # ------------------------------------------------------------------
    # Step 4: Build export maps
    # ------------------------------------------------------------------
    for my $exp (@{ $module->{exports} }) {
        $self->{export_map}{ $exp->{name} } = $exp->{desc};
        if ($exp->{desc}{kind} eq 'func') {
            $self->{func_export_map}{ $exp->{name} } = $exp->{desc}{idx};
        }
    }

    return $self;
}

# ============================================================================
# call_by_index($func_idx, @args) → @results
# ============================================================================
#
# Execute the function at $func_idx (0-based) with the given arguments.
# Returns a list of result values.
#
# This is the internal workhorse; call() uses it after resolving a name.

sub call_by_index {
    my ($self, $func_idx, @args) = @_;

    my $mod = $self->{module};

    croak "WasmSimulator: call_by_index: function index $func_idx out of range"
        if $func_idx < 0 || $func_idx >= scalar @{ $mod->{functions} };

    # Look up the function's type signature
    my $type_idx  = $mod->{functions}[$func_idx];
    my $func_type = $mod->{types}[$type_idx];
    my $n_params  = scalar @{ $func_type->{params} };
    my $n_results = scalar @{ $func_type->{results} };

    # Look up the code entry (locals + body)
    my $code = $mod->{codes}[$func_idx];
    croak "WasmSimulator: no code entry for function $func_idx"
        unless defined $code;

    # ------------------------------------------------------------------
    # Build the locals array
    # ------------------------------------------------------------------
    #
    # Layout: [param_0, param_1, ..., local_0, local_1, ...]
    # Parameters come from @args; declared locals are zero-initialized.

    my @locals;

    # Copy arguments into param slots
    for my $i (0 .. $n_params - 1) {
        $locals[$i] = $args[$i] // 0;
    }

    # Zero-initialize declared locals (expanding groups like "2 × i32")
    my $local_idx = $n_params;
    for my $group (@{ $code->{locals} }) {
        for (1 .. $group->{count}) {
            $locals[$local_idx++] = 0;
        }
    }

    # ------------------------------------------------------------------
    # Execute the function body
    # ------------------------------------------------------------------
    my $result_stack = execute_expr($self, $func_idx, $code->{body}, \@locals);

    # Collect the expected number of results from the top of the stack
    my @results;
    for my $i (reverse 0 .. $n_results - 1) {
        $results[$i] = $result_stack->[-($n_results - $i)] // 0;
    }

    return @results;
}

# ============================================================================
# call($func_name, @args) → @results
# ============================================================================
#
# Call an exported function by name with the given argument list.
# Returns a list of result values.

sub call {
    my ($self, $func_name, @args) = @_;

    my $func_idx = $self->{func_export_map}{$func_name};
    croak "WasmSimulator: no exported function named '$func_name'"
        unless defined $func_idx;

    return $self->call_by_index($func_idx, @args);
}

# ============================================================================
# get_global($name) → $value
# ============================================================================
#
# Get the current value of an exported global variable by name.

sub get_global {
    my ($self, $name) = @_;

    my $desc = $self->{export_map}{$name};
    croak "WasmSimulator: no export named '$name'"
        unless defined $desc;
    croak "WasmSimulator: export '$name' is not a global"
        unless $desc->{kind} eq 'global';

    my $glob = $self->{globals}[ $desc->{idx} ];
    croak "WasmSimulator: global index $desc->{idx} not found"
        unless defined $glob;

    return $glob->{value};
}

# ============================================================================
# set_global($name, $value)
# ============================================================================
#
# Set the value of an exported mutable global variable by name.
# Dies if the global is immutable.

sub set_global {
    my ($self, $name, $value) = @_;

    my $desc = $self->{export_map}{$name};
    croak "WasmSimulator: no export named '$name'"
        unless defined $desc;
    croak "WasmSimulator: export '$name' is not a global"
        unless $desc->{kind} eq 'global';

    my $glob = $self->{globals}[ $desc->{idx} ];
    croak "WasmSimulator: global index $desc->{idx} not found"
        unless defined $glob;
    croak "WasmSimulator: global '$name' is immutable"
        unless $glob->{mutable};

    $glob->{value} = $value;
}

# ============================================================================
# memory_read($offset, $length) → @bytes
# ============================================================================
#
# Read $length bytes from linear memory starting at byte $offset.
# Returns a list of byte values (0–255).

sub memory_read {
    my ($self, $offset, $length) = @_;
    my $bytes = memory_load($self->{memory}, $offset, $length);
    return @$bytes;
}

# ============================================================================
# memory_write($offset, \@bytes)
# ============================================================================
#
# Write an arrayref of byte values into linear memory starting at $offset.

sub memory_write {
    my ($self, $offset, $bytes) = @_;
    memory_store($self->{memory}, $offset, $bytes);
}

1;

__END__

=head1 NAME

CodingAdventures::WasmSimulator - WebAssembly interpreter / simulator

=head1 SYNOPSIS

  use CodingAdventures::WasmModuleParser qw(parse);
  use CodingAdventures::WasmSimulator;

  my $wasm  = do { local $/; open my $f, '<:raw', 'add.wasm'; <$f> };
  my $mod   = parse($wasm);
  my $inst  = CodingAdventures::WasmSimulator->new($mod);

  my @res = $inst->call("add", 3, 4);
  print $res[0];   # 7

=head1 DESCRIPTION

Executes WebAssembly modules by interpreting bytecode on a software-emulated
stack machine. The module parser is a required dependency.

=head1 METHODS

=head2 new($module)

Create a new instance from a parsed Wasm module hashref.

=head2 call($func_name, @args)

Call an exported function by name. Returns a list of results.

=head2 call_by_index($func_idx, @args)

Call a function by its 0-based module index.

=head2 get_global($name)

Get the current value of an exported global variable.

=head2 set_global($name, $value)

Set the value of an exported mutable global variable.

=head2 memory_read($offset, $length)

Read bytes from linear memory. Returns a list of byte values.

=head2 memory_write($offset, \@bytes)

Write bytes into linear memory.

=head1 LICENSE

MIT

=cut
