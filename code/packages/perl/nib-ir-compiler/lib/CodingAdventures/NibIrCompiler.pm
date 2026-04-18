package CodingAdventures::NibIrCompiler;

# ============================================================================
# CodingAdventures::NibIrCompiler — lower typed Nib ASTs into generic IR
# ============================================================================
#
# This package takes the semantic information from nib-type-checker and turns
# it into the shared compiler IR already used by the rest of the repo.
#
# The generated IR stays intentionally conservative:
#
#   - one `_start` entry function
#   - one `_fn_<name>` region for each Nib function
#   - register 0 reserved as a zero value
#   - register 1 used as the expression scratch / return register
#   - registers 2+ used for locals and call arguments
#
# By keeping the lowering simple, the next stage (`ir-to-wasm-compiler`) can
# recognize the structured loop and if patterns without having to solve a full
# control-flow reconstruction problem.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(compile compile_source debug_config release_config);

use Scalar::Util qw(blessed);

use CodingAdventures::CompilerIr;
use CodingAdventures::CompilerIr::IDGenerator;
use CodingAdventures::CompilerIr::IrDataDecl;
use CodingAdventures::CompilerIr::IrImmediate;
use CodingAdventures::CompilerIr::IrInstruction;
use CodingAdventures::CompilerIr::IrLabel;
use CodingAdventures::CompilerIr::IrOp;
use CodingAdventures::CompilerIr::IrProgram;
use CodingAdventures::CompilerIr::IrRegister;
use CodingAdventures::NibTypeChecker qw(check_source);

use constant {
    _REG_ZERO    => 0,
    _REG_SCRATCH => 1,
    _REG_VARBASE => 2,
};

my %_EXPRESSION_RULE = map { $_ => 1 } qw(
    expr or_expr and_expr eq_expr cmp_expr add_expr bitwise_expr unary_expr
    primary call_expr
);

sub debug_config {
    return { insert_debug_comments => 1 };
}

sub release_config {
    return { insert_debug_comments => 0 };
}

sub compile_source {
    my ($source, $config) = @_;
    my $type_result = check_source($source);
    die $type_result->{errors}[0]{message} unless $type_result->{ok};
    return compile($type_result->{typed_ast}, $config);
}

sub compile {
    my ($typed_ast, $config) = @_;
    $config ||= release_config();

    die 'CodingAdventures::NibIrCompiler::compile: typed AST required'
        unless blessed($typed_ast) && $typed_ast->isa('CodingAdventures::NibTypeChecker::TypedAst');

    my $compiler = {
        config       => $config,
        typed_ast    => $typed_ast,
        id_gen       => CodingAdventures::CompilerIr::IDGenerator->new,
        program      => CodingAdventures::CompilerIr::IrProgram->new('_start'),
        loop_count   => 0,
        if_count     => 0,
        const_values => {},
    };

    my $root = $typed_ast->root;
    _collect_globals($compiler, $root);
    _emit_entry_point($compiler, $root);

    for my $top_decl (@{ _child_nodes($root) }) {
        my $decl = _unwrap_top_decl($top_decl);
        next unless defined $decl && $decl->rule_name eq 'fn_decl';
        _compile_function($compiler, $decl);
    }

    return { program => $compiler->{program} };
}

sub _collect_globals {
    my ($compiler, $root) = @_;

    for my $top_decl (@{ _child_nodes($root) }) {
        my $decl = _unwrap_top_decl($top_decl);
        next unless defined $decl;

        if ($decl->rule_name eq 'const_decl') {
            my $info = _extract_decl_info($decl);
            $compiler->{const_values}{ $info->{name} } = $info->{init_value}
                if defined $info->{name};
        }
        elsif ($decl->rule_name eq 'static_decl') {
            _emit_static_data($compiler, $decl);
        }
    }
}

sub _emit_entry_point {
    my ($compiler, $root) = @_;
    _emit_label($compiler, '_start');
    _emit_comment($compiler, 'program entry point: initialize v0=0, call main, halt');
    _emit(
        $compiler,
        CodingAdventures::CompilerIr::IrOp::LOAD_IMM,
        CodingAdventures::CompilerIr::IrRegister->new(_REG_ZERO),
        CodingAdventures::CompilerIr::IrImmediate->new(0),
    );
    if (_has_function_named($root, 'main')) {
        _emit(
            $compiler,
            CodingAdventures::CompilerIr::IrOp::CALL,
            CodingAdventures::CompilerIr::IrLabel->new('_fn_main'),
        );
    }
    _emit($compiler, CodingAdventures::CompilerIr::IrOp::HALT);
}

sub _compile_function {
    my ($compiler, $node) = @_;
    my $name = _first_name($node);
    return unless defined $name;

    my ($block) = grep { $_->rule_name eq 'block' } @{ _child_nodes($node) };
    return unless defined $block;

    my @params = @{ _extract_params($node) };
    my $signature_comment = 'function: ' . $name . '(' . join(', ', map { $_->[0] } @params) . ')';
    _emit_comment($compiler, $signature_comment);
    _emit_label($compiler, '_fn_' . $name);

    my %registers;
    my $next_register = _REG_VARBASE;
    for my $param (@params) {
        $registers{ $param->[0] } = $next_register++;
    }

    _compile_block($compiler, $block, \%registers, $next_register);
    _emit($compiler, CodingAdventures::CompilerIr::IrOp::RET);
}

sub _compile_block {
    my ($compiler, $block, $registers, $next_register) = @_;
    my $current = $next_register;

    for my $stmt (@{ _child_nodes($block) }) {
        next unless $stmt->rule_name eq 'stmt';
        my ($inner) = @{ _child_nodes($stmt) };
        next unless defined $inner;
        $current = _compile_statement($compiler, $inner, $registers, $current);
    }

    return $current;
}

sub _compile_statement {
    my ($compiler, $node, $registers, $next_register) = @_;

    if ($node->rule_name eq 'let_stmt') {
        return _compile_let($compiler, $node, $registers, $next_register);
    }
    if ($node->rule_name eq 'assign_stmt') {
        _compile_assign($compiler, $node, $registers);
        return $next_register;
    }
    if ($node->rule_name eq 'return_stmt') {
        _compile_return($compiler, $node, $registers);
        return $next_register;
    }
    if ($node->rule_name eq 'for_stmt') {
        return _compile_for($compiler, $node, $registers, $next_register);
    }
    if ($node->rule_name eq 'if_stmt') {
        _compile_if($compiler, $node, $registers, $next_register);
        return $next_register;
    }
    if ($node->rule_name eq 'expr_stmt') {
        my ($expr) = @{ _expression_children($node) };
        _compile_expr($compiler, $expr, $registers) if defined $expr;
    }
    return $next_register;
}

sub _compile_let {
    my ($compiler, $node, $registers, $next_register) = @_;
    my $name = _first_name($node);
    my ($expr) = @{ _expression_children($node) };
    return $next_register unless defined $name && defined $expr;

    my $destination = $next_register;
    $registers->{$name} = $destination;
    my $result_register = _compile_expr($compiler, $expr, $registers);
    if ($result_register != $destination) {
        _emit_copy($compiler, $destination, $result_register);
    }

    my $type_node = _type_node($node);
    if (defined $type_node) {
        my $type_name = _first_type_name($type_node);
        _emit_comment($compiler, "let $name: $type_name") if defined $type_name;
    }

    return $next_register + 1;
}

sub _compile_assign {
    my ($compiler, $node, $registers) = @_;
    my $name = _first_name($node);
    my ($expr) = @{ _expression_children($node) };
    return unless defined $name && defined $expr && exists $registers->{$name};

    my $value_register = _compile_expr($compiler, $expr, $registers);
    _emit_copy($compiler, $registers->{$name}, $value_register);
}

sub _compile_return {
    my ($compiler, $node, $registers) = @_;
    my ($expr) = @{ _expression_children($node) };
    if (defined $expr) {
        my $value_register = _compile_expr($compiler, $expr, $registers);
        _emit_copy($compiler, _REG_SCRATCH, $value_register) if $value_register != _REG_SCRATCH;
    }
    _emit($compiler, CodingAdventures::CompilerIr::IrOp::RET);
}

sub _compile_for {
    my ($compiler, $node, $registers, $next_register) = @_;
    my $loop_var = _first_name($node);
    my ($block) = grep { $_->rule_name eq 'block' } @{ _child_nodes($node) };
    my @exprs = @{ _expression_children($node) };
    return $next_register unless defined $loop_var && defined $block && @exprs >= 2;

    my $loop_register = $next_register;
    my $limit_register = $next_register + 1;
    my $start_label = 'loop_' . $compiler->{loop_count} . '_start';
    my $end_label = 'loop_' . $compiler->{loop_count} . '_end';
    $compiler->{loop_count}++;

    $registers->{$loop_var} = $loop_register;

    my $start_value = _compile_expr($compiler, $exprs[0], $registers);
    _emit_copy($compiler, $loop_register, $start_value) if $start_value != $loop_register;

    my $limit_value = _compile_expr($compiler, $exprs[1], $registers);
    _emit_copy($compiler, $limit_register, $limit_value) if $limit_value != $limit_register;

    _emit_label($compiler, $start_label);
    _emit(
        $compiler,
        CodingAdventures::CompilerIr::IrOp::CMP_LT,
        CodingAdventures::CompilerIr::IrRegister->new(_REG_SCRATCH),
        CodingAdventures::CompilerIr::IrRegister->new($loop_register),
        CodingAdventures::CompilerIr::IrRegister->new($limit_register),
    );
    _emit(
        $compiler,
        CodingAdventures::CompilerIr::IrOp::BRANCH_Z,
        CodingAdventures::CompilerIr::IrRegister->new(_REG_SCRATCH),
        CodingAdventures::CompilerIr::IrLabel->new($end_label),
    );

    my %nested = %{$registers};
    _compile_block($compiler, $block, \%nested, $next_register + 2);

    _emit(
        $compiler,
        CodingAdventures::CompilerIr::IrOp::ADD_IMM,
        CodingAdventures::CompilerIr::IrRegister->new($loop_register),
        CodingAdventures::CompilerIr::IrRegister->new($loop_register),
        CodingAdventures::CompilerIr::IrImmediate->new(1),
    );
    _emit($compiler, CodingAdventures::CompilerIr::IrOp::JUMP, CodingAdventures::CompilerIr::IrLabel->new($start_label));
    _emit_label($compiler, $end_label);

    return $next_register + 2;
}

sub _compile_if {
    my ($compiler, $node, $registers, $next_register) = @_;
    my ($condition) = @{ _expression_children($node) };
    return unless defined $condition;

    my $condition_register = _compile_expr($compiler, $condition, $registers);
    my $else_label = 'if_' . $compiler->{if_count} . '_else';
    my $end_label = 'if_' . $compiler->{if_count} . '_end';
    $compiler->{if_count}++;

    _emit(
        $compiler,
        CodingAdventures::CompilerIr::IrOp::BRANCH_Z,
        CodingAdventures::CompilerIr::IrRegister->new($condition_register),
        CodingAdventures::CompilerIr::IrLabel->new($else_label),
    );

    my @blocks = grep { $_->rule_name eq 'block' } @{ _child_nodes($node) };
    if (@blocks) {
        my %then_registers = %{$registers};
        _compile_block($compiler, $blocks[0], \%then_registers, $next_register);
    }

    _emit($compiler, CodingAdventures::CompilerIr::IrOp::JUMP, CodingAdventures::CompilerIr::IrLabel->new($end_label));
    _emit_label($compiler, $else_label);

    if (@blocks > 1) {
        my %else_registers = %{$registers};
        _compile_block($compiler, $blocks[1], \%else_registers, $next_register);
    }

    _emit_label($compiler, $end_label);
}

sub _compile_expr {
    my ($compiler, $subject, $registers) = @_;
    return _REG_SCRATCH unless defined $subject;

    if (!_is_ast_node($subject)) {
        return _compile_token_expr($compiler, $subject, $registers);
    }

    if ($subject->rule_name eq 'call_expr') {
        return _compile_call_expr($compiler, $subject, $registers);
    }
    if ($subject->rule_name eq 'primary') {
        return _compile_primary($compiler, $subject, $registers);
    }
    if ($subject->rule_name eq 'add_expr') {
        return _compile_add_expr($compiler, $subject, $registers);
    }
    if ($subject->rule_name eq 'or_expr'
        || $subject->rule_name eq 'and_expr'
        || $subject->rule_name eq 'eq_expr'
        || $subject->rule_name eq 'cmp_expr'
        || $subject->rule_name eq 'bitwise_expr'
        || $subject->rule_name eq 'unary_expr'
        || $subject->rule_name eq 'expr') {
        return _compile_compound_expr($compiler, $subject, $registers);
    }
    if (@{ $subject->children } == 1) {
        return _compile_expr($compiler, $subject->children->[0], $registers);
    }
    return _REG_SCRATCH;
}

sub _compile_token_expr {
    my ($compiler, $token, $registers) = @_;
    my $type = $token->{type} // '';
    my $value = $token->{value} // '';

    if ($type eq 'INT_LIT' || $type eq 'HEX_LIT') {
        my $parsed = _parse_literal($value, $type);
        _emit(
            $compiler,
            CodingAdventures::CompilerIr::IrOp::LOAD_IMM,
            CodingAdventures::CompilerIr::IrRegister->new(_REG_SCRATCH),
            CodingAdventures::CompilerIr::IrImmediate->new($parsed),
        );
        return _REG_SCRATCH;
    }

    if ($value eq 'true' || $value eq 'false') {
        _emit(
            $compiler,
            CodingAdventures::CompilerIr::IrOp::LOAD_IMM,
            CodingAdventures::CompilerIr::IrRegister->new(_REG_SCRATCH),
            CodingAdventures::CompilerIr::IrImmediate->new($value eq 'true' ? 1 : 0),
        );
        return _REG_SCRATCH;
    }

    return $registers->{$value} if exists $registers->{$value};

    if (exists $compiler->{const_values}{$value}) {
        _emit(
            $compiler,
            CodingAdventures::CompilerIr::IrOp::LOAD_IMM,
            CodingAdventures::CompilerIr::IrRegister->new(_REG_SCRATCH),
            CodingAdventures::CompilerIr::IrImmediate->new($compiler->{const_values}{$value}),
        );
    }

    return _REG_SCRATCH;
}

sub _compile_primary {
    my ($compiler, $node, $registers) = @_;
    return _REG_SCRATCH unless @{ $node->children };
    return _compile_expr($compiler, $node->children->[0], $registers);
}

sub _compile_call_expr {
    my ($compiler, $node, $registers) = @_;
    my $name = _first_name($node);
    return _REG_SCRATCH unless defined $name;

    my @args;
    my ($arg_list) = grep { $_->rule_name eq 'arg_list' } @{ _child_nodes($node) };
    @args = @{ _expression_children($arg_list) } if defined $arg_list;

    for my $index (0 .. $#args) {
        my $value_register = _compile_expr($compiler, $args[$index], $registers);
        my $destination = _REG_VARBASE + $index;
        _emit_copy($compiler, $destination, $value_register) if $value_register != $destination;
    }

    _emit($compiler, CodingAdventures::CompilerIr::IrOp::CALL, CodingAdventures::CompilerIr::IrLabel->new('_fn_' . $name));
    return _REG_SCRATCH;
}

sub _compile_compound_expr {
    my ($compiler, $node, $registers) = @_;
    return _compile_expr($compiler, $node->children->[0], $registers) if @{ $node->children } == 1;

    if ($node->rule_name eq 'unary_expr' && @{ $node->children } >= 2) {
        my $operator = _first_token_value($node);
        my $operand_register = _compile_expr($compiler, $node->children->[1], $registers);
        return _emit_unary($compiler, $operator, $operand_register, $node);
    }

    my $left_register = _compile_expr($compiler, $node->children->[0], $registers);
    for (my $index = 1; $index < @{ $node->children } - 1; $index += 2) {
        my $token = $node->children->[$index];
        next unless ref($token) eq 'HASH';
        my $right_register = _compile_expr($compiler, $node->children->[$index + 1], $registers);
        $left_register = _emit_binary($compiler, $token->{value} // '', $left_register, $right_register);
    }
    return $left_register;
}

sub _compile_add_expr {
    my ($compiler, $node, $registers) = @_;
    return _compile_expr($compiler, $node->children->[0], $registers) if @{ $node->children } == 1;

    my $left_register = _compile_expr($compiler, $node->children->[0], $registers);
    for (my $index = 1; $index < @{ $node->children } - 1; $index += 2) {
        my $token = $node->children->[$index];
        next unless ref($token) eq 'HASH';
        my $right_register = _compile_expr($compiler, $node->children->[$index + 1], $registers);
        my $nib_type = $compiler->{typed_ast}->type_of($node) || 'u8';
        $left_register = _emit_add_op($compiler, $token->{value} // '', $left_register, $right_register, $nib_type);
    }
    return $left_register;
}

sub _emit_unary {
    my ($compiler, $operator, $operand_register, $node) = @_;

    if ($operator eq '!') {
        _emit(
            $compiler,
            CodingAdventures::CompilerIr::IrOp::CMP_EQ,
            CodingAdventures::CompilerIr::IrRegister->new(_REG_SCRATCH),
            CodingAdventures::CompilerIr::IrRegister->new($operand_register),
            CodingAdventures::CompilerIr::IrRegister->new(_REG_ZERO),
        );
        return _REG_SCRATCH;
    }

    my $nib_type = $compiler->{typed_ast}->type_of($node) || 'u4';
    my $mask = $nib_type eq 'u8' ? 255 : 15;
    _emit(
        $compiler,
        CodingAdventures::CompilerIr::IrOp::LOAD_IMM,
        CodingAdventures::CompilerIr::IrRegister->new(_REG_SCRATCH),
        CodingAdventures::CompilerIr::IrImmediate->new($mask),
    );
    _emit(
        $compiler,
        CodingAdventures::CompilerIr::IrOp::SUB,
        CodingAdventures::CompilerIr::IrRegister->new(_REG_SCRATCH),
        CodingAdventures::CompilerIr::IrRegister->new(_REG_SCRATCH),
        CodingAdventures::CompilerIr::IrRegister->new($operand_register),
    );
    return _REG_SCRATCH;
}

sub _emit_binary {
    my ($compiler, $operator, $left_register, $right_register) = @_;

    if ($operator eq '==') {
        _emit_cmp($compiler, CodingAdventures::CompilerIr::IrOp::CMP_EQ, $left_register, $right_register);
        return _REG_SCRATCH;
    }
    if ($operator eq '!=') {
        _emit_cmp($compiler, CodingAdventures::CompilerIr::IrOp::CMP_NE, $left_register, $right_register);
        return _REG_SCRATCH;
    }
    if ($operator eq '<') {
        _emit_cmp($compiler, CodingAdventures::CompilerIr::IrOp::CMP_LT, $left_register, $right_register);
        return _REG_SCRATCH;
    }
    if ($operator eq '>') {
        _emit_cmp($compiler, CodingAdventures::CompilerIr::IrOp::CMP_GT, $left_register, $right_register);
        return _REG_SCRATCH;
    }
    if ($operator eq '<=') {
        _emit_cmp($compiler, CodingAdventures::CompilerIr::IrOp::CMP_GT, $right_register, $left_register);
        return _REG_SCRATCH;
    }
    if ($operator eq '>=') {
        _emit_cmp($compiler, CodingAdventures::CompilerIr::IrOp::CMP_LT, $right_register, $left_register);
        return _REG_SCRATCH;
    }
    if ($operator eq '&&' || $operator eq 'and') {
        _emit_rr($compiler, CodingAdventures::CompilerIr::IrOp::AND, $left_register, $right_register);
        return _REG_SCRATCH;
    }
    if ($operator eq '||' || $operator eq 'or') {
        _emit_rr($compiler, CodingAdventures::CompilerIr::IrOp::ADD, $left_register, $right_register);
        _emit_cmp($compiler, CodingAdventures::CompilerIr::IrOp::CMP_NE, _REG_SCRATCH, _REG_ZERO);
        return _REG_SCRATCH;
    }
    if ($operator eq '&') {
        _emit_rr($compiler, CodingAdventures::CompilerIr::IrOp::AND, $left_register, $right_register);
        return _REG_SCRATCH;
    }

    return $left_register;
}

sub _emit_add_op {
    my ($compiler, $operator, $left_register, $right_register, $nib_type) = @_;

    if ($operator eq '+%' || $operator eq '+' || $operator eq '+?') {
        _emit_rr($compiler, CodingAdventures::CompilerIr::IrOp::ADD, $left_register, $right_register);
        if ($operator eq '+%') {
            my $mask = $nib_type eq 'u4' ? 15 : 255;
            _emit(
                $compiler,
                CodingAdventures::CompilerIr::IrOp::AND_IMM,
                CodingAdventures::CompilerIr::IrRegister->new(_REG_SCRATCH),
                CodingAdventures::CompilerIr::IrRegister->new(_REG_SCRATCH),
                CodingAdventures::CompilerIr::IrImmediate->new($mask),
            );
        }
        return _REG_SCRATCH;
    }

    if ($operator eq '-') {
        _emit_rr($compiler, CodingAdventures::CompilerIr::IrOp::SUB, $left_register, $right_register);
        return _REG_SCRATCH;
    }

    return $left_register;
}

sub _emit_cmp {
    my ($compiler, $opcode, $left_register, $right_register) = @_;
    _emit(
        $compiler,
        $opcode,
        CodingAdventures::CompilerIr::IrRegister->new(_REG_SCRATCH),
        CodingAdventures::CompilerIr::IrRegister->new($left_register),
        CodingAdventures::CompilerIr::IrRegister->new($right_register),
    );
}

sub _emit_rr {
    my ($compiler, $opcode, $left_register, $right_register) = @_;
    _emit(
        $compiler,
        $opcode,
        CodingAdventures::CompilerIr::IrRegister->new(_REG_SCRATCH),
        CodingAdventures::CompilerIr::IrRegister->new($left_register),
        CodingAdventures::CompilerIr::IrRegister->new($right_register),
    );
}

sub _emit_copy {
    my ($compiler, $destination, $source) = @_;
    _emit(
        $compiler,
        CodingAdventures::CompilerIr::IrOp::ADD_IMM,
        CodingAdventures::CompilerIr::IrRegister->new($destination),
        CodingAdventures::CompilerIr::IrRegister->new($source),
        CodingAdventures::CompilerIr::IrImmediate->new(0),
    );
}

sub _emit_static_data {
    my ($compiler, $node) = @_;
    my $info = _extract_decl_info($node);
    return unless defined $info->{name} && defined $info->{nib_type};

    _emit_comment($compiler, 'static ' . $info->{name} . ': ' . $info->{nib_type});
    $compiler->{program}->add_data(
        CodingAdventures::CompilerIr::IrDataDecl->new(
            label => $info->{name},
            size  => _type_size_bytes($info->{nib_type}),
            init  => $info->{init_value},
        ),
    );
}

sub _extract_decl_info {
    my ($node) = @_;
    my $name = _first_name($node);
    my $type_node = _type_node($node);
    my ($expr) = @{ _expression_children($node) };

    return {
        name       => $name,
        nib_type   => defined($type_node) ? _first_type_name($type_node) : undef,
        init_value => defined($expr) ? _extract_const_int($expr) : 0,
    };
}

sub _extract_const_int {
    my ($subject) = @_;
    return 0 unless defined $subject;

    if (!_is_ast_node($subject)) {
        my $type = $subject->{type} // '';
        return _parse_literal($subject->{value}, $type) if $type eq 'INT_LIT' || $type eq 'HEX_LIT';
        return ($subject->{value} // '') eq 'true' ? 1 : 0 if ($subject->{value} // '') eq 'true' || ($subject->{value} // '') eq 'false';
        return 0;
    }

    if ($subject->rule_name eq 'add_expr' && @{ $subject->children } == 3) {
        my $left = _extract_const_int($subject->children->[0]);
        my $right = _extract_const_int($subject->children->[2]);
        my $operator = ref($subject->children->[1]) eq 'HASH' ? ($subject->children->[1]{value} // '') : '';
        return $left + $right if $operator eq '+' || $operator eq '+%' || $operator eq '+?';
        return $left - $right if $operator eq '-';
    }

    return _extract_const_int($subject->children->[0]) if @{ $subject->children };
    return 0;
}

sub _parse_literal {
    my ($value, $type) = @_;
    return hex($value) if $type eq 'HEX_LIT';
    return int($value);
}

sub _type_size_bytes {
    return 1;
}

sub _has_function_named {
    my ($root, $wanted) = @_;
    for my $top_decl (@{ _child_nodes($root) }) {
        my $decl = _unwrap_top_decl($top_decl);
        next unless defined $decl && $decl->rule_name eq 'fn_decl';
        return 1 if (_first_name($decl) || '') eq $wanted;
    }
    return 0;
}

sub _extract_params {
    my ($node) = @_;
    my ($param_list) = grep { $_->rule_name eq 'param_list' } @{ _child_nodes($node) };
    return [] unless defined $param_list;

    my @params;
    for my $param (grep { $_->rule_name eq 'param' } @{ _child_nodes($param_list) }) {
        my $name = _first_name($param);
        my $type = _first_type_name(_type_node($param));
        push @params, [ $name, $type ] if defined $name && defined $type;
    }
    return \@params;
}

sub _emit {
    my ($compiler, $opcode, @operands) = @_;
    my $id = $compiler->{id_gen}->next;
    $compiler->{program}->add_instruction(
        CodingAdventures::CompilerIr::IrInstruction->new(
            opcode   => $opcode,
            operands => \@operands,
            id       => $id,
        )
    );
}

sub _emit_label {
    my ($compiler, $name) = @_;
    $compiler->{program}->add_instruction(
        CodingAdventures::CompilerIr::IrInstruction->new(
            opcode   => CodingAdventures::CompilerIr::IrOp::LABEL,
            operands => [ CodingAdventures::CompilerIr::IrLabel->new($name) ],
            id       => -1,
        )
    );
}

sub _emit_comment {
    my ($compiler, $text) = @_;
    return unless $compiler->{config}{insert_debug_comments};
    $compiler->{program}->add_instruction(
        CodingAdventures::CompilerIr::IrInstruction->new(
            opcode   => CodingAdventures::CompilerIr::IrOp::COMMENT,
            operands => [ CodingAdventures::CompilerIr::IrLabel->new($text) ],
            id       => -1,
        )
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

sub _expression_children {
    my ($node) = @_;
    return [ grep { $_EXPRESSION_RULE{ $_->rule_name } } @{ _child_nodes($node) } ];
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

sub _first_type_name {
    my ($node) = @_;
    return undef unless defined $node;
    my ($token) = @{ _tokens_in($node) };
    return defined $token ? ($token->{value} // undef) : undef;
}

sub _type_node {
    my ($node) = @_;
    for my $child (@{ _child_nodes($node) }) {
        return $child if $child->rule_name eq 'type';
    }
    return undef;
}

sub _first_token_value {
    my ($node) = @_;
    my ($token) = @{ _tokens_in($node) };
    return defined $token ? ($token->{value} // '') : '';
}

sub _is_ast_node {
    my ($value) = @_;
    return blessed($value) && $value->isa('CodingAdventures::Parser::ASTNode') ? 1 : 0;
}

1;

__END__

=head1 NAME

CodingAdventures::NibIrCompiler - lower typed Nib ASTs into compiler IR

=head1 SYNOPSIS

  use CodingAdventures::NibTypeChecker qw(check_source);
  use CodingAdventures::NibIrCompiler qw(compile release_config);

  my $typed = check_source('fn main() -> u4 { return 7; }');
  my $ir    = compile($typed->{typed_ast}, release_config());

=head1 DESCRIPTION

Bridges the Nib semantic layer and the shared target-independent compiler IR.

=cut
