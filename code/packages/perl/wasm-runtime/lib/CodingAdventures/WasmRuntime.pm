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

our $VERSION = '0.01';

# Load sibling packages
use lib '../wasm-leb128/lib';
use lib '../wasm-types/lib';
use lib '../wasm-opcodes/lib';
use lib '../wasm-module-parser/lib';
use lib '../virtual-machine/lib';
use lib '../wasm-validator/lib';
use lib '../wasm-execution/lib';

use CodingAdventures::WasmModuleParser qw(parse);
use CodingAdventures::WasmValidator ();  # import nothing; call fully qualified
use CodingAdventures::WasmExecution qw(i32 i64 f32 f64 evaluate_const_expr);

use Exporter 'import';
our @EXPORT_OK = qw();

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
            push @func_types, $module->{types}[$type_idx];
            push @func_bodies, undef;  # no body for imports

            # Try to resolve from host
            my $host_func = undef;
            if ($self->{host}) {
                $host_func = $self->{host}->resolve_function(
                    $imp->{module}, $imp->{name}
                );
            }
            push @host_functions, $host_func;
        }
        elsif ($kind eq 'mem') {
            if ($self->{host}) {
                $memory = $self->{host}->resolve_memory(
                    $imp->{module}, $imp->{name}
                );
            }
        }
        elsif ($kind eq 'table') {
            if ($self->{host}) {
                my $t = $self->{host}->resolve_table(
                    $imp->{module}, $imp->{name}
                );
                push @tables, $t if $t;
            }
        }
        elsif ($kind eq 'global') {
            if ($self->{host}) {
                my $g = $self->{host}->resolve_global(
                    $imp->{module}, $imp->{name}
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
        $memory = CodingAdventures::WasmExecution::LinearMemory->new(
            $mem_type->{min},
            $mem_type->{max},
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
        for my $seg (@{ $module->{data} || [] }) {
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
# WasiStub — minimal WASI host interface
# ============================================================================
#
# Provides stub implementations of WASI system calls for programs that
# import from "wasi_snapshot_preview1". Only proc_exit and fd_write are
# stubbed; real WASI support would need filesystem and I/O.

package CodingAdventures::WasmRuntime::WasiStub;

sub new {
    my ($class, %args) = @_;
    return bless {
        stdout_callback => $args{stdout} || sub {},
        exit_code       => undef,
    }, $class;
}

sub resolve_function {
    my ($self, $module_name, $name) = @_;

    return undef unless $module_name eq 'wasi_snapshot_preview1'
                     || $module_name eq 'wasi_unstable';

    if ($name eq 'proc_exit') {
        return sub {
            my ($args) = @_;
            $self->{exit_code} = $args->[0]{value};
            return [];
        };
    }
    elsif ($name eq 'fd_write') {
        return sub {
            my ($args) = @_;
            # Stub: return 0 bytes written
            return [ CodingAdventures::WasmExecution::i32(0) ];
        };
    }

    return undef;
}

sub resolve_memory { return undef }
sub resolve_table  { return undef }
sub resolve_global { return undef }

sub exit_code { return $_[0]->{exit_code} }

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
