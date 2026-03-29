package CodingAdventures::Parser;

# ============================================================================
# CodingAdventures::Parser — Recursive descent parser for a simple language
# ============================================================================
#
# A parser takes a flat sequence of tokens (produced by a lexer) and builds
# a tree structure that captures the *meaning* of the program.  This tree is
# called an Abstract Syntax Tree (AST).
#
# For example, the tokens "1 + 2 * 3" become:
#
#       (+)
#      /   \
#    1     (*)
#         /   \
#        2     3
#
# Multiplication binds tighter than addition — the parser encodes operator
# precedence directly in the tree structure.
#
# === WHAT LANGUAGE DO WE PARSE? ===
#
# A simple expression language:
#
#   42            — number literal
#   "hello"       — string literal
#   x             — identifier
#   -x            — unary minus
#   !x            — unary not
#   x + y         — binary operators: + - * / == != < > <= >=
#   f(x, y)       — function call
#   if c then a else b  — if expression (returns a value)
#   let x = e     — let binding
#
# === OPERATOR PRECEDENCE (lowest to highest) ===
#
#   Level 1:  assignment  (right-associative)
#   Level 2:  or
#   Level 3:  and
#   Level 4:  == !=
#   Level 5:  < > <= >=
#   Level 6:  + -   (additive)
#   Level 7:  * /   (multiplicative)
#   Level 8:  unary - !
#   Level 9:  primary (literal, ident, call, group)
#
# === AST NODE TYPES ===
#
# Each node is a plain hashref:
#
#   { type => "number",   value => 42 }
#   { type => "string",   value => "hello" }
#   { type => "ident",    name  => "x" }
#   { type => "unary",    op => "-",  expr => $node }
#   { type => "binop",    op => "+",  left => $l, right => $r }
#   { type => "call",     name => "f", args => [$a, $b] }
#   { type => "if",       cond => $c, then => $t, else => $e }
#   { type => "let",      name => "x", value => $v }
#   { type => "program",  stmts => [$s1, $s2, ...] }
#
# === USAGE ===
#
#   use CodingAdventures::Lexer;
#   use CodingAdventures::Parser;
#
#   my @tokens = CodingAdventures::Lexer->new($src)->tokenize();
#   my $parser = CodingAdventures::Parser->new(\@tokens);
#   my $ast    = $parser->parse();
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# Constructor
# ============================================================================
#
# $tokens is an arrayref of token hashrefs from the lexer.
# We keep a current index _pos into this array.

sub new {
    my ($class, $tokens) = @_;
    # Filter out whitespace tokens — the parser doesn't care about spacing.
    my @filtered = grep { $_->{type} ne 'WHITESPACE' } @$tokens;
    return bless {
        _tokens => \@filtered,
        _pos    => 0,
    }, $class;
}

# ============================================================================
# Token helpers
# ============================================================================

# Peek at the current token without consuming it.
sub _peek {
    my ($self) = @_;
    return $self->{_tokens}[ $self->{_pos} ]
        // { type => 'EOF', value => '', line => 0, col => 0 };
}

# Consume and return the current token.
sub _advance {
    my ($self) = @_;
    my $tok = $self->_peek();
    $self->{_pos}++ unless $tok->{type} eq 'EOF';
    return $tok;
}

# Expect a token of a specific type (and optionally a specific value).
# Advances and returns the token, or dies with a helpful message.
sub _expect {
    my ($self, $type, $value) = @_;
    my $tok = $self->_peek();
    if ($tok->{type} ne $type) {
        die "Parse error at line $tok->{line} col $tok->{col}: "
          . "expected $type but got $tok->{type} ('$tok->{value}')\n";
    }
    if (defined $value && $tok->{value} ne $value) {
        die "Parse error at line $tok->{line} col $tok->{col}: "
          . "expected '$value' but got '$tok->{value}'\n";
    }
    return $self->_advance();
}

# Check whether the current token matches type (and optionally value).
sub _check {
    my ($self, $type, $value) = @_;
    my $tok = $self->_peek();
    return 0 if $tok->{type} ne $type;
    return 1 if !defined $value;
    return $tok->{value} eq $value;
}

# Consume the current token if it matches, return it; otherwise return undef.
sub _match {
    my ($self, $type, $value) = @_;
    return $self->_advance() if $self->_check($type, $value);
    return undef;
}

# ============================================================================
# parse — entry point: parse an entire program
# ============================================================================
#
# A "program" is a sequence of statements.  We keep parsing until EOF.

sub parse {
    my ($self) = @_;
    my @stmts;
    while (!$self->_check('EOF')) {
        # Skip bare newlines at statement level
        if ($self->_check('NEWLINE')) { $self->_advance(); next; }
        push @stmts, $self->parse_statement();
    }
    return { type => 'program', stmts => \@stmts };
}

# ============================================================================
# parse_statement — parse one statement
# ============================================================================
#
# Statements:
#   let x = expr  — variable binding
#   expr          — expression used as a statement

sub parse_statement {
    my ($self) = @_;

    # let binding
    if ($self->_check('KEYWORD', 'let')) {
        $self->_advance();
        my $name_tok = $self->_expect('IDENT');
        $self->_expect('SYMBOL', '=');
        my $val = $self->parse_expr();
        return { type => 'let', name => $name_tok->{value}, value => $val };
    }

    # Expression statement
    my $expr = $self->parse_expr();
    return $expr;
}

# ============================================================================
# parse_expr — parse an expression (starts at lowest precedence)
# ============================================================================

sub parse_expr {
    my ($self) = @_;
    return $self->_parse_or();
}

# ============================================================================
# Precedence climbing — each level calls the next higher level
# ============================================================================

sub _parse_or {
    my ($self) = @_;
    my $left = $self->_parse_and();
    while ($self->_check('KEYWORD', 'or')) {
        my $op = $self->_advance()->{value};
        my $right = $self->_parse_and();
        $left = { type => 'binop', op => $op, left => $left, right => $right };
    }
    return $left;
}

sub _parse_and {
    my ($self) = @_;
    my $left = $self->_parse_equality();
    while ($self->_check('KEYWORD', 'and')) {
        my $op = $self->_advance()->{value};
        my $right = $self->_parse_equality();
        $left = { type => 'binop', op => $op, left => $left, right => $right };
    }
    return $left;
}

sub _parse_equality {
    my ($self) = @_;
    my $left = $self->_parse_comparison();
    while ($self->_check('SYMBOL', '==') || $self->_check('SYMBOL', '!=')) {
        my $op    = $self->_advance()->{value};
        my $right = $self->_parse_comparison();
        $left = { type => 'binop', op => $op, left => $left, right => $right };
    }
    return $left;
}

sub _parse_comparison {
    my ($self) = @_;
    my $left = $self->_parse_additive();
    while (
        $self->_check('SYMBOL', '<')  || $self->_check('SYMBOL', '>')  ||
        $self->_check('SYMBOL', '<=') || $self->_check('SYMBOL', '>=')
    ) {
        my $op    = $self->_advance()->{value};
        my $right = $self->_parse_additive();
        $left = { type => 'binop', op => $op, left => $left, right => $right };
    }
    return $left;
}

sub _parse_additive {
    my ($self) = @_;
    my $left = $self->_parse_multiplicative();
    while ($self->_check('SYMBOL', '+') || $self->_check('SYMBOL', '-')) {
        my $op    = $self->_advance()->{value};
        my $right = $self->_parse_multiplicative();
        $left = { type => 'binop', op => $op, left => $left, right => $right };
    }
    return $left;
}

sub _parse_multiplicative {
    my ($self) = @_;
    my $left = $self->_parse_unary();
    while ($self->_check('SYMBOL', '*') || $self->_check('SYMBOL', '/')) {
        my $op    = $self->_advance()->{value};
        my $right = $self->_parse_unary();
        $left = { type => 'binop', op => $op, left => $left, right => $right };
    }
    return $left;
}

sub _parse_unary {
    my ($self) = @_;
    if ($self->_check('SYMBOL', '-') || $self->_check('SYMBOL', '!') ||
        $self->_check('KEYWORD', 'not')) {
        my $op   = $self->_advance()->{value};
        my $expr = $self->_parse_unary();
        return { type => 'unary', op => $op, expr => $expr };
    }
    return $self->_parse_primary();
}

# ============================================================================
# _parse_primary — parse a primary expression
# ============================================================================
#
# Primary expressions:
#   42           — number literal
#   "hello"      — string literal
#   true / false — boolean literals
#   nil          — nil literal
#   x            — identifier (possibly followed by '(' for a call)
#   if c then t else e  — conditional expression
#   ( expr )     — parenthesized expression

sub _parse_primary {
    my ($self) = @_;
    my $tok = $self->_peek();

    # Number literal
    if ($tok->{type} eq 'NUMBER') {
        $self->_advance();
        return { type => 'number', value => $tok->{value} + 0 };
    }

    # String literal — strip the surrounding quotes
    if ($tok->{type} eq 'STRING') {
        $self->_advance();
        my $raw = $tok->{value};
        $raw =~ s/^"(.*)"$/$1/s;
        $raw =~ s/\\n/\n/g;
        $raw =~ s/\\t/\t/g;
        $raw =~ s/\\"/"/g;
        $raw =~ s/\\\\/\\/g;
        return { type => 'string', value => $raw };
    }

    # Boolean / nil keywords
    if ($tok->{type} eq 'KEYWORD' && $tok->{value} eq 'true') {
        $self->_advance();
        return { type => 'bool', value => 1 };
    }
    if ($tok->{type} eq 'KEYWORD' && $tok->{value} eq 'false') {
        $self->_advance();
        return { type => 'bool', value => 0 };
    }
    if ($tok->{type} eq 'KEYWORD' && $tok->{value} eq 'nil') {
        $self->_advance();
        return { type => 'nil' };
    }

    # if expression
    if ($tok->{type} eq 'KEYWORD' && $tok->{value} eq 'if') {
        return $self->_parse_if();
    }

    # Identifier or function call
    if ($tok->{type} eq 'IDENT') {
        $self->_advance();
        # Function call: name(arg1, arg2, ...)
        if ($self->_check('SYMBOL', '(')) {
            $self->_advance();  # consume '('
            my @args;
            unless ($self->_check('SYMBOL', ')')) {
                push @args, $self->parse_expr();
                while ($self->_check('SYMBOL', ',')) {
                    $self->_advance();
                    push @args, $self->parse_expr();
                }
            }
            $self->_expect('SYMBOL', ')');
            return { type => 'call', name => $tok->{value}, args => \@args };
        }
        return { type => 'ident', name => $tok->{value} };
    }

    # Parenthesized expression
    if ($tok->{type} eq 'SYMBOL' && $tok->{value} eq '(') {
        $self->_advance();
        my $expr = $self->parse_expr();
        $self->_expect('SYMBOL', ')');
        return $expr;
    }

    die "Parse error at line $tok->{line} col $tok->{col}: "
      . "unexpected token '$tok->{value}' (type: $tok->{type})\n";
}

# ============================================================================
# _parse_if — parse an if expression
# ============================================================================
#
# Syntax:  if <cond> then <then_expr> [else <else_expr>]

sub _parse_if {
    my ($self) = @_;
    $self->_expect('KEYWORD', 'if');
    my $cond = $self->parse_expr();
    $self->_expect('KEYWORD', 'then');
    my $then = $self->parse_expr();
    my $else_branch;
    if ($self->_check('KEYWORD', 'else')) {
        $self->_advance();
        $else_branch = $self->parse_expr();
    }
    return { type => 'if', cond => $cond, then => $then, else => $else_branch };
}

1;

__END__

=head1 NAME

CodingAdventures::Parser - Recursive descent parser for a simple expression language

=head1 SYNOPSIS

    use CodingAdventures::Lexer;
    use CodingAdventures::Parser;

    my @tokens = CodingAdventures::Lexer->new("1 + 2 * 3")->tokenize();
    my $parser = CodingAdventures::Parser->new(\@tokens);
    my $ast    = $parser->parse();

=head1 DESCRIPTION

A recursive descent parser that builds an AST from a token stream.  Handles
numbers, strings, identifiers, binary/unary operators, function calls, if
expressions, and let bindings.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
