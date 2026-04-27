package CodingAdventures::WasmRuntime;

# ============================================================================
# CodingAdventures::WasmRuntime — Complete WebAssembly 1.0 Runtime
# ============================================================================
#
# This module is the user-facing entry point that composes all the lower-level
# packages into a single, easy-to-use API. It handles the full pipeline:
#
#   .wasm bytes  ->  Parse  ->  Validate  ->  Instantiate  ->  Execute
#
# ## Quick Start
#
#   use CodingAdventures::WasmRuntime;
#
#   my $runtime = CodingAdventures::WasmRuntime->new();
#   my $results = $runtime->load_and_run($wasm_bytes, 'square', [5]);
#   # $results = [25]
#
# ## Instantiation Steps
#
# When instantiate() is called, the runtime:
#
#   1. Resolves imports from the host interface
#   2. Allocates linear memory from the memory section
#   3. Allocates tables from the table section
#   4. Initializes globals by evaluating constant expressions
#   5. Applies data segments (copies bytes into memory)
#   6. Applies element segments (copies function refs into tables)
#   7. Calls the start function (if one is declared)
#
# ## WasiStub
#
# For programs that import WASI functions, a WasiStub is provided that
# implements minimal "proc_exit" and "fd_write" stubs. Real WASI support
# would require a full filesystem and I/O implementation.
#
# ============================================================================

use strict;
use warnings;
use Carp qw(croak);

our $VERSION = '0.03';

# Load sibling packages
use lib '../wasm-leb128/lib';
use lib '../wasm-types/lib';
use lib '../wasm-opcodes/lib';
use lib '../wasm-module-parser/lib';
use lib '../virtual-machine/lib';
use lib '../wasm-validator/lib';
use lib '../wasm-execution/lib';

use CodingAdventures::WasmModuleParser qw(parse);
use CodingAdventures::WasmLeb128 ();
use CodingAdventures::WasmValidator ();  # import nothing; call fully qualified
use CodingAdventures::WasmExecution qw(i32 i64 f32 f64 evaluate_const_expr);
use CodingAdventures::WasmRuntime::WasiClockRandom ();

# Encode is a core Perl module (included since perl 5.8) that handles
# character encoding conversions. We use it to encode Perl strings as
# UTF-8 bytes when writing arg/environ strings into WASM memory.
use Encode ();

use Exporter 'import';
our @EXPORT_OK = qw();

use constant {
    _MAX_DATA_SEGMENTS      => 4096,
    _MAX_DATA_SEGMENT_BYTES => 16 * 1024 * 1024,
    _MAX_TOTAL_DATA_BYTES   => 16 * 1024 * 1024,
};

# ============================================================================
# WasmRuntime
# ============================================================================

sub new {
    my ($class, %args) = @_;
    return bless {
        host => $args{host},  # optional host interface for import resolution
    }, $class;
}

# ---- Parse ----------------------------------------------------------------

# load($wasm_bytes) — parse a binary string of .wasm data into a module hashref.
sub load {
    my ($self, $wasm_bytes) = @_;
    return parse($wasm_bytes);
}

# ---- Validate -------------------------------------------------------------

# validate($module) — validate a parsed module. Returns the validated result.
sub validate {
    my ($self, $module) = @_;
    return CodingAdventures::WasmValidator::validate($module);
}

# ---- Instantiate ----------------------------------------------------------

# instantiate($module) — create a live instance from a parsed module.
# Returns an instance hashref with all runtime state.
sub instantiate {
    my ($self, $module) = @_;

    # Build combined function type arrays (imports first, then module funcs)
    my @func_types;
    my @func_bodies;
    my @host_functions;
    my @global_types;
    my @globals;
    my $memory = undef;
    my @tables;

    # Step 1: Resolve imports
    for my $imp (@{ $module->{imports} || [] }) {
        my $kind = $imp->{desc}{kind};

        if ($kind eq 'func') {
            my $type_idx = $imp->{desc}{idx};
            $type_idx = $imp->{desc}{type_idx} unless defined $type_idx;
            push @func_types, $module->{types}[$type_idx];
            push @func_bodies, undef;  # no body for imports

            # Try to resolve from host
            my $host_func = undef;
            if ($self->{host}) {
                my $module_name = defined $imp->{module} ? $imp->{module} : $imp->{mod};
                $host_func = $self->{host}->resolve_function(
                    $module_name, $imp->{name}
                );
            }
            push @host_functions, $host_func;
        }
        elsif ($kind eq 'mem') {
            if ($self->{host}) {
                my $module_name = defined $imp->{module} ? $imp->{module} : $imp->{mod};
                $memory = $self->{host}->resolve_memory(
                    $module_name, $imp->{name}
                );
            }
        }
        elsif ($kind eq 'table') {
            if ($self->{host}) {
                my $module_name = defined $imp->{module} ? $imp->{module} : $imp->{mod};
                my $t = $self->{host}->resolve_table(
                    $module_name, $imp->{name}
                );
                push @tables, $t if $t;
            }
        }
        elsif ($kind eq 'global') {
            if ($self->{host}) {
                my $module_name = defined $imp->{module} ? $imp->{module} : $imp->{mod};
                my $g = $self->{host}->resolve_global(
                    $module_name, $imp->{name}
                );
                if ($g) {
                    push @global_types, $g->{type};
                    push @globals, $g->{value};
                }
            }
        }
    }

    # Step 2: Add module-defined functions
    my $raw_codes = $module->{codes} || $module->{code} || [];
    for my $i (0 .. $#{ $module->{functions} || [] }) {
        my $type_idx = $module->{functions}[$i];
        push @func_types, $module->{types}[$type_idx];

        # Convert parser's code format to execution engine format.
        # Parser returns: { locals => [{count => N, type => T}, ...], body => \@bytes }
        # Engine expects: { locals => \@expanded_type_codes, code => \@bytes }
        my $raw = $raw_codes->[$i];
        if ($raw) {
            my @expanded_locals;
            for my $lg (@{ $raw->{locals} || [] }) {
                for (1 .. $lg->{count}) {
                    push @expanded_locals, $lg->{type};
                }
            }
            push @func_bodies, {
                locals => \@expanded_locals,
                code   => $raw->{body} || $raw->{code} || [],
            };
        } else {
            push @func_bodies, undef;
        }
        push @host_functions, undef;
    }

    # Step 3: Allocate memory (from memory section, if not imported)
    if (!$memory && scalar(@{ $module->{memories} || [] }) > 0) {
        my $mem_type = $module->{memories}[0];
        my $limits = $mem_type->{limits} || $mem_type;
        $memory = CodingAdventures::WasmExecution::LinearMemory->new(
            $mem_type->{min} // $limits->{min},
            $mem_type->{max} // $limits->{max},
        );
    }

    # Step 4: Allocate tables (from table section, if not imported via host)
    for my $table_type (@{ $module->{tables} || [] }) {
        push @tables, CodingAdventures::WasmExecution::Table->new(
            $table_type->{limits}{min},
            $table_type->{limits}{max},
        );
    }

    # Step 5: Initialize globals
    for my $global (@{ $module->{globals} || [] }) {
        push @global_types, $global->{global_type};
        my $value = evaluate_const_expr($global->{init_expr}, \@globals);
        push @globals, $value;
    }

    # Step 6: Apply data segments
    if ($memory) {
        for my $seg (@{ _normalize_data_segments($module->{data} || []) }) {
            my $offset = evaluate_const_expr($seg->{offset_expr}, \@globals);
            my $offset_num = $offset->{value};
            $memory->write_bytes($offset_num, $seg->{data});
        }
    }

    # Step 7: Apply element segments
    for my $elem (@{ $module->{elements} || [] }) {
        my $table = $tables[$elem->{table_index} || 0];
        if ($table) {
            my $offset = evaluate_const_expr($elem->{offset_expr}, \@globals);
            my $offset_num = $offset->{value};
            my $func_indices = $elem->{function_indices} || $elem->{func_indices} || [];
            for my $j (0 .. $#$func_indices) {
                $table->set($offset_num + $j, $func_indices->[$j]);
            }
        }
    }

    # Build exports map
    my %exports;
    for my $exp (@{ $module->{exports} || [] }) {
        $exports{ $exp->{name} } = {
            kind  => $exp->{desc}{kind},
            index => $exp->{desc}{idx},
        };
    }

    my $instance = {
        module         => $module,
        memory         => $memory,
        tables         => \@tables,
        globals        => \@globals,
        global_types   => \@global_types,
        func_types     => \@func_types,
        func_bodies    => \@func_bodies,
        host_functions => \@host_functions,
        exports        => \%exports,
    };

    if ($self->{host} && $memory && $self->{host}->can('set_memory')) {
        $self->{host}->set_memory($memory);
    }

    # Step 8: Call start function (if present)
    if (defined $module->{start}) {
        my $engine = CodingAdventures::WasmExecution::Engine->new(
            memory         => $instance->{memory},
            tables         => $instance->{tables},
            globals        => $instance->{globals},
            global_types   => $instance->{global_types},
            func_types     => $instance->{func_types},
            func_bodies    => $instance->{func_bodies},
            host_functions => $instance->{host_functions},
        );
        $engine->call_function($module->{start}, []);
    }

    return $instance;
}

sub _normalize_data_segments {
    my ($segments) = @_;
    croak 'WasmRuntime: data segments arrayref required'
        unless ref($segments || []) eq 'ARRAY';

    my @normalized;
    my $segment_count = 0;
    my $total_data = 0;

    for my $segment (@{ $segments || [] }) {
        if (ref($segment) eq 'HASH') {
            my $data_len = _data_payload_length($segment->{data});
            ($segment_count, $total_data) = _check_data_segment_limits(
                $segment_count,
                $total_data,
                $data_len,
            );
            push @normalized, $segment;
            next;
        }

        my $bytes = $segment || [];
        croak 'WasmRuntime: raw data section must be an arrayref of bytes'
            unless ref($bytes) eq 'ARRAY';

        my $pos = 0;
        my ($count, $cc) = CodingAdventures::WasmLeb128::decode_unsigned($bytes, $pos);
        $pos += $cc;
        croak 'WasmRuntime: data segment count exceeds limit'
            if $segment_count + $count > _MAX_DATA_SEGMENTS;

        for (my $i = 0; $i < $count; $i++) {
            my ($memory_index, $mc) = CodingAdventures::WasmLeb128::decode_unsigned($bytes, $pos);
            $pos += $mc;

            my @offset_expr;
            my $saw_end = 0;
            while ($pos <= $#$bytes) {
                my $byte = $bytes->[$pos++];
                push @offset_expr, $byte;
                if ($byte == 0x0B) {
                    $saw_end = 1;
                    last;
                }
            }
            croak 'WasmRuntime: unterminated data segment offset expression'
                unless $saw_end;

            my ($size, $sc) = CodingAdventures::WasmLeb128::decode_unsigned($bytes, $pos);
            $pos += $sc;
            ($segment_count, $total_data) = _check_data_segment_limits(
                $segment_count,
                $total_data,
                $size,
            );
            croak 'WasmRuntime: data segment payload shorter than declared size'
                if $size > scalar(@$bytes) - $pos;

            my @data = $size > 0 ? @{$bytes}[$pos .. $pos + $size - 1] : ();
            $pos += $size;

            push @normalized, {
                memory_index => $memory_index,
                offset_expr  => \@offset_expr,
                data         => \@data,
            };
        }

        croak 'WasmRuntime: trailing bytes after data section'
            if $pos != scalar(@$bytes);
    }

    return \@normalized;
}

sub _data_payload_length {
    my ($payload) = @_;
    return 0 unless defined $payload;
    return scalar(@$payload) if ref($payload) eq 'ARRAY';
    return length($payload) unless ref($payload);
    croak 'WasmRuntime: data segment payload must be bytes or an arrayref';
}

sub _check_data_segment_limits {
    my ($segment_count, $total_data, $size) = @_;
    croak 'WasmRuntime: data segment size exceeds limit'
        if $size > _MAX_DATA_SEGMENT_BYTES;

    $segment_count++;
    croak 'WasmRuntime: data segment count exceeds limit'
        if $segment_count > _MAX_DATA_SEGMENTS;

    $total_data += $size;
    croak 'WasmRuntime: total data segment bytes exceed limit'
        if $total_data > _MAX_TOTAL_DATA_BYTES;

    return ($segment_count, $total_data);
}

# ---- Call -----------------------------------------------------------------

# call($instance, $name, \@args) — call an exported function by name.
# Arguments and results are plain Perl numbers (auto-converted to WasmValues).
sub call {
    my ($self, $instance, $name, $args) = @_;
    $args //= [];

    my $exp = $instance->{exports}{$name};
    CodingAdventures::WasmExecution::TrapError->throw(
        "export \"$name\" not found"
    ) unless $exp;
    CodingAdventures::WasmExecution::TrapError->throw(
        "export \"$name\" is not a function"
    ) unless $exp->{kind} eq 'func';

    my $func_type = $instance->{func_types}[$exp->{index}];
    CodingAdventures::WasmExecution::TrapError->throw(
        "function type not found for export \"$name\""
    ) unless $func_type;

    # Convert plain numbers to WasmValues
    my @wasm_args;
    for my $i (0 .. $#$args) {
        my $param_type = $func_type->{params}[$i];
        if ($param_type == 0x7F) {
            push @wasm_args, i32($args->[$i]);
        } elsif ($param_type == 0x7E) {
            push @wasm_args, i64($args->[$i]);
        } elsif ($param_type == 0x7D) {
            push @wasm_args, f32($args->[$i]);
        } elsif ($param_type == 0x7C) {
            push @wasm_args, f64($args->[$i]);
        } else {
            push @wasm_args, i32($args->[$i]);
        }
    }

    # Create execution engine and call
    my $engine = CodingAdventures::WasmExecution::Engine->new(
        memory         => $instance->{memory},
        tables         => $instance->{tables},
        globals        => $instance->{globals},
        global_types   => $instance->{global_types},
        func_types     => $instance->{func_types},
        func_bodies    => $instance->{func_bodies},
        host_functions => $instance->{host_functions},
    );

    my $results = $engine->call_function($exp->{index}, \@wasm_args);

    # Convert WasmValues back to plain numbers
    return [ map { $_->{value} } @$results ];
}

# ---- Convenience ----------------------------------------------------------

# load_and_run($wasm_bytes, $entry, \@args) — parse, validate, instantiate,
# and call in one step.
sub load_and_run {
    my ($self, $wasm_bytes, $entry, $args) = @_;
    $entry //= '_start';
    $args  //= [];

    my $module = $self->load($wasm_bytes);
    $self->validate($module);
    my $instance = $self->instantiate($module);
    return $self->call($instance, $entry, $args);
}

# ============================================================================
# WasiStub — WASI host interface with Tier 3 functions
# ============================================================================
#
# Implements WASI host functions for programs that import from
# "wasi_snapshot_preview1". Covers two tiers:
#
#   Tier 1 (existing): proc_exit, fd_write
#   Tier 3 (new):      args_sizes_get, args_get, environ_sizes_get,
#                      environ_get, clock_res_get, clock_time_get,
#                      random_get, sched_yield
#
# ## Tier 3 overview
#
# Tier 3 brings in three broad capability areas:
#
#   1. Program arguments (args_*): expose argv/argc to WASM programs, just
#      as the shell passes them to native processes. A WASM program calls
#      args_sizes_get first to learn how much memory to allocate, then
#      args_get to fill it.
#
#   2. Environment variables (environ_*): expose the KEY=VALUE pairs that
#      configure programs (like $HOME, $PATH). Same two-phase protocol as
#      args.
#
#   3. Clocks (clock_*): expose the system clock and monotonic timer for
#      measuring time and computing timeouts.
#
#   4. Random bytes (random_get): expose cryptographically secure random
#      bytes for seeding PRNGs, generating nonces, etc.
#
#   5. Scheduling (sched_yield): hint to the scheduler that this thread is
#      willing to yield. Always a no-op in our single-threaded runtime.
#
# ## Memory layout for args/environ
#
#   WASI's two-phase arg/environ protocol mirrors the POSIX argv convention:
#
#     argv_buf: [arg0\0][arg1\0]...[argN\0]   ← raw bytes, NUL-terminated
#     argv:     [ptr0][ptr1]...[ptrN]         ← array of i32 pointers into argv_buf
#
#   The caller allocates both buffers (using sizes from args_sizes_get), then
#   passes the base pointers to args_get. We fill argv_buf first, tracking
#   each arg's offset so we can write the correct pointer into argv.
#
# ## WASI errno values
#
#   0  = ESUCCESS (no error)
#   28 = EINVAL   (invalid argument, e.g., unknown clock id)

package CodingAdventures::WasmRuntime::WasiStub;

use constant {
    _WASI_MAX_IOVECS   => 1024,
    _WASI_MAX_RW_BYTES => 1024 * 1024,
    _WASI_ERRNO_INVAL  => 28,
};

# constructor — create a new WasiStub with optional configuration.
#
# Parameters (all optional):
#   args    => \@array   — command-line arguments (like ARGV). Default: [].
#   env     => \%hash    — environment variables {KEY => VALUE}. Default: {}.
#   stdin   => \&coderef — callback invoked with a byte count for fd_read.
#   stdout  => \&coderef — callback invoked with each line of stdout output.
#   stderr  => \&coderef — callback invoked with stderr output (reserved).
#   clock   => $obj      — object implementing WasiClock interface.
#   random  => $obj      — object implementing WasiRandom interface.
#
# Example:
#   my $stub = CodingAdventures::WasmRuntime::WasiStub->new(
#       args   => ['myapp', '--verbose'],
#       env    => { HOME => '/home/user', PATH => '/usr/bin' },
#       clock  => FakeClock->new(),
#       random => FakeRandom->new(),
#   );
sub new {
    my ($class, %args) = @_;
    return bless {
        args            => $args{args}   // [],
        env             => $args{env}    // {},
        stdin_callback  => $args{stdin}  // sub { [] },
        stdout_callback => $args{stdout} // sub {},
        stderr_callback => $args{stderr} // sub {},
        clock           => $args{clock}  // CodingAdventures::WasmRuntime::SystemClock->new(),
        random          => $args{random} // CodingAdventures::WasmRuntime::SystemRandom->new(),
        exit_code       => undef,
        instance_memory => undef,
    }, $class;
}

# resolve_function($module_name, $func_name) — look up a WASI host function.
#
# Returns a coderef that the execution engine will call when the WASM program
# invokes an imported function. Returns undef for unknown names.
sub resolve_function {
    my ($self, $module_name, $name) = @_;

    return undef unless $module_name eq 'wasi_snapshot_preview1'
                     || $module_name eq 'wasi_unstable';

    # ---- Tier 1: proc lifecycle ----------------------------------------

    if ($name eq 'proc_exit') {
        # proc_exit(exit_code: i32) → (no return)
        #
        # Terminates the WASM program with the given exit code. We record the
        # code for inspection after execution ends.
        return sub {
            my ($args) = @_;
            $self->{exit_code} = $args->[0]{value};
            return [];
        };
    }

    if ($name eq 'fd_write') {
        # fd_write(fd, iovs_ptr, iovs_len, nwritten_ptr) → errno
        #
        # Writes scattered buffers to a file descriptor. This implementation
        # supports stdout/stderr so compiled Brainfuck and Nib programs can
        # exercise observable WASI output during tests.
        return sub {
            my ($args) = @_;
            my $memory       = $self->{instance_memory};
            return [ CodingAdventures::WasmExecution::i32(52) ] unless $memory;

            my $fd           = $args->[0]{value};
            my $iovs_ptr     = $args->[1]{value} & 0xFFFFFFFF;
            my $iovs_len     = $args->[2]{value} & 0x7FFFFFFF;
            my $nwritten_ptr = $args->[3]{value} & 0xFFFFFFFF;

            return [ CodingAdventures::WasmExecution::i32(8) ] unless $fd == 1 || $fd == 2;
            return [ CodingAdventures::WasmExecution::i32(_WASI_ERRNO_INVAL) ]
                if $iovs_len > _WASI_MAX_IOVECS;

            my $callback = $fd == 1 ? $self->{stdout_callback} : $self->{stderr_callback};
            my $total_written = 0;
            for my $i (0 .. $iovs_len - 1) {
                my $buf_ptr = $memory->load_i32($iovs_ptr + $i * 8) & 0xFFFFFFFF;
                my $buf_len = $memory->load_i32($iovs_ptr + $i * 8 + 4) & 0xFFFFFFFF;
                return [ CodingAdventures::WasmExecution::i32(_WASI_ERRNO_INVAL) ]
                    if $buf_len > _WASI_MAX_RW_BYTES || $total_written + $buf_len > _WASI_MAX_RW_BYTES;

                my $offset = 0;
                while ($offset < $buf_len) {
                    my $chunk_len = $buf_len - $offset;
                    $chunk_len = 4096 if $chunk_len > 4096;
                    my @chunk_bytes;
                    for my $j (0 .. $chunk_len - 1) {
                        push @chunk_bytes, $memory->load_i32_8u($buf_ptr + $offset + $j);
                    }
                    $callback->(pack('C*', @chunk_bytes));
                    $offset += $chunk_len;
                }

                $total_written += $buf_len;
            }

            $memory->store_i32($nwritten_ptr, $total_written);
            return [ CodingAdventures::WasmExecution::i32(0) ];
        };
    }

    if ($name eq 'fd_read') {
        # fd_read(fd, iovs_ptr, iovs_len, nread_ptr) → errno
        #
        # Reads bytes from stdin into the guest buffers described by the iovec
        # array. Only fd 0 is supported.
        return sub {
            my ($args) = @_;
            my $memory    = $self->{instance_memory};
            return [ CodingAdventures::WasmExecution::i32(52) ] unless $memory;

            my $fd        = $args->[0]{value};
            my $iovs_ptr  = $args->[1]{value} & 0xFFFFFFFF;
            my $iovs_len  = $args->[2]{value} & 0x7FFFFFFF;
            my $nread_ptr = $args->[3]{value} & 0xFFFFFFFF;

            return [ CodingAdventures::WasmExecution::i32(8) ] unless $fd == 0;
            return [ CodingAdventures::WasmExecution::i32(_WASI_ERRNO_INVAL) ]
                if $iovs_len > _WASI_MAX_IOVECS;

            my $total_read = 0;
            for my $i (0 .. $iovs_len - 1) {
                my $buf_ptr = $memory->load_i32($iovs_ptr + $i * 8) & 0xFFFFFFFF;
                my $buf_len = $memory->load_i32($iovs_ptr + $i * 8 + 4) & 0xFFFFFFFF;
                return [ CodingAdventures::WasmExecution::i32(_WASI_ERRNO_INVAL) ]
                    if $buf_len > _WASI_MAX_RW_BYTES || $total_read + $buf_len > _WASI_MAX_RW_BYTES;

                my $raw = $self->{stdin_callback}->($buf_len);
                my @bytes =
                    !defined $raw ? ()
                  : ref($raw) eq 'ARRAY' ? @$raw
                  : unpack('C*', $raw);
                splice(@bytes, $buf_len) if @bytes > $buf_len;

                for my $j (0 .. $#bytes) {
                    $memory->store_i32_8($buf_ptr + $j, $bytes[$j]);
                }

                $total_read += scalar @bytes;
                last if scalar(@bytes) < $buf_len;
            }

            $memory->store_i32($nread_ptr, $total_read);
            return [ CodingAdventures::WasmExecution::i32(0) ];
        };
    }

    # ---- Tier 3: arguments ---------------------------------------------

    if ($name eq 'args_sizes_get') {
        # args_sizes_get(argc_ptr: i32, argv_buf_size_ptr: i32) → errno
        #
        # Writes two values into WASM memory:
        #   *argc_ptr          = number of arguments
        #   *argv_buf_size_ptr = total bytes needed for all arg strings
        #                        (each NUL-terminated)
        #
        # The caller uses these sizes to allocate exactly the right amount of
        # memory before calling args_get.
        #
        # Memory is accessed via $self->{instance_memory}, which must be set
        # before calling this function (use set_memory() after instantiation).
        return sub {
            my ($args) = @_;
            my $memory       = $self->{instance_memory};
            my $argc_ptr     = $args->[0]{value} & 0xFFFFFFFF;
            my $argv_buf_ptr = $args->[1]{value} & 0xFFFFFFFF;

            my $argc = scalar @{$self->{args}};
            my $buf_size = 0;
            for my $arg (@{$self->{args}}) {
                # Each arg is stored as UTF-8 bytes followed by a NUL terminator.
                $buf_size += length(Encode::encode('UTF-8', $arg)) + 1;
            }

            $memory->store_i32($argc_ptr,     $argc);
            $memory->store_i32($argv_buf_ptr, $buf_size);
            return [ CodingAdventures::WasmExecution::i32(0) ];
        };
    }

    if ($name eq 'args_get') {
        # args_get(argv_ptr: i32, argv_buf_ptr: i32) → errno
        #
        # Fills two caller-allocated buffers:
        #
        #   argv_buf (at argv_buf_ptr): contiguous NUL-terminated arg strings.
        #   argv     (at argv_ptr):     array of i32 pointers, one per arg,
        #                               each pointing into argv_buf.
        #
        # Memory layout diagram (for args = ["hi", "world"]):
        #
        #   argv_buf:  ['h','i',0,'w','o','r','l','d',0]
        #                ^              ^
        #   argv:      [argv_buf+0,    argv_buf+3]
        #
        # The WASM program reads argv[i] to get a pointer, then dereferences
        # it to read the NUL-terminated string — exactly like C's main(argc, argv).
        return sub {
            my ($args) = @_;
            my $memory       = $self->{instance_memory};
            my $argv_ptr     = $args->[0]{value} & 0xFFFFFFFF;
            my $argv_buf_ptr = $args->[1]{value} & 0xFFFFFFFF;

            my $offset = $argv_buf_ptr;
            my $i = 0;
            for my $arg (@{$self->{args}}) {
                # Write the pointer to this arg into the argv array.
                $memory->store_i32($argv_ptr + $i * 4, $offset);

                # Write the arg bytes (UTF-8) followed by NUL.
                my @bytes = (unpack('C*', Encode::encode('UTF-8', $arg)), 0);
                for my $j (0 .. $#bytes) {
                    $memory->store_i32_8($offset + $j, $bytes[$j]);
                }

                $offset += scalar @bytes;
                $i++;
            }

            return [ CodingAdventures::WasmExecution::i32(0) ];
        };
    }

    # ---- Tier 3: environment variables ---------------------------------

    if ($name eq 'environ_sizes_get') {
        # environ_sizes_get(environ_count_ptr: i32, environ_buf_size_ptr: i32) → errno
        #
        # Same two-phase protocol as args_sizes_get, but for environment
        # variables. Each entry is stored as "KEY=VALUE\0".
        return sub {
            my ($args) = @_;
            my $memory       = $self->{instance_memory};
            my $count_ptr    = $args->[0]{value} & 0xFFFFFFFF;
            my $buf_size_ptr = $args->[1]{value} & 0xFFFFFFFF;

            my $count = scalar keys %{$self->{env}};
            my $buf_size = 0;
            for my $key (sort keys %{$self->{env}}) {
                my $entry = "$key=" . $self->{env}{$key};
                $buf_size += length(Encode::encode('UTF-8', $entry)) + 1;
            }

            $memory->store_i32($count_ptr,    $count);
            $memory->store_i32($buf_size_ptr, $buf_size);
            return [ CodingAdventures::WasmExecution::i32(0) ];
        };
    }

    if ($name eq 'environ_get') {
        # environ_get(environ_ptr: i32, environ_buf_ptr: i32) → errno
        #
        # Same pattern as args_get but for "KEY=VALUE\0" strings.
        # environ_ptr is an array of i32 pointers into environ_buf.
        return sub {
            my ($args) = @_;
            my $memory          = $self->{instance_memory};
            my $environ_ptr     = $args->[0]{value} & 0xFFFFFFFF;
            my $environ_buf_ptr = $args->[1]{value} & 0xFFFFFFFF;

            my $offset = $environ_buf_ptr;
            my $i = 0;
            for my $key (sort keys %{$self->{env}}) {
                my $entry = "$key=" . $self->{env}{$key};
                $memory->store_i32($environ_ptr + $i * 4, $offset);

                my @bytes = (unpack('C*', Encode::encode('UTF-8', $entry)), 0);
                for my $j (0 .. $#bytes) {
                    $memory->store_i32_8($offset + $j, $bytes[$j]);
                }

                $offset += scalar @bytes;
                $i++;
            }

            return [ CodingAdventures::WasmExecution::i32(0) ];
        };
    }

    # ---- Tier 3: clocks ------------------------------------------------

    if ($name eq 'clock_res_get') {
        # clock_res_get(id: i32, resolution_ptr: i32) → errno
        #
        # Writes the resolution of clock `id` (as a 64-bit nanosecond value,
        # little-endian) into *resolution_ptr.
        #
        # We write the i64 as two i32 halves:
        #   low  32 bits → at resolution_ptr
        #   high 32 bits → at resolution_ptr + 4
        #
        # This is safe on 64-bit Perl where integers are 64-bit natively.
        return sub {
            my ($args) = @_;
            my $memory         = $self->{instance_memory};
            my $id             = $args->[0]{value};
            my $resolution_ptr = $args->[1]{value} & 0xFFFFFFFF;

            my $ns = $self->{clock}->resolution_ns($id);
            my $lo = $ns & 0xFFFFFFFF;
            my $hi = ($ns >> 32) & 0xFFFFFFFF;
            $memory->store_i32($resolution_ptr,     $lo);
            $memory->store_i32($resolution_ptr + 4, $hi);
            return [ CodingAdventures::WasmExecution::i32(0) ];
        };
    }

    if ($name eq 'clock_time_get') {
        # clock_time_get(id: i32, precision: i64, time_ptr: i32) → errno
        #
        # Writes the current time for clock `id` as an i64 nanosecond count
        # into *time_ptr.
        #
        # WASI clock IDs:
        #   0 = CLOCK_REALTIME   — wall-clock time since Unix epoch
        #   1 = CLOCK_MONOTONIC  — monotonically increasing counter
        #   2 = CLOCK_PROCESS    — process CPU time (mapped to realtime here)
        #   3 = CLOCK_THREAD     — thread CPU time  (mapped to realtime here)
        #
        # The `precision` parameter (args->[1]) hints at the desired accuracy;
        # we ignore it since we always return the maximum available precision.
        #
        # Returns EINVAL (28) for unknown clock IDs.
        return sub {
            my ($args) = @_;
            my $memory   = $self->{instance_memory};
            my $id       = $args->[0]{value};
            my $time_ptr = $args->[2]{value} & 0xFFFFFFFF;

            my $ns;
            if ($id == 0 || $id == 2 || $id == 3) {
                $ns = $self->{clock}->realtime_ns();
            } elsif ($id == 1) {
                $ns = $self->{clock}->monotonic_ns();
            } else {
                # Unknown clock ID → EINVAL
                return [ CodingAdventures::WasmExecution::i32(28) ];
            }

            # Write the 64-bit nanosecond count as two 32-bit little-endian words.
            # On 64-bit Perl (the norm), $ns is a native 64-bit integer, so
            # bitwise ops work correctly without any bignum library.
            my $lo = $ns & 0xFFFFFFFF;
            my $hi = ($ns >> 32) & 0xFFFFFFFF;
            $memory->store_i32($time_ptr,     $lo);
            $memory->store_i32($time_ptr + 4, $hi);
            return [ CodingAdventures::WasmExecution::i32(0) ];
        };
    }

    # ---- Tier 3: random ------------------------------------------------

    if ($name eq 'random_get') {
        # random_get(buf_ptr: i32, buf_len: i32) → errno
        #
        # Fills buf_len bytes starting at buf_ptr with cryptographically
        # secure random data from the injected $random source.
        #
        # The buf_len & 0x7FFFFFFF strips the sign bit, treating the i32
        # as unsigned — a WASM i32 has no unsigned type, but lengths are
        # always non-negative.
        return sub {
            my ($args) = @_;
            my $memory  = $self->{instance_memory};
            my $buf_ptr = $args->[0]{value} & 0xFFFFFFFF;
            my $buf_len = $args->[1]{value} & 0x7FFFFFFF;

            return [ CodingAdventures::WasmExecution::i32(_WASI_ERRNO_INVAL) ]
                if $buf_len > _WASI_MAX_RW_BYTES;
            return [ CodingAdventures::WasmExecution::i32(_WASI_ERRNO_INVAL) ]
                if $buf_ptr > $memory->byte_length || $buf_len > $memory->byte_length - $buf_ptr;

            my $bytes = $self->{random}->fill_bytes($buf_len);
            for my $i (0 .. $#$bytes) {
                $memory->store_i32_8($buf_ptr + $i, $bytes->[$i]);
            }
            return [ CodingAdventures::WasmExecution::i32(0) ];
        };
    }

    # ---- Tier 3: scheduling --------------------------------------------

    if ($name eq 'sched_yield') {
        # sched_yield() → errno
        #
        # Suggests to the OS scheduler that this thread is willing to give
        # up its time slice. In our single-threaded synchronous runtime, this
        # is a pure no-op. We return ESUCCESS (0) immediately.
        return sub {
            return [ CodingAdventures::WasmExecution::i32(0) ];
        };
    }

    return undef;
}

# set_memory($mem) — inject the LinearMemory instance after instantiation.
#
# The WASI host functions that access linear memory (args_get, environ_get,
# clock_time_get, etc.) need a reference to the instance's memory object.
# Since memory is created during instantiate(), the caller must call
# set_memory() after the instance is built:
#
#   my $instance = $rt->instantiate($module);
#   $wasi->set_memory($instance->{memory});
#
# Tests use this directly:
#   my $mem = CodingAdventures::WasmExecution::LinearMemory->new(1, undef);
#   $wasi->set_memory($mem);
sub set_memory {
    my ($self, $mem) = @_;
    $self->{instance_memory} = $mem;
}

sub resolve_memory { return undef }
sub resolve_table  { return undef }
sub resolve_global { return undef }

sub exit_code { return $_[0]->{exit_code} }

package CodingAdventures::WasmRuntime::WasiHost;

our @ISA = ('CodingAdventures::WasmRuntime::WasiStub');

1;

__END__

=head1 NAME

CodingAdventures::WasmRuntime - Complete WebAssembly 1.0 runtime

=head1 SYNOPSIS

    use CodingAdventures::WasmRuntime;

    my $runtime = CodingAdventures::WasmRuntime->new();
    my $results = $runtime->load_and_run($wasm_bytes, 'square', [5]);
    # $results = [25]

=head1 DESCRIPTION

Composes the WASM module parser, validator, and execution engine into a
single user-facing API. Handles parsing, validation, instantiation
(memory allocation, global initialization, data/element segments),
and function calling.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
