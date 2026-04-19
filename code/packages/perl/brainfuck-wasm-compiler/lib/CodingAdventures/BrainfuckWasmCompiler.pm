package CodingAdventures::BrainfuckWasmCompiler;

# ============================================================================
# CodingAdventures::BrainfuckWasmCompiler — source to `.wasm` in one package
# ============================================================================
#
# This orchestrator mirrors the higher-level packages in the Python tree:
#
#   Brainfuck source
#      -> parse
#      -> Brainfuck IR
#      -> Wasm module hashref
#      -> encoded bytes
#      -> parse+validate round-trip
#
# The returned package result keeps those intermediate artifacts around so
# tests and downstream tools can inspect any stage of the pipeline.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(compile_source pack_source write_wasm_file);

use Carp qw(croak);
use File::Basename qw(dirname);
use File::Path qw(make_path);
use Scalar::Util qw(blessed);

use CodingAdventures::Brainfuck::Parser;
use CodingAdventures::BrainfuckIrCompiler qw(compile);
use CodingAdventures::BrainfuckIrCompiler::BuildConfig;
use CodingAdventures::IrToWasmCompiler qw(new_function_signature);
use CodingAdventures::IrToWasmValidator qw(validate);
use CodingAdventures::WasmModuleEncoder qw(encode_module);
use CodingAdventures::WasmModuleParser qw(parse);
use CodingAdventures::WasmValidator ();

sub new {
    my ($class, %args) = @_;
    return bless {
        filename     => $args{filename} // 'program.bf',
        build_config => $args{build_config},
    }, $class;
}

sub compile_source {
    my ($self_or_source, @rest) = @_;
    if (!ref($self_or_source) && $self_or_source eq __PACKAGE__) {
        return __PACKAGE__->new()->_compile_source(@rest);
    }
    return __PACKAGE__->new()->_compile_source($self_or_source, @rest)
        unless ref($self_or_source);
    return $self_or_source->_compile_source(@rest);
}

sub pack_source {
    return compile_source(@_);
}

sub write_wasm_file {
    my ($self_or_source, @rest) = @_;
    if (!ref($self_or_source) && $self_or_source eq __PACKAGE__) {
        return __PACKAGE__->new()->_write_wasm_file(@rest);
    }
    return __PACKAGE__->new()->_write_wasm_file($self_or_source, @rest)
        unless ref($self_or_source);
    return $self_or_source->_write_wasm_file(@rest);
}

sub _compile_source {
    my ($self, $source, %args) = @_;
    my $filename = $args{filename} // $self->{filename};
    my $config = $args{build_config} // $self->{build_config} // CodingAdventures::BrainfuckIrCompiler::BuildConfig->release_config;

    my $ast = eval { CodingAdventures::Brainfuck::Parser->parse($source) };
    _raise('parse', $@) if $@;

    my $ir_result = eval { compile($ast, $filename, $config) };
    _raise('ir-compile', $@) if $@;

    my $signatures = [
        new_function_signature('_start', 0, '_start'),
    ];

    my $lowering_errors = validate($ir_result->{program}, $signatures);
    if (@$lowering_errors) {
        _raise('validate-ir', $lowering_errors->[0]{message} || $lowering_errors->[0]{rule} || 'IR validation failed');
    }

    my $module = eval { CodingAdventures::IrToWasmCompiler::compile($ir_result->{program}, $signatures) };
    _raise('lower', $@) if $@;

    my $binary = eval { encode_module($module) };
    _raise('encode', $@) if $@;

    my $parsed = eval { parse($binary) };
    _raise('parse-wasm', $@) if $@;

    my $validated = eval { CodingAdventures::WasmValidator::validate($parsed) };
    _raise('validate-wasm', $@) if $@;

    return {
        source           => $source,
        filename         => $filename,
        ast              => $ast,
        raw_ir           => $ir_result->{program},
        optimized_ir     => $ir_result->{program},
        module           => $module,
        parsed_module    => $parsed,
        validated_module => $validated,
        binary           => $binary,
        wasm_path        => undef,
    };
}

sub _write_wasm_file {
    my ($self, $source, $output_path, %args) = @_;
    my $result = $self->_compile_source($source, %args);

    my $directory = dirname($output_path);
    make_path($directory) if defined $directory && length($directory) && !-d $directory;

    open my $fh, '>:raw', $output_path
        or _raise('write', "unable to open '$output_path' for writing: $!");
    print {$fh} $result->{binary}
        or _raise('write', "unable to write '$output_path': $!");
    close $fh
        or _raise('write', "unable to close '$output_path': $!");

    $result->{wasm_path} = $output_path;
    return $result;
}

sub _raise {
    my ($stage, $message) = @_;
    die CodingAdventures::BrainfuckWasmCompiler::PackageError->new(
        stage   => $stage,
        message => $message,
    );
}

package CodingAdventures::BrainfuckWasmCompiler::PackageError;

use strict;
use warnings;
use overload q{""} => 'as_string', fallback => 1;

sub new {
    my ($class, %args) = @_;
    return bless {
        stage   => $args{stage},
        message => $args{message},
    }, $class;
}

sub stage   { $_[0]->{stage} }
sub message { $_[0]->{message} }

sub as_string {
    my ($self) = @_;
    return '[' . ($self->{stage} // 'error') . '] ' . ($self->{message} // '');
}

1;

__END__

=head1 NAME

CodingAdventures::BrainfuckWasmCompiler - end-to-end Brainfuck to Wasm compiler

=head1 SYNOPSIS

  use CodingAdventures::BrainfuckWasmCompiler qw(compile_source);

  my $result = compile_source('+++++.');
  print length($result->{binary});

=cut
