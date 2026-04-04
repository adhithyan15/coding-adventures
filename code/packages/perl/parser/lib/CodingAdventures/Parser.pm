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

# ============================================================================
# ASTNode — generic AST node for grammar-driven parsing
# ============================================================================
#
# Each node stores:
#   rule_name    — which grammar rule produced this node
#   children     — arrayref of child ASTNodes and/or lexer token hashrefs
#   start_line, start_column, end_line, end_column — optional position info
#
# Leaf nodes wrap a single token. Use is_leaf() and token() to distinguish.

package CodingAdventures::Parser::ASTNode;

sub new {
    my ($class, %fields) = @_;
    return bless {
        rule_name    => $fields{rule_name}    // '',
        children     => $fields{children}     // [],
        start_line   => $fields{start_line},
        start_column => $fields{start_column},
        end_line     => $fields{end_line},
        end_column   => $fields{end_column},
    }, $class;
}

sub rule_name    { $_[0]->{rule_name} }
sub children     { $_[0]->{children} }
sub start_line   { $_[0]->{start_line} }
sub start_column { $_[0]->{start_column} }
sub end_line     { $_[0]->{end_line} }
sub end_column   { $_[0]->{end_column} }

# is_leaf() — true if this node wraps exactly one token (not another ASTNode).
# Tokens are plain hashrefs; ASTNodes are blessed into this class.
sub is_leaf {
    my ($self) = @_;
    my $ch = $self->{children};
    return 0 unless @$ch == 1;
    my $child = $ch->[0];
    # If it's a blessed ASTNode, it's not a leaf
    return 0 if ref($child) && ref($child) ne 'HASH'
        && eval { $child->isa('CodingAdventures::Parser::ASTNode') };
    # Plain hashref = token
    return ref($child) eq 'HASH' ? 1 : 0;
}

# token() — return the wrapped token if leaf, undef otherwise.
sub token {
    my ($self) = @_;
    return $self->{children}[0] if $self->is_leaf();
    return undef;
}

package CodingAdventures::Parser;

# ============================================================================
# GrammarParser — grammar-driven packrat parser
# ============================================================================
#
# Interprets grammar rules at runtime and builds an AST from a token stream.
# Uses packrat memoization for O(n * g) performance.

sub new_grammar_parser {
    my ($class, $tokens, $grammar, %opts) = @_;

    # Build rule lookup tables
    my %rules;
    my %rule_index;
    my $i = 0;
    for my $rule (@{ $grammar->{rules} }) {
        $rules{ $rule->{name} } = $rule;
        $rule_index{ $rule->{name} } = $i++;
    }

    my $self = bless {
        _gp_tokens             => $tokens,
        _gp_grammar            => $grammar,
        _gp_pos                => 0,
        _gp_rules              => \%rules,
        _gp_rule_index         => \%rule_index,
        _gp_newlines_significant => 0,
        _gp_memo               => {},
        _gp_furthest_pos       => 0,
        _gp_furthest_expected  => [],
        _gp_trace              => $opts{trace} // 0,
    }, $class;

    $self->{_gp_newlines_significant} = $self->_gp_grammar_references_newline();
    return $self;
}

sub _gp_token_type_name {
    my ($self, $tok) = @_;
    return $tok->{type_name} if $tok->{type_name} && $tok->{type_name} ne '';
    return $tok->{type} if defined $tok->{type};
    return 'UNKNOWN';
}

sub _gp_current {
    my ($self) = @_;
    my $pos = $self->{_gp_pos};
    my $toks = $self->{_gp_tokens};
    return $pos < @$toks ? $toks->[$pos] : $toks->[-1];
}

sub _gp_record_failure {
    my ($self, $expected) = @_;
    if ($self->{_gp_pos} > $self->{_gp_furthest_pos}) {
        $self->{_gp_furthest_pos} = $self->{_gp_pos};
        $self->{_gp_furthest_expected} = [$expected];
    } elsif ($self->{_gp_pos} == $self->{_gp_furthest_pos}) {
        push @{ $self->{_gp_furthest_expected} }, $expected
            unless grep { $_ eq $expected } @{ $self->{_gp_furthest_expected} };
    }
}

sub _gp_grammar_references_newline {
    my ($self) = @_;
    for my $rule (@{ $self->{_gp_grammar}{rules} }) {
        return 1 if $self->_gp_element_references_newline($rule->{body});
    }
    return 0;
}

sub _gp_element_references_newline {
    my ($self, $el) = @_;
    return 0 unless $el;
    my $t = $el->{type};
    if ($t eq 'rule_reference') {
        return $el->{is_token} && $el->{name} eq 'NEWLINE';
    } elsif ($t eq 'sequence') {
        for (@{ $el->{elements} }) { return 1 if $self->_gp_element_references_newline($_); }
    } elsif ($t eq 'alternation') {
        for (@{ $el->{choices} }) { return 1 if $self->_gp_element_references_newline($_); }
    } elsif ($t eq 'repetition' || $t eq 'optional' || $t eq 'group'
          || $t eq 'positive_lookahead' || $t eq 'negative_lookahead'
          || $t eq 'one_or_more') {
        return $self->_gp_element_references_newline($el->{element});
    } elsif ($t eq 'separated_repetition') {
        return $self->_gp_element_references_newline($el->{element})
            || $self->_gp_element_references_newline($el->{separator});
    }
    return 0;
}

# grammar_parse() — parse the token stream, return (ASTNode, undef) or (undef, error_string)
sub grammar_parse {
    my ($self) = @_;
    my $rules = $self->{_gp_grammar}{rules};
    return (undef, "Grammar has no rules") unless @$rules;

    my $entry = $rules->[0];
    my $result = $self->_gp_parse_rule($entry->{name});

    unless ($result) {
        my $tok = $self->_gp_current();
        if (@{ $self->{_gp_furthest_expected} }) {
            my $expected = join(' or ', @{ $self->{_gp_furthest_expected} });
            return (undef, "Parse error: Expected $expected, got '$tok->{value}'");
        }
        return (undef, "Parse error: Failed to parse");
    }

    # Skip trailing newlines
    while ($self->{_gp_pos} < @{ $self->{_gp_tokens} }
        && $self->_gp_token_type_name($self->_gp_current()) eq 'NEWLINE') {
        $self->{_gp_pos}++;
    }

    # Check for unconsumed tokens
    if ($self->{_gp_pos} < @{ $self->{_gp_tokens} }
        && $self->_gp_token_type_name($self->_gp_current()) ne 'EOF') {
        my $tok = $self->_gp_current();
        return (undef, "Parse error: Unexpected token '$tok->{value}'");
    }

    return ($result, undef);
}

sub _gp_parse_rule {
    my ($self, $rule_name) = @_;
    my $rule = $self->{_gp_rules}{$rule_name};
    return undef unless $rule;

    my $idx = $self->{_gp_rule_index}{$rule_name};
    if (defined $idx) {
        my $key = "$idx:$self->{_gp_pos}";
        my $cached = $self->{_gp_memo}{$key};
        if ($cached) {
            $self->{_gp_pos} = $cached->{end_pos};
            return undef unless $cached->{ok};
            return CodingAdventures::Parser::ASTNode->new(
                rule_name => $rule_name, children => $cached->{children});
        }
    }

    my $start_pos = $self->{_gp_pos};

    # Left-recursion guard
    if (defined $idx) {
        my $key = "$idx:$start_pos";
        $self->{_gp_memo}{$key} = { children => undef, end_pos => $start_pos, ok => 0 };
    }

    my $children = $self->_gp_match_element($rule->{body});

    # Cache
    if (defined $idx) {
        my $key = "$idx:$start_pos";
        if ($children) {
            $self->{_gp_memo}{$key} = { children => $children, end_pos => $self->{_gp_pos}, ok => 1 };
        } else {
            $self->{_gp_memo}{$key} = { children => undef, end_pos => $self->{_gp_pos}, ok => 0 };
        }
    }

    unless ($children) {
        $self->{_gp_pos} = $start_pos;
        $self->_gp_record_failure($rule_name);
        return undef;
    }

    $children = [] unless $children;

    # Compute position info
    my $first_tok = _find_first_token_in($children);
    my $last_tok  = _find_last_token_in($children);
    if ($first_tok && $last_tok) {
        return CodingAdventures::Parser::ASTNode->new(
            rule_name => $rule_name, children => $children,
            start_line => $first_tok->{line}, start_column => $first_tok->{col} // $first_tok->{column},
            end_line => $last_tok->{line}, end_column => $last_tok->{col} // $last_tok->{column});
    }
    return CodingAdventures::Parser::ASTNode->new(rule_name => $rule_name, children => $children);
}

sub _gp_match_element {
    my ($self, $element) = @_;
    my $save_pos = $self->{_gp_pos};
    my $type = $element->{type};

    if ($type eq 'sequence') {
        my @children;
        for my $sub (@{ $element->{elements} }) {
            my $res = $self->_gp_match_element($sub);
            unless ($res) { $self->{_gp_pos} = $save_pos; return undef; }
            push @children, @$res;
        }
        return \@children;
    }
    if ($type eq 'alternation') {
        for my $choice (@{ $element->{choices} }) {
            $self->{_gp_pos} = $save_pos;
            my $res = $self->_gp_match_element($choice);
            return $res if $res;
        }
        $self->{_gp_pos} = $save_pos;
        return undef;
    }
    if ($type eq 'repetition') {
        my @children;
        while (1) {
            my $sr = $self->{_gp_pos};
            my $res = $self->_gp_match_element($element->{element});
            unless ($res) { $self->{_gp_pos} = $sr; last; }
            push @children, @$res;
        }
        return \@children;
    }
    if ($type eq 'optional') {
        my $res = $self->_gp_match_element($element->{element});
        return $res // [];
    }
    if ($type eq 'group') {
        return $self->_gp_match_element($element->{element});
    }
    if ($type eq 'rule_reference') {
        if ($element->{is_token}) {
            return $self->_gp_match_token_reference($element->{name});
        }
        my $node = $self->_gp_parse_rule($element->{name});
        return [$node] if $node;
        $self->{_gp_pos} = $save_pos;
        return undef;
    }
    if ($type eq 'literal') {
        my $tok = $self->_gp_current();
        unless ($self->{_gp_newlines_significant}) {
            while ($self->_gp_token_type_name($tok) eq 'NEWLINE') {
                $self->{_gp_pos}++;
                $tok = $self->_gp_current();
            }
        }
        if ($tok->{value} eq $element->{value}) {
            $self->{_gp_pos}++;
            return [$tok];
        }
        $self->_gp_record_failure("\"$element->{value}\"");
        return undef;
    }

    # Extension: Syntactic predicates
    if ($type eq 'positive_lookahead') {
        my $res = $self->_gp_match_element($element->{element});
        $self->{_gp_pos} = $save_pos;
        return $res ? [] : undef;
    }
    if ($type eq 'negative_lookahead') {
        my $res = $self->_gp_match_element($element->{element});
        $self->{_gp_pos} = $save_pos;
        return $res ? undef : [];
    }

    # Extension: One-or-more
    if ($type eq 'one_or_more') {
        my $first = $self->_gp_match_element($element->{element});
        unless ($first) { $self->{_gp_pos} = $save_pos; return undef; }
        my @children = @$first;
        while (1) {
            my $sr = $self->{_gp_pos};
            my $res = $self->_gp_match_element($element->{element});
            unless ($res) { $self->{_gp_pos} = $sr; last; }
            push @children, @$res;
        }
        return \@children;
    }

    # Extension: Separated repetition
    if ($type eq 'separated_repetition') {
        my $first = $self->_gp_match_element($element->{element});
        unless ($first) {
            $self->{_gp_pos} = $save_pos;
            return $element->{at_least_one} ? undef : [];
        }
        my @children = @$first;
        while (1) {
            my $ss = $self->{_gp_pos};
            my $sep = $self->_gp_match_element($element->{separator});
            unless ($sep) { $self->{_gp_pos} = $ss; last; }
            my $nxt = $self->_gp_match_element($element->{element});
            unless ($nxt) { $self->{_gp_pos} = $ss; last; }
            push @children, @$sep, @$nxt;
        }
        return \@children;
    }

    return undef;
}

sub _gp_match_token_reference {
    my ($self, $expected_type) = @_;
    my $tok = $self->_gp_current();

    unless ($self->{_gp_newlines_significant}) {
        if ($expected_type ne 'NEWLINE') {
            while ($self->_gp_token_type_name($tok) eq 'NEWLINE') {
                $self->{_gp_pos}++;
                $tok = $self->_gp_current();
            }
        }
    }

    if ($self->_gp_token_type_name($tok) eq $expected_type) {
        $self->{_gp_pos}++;
        return [$tok];
    }

    $self->_gp_record_failure($expected_type);
    return undef;
}

# ============================================================================
# AST Position Helpers
# ============================================================================

sub _find_first_token_in {
    my ($children) = @_;
    for my $child (@$children) {
        if (is_ast_node($child)) {
            my $tok = _find_first_token_in($child->children);
            return $tok if $tok;
        } elsif (ref($child) eq 'HASH') {
            return $child;
        }
    }
    return undef;
}

sub _find_last_token_in {
    my ($children) = @_;
    for my $i (reverse 0 .. $#$children) {
        my $child = $children->[$i];
        if (is_ast_node($child)) {
            my $tok = _find_last_token_in($child->children);
            return $tok if $tok;
        } elsif (ref($child) eq 'HASH') {
            return $child;
        }
    }
    return undef;
}

# ============================================================================
# AST Walking Utilities
# ============================================================================

# is_ast_node($child) — check if a child is an ASTNode (not a token hashref)
# Can be called as class method or plain function.
sub is_ast_node {
    my $child;
    if (@_ == 2) {
        $child = $_[1];  # class method call
    } else {
        $child = $_[0];  # plain function call
    }
    return 0 unless ref($child);
    return 0 if ref($child) eq 'HASH';  # plain hashref = token
    return eval { $child->isa('CodingAdventures::Parser::ASTNode') } ? 1 : 0;
}

# walk_ast($node, $visitor) — depth-first walk with enter/leave callbacks.
# $visitor is a hashref { enter => sub($node,$parent), leave => sub($node,$parent) }.
# Callbacks may return a replacement ASTNode or undef to keep the original.
sub walk_ast {
    my ($class, $node, $visitor) = @_;
    unless (defined $visitor) { ($node, $visitor) = ($class, $node); }
    return _walk_node_impl($node, undef, $visitor);
}

sub _walk_node_impl {
    my ($node, $parent, $visitor) = @_;
    my $current = $node;

    if ($visitor->{enter}) {
        my $repl = $visitor->{enter}->($current, $parent);
        $current = $repl if defined $repl;
    }

    my $children_changed = 0;
    my @new_children;
    for my $child (@{ $current->children }) {
        if (is_ast_node($child)) {
            my $walked = _walk_node_impl($child, $current, $visitor);
            $children_changed = 1 if $walked ne $child;
            push @new_children, $walked;
        } else {
            push @new_children, $child;
        }
    }

    if ($children_changed) {
        $current = CodingAdventures::Parser::ASTNode->new(
            rule_name    => $current->rule_name,
            children     => \@new_children,
            start_line   => $current->start_line,
            start_column => $current->start_column,
            end_line     => $current->end_line,
            end_column   => $current->end_column,
        );
    }

    if ($visitor->{leave}) {
        my $repl = $visitor->{leave}->($current, $parent);
        $current = $repl if defined $repl;
    }

    return $current;
}

# find_nodes($node, $rule_name) — find all nodes matching a rule name (depth-first).
sub find_nodes {
    my ($class, $node, $rule_name) = @_;
    unless (defined $rule_name) { ($node, $rule_name) = ($class, $node); }
    my @results;
    walk_ast($node, {
        enter => sub {
            my ($n) = @_;
            push @results, $n if $n->rule_name eq $rule_name;
            return undef;
        },
    });
    return @results;
}

# collect_tokens($node, $type) — collect all tokens depth-first, optionally filtered.
sub collect_tokens {
    my ($class, $node, $type) = @_;
    unless (ref($node) && $node->isa('CodingAdventures::Parser::ASTNode')) {
        ($node, $type) = ($class, $node);
    }
    my @results;
    my $walk;
    $walk = sub {
        my ($n) = @_;
        for my $child (@{ $n->children }) {
            if (is_ast_node($child)) {
                $walk->($child);
            } else {
                if (!defined $type || ($child->{type} // '') eq $type
                    || ($child->{type_name} // '') eq $type) {
                    push @results, $child;
                }
            }
        }
    };
    $walk->($node);
    return @results;
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
