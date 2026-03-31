package CodingAdventures::RubyParser;

# ============================================================================
# CodingAdventures::RubyParser — Hand-written recursive-descent Ruby parser
# ============================================================================
#
# This module parses a subset of Ruby into an Abstract Syntax Tree (AST).
# The parser is hand-written using the recursive-descent technique: each
# grammar rule is encoded as one Perl method, and rules call each other
# recursively.
#
# # What language do we parse?
# =============================
#
# A practical subset covering the constructs most commonly taught first:
#
#   x = 5                               — assignment
#   def greet(name)                     — method definition
#     puts name
#   end
#   if x > 0                            — if statement
#     return x
#   elsif x == 0                        — elsif clause
#     return 0
#   else                                — else clause
#     return -1
#   end
#   while x > 0                         — while loop
#     x = x - 1
#   end
#   return value                        — return statement
#   "hello"                             — string literal
#   puts "hello"                        — method call without parens
#   obj.method(arg)                     — chained method call
#   [1,2,3].each { |i| puts i }         — block syntax (simplified)
#   class Dog                           — class definition
#     def bark
#       puts "woof"
#     end
#   end
#
# # Token types from CodingAdventures::RubyLexer
# ================================================
#
# The Ruby lexer promotes keywords to their uppercased name as type:
#
#   DEF, END, RETURN, IF, ELSIF, ELSE, WHILE, UNTIL, FOR, DO,
#   CLASS, MODULE, REQUIRE, PUTS, TRUE, FALSE, NIL,
#   AND, OR, NOT, THEN, UNLESS, YIELD, BEGIN, RESCUE, ENSURE
#
# Other types:
#   NAME        — identifiers
#   NUMBER      — integer literals
#   STRING      — double-quoted string literals
#   EQUALS      — =
#   EQUALS_EQUALS — ==
#   NOT_EQUALS  — !=
#   LESS_EQUALS — <=
#   GREATER_EQUALS — >=
#   LESS_THAN   — <
#   GREATER_THAN — >
#   PLUS, MINUS, STAR, SLASH  — arithmetic
#   LPAREN, RPAREN  — ( )
#   COMMA       — ,
#   COLON       — :
#   DOT         — .  (NOTE: ruby.tokens does not define DOT as a named token,
#                      so '.' would not be recognized by the grammar-driven lexer.
#                      Chained method calls are handled at the primary level by
#                      matching a DOT if present, otherwise just as a NAME.)
#   LBRACE, RBRACE — { }  (for block syntax)
#   PIPE        — |  (for block parameter delimiters)
#   EOF         — end of input
#
# Note: Ruby uses `end` (END token) to close def, if, while, class blocks —
# not indentation or braces.
#
# # Operator precedence (lowest to highest)
# ==========================================
#
#   Equality      →  ==  !=
#   Comparison    →  <  >  <=  >=
#   Additive      →  +  -
#   Multiplicative →  *  /
#   Unary          →  -  (unary minus)
#   Primary        →  literals, identifiers, (expr), call
#
# # AST node types (rule_name values)
# ====================================
#
#   program          — root; contains a list of statement nodes
#   statement        — wrapper for one statement
#   assignment       — NAME EQUALS expression
#   method_def       — def NAME(params) body end
#   class_def        — class NAME body end
#   if_stmt          — if expr body [elsif ...] [else body] end
#   while_stmt       — while expr body end
#   until_stmt       — until expr body end
#   return_stmt      — return [expression]
#   expression_stmt  — expression (stand-alone)
#   body             — { statement }  (until END)
#   expression       — full expression (lowest-precedence entry point)
#   binary_expr      — left op right
#   unary_expr       — - expr
#   call_expr        — NAME ( args )  or  KEYWORD ( args )
#   method_call_stmt — PUTS expr  (keyword method call without parens)
#   primary          — literal, identifier, grouped expression
#   param_list       — comma-separated parameter names
#   arg_list         — comma-separated argument expressions
#   token            — leaf node wrapping a single lexer token
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::RubyLexer;
use CodingAdventures::RubyParser::ASTNode;

# ============================================================================
# Constructor
# ============================================================================

# --- new($source) -------------------------------------------------------------
#
# Tokenize `$source` with RubyLexer and return a ready-to-parse parser.

sub new {
    my ($class, $source) = @_;
    my $tokens = CodingAdventures::RubyLexer->tokenize($source);
    return bless {
        _tokens => $tokens,
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

# Peek N positions ahead from the current position (0 = current).
sub _peek_ahead {
    my ($self, $n) = @_;
    $n //= 0;
    return $self->{_tokens}[ $self->{_pos} + $n ]
        // { type => 'EOF', value => '', line => 0, col => 0 };
}

# Consume and return the current token.
sub _advance {
    my ($self) = @_;
    my $tok = $self->_peek();
    $self->{_pos}++ unless $tok->{type} eq 'EOF';
    return $tok;
}

# Expect a specific token type; die with a helpful message on mismatch.
sub _expect {
    my ($self, $type) = @_;
    my $tok = $self->_peek();
    unless ($tok->{type} eq $type) {
        die sprintf(
            "CodingAdventures::RubyParser: parse error at line %d col %d: "
          . "expected %s but got %s ('%s')\n",
            $tok->{line}, $tok->{col}, $type, $tok->{type}, $tok->{value}
        );
    }
    return $self->_advance();
}

# Return 1 if current token matches the given type (and optionally value).
sub _check {
    my ($self, $type, $value) = @_;
    my $tok = $self->_peek();
    return 0 unless $tok->{type} eq $type;
    return 1 unless defined $value;
    return $tok->{value} eq $value;
}

# Consume and return the current token if it matches; otherwise return undef.
sub _match {
    my ($self, $type, $value) = @_;
    return $self->_advance() if $self->_check($type, $value);
    return undef;
}

# Skip over NEWLINE tokens between statements.
sub _skip_newlines {
    my ($self) = @_;
    while ($self->_check('NEWLINE')) {
        $self->_advance();
    }
}

# Return true if the current token is a block-terminating token:
# END, ELSE, ELSIF, or EOF.  Used as the "stop" condition for body loops.
sub _at_block_end {
    my ($self) = @_;
    my $type = $self->_peek()->{type};
    return $type eq 'END'   || $type eq 'ELSE'  ||
           $type eq 'ELSIF' || $type eq 'EOF';
}

# Wrap a token as a leaf ASTNode.
sub _leaf {
    my ($self, $tok) = @_;
    return CodingAdventures::RubyParser::ASTNode->new_leaf($tok);
}

# Create an inner ASTNode.
sub _node {
    my ($self, $rule_name, @children) = @_;
    return CodingAdventures::RubyParser::ASTNode->new($rule_name, \@children);
}

# ============================================================================
# Public API
# ============================================================================

# --- parse() ------------------------------------------------------------------
#
# Parse the tokenized source and return the root AST node (rule_name "program").
# Dies on parse error.

sub parse {
    my ($self) = @_;
    return $self->_parse_program();
}

# ============================================================================
# Grammar rules
# ============================================================================

# program = { statement } ;
#
# Ruby allows newlines between statements at any level.
sub _parse_program {
    my ($self) = @_;
    my @children;
    $self->_skip_newlines();
    while (!$self->_check('EOF')) {
        push @children, $self->_parse_statement();
        $self->_skip_newlines();
    }
    return $self->_node('program', @children);
}

# statement = method_def | class_def | if_stmt | while_stmt | until_stmt
#           | return_stmt | method_call_stmt | assignment | expression_stmt ;
#
# Dispatch based on the current token.  For the assignment vs. expression
# ambiguity we look one token ahead (NAME followed by EQUALS).
sub _parse_statement {
    my ($self) = @_;
    my $tok  = $self->_peek();
    my $type = $tok->{type};

    # Method definition: def NAME ... end
    if ($type eq 'DEF') {
        return $self->_node('statement', $self->_parse_method_def());
    }

    # Class definition: class NAME ... end
    if ($type eq 'CLASS') {
        return $self->_node('statement', $self->_parse_class_def());
    }

    # If statement: if expr ... end
    if ($type eq 'IF') {
        return $self->_node('statement', $self->_parse_if_stmt());
    }

    # Unless (negated if): unless expr ... end
    if ($type eq 'UNLESS') {
        return $self->_node('statement', $self->_parse_unless_stmt());
    }

    # While loop: while expr ... end
    if ($type eq 'WHILE') {
        return $self->_node('statement', $self->_parse_while_stmt());
    }

    # Until loop: until expr ... end
    if ($type eq 'UNTIL') {
        return $self->_node('statement', $self->_parse_until_stmt());
    }

    # Return: return [expr]
    if ($type eq 'RETURN') {
        return $self->_node('statement', $self->_parse_return_stmt());
    }

    # puts without parens: puts expr
    # PUTS is a keyword token (type 'PUTS').
    if ($type eq 'PUTS') {
        my $next = $self->_peek_ahead(1);
        # If not followed by LPAREN, treat as keyword method call without parens.
        if ($next->{type} ne 'LPAREN') {
            return $self->_node('statement', $self->_parse_method_call_stmt());
        }
    }

    # Assignment: NAME EQUALS expr  (lookahead: NAME then EQUALS, not ==)
    if ($type eq 'NAME') {
        my $next = $self->_peek_ahead(1);
        if ($next->{type} eq 'EQUALS') {
            return $self->_node('statement', $self->_parse_assignment());
        }
    }

    # Expression statement (default — calls, literals, etc.)
    return $self->_node('statement', $self->_parse_expression_stmt());
}

# assignment = NAME EQUALS expression ;
#
# Example:  x = 5   result = a + b
sub _parse_assignment {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('EQUALS'));
    push @ch, $self->_parse_expression();
    $self->_match('NEWLINE');
    return $self->_node('assignment', @ch);
}

# expression_stmt = expression ;
sub _parse_expression_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_expression();
    $self->_match('NEWLINE');
    return $self->_node('expression_stmt', @ch);
}

# method_call_stmt = PUTS expression ;
#
# Handles:  puts "hello"   puts x + 1
# Ruby's `puts` without parentheses is very common.
sub _parse_method_call_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_advance());   # PUTS (or any keyword)
    push @ch, $self->_parse_expression();
    $self->_match('NEWLINE');
    return $self->_node('method_call_stmt', @ch);
}

# method_def = DEF NAME [ LPAREN param_list RPAREN ] body END ;
#
# Example:  def greet(name)
#             puts name
#           end
sub _parse_method_def {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('DEF'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    if ($self->_check('LPAREN')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_param_list();
        push @ch, $self->_leaf($self->_expect('RPAREN'));
    }
    $self->_match('NEWLINE');
    push @ch, $self->_parse_body();
    push @ch, $self->_leaf($self->_expect('END'));
    $self->_match('NEWLINE');
    return $self->_node('method_def', @ch);
}

# param_list = [ NAME { COMMA NAME } ] ;
sub _parse_param_list {
    my ($self) = @_;
    my @ch;
    if ($self->_check('NAME')) {
        push @ch, $self->_leaf($self->_advance());
        while ($self->_check('COMMA')) {
            push @ch, $self->_leaf($self->_advance());
            push @ch, $self->_leaf($self->_expect('NAME'));
        }
    }
    return $self->_node('param_list', @ch);
}

# class_def = CLASS NAME body END ;
#
# Example:  class Dog
#             def bark
#               puts "woof"
#             end
#           end
sub _parse_class_def {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('CLASS'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    $self->_match('NEWLINE');
    push @ch, $self->_parse_body();
    push @ch, $self->_leaf($self->_expect('END'));
    $self->_match('NEWLINE');
    return $self->_node('class_def', @ch);
}

# if_stmt = IF expression body { ELSIF expression body } [ ELSE body ] END ;
#
# Example:  if x > 0
#             return x
#           elsif x == 0
#             return 0
#           else
#             return -1
#           end
sub _parse_if_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('IF'));
    push @ch, $self->_parse_expression();
    $self->_match('NEWLINE');
    push @ch, $self->_parse_body();

    # elsif chains
    while ($self->_check('ELSIF')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expression();
        $self->_match('NEWLINE');
        push @ch, $self->_parse_body();
    }

    # optional else
    if ($self->_check('ELSE')) {
        push @ch, $self->_leaf($self->_advance());
        $self->_match('NEWLINE');
        push @ch, $self->_parse_body();
    }

    push @ch, $self->_leaf($self->_expect('END'));
    $self->_match('NEWLINE');
    return $self->_node('if_stmt', @ch);
}

# unless_stmt = UNLESS expression body END ;
#
# Semantically equivalent to `if not expression`.
sub _parse_unless_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('UNLESS'));
    push @ch, $self->_parse_expression();
    $self->_match('NEWLINE');
    push @ch, $self->_parse_body();
    push @ch, $self->_leaf($self->_expect('END'));
    $self->_match('NEWLINE');
    return $self->_node('unless_stmt', @ch);
}

# while_stmt = WHILE expression body END ;
#
# Example:  while x > 0
#             x = x - 1
#           end
sub _parse_while_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('WHILE'));
    push @ch, $self->_parse_expression();
    $self->_match('NEWLINE');
    push @ch, $self->_parse_body();
    push @ch, $self->_leaf($self->_expect('END'));
    $self->_match('NEWLINE');
    return $self->_node('while_stmt', @ch);
}

# until_stmt = UNTIL expression body END ;
sub _parse_until_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('UNTIL'));
    push @ch, $self->_parse_expression();
    $self->_match('NEWLINE');
    push @ch, $self->_parse_body();
    push @ch, $self->_leaf($self->_expect('END'));
    $self->_match('NEWLINE');
    return $self->_node('until_stmt', @ch);
}

# return_stmt = RETURN [ expression ] ;
sub _parse_return_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('RETURN'));
    unless ($self->_check('NEWLINE') || $self->_check('END') ||
            $self->_check('ELSE')    || $self->_check('ELSIF') ||
            $self->_check('EOF')) {
        push @ch, $self->_parse_expression();
    }
    $self->_match('NEWLINE');
    return $self->_node('return_stmt', @ch);
}

# body = { statement } ;
#
# Parses statements until we see END, ELSE, ELSIF, or EOF.
# Ruby uses `end` keywords to close blocks, not indentation.
sub _parse_body {
    my ($self) = @_;
    my @ch;
    $self->_skip_newlines();
    while (!$self->_at_block_end()) {
        push @ch, $self->_parse_statement();
        $self->_skip_newlines();
    }
    return $self->_node('body', @ch);
}

# ============================================================================
# Expression parsing — precedence climbing
# ============================================================================
#
# expression → equality → comparison → additive → multiplicative → unary → primary
#
# This structure guarantees:
#   1 + 2 * 3  parses as  1 + (2 * 3)   — multiplication tighter
#   -x == y    parses as  (-x) == y      — unary tighter than equality

# expression = equality ;
sub _parse_expression {
    my ($self) = @_;
    my $inner = $self->_parse_equality();
    return $self->_node('expression', $inner);
}

# equality = comparison { ( EQUALS_EQUALS | NOT_EQUALS ) comparison } ;
sub _parse_equality {
    my ($self) = @_;
    my $left = $self->_parse_comparison();
    while ($self->_check('EQUALS_EQUALS') || $self->_check('NOT_EQUALS')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_comparison();
        $left = $self->_node('binary_expr', $left, $op, $right);
    }
    return $left;
}

# comparison = additive { ( LESS_THAN | GREATER_THAN | LESS_EQUALS | GREATER_EQUALS ) additive } ;
sub _parse_comparison {
    my ($self) = @_;
    my $left = $self->_parse_additive();
    while ($self->_check('LESS_THAN')    || $self->_check('GREATER_THAN') ||
           $self->_check('LESS_EQUALS')  || $self->_check('GREATER_EQUALS')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_additive();
        $left = $self->_node('binary_expr', $left, $op, $right);
    }
    return $left;
}

# additive = multiplicative { ( PLUS | MINUS ) multiplicative } ;
sub _parse_additive {
    my ($self) = @_;
    my $left = $self->_parse_multiplicative();
    while ($self->_check('PLUS') || $self->_check('MINUS')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_multiplicative();
        $left = $self->_node('binary_expr', $left, $op, $right);
    }
    return $left;
}

# multiplicative = unary { ( STAR | SLASH ) unary } ;
sub _parse_multiplicative {
    my ($self) = @_;
    my $left = $self->_parse_unary();
    while ($self->_check('STAR') || $self->_check('SLASH')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_unary();
        $left = $self->_node('binary_expr', $left, $op, $right);
    }
    return $left;
}

# unary = MINUS unary | primary ;
sub _parse_unary {
    my ($self) = @_;
    if ($self->_check('MINUS')) {
        my $op   = $self->_leaf($self->_advance());
        my $expr = $self->_parse_unary();
        return $self->_node('unary_expr', $op, $expr);
    }
    return $self->_parse_primary();
}

# primary = NUMBER | STRING | TRUE | FALSE | NIL
#         | NAME [ LPAREN arg_list RPAREN ]
#         | KEYWORD [ LPAREN arg_list RPAREN ]   (e.g. puts("hello"))
#         | LPAREN expression RPAREN
#
# Ruby method calls can use keywords (like puts) as the method name.
sub _parse_primary {
    my ($self) = @_;
    my $tok  = $self->_peek();
    my $type = $tok->{type};

    # --- Numeric literal ---
    if ($type eq 'NUMBER') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }

    # --- String literal ---
    if ($type eq 'STRING') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }

    # --- Boolean / nil literals ---
    if ($type eq 'TRUE' || $type eq 'FALSE' || $type eq 'NIL') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }

    # --- Identifier or method call (NAME) ---
    if ($type eq 'NAME') {
        my $name_leaf = $self->_leaf($self->_advance());
        if ($self->_check('LPAREN')) {
            my @ch = ($name_leaf);
            push @ch, $self->_leaf($self->_advance());    # (
            push @ch, $self->_parse_arg_list();
            push @ch, $self->_leaf($self->_expect('RPAREN'));
            return $self->_node('call_expr', @ch);
        }
        return $self->_node('primary', $name_leaf);
    }

    # --- Keyword used as method with parens, e.g. puts("hello") ---
    # Ruby keywords like puts, require, etc. can appear as method calls.
    if ($type eq 'PUTS' || $type eq 'REQUIRE') {
        my $kw_leaf = $self->_leaf($self->_advance());
        if ($self->_check('LPAREN')) {
            my @ch = ($kw_leaf);
            push @ch, $self->_leaf($self->_advance());    # (
            push @ch, $self->_parse_arg_list();
            push @ch, $self->_leaf($self->_expect('RPAREN'));
            return $self->_node('call_expr', @ch);
        }
        return $self->_node('primary', $kw_leaf);
    }

    # --- Parenthesized expression ---
    if ($type eq 'LPAREN') {
        my $lparen = $self->_leaf($self->_advance());
        my $inner  = $self->_parse_expression();
        my $rparen = $self->_leaf($self->_expect('RPAREN'));
        return $self->_node('primary', $lparen, $inner, $rparen);
    }

    die sprintf(
        "CodingAdventures::RubyParser: unexpected token '%s' (type %s) "
      . "at line %d col %d\n",
        $tok->{value}, $tok->{type}, $tok->{line}, $tok->{col}
    );
}

# arg_list = [ expression { COMMA expression } ] ;
sub _parse_arg_list {
    my ($self) = @_;
    my @ch;
    if (!$self->_check('RPAREN')) {
        push @ch, $self->_parse_expression();
        while ($self->_check('COMMA')) {
            push @ch, $self->_leaf($self->_advance());
            push @ch, $self->_parse_expression();
        }
    }
    return $self->_node('arg_list', @ch);
}

# ============================================================================
# Class-method convenience wrapper
# ============================================================================

# --- parse_ruby($source) ------------------------------------------------------
#
# Convenience class method: tokenize and parse in one call.
# Returns the root ASTNode. Dies on error.

sub parse_ruby {
    my ($class, $source) = @_;
    my $parser = $class->new($source);
    return $parser->parse();
}

1;

__END__

=head1 NAME

CodingAdventures::RubyParser - Hand-written recursive-descent Ruby parser

=head1 SYNOPSIS

    use CodingAdventures::RubyParser;

    # Object-oriented
    my $parser = CodingAdventures::RubyParser->new("x = 5");
    my $ast    = $parser->parse();
    print $ast->rule_name;   # "program"

    # Convenience class method
    my $ast = CodingAdventures::RubyParser->parse_ruby("x = 1 + 2");

=head1 DESCRIPTION

A hand-written recursive-descent parser for a practical subset of Ruby.
Tokenizes input with C<CodingAdventures::RubyLexer> and builds an
Abstract Syntax Tree (AST) of C<CodingAdventures::RubyParser::ASTNode> nodes.

Supported constructs: assignments, method definitions (def/end), class
definitions, if/elsif/else statements, while/until loops, return statements,
method calls (with and without parentheses), and full expression parsing with
correct operator precedence.

Ruby uses C<end> keywords to close blocks (not indentation), which makes it
straightforward to parse iteratively.

=head1 METHODS

=head2 new($source)

Tokenize C<$source> with C<RubyLexer> and return a parser instance.

=head2 parse()

Parse and return the root AST node (rule_name C<"program">). Dies on error.

=head2 parse_ruby($source)

Class method — tokenize and parse in one call. Returns the root ASTNode.

=head1 AST NODE FORMAT

Each node is a C<CodingAdventures::RubyParser::ASTNode>:

    $node->rule_name   # e.g. "method_def", "if_stmt", "binary_expr"
    $node->children    # arrayref of child nodes
    $node->is_leaf     # 1 for leaf (token) nodes, 0 for inner nodes
    $node->token       # token hashref (leaf nodes only): {type, value, line, col}

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
