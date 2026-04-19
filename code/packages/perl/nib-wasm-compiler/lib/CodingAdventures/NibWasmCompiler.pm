package CodingAdventures::NibWasmCompiler;

# ============================================================================
# CodingAdventures::NibWasmCompiler — orchestrate the whole Nib Wasm lane
# ============================================================================
#
# The package stitches together the newly-added Perl frontend stages:
#
#   Nib source
#     -> nib-parser
#     -> nib-type-checker
#     -> nib-ir-compiler
#     -> ir-to-wasm-compiler
#     -> wasm-module-encoder
#     -> wasm-module-parser + wasm-validator
#
# The result is both machine-consumable (`binary`) and human-inspectable
# (`typed_ast`, `raw_ir`, `module`, ...).
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(compile_source pack_source write_wasm_file);

use File::Basename qw(dirname);
use File::Path qw(make_path);

use CodingAdventures::IrToWasmCompiler qw(new_function_signature);
use CodingAdventures::IrToWasmValidator qw(validate);
use CodingAdventures::NibIrCompiler qw(compile release_config);
use CodingAdventures::NibParser;
use CodingAdventures::NibTypeChecker qw(check);
use CodingAdventures::WasmModuleEncoder qw(encode_module);
use CodingAdventures::WasmModuleParser qw(parse);
use CodingAdventures::WasmValidator ();

sub new {
    my ($class, %args) = @_;
    return bless {
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
    my $config = $args{build_config} // $self->{build_config} // release_config();

    my ($ast, $err);
    eval { ($ast, $err) = CodingAdventures::NibParser->parse($source); 1 }
        or _raise('parse', $@);
    _raise('parse', $err || 'parse failed') if $err || !defined $ast;

    my $type_result = check($ast);
    if (!$type_result->{ok}) {
        my $message = join(
            "\n",
            map {
                'Line ' . ($_->{line} // 1) . ', Col ' . ($_->{column} // 1) . ': ' . ($_->{message} // '')
            } @{ $type_result->{errors} || [] }
        );
        _raise('type-check', $message);
    }

    my $ir_result = eval { compile($type_result->{typed_ast}, $config) };
    _raise('ir-compile', $@) if $@;

    my $signatures = _extract_signatures($type_result->{typed_ast});
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
        ast              => $ast,
        typed_ast        => $type_result->{typed_ast},
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

sub _extract_signatures {
    my ($typed_ast) = @_;
    my @signatures = (
        new_function_signature('_start', 0, '_start'),
    );

    my $root = $typed_ast->root;
    for my $top_decl (@{ _child_nodes($root) }) {
        my $decl = _unwrap_top_decl($top_decl);
        next unless defined $decl && $decl->rule_name eq 'fn_decl';

        my $name = _first_name($decl);
        next unless defined $name;
        my @params = @{ _extract_params($decl) };

        push @signatures, new_function_signature('_fn_' . $name, scalar @params, $name);
    }

    return \@signatures;
}

sub _raise {
    my ($stage, $message) = @_;
    die CodingAdventures::NibWasmCompiler::PackageError->new(
        stage   => $stage,
        message => $message,
    );
}

sub _unwrap_top_decl {
    my ($node) = @_;
    for my $child (@{ $node->children }) {
        return $child if _is_ast_node($child);
    }
    return undef;
}

sub _child_nodes {
    my ($node) = @_;
    return [] unless defined $node && _is_ast_node($node);
    return [ grep { _is_ast_node($_) } @{ $node->children } ];
}

sub _tokens_in {
    my ($subject) = @_;
    return [] unless defined $subject;
    return [ $subject ] if ref($subject) eq 'HASH';
    return [] unless _is_ast_node($subject);

    my @tokens;
    for my $child (@{ $subject->children }) {
        push @tokens, @{ _tokens_in($child) };
    }
    return \@tokens;
}

sub _first_name {
    my ($node) = @_;
    for my $token (@{ _tokens_in($node) }) {
        return $token->{value} if ($token->{type} // '') eq 'NAME';
    }
    return undef;
}

sub _extract_params {
    my ($node) = @_;
    my ($param_list) = grep { $_->rule_name eq 'param_list' } @{ _child_nodes($node) };
    return [] unless defined $param_list;
    return [ grep { $_->rule_name eq 'param' } @{ _child_nodes($param_list) } ];
}

sub _is_ast_node {
    my ($value) = @_;
    return eval { $value->isa('CodingAdventures::Parser::ASTNode') } ? 1 : 0;
}

package CodingAdventures::NibWasmCompiler::PackageError;

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

CodingAdventures::NibWasmCompiler - end-to-end Nib to Wasm compiler

=head1 SYNOPSIS

  use CodingAdventures::NibWasmCompiler qw(compile_source);

  my $result = compile_source('fn answer() -> u4 { return 7; }');
  print length($result->{binary});

=cut
