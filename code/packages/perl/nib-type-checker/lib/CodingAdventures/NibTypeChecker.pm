package CodingAdventures::NibTypeChecker;

# ============================================================================
# CodingAdventures::NibTypeChecker — semantic checks for the Nib language
# ============================================================================
#
# Nib's parser tells us whether the source *fits the grammar*.
# The type checker answers the next question:
#
#   "Does this program make semantic sense?"
#
# In concrete terms we verify:
#
#   - every variable is declared before use
#   - assignments preserve the declared type
#   - function calls use the right arity and argument types
#   - loop bounds are numeric
#   - boolean operators receive booleans
#   - comparison operators compare compatible operands
#
# The result mirrors the protocol used elsewhere in the repo:
#
#   {
#     typed_ast => TypedAst object,
#     errors    => [ diagnostics... ],
#     ok        => 1|0,
#   }
#
# The typed AST simply wraps the original parser AST plus a side table that
# records the inferred type for expression nodes. Later passes, such as the
# IR compiler, can ask "what type did the checker assign to this node?"
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(check check_source);

use Scalar::Util qw(blessed refaddr);

use CodingAdventures::NibParser;
use CodingAdventures::TypeCheckerProtocol qw(
    new_type_check_result
    new_type_error_diagnostic
);

use constant {
    TYPE_U4   => 'u4',
    TYPE_U8   => 'u8',
    TYPE_BCD  => 'bcd',
    TYPE_BOOL => 'bool',
};

my %_EXPRESSION_RULE = map { $_ => 1 } qw(
    expr or_expr and_expr eq_expr cmp_expr add_expr bitwise_expr unary_expr
    primary call_expr
);

sub check_source {
    my ($source) = @_;

    my ($ast, $err);
    eval { ($ast, $err) = CodingAdventures::NibParser->parse($source); 1 }
        or return new_type_check_result(
            undef,
            [ new_type_error_diagnostic("$@", 1, 1) ],
        );
    if ($err || !defined $ast) {
        return new_type_check_result(
            undef,
            [ new_type_error_diagnostic($err || 'parse failed', 1, 1) ],
        );
    }

    return check($ast);
}

sub check {
    my ($ast) = @_;

    my $state = {
        errors => [],
        types  => {},
        scopes => [ {} ],
    };

    _collect_program($state, $ast) if defined $ast;
    _check_program($state, $ast)   if defined $ast;

    return new_type_check_result(
        CodingAdventures::NibTypeChecker::TypedAst->new(
            root  => $ast,
            types => $state->{types},
        ),
        [ @{ $state->{errors} } ],
    );
}

sub _collect_program {
    my ($state, $root) = @_;

    for my $top_decl (@{ _child_nodes($root) }) {
        my $decl = _unwrap_top_decl($top_decl);
        next unless defined $decl;

        if ($decl->rule_name eq 'const_decl') {
            _collect_const_or_static($state, $decl, 1);
        }
        elsif ($decl->rule_name eq 'static_decl') {
            _collect_const_or_static($state, $decl, 0);
        }
        elsif ($decl->rule_name eq 'fn_decl') {
            _collect_function_signature($state, $decl);
        }
    }
}

sub _check_program {
    my ($state, $root) = @_;

    for my $top_decl (@{ _child_nodes($root) }) {
        my $decl = _unwrap_top_decl($top_decl);
        next unless defined $decl && $decl->rule_name eq 'fn_decl';
        _check_function_body($state, $decl);
    }
}

sub _collect_const_or_static {
    my ($state, $node, $is_const) = @_;
    my $name = _first_name_token($node);
    my $type_node = _type_node($node);
    return unless defined $name && defined $type_node;

    my $nib_type = _resolve_type_node($state, $type_node);
    return unless defined $nib_type;

    _define_global(
        $state,
        $name->{value},
        {
            name      => $name->{value},
            nib_type  => $nib_type,
            has_type  => 1,
            is_const  => $is_const ? 1 : 0,
            is_static => $is_const ? 0 : 1,
        },
    );
}

sub _collect_function_signature {
    my ($state, $node) = @_;
    my $name = _first_name_token($node);
    return unless defined $name;

    my $return_type = TYPE_U4;
    my $has_return  = 0;
    my @params;

    for my $child (@{ _child_nodes($node) }) {
        if ($child->rule_name eq 'param_list') {
            @params = @{ _extract_params($state, $child) };
        }
        elsif ($child->rule_name eq 'type') {
            my $resolved = _resolve_type_node($state, $child);
            if (defined $resolved) {
                $return_type = $resolved;
                $has_return = 1;
            }
        }
    }

    _define_global(
        $state,
        $name->{value},
        {
            name           => $name->{value},
            is_fn          => 1,
            fn_params      => \@params,
            fn_return_type => $return_type,
            has_return     => $has_return,
        },
    );
}

sub _check_function_body {
    my ($state, $node) = @_;
    my $name = _first_name_token($node);
    return unless defined $name;

    my $symbol = _lookup($state, $name->{value});
    return unless defined $symbol;

    my ($block) = grep { $_->rule_name eq 'block' } @{ _child_nodes($node) };
    return unless defined $block;

    _push_scope($state);
    for my $param (@{ $symbol->{fn_params} || [] }) {
        _define_local(
            $state,
            $param->[0],
            {
                name     => $param->[0],
                nib_type => $param->[1],
                has_type => 1,
            },
        );
    }

    _check_block($state, $block, $symbol->{fn_return_type}, $symbol->{has_return}, 0);
    _pop_scope($state);
}

sub _check_block {
    my ($state, $block, $expected_return, $has_return, $create_scope) = @_;
    _push_scope($state) if $create_scope;

    for my $stmt (@{ _child_nodes($block) }) {
        next unless $stmt->rule_name eq 'stmt';
        my ($inner) = @{ _child_nodes($stmt) };
        next unless defined $inner;
        _check_statement($state, $inner, $expected_return, $has_return);
    }

    _pop_scope($state) if $create_scope;
}

sub _check_statement {
    my ($state, $node, $expected_return, $has_return) = @_;

    if ($node->rule_name eq 'let_stmt') {
        _check_let_statement($state, $node);
    }
    elsif ($node->rule_name eq 'assign_stmt') {
        _check_assign_statement($state, $node);
    }
    elsif ($node->rule_name eq 'return_stmt') {
        _check_return_statement($state, $node, $expected_return, $has_return);
    }
    elsif ($node->rule_name eq 'for_stmt') {
        _check_for_statement($state, $node, $expected_return, $has_return);
    }
    elsif ($node->rule_name eq 'if_stmt') {
        _check_if_statement($state, $node, $expected_return, $has_return);
    }
    elsif ($node->rule_name eq 'expr_stmt') {
        my ($expr) = @{ _expression_children($node) };
        _check_expression($state, $expr) if defined $expr;
    }
}

sub _check_let_statement {
    my ($state, $node) = @_;
    my $name = _first_name_token($node);
    my $type_node = _type_node($node);
    my ($expr) = @{ _expression_children($node) };
    return unless defined $name && defined $type_node && defined $expr;

    my $declared = _resolve_type_node($state, $type_node);
    return unless defined $declared;

    my $actual = _check_expression($state, $expr);
    if (defined $actual && !_is_numeric_literal_expr($expr) && !_types_are_compatible($declared, $actual)) {
        _error(
            $state,
            "Cannot initialize '$name->{value}' of type '$declared' with expression of type '$actual'.",
            $expr,
        );
    }

    _define_local(
        $state,
        $name->{value},
        {
            name     => $name->{value},
            nib_type => $declared,
            has_type => 1,
        },
    );
}

sub _check_assign_statement {
    my ($state, $node) = @_;
    my $name = _first_name_token($node);
    my ($expr) = @{ _expression_children($node) };
    return unless defined $name && defined $expr;

    my $symbol = _lookup($state, $name->{value});
    if (!defined $symbol || !$symbol->{has_type}) {
        _error($state, "'$name->{value}' is not defined.", $name);
        return;
    }

    my $actual = _check_expression($state, $expr);
    if (defined $actual && !_is_numeric_literal_expr($expr) && !_types_are_compatible($symbol->{nib_type}, $actual)) {
        _error(
            $state,
            "Cannot assign expression of type '$actual' to '$name->{value}' of type '$symbol->{nib_type}'.",
            $expr,
        );
    }
}

sub _check_return_statement {
    my ($state, $node, $expected_return, $has_return) = @_;
    my ($expr) = @{ _expression_children($node) };
    return unless defined $expr;

    my $actual = _check_expression($state, $expr);
    if ($has_return && defined $actual && !_types_are_compatible($expected_return, $actual)) {
        _error(
            $state,
            "Return type mismatch: expected '$expected_return' but got '$actual'.",
            $expr,
        );
    }
}

sub _check_for_statement {
    my ($state, $node, $expected_return, $has_return) = @_;
    my $loop_var = _first_name_token($node);
    my $loop_type_node = _type_node($node);
    my ($block) = grep { $_->rule_name eq 'block' } @{ _child_nodes($node) };
    my @exprs = @{ _expression_children($node) };

    for my $expr (@exprs) {
        my $expr_type = _check_expression($state, $expr);
        if (defined $expr_type && !_is_numeric($expr_type)) {
            _error($state, "For-loop bounds must be numeric, but got '$expr_type'.", $expr);
        }
    }

    return unless defined $loop_var && defined $loop_type_node && defined $block;
    my $loop_type = _resolve_type_node($state, $loop_type_node);
    return unless defined $loop_type;

    _push_scope($state);
    _define_local(
        $state,
        $loop_var->{value},
        {
            name     => $loop_var->{value},
            nib_type => $loop_type,
            has_type => 1,
        },
    );
    _check_block($state, $block, $expected_return, $has_return, 0);
    _pop_scope($state);
}

sub _check_if_statement {
    my ($state, $node, $expected_return, $has_return) = @_;
    my ($condition) = @{ _expression_children($node) };
    if (defined $condition) {
        my $condition_type = _check_expression($state, $condition);
        if (defined $condition_type && $condition_type ne TYPE_BOOL) {
            _error($state, "The condition of 'if' must have type 'bool', but got '$condition_type'.", $condition);
        }
    }

    for my $block (grep { $_->rule_name eq 'block' } @{ _child_nodes($node) }) {
        _check_block($state, $block, $expected_return, $has_return, 1);
    }
}

sub _check_expression {
    my ($state, $subject) = @_;
    return undef unless defined $subject;

    if (!_is_ast_node($subject)) {
        my $type = _check_token_expression($state, $subject);
        return $type;
    }

    my $result;
    if ($subject->rule_name eq 'call_expr') {
        $result = _check_call_expression($state, $subject);
    }
    elsif ($subject->rule_name eq 'primary') {
        $result = _check_primary($state, $subject);
    }
    elsif ($subject->rule_name eq 'add_expr') {
        $result = _check_add_expression($state, $subject);
    }
    elsif ($subject->rule_name eq 'or_expr'
        || $subject->rule_name eq 'and_expr'
        || $subject->rule_name eq 'eq_expr'
        || $subject->rule_name eq 'cmp_expr'
        || $subject->rule_name eq 'bitwise_expr'
        || $subject->rule_name eq 'unary_expr'
        || $subject->rule_name eq 'expr') {
        $result = _check_compound_expression($state, $subject);
    }
    elsif (@{ $subject->children } == 1) {
        $result = _check_expression($state, $subject->children->[0]);
    }

    if (defined $result) {
        $state->{types}{ refaddr($subject) } = $result;
    }
    return $result;
}

sub _check_token_expression {
    my ($state, $token) = @_;
    return undef unless ref($token) eq 'HASH';

    return TYPE_U4   if ($token->{type} // '') eq 'INT_LIT';
    return TYPE_U4   if ($token->{type} // '') eq 'HEX_LIT';
    return TYPE_BOOL if ($token->{value} // '') eq 'true' || ($token->{value} // '') eq 'false';
    return undef unless ($token->{type} // '') eq 'NAME';

    my $symbol = _lookup($state, $token->{value});
    if (!defined $symbol || !$symbol->{has_type}) {
        _error($state, "'$token->{value}' is not defined.", $token);
        return undef;
    }
    if ($symbol->{is_fn}) {
        _error($state, "'$token->{value}' is a function. Use parentheses to call it.", $token);
        return undef;
    }
    return $symbol->{nib_type};
}

sub _check_compound_expression {
    my ($state, $node) = @_;
    return _check_expression($state, $node->children->[0]) if @{ $node->children } == 1;

    if ($node->rule_name eq 'or_expr' || $node->rule_name eq 'and_expr') {
        for my $expr (@{ _expression_children($node) }) {
            my $expr_type = _check_expression($state, $expr);
            if (defined $expr_type && $expr_type ne TYPE_BOOL) {
                _error($state, 'Logical operators require bool operands.', $expr);
            }
        }
        return TYPE_BOOL;
    }

    if ($node->rule_name eq 'eq_expr' || $node->rule_name eq 'cmp_expr') {
        my @types = grep { defined $_ } map { _check_expression($state, $_) } @{ _expression_children($node) };
        if (@types >= 2 && $types[0] ne $types[1]) {
            _error($state, "Comparison operands must have the same type. Got '$types[0]' and '$types[1]'.", $node);
        }
        if ($node->rule_name eq 'cmp_expr' && @types && !_is_numeric($types[0])) {
            _error($state, "Comparison operands must be numeric, but got '$types[0]'.", $node);
        }
        return TYPE_BOOL;
    }

    if ($node->rule_name eq 'bitwise_expr') {
        my @types = grep { defined $_ } map { _check_expression($state, $_) } @{ _expression_children($node) };
        if (@types >= 2 && $types[0] ne $types[1]) {
            _error($state, "Bitwise operands must have the same type. Got '$types[0]' and '$types[1]'.", $node);
        }
        if (@types && !_is_numeric($types[0])) {
            _error($state, "Bitwise operands must be numeric, but got '$types[0]'.", $node);
        }
        return $types[0];
    }

    if ($node->rule_name eq 'unary_expr' && @{ $node->children } >= 2) {
        my $operator = _first_token_value($node);
        my $operand_type = _check_expression($state, $node->children->[1]);
        return undef unless defined $operand_type;

        if ($operator eq '!') {
            if ($operand_type ne TYPE_BOOL) {
                _error($state, "Logical NOT requires a bool operand, but got '$operand_type'.", $node->children->[1]);
            }
            return TYPE_BOOL;
        }

        if ($operator eq '~' && !_is_numeric($operand_type)) {
            _error($state, "Bitwise NOT requires a numeric operand, but got '$operand_type'.", $node->children->[1]);
        }
        return $operand_type;
    }

    my ($expr) = @{ _expression_children($node) };
    return defined $expr ? _check_expression($state, $expr) : undef;
}

sub _check_add_expression {
    my ($state, $node) = @_;
    return _check_expression($state, $node->children->[0]) if @{ $node->children } == 1;

    my @exprs = @{ _expression_children($node) };
    my $result = _check_expression($state, $exprs[0]);

    for my $index (1 .. $#exprs) {
        my $rhs = _check_expression($state, $exprs[$index]);
        next unless defined $result && defined $rhs;

        if (_is_numeric_literal_expr($exprs[$index - 1]) && _is_numeric($rhs)) {
            $result = $rhs;
            next;
        }
        if (_is_numeric_literal_expr($exprs[$index]) && _is_numeric($result)) {
            next;
        }
        if (_is_numeric_literal_expr($exprs[$index - 1]) && _is_numeric_literal_expr($exprs[$index])) {
            $result = TYPE_U4;
            next;
        }
        if ($result eq $rhs && _is_numeric($result)) {
            next;
        }

        _error($state, "Binary expression type mismatch: $result vs $rhs.", $node);
        return undef;
    }

    return $result;
}

sub _check_primary {
    my ($state, $node) = @_;
    return undef unless @{ $node->children };
    return _check_expression($state, $node->children->[0]);
}

sub _check_call_expression {
    my ($state, $node) = @_;
    my $name = _first_name_token($node);
    return undef unless defined $name;

    my $symbol = _lookup($state, $name->{value});
    if (!defined $symbol || !$symbol->{is_fn}) {
        _error($state, "Unknown function '$name->{value}'.", $name);
        return undef;
    }

    my @args;
    my ($arg_list) = grep { $_->rule_name eq 'arg_list' } @{ _child_nodes($node) };
    if (defined $arg_list) {
        @args = grep { $_->rule_name eq 'expr' } @{ _child_nodes($arg_list) };
    }

    if (@args != @{ $symbol->{fn_params} || [] }) {
        _error(
            $state,
            "Function '$name->{value}' expects " . scalar(@{ $symbol->{fn_params} || [] })
                . " arguments but got " . scalar(@args) . '.',
            $node,
        );
        return $symbol->{fn_return_type};
    }

    for my $index (0 .. $#args) {
        my $actual = _check_expression($state, $args[$index]);
        my $expected = $symbol->{fn_params}[$index][1];
        if (defined $actual && !_is_numeric_literal_expr($args[$index]) && !_types_are_compatible($expected, $actual)) {
            _error(
                $state,
                "Argument " . ($index + 1) . " for '$name->{value}' expects '$expected' but got '$actual'.",
                $args[$index],
            );
        }
    }

    return $symbol->{fn_return_type};
}

sub _extract_params {
    my ($state, $param_list) = @_;
    my @params;

    for my $param (grep { $_->rule_name eq 'param' } @{ _child_nodes($param_list) }) {
        my $name = _first_name_token($param);
        my $type_node = _type_node($param);
        next unless defined $name && defined $type_node;

        my $nib_type = _resolve_type_node($state, $type_node);
        next unless defined $nib_type;
        push @params, [ $name->{value}, $nib_type ];
    }

    return \@params;
}

sub _resolve_type_node {
    my ($state, $node) = @_;
    my ($token) = grep { ref($_) eq 'HASH' } @{ _tokens_in($node) };
    return undef unless defined $token;

    my $value = $token->{value} // '';
    return $value if $value eq TYPE_U4 || $value eq TYPE_U8 || $value eq TYPE_BCD || $value eq TYPE_BOOL;

    _error($state, "Unknown type '$value'.", $token);
    return undef;
}

sub _is_numeric_literal_expr {
    my ($subject) = @_;
    return 0 unless defined $subject;

    if (!_is_ast_node($subject)) {
        return (($subject->{type} // '') eq 'INT_LIT' || ($subject->{type} // '') eq 'HEX_LIT') ? 1 : 0;
    }

    my $saw_ast_child = 0;
    for my $child (@{ $subject->children }) {
        if (_is_ast_node($child)) {
            $saw_ast_child = 1;
            return 0 unless _is_numeric_literal_expr($child);
            next;
        }

        my $type = $child->{type} // '';
        my $value = $child->{value} // '';
        return 0 if $type eq 'NAME' || $value eq 'true' || $value eq 'false';
        return 0 if $value eq '==' || $value eq '!=' || $value eq '<=' || $value eq '>=' || $value eq '<' || $value eq '>';
        return 0 if $value eq '&&' || $value eq '||' || $value eq 'and' || $value eq 'or';
    }

    return $saw_ast_child ? 1 : 0;
}

sub _types_are_compatible {
    my ($lhs, $rhs) = @_;
    return defined($lhs) && defined($rhs) && $lhs eq $rhs ? 1 : 0;
}

sub _is_numeric {
    my ($value) = @_;
    return defined($value) && ($value eq TYPE_U4 || $value eq TYPE_U8 || $value eq TYPE_BCD) ? 1 : 0;
}

sub _error {
    my ($state, $message, $subject) = @_;
    my ($line, $column) = _locate($subject);
    push @{ $state->{errors} }, new_type_error_diagnostic($message, $line, $column);
    return undef;
}

sub _locate {
    my ($subject) = @_;
    if (_is_ast_node($subject)) {
        my ($token) = @{ _tokens_in($subject) };
        return ($token->{line}, $token->{column}) if defined $token;
        return ($subject->start_line || 1, $subject->start_column || 1);
    }
    if (ref($subject) eq 'HASH') {
        return ($subject->{line} || 1, $subject->{column} || 1);
    }
    return (1, 1);
}

sub _define_global {
    my ($state, $name, $symbol) = @_;
    $state->{scopes}[0]{$name} = $symbol;
}

sub _define_local {
    my ($state, $name, $symbol) = @_;
    $state->{scopes}[-1]{$name} = $symbol;
}

sub _lookup {
    my ($state, $name) = @_;
    for my $index (reverse 0 .. $#{ $state->{scopes} }) {
        return $state->{scopes}[$index]{$name} if exists $state->{scopes}[$index]{$name};
    }
    return undef;
}

sub _push_scope {
    my ($state) = @_;
    push @{ $state->{scopes} }, {};
}

sub _pop_scope {
    my ($state) = @_;
    pop @{ $state->{scopes} } if @{ $state->{scopes} } > 1;
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

sub _first_name_token {
    my ($node) = @_;
    for my $token (@{ _tokens_in($node) }) {
        return $token if ($token->{type} // '') eq 'NAME';
    }
    return undef;
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

package CodingAdventures::NibTypeChecker::TypedAst;

use strict;
use warnings;
use Scalar::Util qw(refaddr);

sub new {
    my ($class, %args) = @_;
    return bless {
        root  => $args{root},
        types => $args{types} || {},
    }, $class;
}

sub root {
    my ($self) = @_;
    return $self->{root};
}

sub type_of {
    my ($self, $node) = @_;
    return undef unless defined $node;
    return $self->{types}{ refaddr($node) };
}

1;

__END__

=head1 NAME

CodingAdventures::NibTypeChecker - semantic checks for Nib programs

=head1 SYNOPSIS

  use CodingAdventures::NibTypeChecker qw(check_source);

  my $result = check_source('fn main() { let x: u4 = 5; }');
  die $result->{errors}[0]{message} unless $result->{ok};

=head1 DESCRIPTION

Parses Nib's AST shape, validates variable and function usage, and records
inferred expression types for later compiler passes.

=cut
