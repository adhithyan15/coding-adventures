package CodingAdventures::PythonParser;

# ============================================================================
# CodingAdventures::PythonParser — Hand-written recursive-descent Python parser
# ============================================================================
#
# This module parses a subset of Python into an Abstract Syntax Tree (AST).
# The parser is hand-written using the recursive-descent technique: each
# grammar rule is encoded as one Perl method, and rules call each other
# recursively.
#
# # What language do we parse?
# =============================
#
# A practical subset covering the constructs most commonly taught first:
#
#   x = 5                           — assignment
#   def add(a, b):                  — function definition
#       return a + b
#   if x > 0:                       — if statement
#       return x
#   elif x == 0:                    — elif clause
#       return 0
#   else:                           — else clause
#       return -1
#   for i in range(10):             — for loop (for NAME in expr)
#       print(i)
#   while x > 0:                    — while loop
#       x = x - 1
#   return value                    — return statement
#   import math                     — import statement
#   from math import sqrt           — from-import statement
#   print("hello")                  — function call
#   1 + 2 * 3                       — binary expression (precedence)
#
# # Token types from CodingAdventures::PythonLexer
# =================================================
#
# The Python lexer emits keyword tokens using the keyword name as the type:
#
#   DEF, RETURN, IF, ELIF, ELSE, FOR, WHILE, CLASS,
#   IMPORT, FROM, AS, TRUE, FALSE, NONE
#
# Other types:
#   NAME        — identifiers (e.g. x, foo, range)
#   NUMBER      — integer literals (e.g. 42, 0)
#   STRING      — double-quoted string literals
#   EQUALS      — =
#   EQUALS_EQUALS — ==
#   PLUS        — +
#   MINUS       — -
#   STAR        — *
#   SLASH       — /
#   LPAREN      — (
#   RPAREN      — )
#   COMMA       — ,
#   COLON       — :
#   INDENT      — indentation increase (block start)
#   DEDENT      — indentation decrease (block end)
#   NEWLINE     — end of logical line
#   EOF         — end of input
#
# # Python's indentation-based blocks
# =====================================
#
# Python uses INDENT/DEDENT tokens to delimit blocks, not braces:
#
#   def add(a, b):
#       return a + b      ← INDENT before, DEDENT after
#
# The lexer emits INDENT before the first indented line and DEDENT when
# returning to the outer level.  Our block rule consumes:
#
#   COLON NEWLINE INDENT { statement } DEDENT
#
# # Operator precedence (lowest to highest)
# ==========================================
#
#   Comparison    →  ==  >  <  >=  <=
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
#   function_def     — def NAME(params): block
#   if_stmt          — if expr: block [elif …] [else: block]
#   for_stmt         — for NAME in expression: block
#   while_stmt       — while expression: block
#   return_stmt      — return [expression]
#   import_stmt      — import NAME [as NAME]
#   from_import_stmt — from NAME import NAME [as NAME]
#   expression_stmt  — expression (stand-alone)
#   block            — INDENT { statement } DEDENT
#   expression       — full expression (lowest-precedence entry point)
#   binary_expr      — left op right
#   unary_expr       — - expr
#   call_expr        — NAME ( args )
#   primary          — literal, identifier, grouped expression
#   param_list       — comma-separated parameter names
#   arg_list         — comma-separated argument expressions
#   token            — leaf node wrapping a single lexer token
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::PythonLexer;
use CodingAdventures::PythonParser::ASTNode;

# ============================================================================
# Constructor
# ============================================================================

# --- new($source) -------------------------------------------------------------
#
# Tokenize `$source` with PythonLexer and return a ready-to-parse parser.

sub new {
    my ($class, $source) = @_;
    # Pin to legacy grammar — the Perl lexer's regex engine has compatibility
    # issues with the versioned Python grammars (complex string patterns and
    # indentation mode). Use the old python.tokens which has simple patterns
    # and no indentation mode.
    # TODO: fix Perl lexer regex compatibility for versioned grammars.
    my $tokens = CodingAdventures::PythonLexer->tokenize($source, 'legacy');
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
            "CodingAdventures::PythonParser: parse error at line %d col %d: "
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

# Skip over NEWLINE tokens (used between statements at the top level).
sub _skip_newlines {
    my ($self) = @_;
    while ($self->_check('NEWLINE')) {
        $self->_advance();
    }
}

# Wrap a token as a leaf ASTNode.
sub _leaf {
    my ($self, $tok) = @_;
    return CodingAdventures::PythonParser::ASTNode->new_leaf($tok);
}

# Create an inner ASTNode.
sub _node {
    my ($self, $rule_name, @children) = @_;
    return CodingAdventures::PythonParser::ASTNode->new($rule_name, \@children);
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
# Grammar rules — each method parses one grammar rule
# ============================================================================

# program = { statement } ;
#
# Keeps parsing until EOF.  NEWLINE tokens between statements are skipped.
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

# statement = assignment | function_def | if_stmt | for_stmt | while_stmt
#           | return_stmt | import_stmt | from_import_stmt | expression_stmt ;
#
# Dispatch based on the current token.  For the assignment vs. call/expression
# ambiguity we look one token ahead.
sub _parse_statement {
    my ($self) = @_;
    my $tok  = $self->_peek();
    my $type = $tok->{type};

    # Function definition: def NAME(params): block
    if ($type eq 'DEF') {
        return $self->_node('statement', $self->_parse_function_def());
    }

    # If statement
    if ($type eq 'IF') {
        return $self->_node('statement', $self->_parse_if_stmt());
    }

    # For loop: for NAME in expr: block
    if ($type eq 'FOR') {
        return $self->_node('statement', $self->_parse_for_stmt());
    }

    # While loop
    if ($type eq 'WHILE') {
        return $self->_node('statement', $self->_parse_while_stmt());
    }

    # Return statement
    if ($type eq 'RETURN') {
        return $self->_node('statement', $self->_parse_return_stmt());
    }

    # Import: import NAME [as NAME]
    if ($type eq 'IMPORT') {
        return $self->_node('statement', $self->_parse_import_stmt());
    }

    # From-import: from NAME import NAME [as NAME]
    if ($type eq 'FROM') {
        return $self->_node('statement', $self->_parse_from_import_stmt());
    }

    # Assignment: NAME EQUALS expr   (lookahead: NAME followed by plain EQUALS)
    if ($type eq 'NAME') {
        my $next = $self->_peek_ahead(1);
        if ($next->{type} eq 'EQUALS') {
            return $self->_node('statement', $self->_parse_assignment());
        }
    }

    # Expression statement (default — function calls, standalone expressions)
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
    # Consume optional trailing NEWLINE
    $self->_match('NEWLINE');
    return $self->_node('assignment', @ch);
}

# expression_stmt = expression ;
#
# A bare expression used as a statement (e.g. a function call).
sub _parse_expression_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_expression();
    $self->_match('NEWLINE');
    return $self->_node('expression_stmt', @ch);
}

# function_def = DEF NAME LPAREN param_list RPAREN COLON block ;
#
# Example:  def add(a, b):
#               return a + b
sub _parse_function_def {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('DEF'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    push @ch, $self->_parse_param_list();
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    push @ch, $self->_leaf($self->_expect('COLON'));
    push @ch, $self->_parse_block();
    return $self->_node('function_def', @ch);
}

# param_list = [ NAME { COMMA NAME } ] ;
#
# Zero or more comma-separated parameter names.
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

# if_stmt = IF expression COLON block
#           { ELIF expression COLON block }
#           [ ELSE COLON block ] ;
#
# Example:  if x > 0:
#               return x
#           elif x == 0:
#               return 0
#           else:
#               return -1
sub _parse_if_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('IF'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('COLON'));
    push @ch, $self->_parse_block();

    # elif chains
    while ($self->_check('ELIF')) {
        push @ch, $self->_leaf($self->_advance());   # elif keyword
        push @ch, $self->_parse_expression();
        push @ch, $self->_leaf($self->_expect('COLON'));
        push @ch, $self->_parse_block();
    }

    # optional else
    if ($self->_check('ELSE')) {
        push @ch, $self->_leaf($self->_advance());   # else keyword
        push @ch, $self->_leaf($self->_expect('COLON'));
        push @ch, $self->_parse_block();
    }

    return $self->_node('if_stmt', @ch);
}

# for_stmt = FOR NAME IN expression COLON block ;
#
# Python for-loops iterate over an expression (commonly range(n)).
# The IN keyword is emitted as a NAME token with value "in" by the lexer
# (it is not in the keywords list), so we match by value.
#
# Example:  for i in range(10):
#               print(i)
sub _parse_for_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('FOR'));
    push @ch, $self->_leaf($self->_expect('NAME'));   # loop variable

    # `in` is not a dedicated keyword in python.tokens; it tokenizes as NAME.
    my $in_tok = $self->_peek();
    unless ($in_tok->{type} eq 'NAME' && $in_tok->{value} eq 'in') {
        die sprintf(
            "CodingAdventures::PythonParser: parse error at line %d col %d: "
          . "expected 'in' but got %s ('%s')\n",
            $in_tok->{line}, $in_tok->{col}, $in_tok->{type}, $in_tok->{value}
        );
    }
    push @ch, $self->_leaf($self->_advance());        # in

    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('COLON'));
    push @ch, $self->_parse_block();
    return $self->_node('for_stmt', @ch);
}

# while_stmt = WHILE expression COLON block ;
#
# Example:  while x > 0:
#               x = x - 1
sub _parse_while_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('WHILE'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('COLON'));
    push @ch, $self->_parse_block();
    return $self->_node('while_stmt', @ch);
}

# return_stmt = RETURN [ expression ] ;
#
# In Python, `return` may stand alone (returns None) or carry an expression.
sub _parse_return_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('RETURN'));
    # Return value present unless at end-of-line, DEDENT, or EOF
    unless ($self->_check('NEWLINE') || $self->_check('DEDENT') || $self->_check('EOF')) {
        push @ch, $self->_parse_expression();
    }
    $self->_match('NEWLINE');
    return $self->_node('return_stmt', @ch);
}

# import_stmt = IMPORT NAME [ AS NAME ] ;
#
# Example:  import math       import os.path as osp
sub _parse_import_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('IMPORT'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    if ($self->_check('AS')) {
        push @ch, $self->_leaf($self->_advance());   # as
        push @ch, $self->_leaf($self->_expect('NAME'));
    }
    $self->_match('NEWLINE');
    return $self->_node('import_stmt', @ch);
}

# from_import_stmt = FROM NAME IMPORT NAME [ AS NAME ] ;
#
# Example:  from math import sqrt       from os import path as p
sub _parse_from_import_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('FROM'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('IMPORT'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    if ($self->_check('AS')) {
        push @ch, $self->_leaf($self->_advance());   # as
        push @ch, $self->_leaf($self->_expect('NAME'));
    }
    $self->_match('NEWLINE');
    return $self->_node('from_import_stmt', @ch);
}

# block = NEWLINE INDENT { statement } DEDENT
#       | statement ;   ← single-line form: def f(): return x
#
# Python allows both the indented multi-line form and a single-statement
# inline form (e.g., `def f(): return x`). We detect which form by
# checking whether the next token is NEWLINE or the start of a statement.
sub _parse_block {
    my ($self) = @_;
    my @ch;
    # Single-line form: body follows immediately on the same line.
    if (!$self->_check('NEWLINE') && !$self->_check('EOF')) {
        push @ch, $self->_parse_statement();
        return $self->_node('block', @ch);
    }
    push @ch, $self->_leaf($self->_expect('NEWLINE'));
    push @ch, $self->_leaf($self->_expect('INDENT'));
    $self->_skip_newlines();
    while (!$self->_check('DEDENT') && !$self->_check('EOF')) {
        push @ch, $self->_parse_statement();
        $self->_skip_newlines();
    }
    push @ch, $self->_leaf($self->_expect('DEDENT'));
    return $self->_node('block', @ch);
}

# ============================================================================
# Expression parsing — precedence climbing
# ============================================================================
#
# We implement operator precedence by nesting rules, each level handling one
# tier of operators and delegating to the next higher tier for operands.
#
# expression → comparison → additive → multiplicative → unary → primary
#
# This structure guarantees:
#   1 + 2 * 3  parses as  1 + (2 * 3)   — multiplication tighter
#   -x == y    parses as  (-x) == y      — unary tighter than comparison

# expression = comparison ;
sub _parse_expression {
    my ($self) = @_;
    my $inner = $self->_parse_comparison();
    return $self->_node('expression', $inner);
}

# comparison = additive { ( EQUALS_EQUALS | GREATER_THAN | LESS_THAN |
#                           GREATER_EQUALS | LESS_EQUALS ) additive } ;
#
# Python.tokens defines EQUALS_EQUALS but not the comparison operators
# GREATER_THAN, LESS_THAN etc. as named tokens — they are not in the
# grammar file.  We map the raw token values to type names here.
# Looking at python.tokens: EQUALS_EQUALS is defined; LESS_THAN/GREATER_THAN
# are not, so the lexer will emit them as part of NAME or leave them
# unrecognized. Since python.tokens does NOT define >, < etc., the lexer
# would fail on those characters unless they are handled another way.
#
# Given the limited grammar in python.grammar and python.tokens,
# we handle EQUALS_EQUALS for comparison only, plus NAME-as-operator
# fallback for robustness.
sub _parse_comparison {
    my ($self) = @_;
    my $left = $self->_parse_additive();
    while ($self->_check('EQUALS_EQUALS')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_additive();
        $left = $self->_node('binary_expr', $left, $op, $right);
    }
    return $left;
}

# additive = multiplicative { ( PLUS | MINUS ) multiplicative } ;
#
# This tier encodes that + and - bind less tightly than * and /.
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
#
# Unary minus:  -x   -1
sub _parse_unary {
    my ($self) = @_;
    if ($self->_check('MINUS')) {
        my $op   = $self->_leaf($self->_advance());
        my $expr = $self->_parse_unary();
        return $self->_node('unary_expr', $op, $expr);
    }
    return $self->_parse_primary();
}

# primary = NUMBER | STRING | TRUE | FALSE | NONE
#         | NAME [ LPAREN arg_list RPAREN ]
#         | LPAREN expression RPAREN
#
# This is the highest-precedence rule.  It handles literals, identifiers,
# function calls, and grouped expressions.
sub _parse_primary {
    my ($self) = @_;
    my $tok  = $self->_peek();
    my $type = $tok->{type};

    # --- Numeric literal ---
    # Accept NUMBER (old grammar), INT, and FLOAT (versioned grammars).
    if ($type eq 'NUMBER' || $type eq 'INT' || $type eq 'FLOAT') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }

    # --- String literal ---
    if ($type eq 'STRING') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }

    # --- Boolean / None literals ---
    if ($type eq 'TRUE' || $type eq 'FALSE' || $type eq 'NONE') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }

    # --- Identifier or function call ---
    # If a NAME is immediately followed by LPAREN, it is a call.
    if ($type eq 'NAME') {
        my $name_leaf = $self->_leaf($self->_advance());
        if ($self->_check('LPAREN')) {
            my @ch = ($name_leaf);
            push @ch, $self->_leaf($self->_advance());      # (
            push @ch, $self->_parse_arg_list();
            push @ch, $self->_leaf($self->_expect('RPAREN'));
            return $self->_node('call_expr', @ch);
        }
        return $self->_node('primary', $name_leaf);
    }

    # --- Parenthesized expression ---
    if ($type eq 'LPAREN') {
        my $lparen = $self->_leaf($self->_advance());
        my $inner  = $self->_parse_expression();
        my $rparen = $self->_leaf($self->_expect('RPAREN'));
        return $self->_node('primary', $lparen, $inner, $rparen);
    }

    die sprintf(
        "CodingAdventures::PythonParser: unexpected token '%s' (type %s) "
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

# --- parse_python($source) ----------------------------------------------------
#
# Convenience class method: tokenize and parse in one call.
# Returns the root ASTNode. Dies on error.

sub parse_python {
    my ($class, $source) = @_;
    my $parser = $class->new($source);
    return $parser->parse();
}

1;

__END__

=head1 NAME

CodingAdventures::PythonParser - Hand-written recursive-descent Python parser

=head1 SYNOPSIS

    use CodingAdventures::PythonParser;

    # Object-oriented
    my $parser = CodingAdventures::PythonParser->new("x = 5");
    my $ast    = $parser->parse();
    print $ast->rule_name;   # "program"

    # Convenience class method
    my $ast = CodingAdventures::PythonParser->parse_python("x = 1 + 2");

=head1 DESCRIPTION

A hand-written recursive-descent parser for a practical subset of Python.
Tokenizes input with C<CodingAdventures::PythonLexer> and builds an
Abstract Syntax Tree (AST) of C<CodingAdventures::PythonParser::ASTNode> nodes.

Supported constructs: assignments, function definitions (def), if/elif/else
statements, for loops, while loops, return statements, import and from-import
statements, function calls, and full expression parsing with correct operator
precedence (comparison > additive > multiplicative > unary > primary).

=head1 METHODS

=head2 new($source)

Tokenize C<$source> with C<PythonLexer> and return a parser instance.

=head2 parse()

Parse and return the root AST node (rule_name C<"program">). Dies on error.

=head2 parse_python($source)

Class method — tokenize and parse in one call. Returns the root ASTNode.

=head1 AST NODE FORMAT

Each node is a C<CodingAdventures::PythonParser::ASTNode>:

    $node->rule_name   # e.g. "assignment", "if_stmt", "binary_expr"
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
