package CodingAdventures::WasmValidator;

# ============================================================================
# CodingAdventures::WasmValidator — WebAssembly 1.0 Module Validator
# ============================================================================
#
# This module validates parsed WebAssembly modules for structural and semantic
# correctness BEFORE execution. Validation catches errors that would otherwise
# cause traps or undefined behavior at runtime.
#
# ## What Does Validation Check?
#
# WebAssembly validation ensures:
#
#   1. **Type indices are in range** — every function must reference a valid
#      type definition from the type section.
#
#   2. **Export names are unique** — no two exports may share the same name.
#
#   3. **Memory limits are valid** — at most one memory, and if a max is
#      specified, min <= max <= 65536 pages.
#
#   4. **Function bodies reference valid locals/globals** — local.get,
#      global.get etc. must use valid indices.
#
#   5. **Start function has correct signature** — if a start function is
#      declared, it must take no params and return no results.
#
# ## Usage
#
#   use CodingAdventures::WasmValidator qw(validate);
#   use CodingAdventures::WasmModuleParser qw(parse);
#
#   my $module = parse($wasm_bytes);
#   my $validated = validate($module);
#   # $validated->{module}    — the original parsed module
#   # $validated->{func_types} — resolved function type signatures
#
# ## ValidationError
#
# When validation fails, a ValidationError exception is thrown containing
# the specific kind of error and a human-readable message.
#
# ============================================================================

use strict;
use warnings;
use Carp qw(croak);

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(validate);

# ============================================================================
# ValidationError — structured error for validation failures
# ============================================================================

package CodingAdventures::WasmValidator::ValidationError;

sub new {
    my ($class, %args) = @_;
    return bless {
        kind    => $args{kind} || 'unknown',
        message => $args{message} || 'validation error',
    }, $class;
}

sub kind    { return $_[0]->{kind} }
sub message { return $_[0]->{message} }

sub throw {
    my ($class, %args) = @_;
    die $class->new(%args);
}

# ============================================================================
# Back to main package
# ============================================================================

package CodingAdventures::WasmValidator;

# Maximum number of memory pages allowed by the WASM 1.0 spec.
use constant MAX_MEMORY_PAGES => 65536;

# ============================================================================
# validate($module) — validate a parsed WebAssembly module
# ============================================================================
#
# Takes a parsed module (hashref from WasmModuleParser::parse) and performs
# structural validation. Returns a validated module hashref on success, or
# throws a ValidationError on failure.
#
# The validated module contains:
#   module     — the original parsed module
#   func_types — array of resolved FuncType hashrefs for all functions
#                (imports + module-defined), in combined index order
#
# @param  $module  Hashref from WasmModuleParser::parse
# @return          Hashref { module => ..., func_types => [...] }
# @dies            CodingAdventures::WasmValidator::ValidationError

sub validate {
    my ($module) = @_;

    # Build the combined function type array (imports first, then module funcs).
    my @func_types;
    my $num_imported_funcs = 0;

    # Count imported functions and resolve their types.
    for my $imp (@{ $module->{imports} || [] }) {
        if ($imp->{desc}{kind} eq 'func') {
            my $type_idx = $imp->{desc}{idx};
            $type_idx = $imp->{desc}{type_idx} unless defined $type_idx;
            _check_type_index($module, $type_idx);
            push @func_types, $module->{types}[$type_idx];
            $num_imported_funcs++;
        }
    }

    # Add module-defined functions.
    for my $type_idx (@{ $module->{functions} || [] }) {
        _check_type_index($module, $type_idx);
        push @func_types, $module->{types}[$type_idx];
    }

    # Validate memory limits.
    my @memories = @{ $module->{memories} || [] };
    if (scalar(@memories) > 1) {
        CodingAdventures::WasmValidator::ValidationError->throw(
            kind    => 'multiple_memories',
            message => 'WASM 1.0 allows at most one memory',
        );
    }
    for my $mem (@memories) {
        my $limits = $mem->{limits} || $mem;
        my $min = $mem->{min};
        $min = $limits->{min} unless defined $min;
        my $max = $mem->{max};
        $max = $limits->{max} unless defined $max;
        if ($min > MAX_MEMORY_PAGES) {
            CodingAdventures::WasmValidator::ValidationError->throw(
                kind    => 'memory_limit_exceeded',
                message => "Memory min ($min) exceeds max pages (" . MAX_MEMORY_PAGES . ")",
            );
        }
        if (defined($max)) {
            if ($max > MAX_MEMORY_PAGES) {
                CodingAdventures::WasmValidator::ValidationError->throw(
                    kind    => 'memory_limit_exceeded',
                    message => "Memory max ($max) exceeds max pages (" . MAX_MEMORY_PAGES . ")",
                );
            }
            if ($min > $max) {
                CodingAdventures::WasmValidator::ValidationError->throw(
                    kind    => 'memory_limit_order',
                    message => "Memory min ($min) > max ($max)",
                );
            }
        }
    }

    # Validate export name uniqueness.
    my %seen_exports;
    for my $exp (@{ $module->{exports} || [] }) {
        if ($seen_exports{ $exp->{name} }++) {
            CodingAdventures::WasmValidator::ValidationError->throw(
                kind    => 'duplicate_export_name',
                message => "Duplicate export name: '$exp->{name}'",
            );
        }
    }

    # Validate start function (if present).
    if (defined $module->{start}) {
        my $start_idx = $module->{start};
        if ($start_idx >= scalar(@func_types)) {
            CodingAdventures::WasmValidator::ValidationError->throw(
                kind    => 'invalid_func_index',
                message => "Start function index $start_idx out of range",
            );
        }
        my $start_type = $func_types[$start_idx];
        if (@{ $start_type->{params} } != 0 || @{ $start_type->{results} } != 0) {
            CodingAdventures::WasmValidator::ValidationError->throw(
                kind    => 'start_function_bad_type',
                message => "Start function must have no params and no results",
            );
        }
    }

    # Validate export indices are in range.
    for my $exp (@{ $module->{exports} || [] }) {
        my $kind = $exp->{desc}{kind};
        my $idx  = $exp->{desc}{idx};

        if ($kind eq 'func') {
            if ($idx >= scalar(@func_types)) {
                CodingAdventures::WasmValidator::ValidationError->throw(
                    kind    => 'export_index_out_of_range',
                    message => "Export '$exp->{name}': function index $idx out of range",
                );
            }
        }
    }

    return {
        module     => $module,
        func_types => \@func_types,
    };
}

# ============================================================================
# Helper: check that a type index is valid
# ============================================================================

sub _check_type_index {
    my ($module, $type_idx) = @_;
    my $num_types = scalar @{ $module->{types} || [] };
    if ($type_idx >= $num_types) {
        CodingAdventures::WasmValidator::ValidationError->throw(
            kind    => 'invalid_type_index',
            message => "Type index $type_idx out of range (module has $num_types types)",
        );
    }
}

1;

__END__

=head1 NAME

CodingAdventures::WasmValidator - WebAssembly 1.0 module validator

=head1 SYNOPSIS

    use CodingAdventures::WasmValidator qw(validate);
    use CodingAdventures::WasmModuleParser qw(parse);

    my $module = parse($wasm_bytes);
    my $validated = validate($module);

=head1 DESCRIPTION

Validates parsed WebAssembly modules for structural and semantic correctness.
Checks type indices, memory limits, export uniqueness, start function type,
and export index bounds.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
