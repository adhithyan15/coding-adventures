package CodingAdventures::WasmModuleEncoder;

# ============================================================================
# CodingAdventures::WasmModuleEncoder — WebAssembly 1.0 module encoder
# ============================================================================
#
# This package is the mirror image of the existing Perl Wasm module parser:
# it takes the structured Perl hashref representation of a module and emits
# raw `.wasm` bytes.
#
# We intentionally keep the accepted module shape close to the structures
# already used elsewhere in the Perl Wasm stack:
#
#   {
#     types     => [ { params => [...], results => [...] }, ... ],
#     imports   => [ { module => "...", name => "...", desc => {...} }, ... ],
#     functions => [ 0, 1, ... ],             # type indices
#     memories  => [ { limits => { min => 1, max => undef } } ],
#     exports   => [ { name => "main", desc => { kind => "func", idx => 0 } } ],
#     codes     => [ { locals => [...], body => $bytes }, ... ],
#     data      => [ { memory_index => 0, offset_expr => $bytes, data => $bytes } ],
#   }
#
# The encoder is permissive about a few field aliases because different Perl
# packages in this repo already use slightly different keys:
#
#   module / module_name / mod
#   idx / type_idx
#   code / body
#   func_indices / function_indices
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Carp qw(croak);
use Exporter 'import';

use CodingAdventures::WasmLeb128 qw(encode_unsigned);

our @EXPORT_OK = qw(encode_module WASM_MAGIC WASM_VERSION);

use constant WASM_MAGIC   => "\x00asm";
use constant WASM_VERSION => "\x01\x00\x00\x00";

sub encode_module {
    my ($module) = @_;
    croak 'CodingAdventures::WasmModuleEncoder::encode_module: module hashref required'
        unless ref($module) eq 'HASH';

    my @sections;

    for my $custom (@{ $module->{customs} || $module->{custom} || [] }) {
        push @sections, _section(0, _encode_custom($custom));
    }
    push @sections, _section(1, _vector($module->{types}, \&_encode_func_type))
        if @{ $module->{types} || [] };
    push @sections, _section(2, _vector($module->{imports}, \&_encode_import))
        if @{ $module->{imports} || [] };
    push @sections, _section(3, _vector($module->{functions}, \&_u32))
        if @{ $module->{functions} || [] };
    push @sections, _section(4, _vector($module->{tables}, \&_encode_table_type))
        if @{ $module->{tables} || [] };
    push @sections, _section(5, _vector($module->{memories}, \&_encode_memory_type))
        if @{ $module->{memories} || [] };
    push @sections, _section(6, _vector($module->{globals}, \&_encode_global))
        if @{ $module->{globals} || [] };
    push @sections, _section(7, _vector($module->{exports}, \&_encode_export))
        if @{ $module->{exports} || [] };
    push @sections, _section(8, _u32($module->{start}))
        if exists($module->{start}) && defined($module->{start});
    push @sections, _section(9, _vector($module->{elements}, \&_encode_element))
        if @{ $module->{elements} || [] };
    push @sections, _section(10, _vector($module->{codes} || $module->{code}, \&_encode_function_body))
        if @{ $module->{codes} || $module->{code} || [] };
    push @sections, _section(11, _vector($module->{data}, \&_encode_data_segment))
        if @{ $module->{data} || [] };

    return WASM_MAGIC . WASM_VERSION . join('', @sections);
}

sub _section {
    my ($section_id, $payload) = @_;
    return pack('C', $section_id) . _u32(length($payload)) . $payload;
}

sub _u32 {
    my ($value) = @_;
    return pack('C*', encode_unsigned($value));
}

sub _name {
    my ($text) = @_;
    my $bytes = defined($text) ? "$text" : '';
    return _u32(length($bytes)) . $bytes;
}

sub _vector {
    my ($values, $encoder) = @_;
    $values ||= [];

    my $encoded = _u32(scalar @$values);
    for my $value (@$values) {
        $encoded .= $encoder->($value);
    }
    return $encoded;
}

sub _value_types {
    my ($types) = @_;
    $types ||= [];
    return _u32(scalar @$types) . pack('C*', @$types);
}

sub _encode_func_type {
    my ($func_type) = @_;
    return pack('C', 0x60)
        . _value_types($func_type->{params})
        . _value_types($func_type->{results});
}

sub _encode_limits {
    my ($limits) = @_;
    my $min = $limits->{min} // 0;
    my $max = $limits->{max};

    return pack('C', 0x00) . _u32($min)
        unless defined $max;

    return pack('C', 0x01) . _u32($min) . _u32($max);
}

sub _encode_memory_type {
    my ($memory_type) = @_;
    return _encode_limits($memory_type->{limits} || $memory_type);
}

sub _encode_table_type {
    my ($table_type) = @_;
    my $element_type = $table_type->{element_type};
    $element_type = $table_type->{ref_type} unless defined $element_type;
    croak 'CodingAdventures::WasmModuleEncoder: table type requires element_type/ref_type'
        unless defined $element_type;

    return pack('C', $element_type)
        . _encode_limits($table_type->{limits} || {});
}

sub _encode_global_type {
    my ($global_type) = @_;
    my $value_type = $global_type->{value_type};
    $value_type = $global_type->{val_type} unless defined $value_type;
    croak 'CodingAdventures::WasmModuleEncoder: global type requires value_type/val_type'
        unless defined $value_type;

    return pack('CC', $value_type, ($global_type->{mutable} ? 0x01 : 0x00));
}

sub _encode_import {
    my ($import) = @_;
    my $module_name = $import->{module_name};
    $module_name = $import->{module} unless defined $module_name;
    $module_name = $import->{mod} unless defined $module_name;
    my $name = $import->{name};
    my $desc = $import->{desc} || {};

    croak 'CodingAdventures::WasmModuleEncoder: import requires module name'
        unless defined $module_name;
    croak 'CodingAdventures::WasmModuleEncoder: import requires field name'
        unless defined $name;

    my %kind_tag = (
        func   => 0x00,
        table  => 0x01,
        mem    => 0x02,
        memory => 0x02,
        global => 0x03,
    );
    my $kind = $desc->{kind} // '';
    croak "CodingAdventures::WasmModuleEncoder: unsupported import kind '$kind'"
        unless exists $kind_tag{$kind};

    my $payload = _name($module_name) . _name($name) . pack('C', $kind_tag{$kind});
    if ($kind eq 'func') {
        my $type_idx = $desc->{idx};
        $type_idx = $desc->{type_idx} unless defined $type_idx;
        croak 'CodingAdventures::WasmModuleEncoder: function import requires idx/type_idx'
            unless defined $type_idx;
        $payload .= _u32($type_idx);
    }
    elsif ($kind eq 'table') {
        $payload .= _encode_table_type($desc);
    }
    elsif ($kind eq 'mem' || $kind eq 'memory') {
        $payload .= _encode_limits($desc->{limits} || $desc);
    }
    elsif ($kind eq 'global') {
        $payload .= _encode_global_type($desc);
    }

    return $payload;
}

sub _encode_export {
    my ($export) = @_;
    my $desc = $export->{desc} || {};

    my %kind_tag = (
        func   => 0x00,
        table  => 0x01,
        mem    => 0x02,
        memory => 0x02,
        global => 0x03,
    );
    my $kind = $desc->{kind} // '';
    croak "CodingAdventures::WasmModuleEncoder: unsupported export kind '$kind'"
        unless exists $kind_tag{$kind};

    my $idx = $desc->{idx};
    croak 'CodingAdventures::WasmModuleEncoder: export descriptor requires idx'
        unless defined $idx;

    return _name($export->{name}) . pack('C', $kind_tag{$kind}) . _u32($idx);
}

sub _encode_global {
    my ($global) = @_;
    return _encode_global_type($global->{global_type} || $global->{type} || {})
        . ($global->{init_expr} // '');
}

sub _encode_element {
    my ($element) = @_;
    my $function_indices = $element->{function_indices};
    $function_indices = $element->{func_indices} unless defined $function_indices;
    $function_indices ||= [];

    my $payload = _u32($element->{table_index} // 0);
    $payload .= ($element->{offset_expr} // '');
    $payload .= _u32(scalar @$function_indices);
    for my $index (@$function_indices) {
        $payload .= _u32($index);
    }
    return $payload;
}

sub _encode_data_segment {
    my ($segment) = @_;
    my $data = $segment->{data} // '';
    return _u32($segment->{memory_index} // 0)
        . ($segment->{offset_expr} // '')
        . _u32(length($data))
        . $data;
}

sub _encode_function_body {
    my ($body) = @_;
    my $locals = $body->{locals} || [];
    my $groups = ref($locals->[0]) eq 'HASH'
        ? $locals
        : _group_locals($locals);

    my $payload = _u32(scalar @$groups);
    for my $group (@$groups) {
        $payload .= _u32($group->{count});
        $payload .= pack('C', $group->{type});
    }

    my $code = $body->{body};
    $code = $body->{code} unless defined $code;
    $code //= '';
    $payload .= $code;

    return _u32(length($payload)) . $payload;
}

sub _group_locals {
    my ($locals) = @_;
    return [] unless @$locals;

    my @groups;
    my $current = $locals->[0];
    my $count = 1;

    for my $value_type (@$locals[1 .. $#$locals]) {
        if ($value_type == $current) {
            $count++;
            next;
        }
        push @groups, { count => $count, type => $current };
        $current = $value_type;
        $count = 1;
    }
    push @groups, { count => $count, type => $current };
    return \@groups;
}

sub _encode_custom {
    my ($custom) = @_;
    return _name($custom->{name}) . ($custom->{data} // '');
}

1;

__END__

=head1 NAME

CodingAdventures::WasmModuleEncoder - WebAssembly 1.0 module encoder

=head1 SYNOPSIS

    use CodingAdventures::WasmModuleEncoder qw(encode_module);

    my $bytes = encode_module({
        types => [
            { params => [], results => [0x7F] },
        ],
        functions => [0],
        exports => [
            { name => 'answer', desc => { kind => 'func', idx => 0 } },
        ],
        codes => [
            { locals => [], body => "\x41\x07\x0f\x0b" },
        ],
    });

=head1 DESCRIPTION

Encodes the structured Perl Wasm module representation into raw `.wasm` bytes.

=cut
