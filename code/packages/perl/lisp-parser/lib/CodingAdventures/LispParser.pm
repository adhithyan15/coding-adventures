package CodingAdventures::LispParser;

# ============================================================================
# CodingAdventures::LispParser — Hand-written recursive-descent Lisp parser
# ============================================================================
#
# This module parses Lisp/Scheme source text into an Abstract Syntax Tree
# (AST) using a hand-written recursive-descent approach.
#
# # Why recursive descent for Lisp?
#
# Lisp has the smallest and most regular grammar of any practical programming
# language.  Every production rule maps directly to a function.  The grammar
# has no ambiguity, no operator precedence table, no dangling-else problem.
# A recursive descent parser for Lisp is the simplest possible parser for
# any language with nontrivial structure.
#
# # The Lisp Grammar
#
#   program   = { sexpr }
#   sexpr     = atom | list | quoted
#   atom      = NUMBER | SYMBOL | STRING
#   list      = LPAREN list_body RPAREN
#   list_body = [ sexpr { sexpr } [ DOT sexpr ] ]
#   quoted    = QUOTE sexpr
#
# Reading this grammar aloud:
#
#   A **program** is zero or more S-expressions.
#
#   An **S-expression** (sexpr) is one of:
#     - an atom (a leaf value)
#     - a list (a parenthesized collection)
#     - a quoted expression (tick shorthand for (quote ...))
#
#   An **atom** is a number, symbol, or string literal.
#
#   A **list** is an open paren, a list_body, and a close paren.
#
#   A **list_body** is optionally: one or more sexprs, optionally followed
#     by a DOT and one more sexpr.  (The DOT case gives dotted pairs.)
#
#   A **quoted** expression is a tick ' followed by any sexpr.
#
# # Decision table
#
# At each point in the parse, we look at the current token to decide which
# production to apply (LL(1) grammar — one token of lookahead suffices):
#
#   parse_sexpr:
#     LPAREN  → parse_list()
#     QUOTE   → parse_quoted()
#     NUMBER  → parse_atom()
#     SYMBOL  → parse_atom()
#     STRING  → parse_atom()
#     else    → die (unexpected token)
#
#   parse_list_body:
#     RPAREN  → empty body (return immediately)
#     else    → parse one sexpr, then loop:
#       RPAREN → done
#       DOT    → consume DOT, parse one more sexpr, then expect RPAREN
#       else   → parse another sexpr (continue loop)
#
#   parse_program:
#     EOF     → empty program or end of program
#     else    → parse a sexpr, recurse
#
# # What is a dotted pair?
#
# Lisp lists are built from cons cells.  A cons cell is a pair (car, cdr).
# A proper list (1 2 3) is:
#
#   cons(1, cons(2, cons(3, nil)))
#
# The DOT notation writes cons cells literally:
#   (1 . (2 . (3 . nil)))  ← same as (1 2 3)
#   (a . b)                ← a "dotted pair" / improper list
#
# In list_body, the optional [DOT sexpr] captures the cdr value explicitly.
#
# # What is a quoted form?
#
# 'x means (quote x).  Quoting prevents evaluation.
#
#   (define colors '(red green blue))
#
# Without the quote, the interpreter would try to call (red green blue) as
# a function invocation.  With the quote, it receives the list as data.
#
# # Parse state
#
# The parser keeps two package-level variables:
#   $_tokens — arrayref of token hashrefs from CodingAdventures::LispLexer
#   $_pos    — current position (0-based index into @$_tokens)
#
# These are set by `parse()` on each call.  The parser is not re-entrant,
# but Lisp parsing is synchronous so that's fine in practice.
#
# # Error messages
#
# When a token doesn't match expectations, `_expect` dies with:
#   "Expected TYPE, got ACTUAL_TYPE ('value') at line L col C"
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::LispLexer;
use CodingAdventures::LispParser::ASTNode;

# Package-level parse state.
# Set at the start of each `parse()` call.
my ($_tokens, $_pos);

# ============================================================================
# Public API
# ============================================================================

# --- parse($class, $source) ---------------------------------------------------
#
# Parse a Lisp/Scheme source string and return the root ASTNode.
#
# Steps:
#   1. Tokenize `$source` using CodingAdventures::LispLexer.
#   2. Initialize parser state ($_tokens, $_pos).
#   3. Parse the `program` production (zero or more sexprs).
#   4. Assert that we consumed all tokens (only EOF remains).
#   5. Return the root ASTNode (rule_name == "program").
#
# Dies on lexer errors or parse errors.
#
# @param  $source  string  The Lisp/Scheme text to parse.
# @return ASTNode          Root node with rule_name "program".

sub parse {
    my ($class, $source) = @_;

    # Tokenize
    my $toks = CodingAdventures::LispLexer->tokenize($source);
    $_tokens = $toks;
    $_pos    = 0;

    # Parse the top-level program
    my $ast = _parse_program();

    # Verify we consumed everything — only EOF should remain
    my $t = _peek();
    if ($t->{type} ne 'EOF') {
        die sprintf(
            "CodingAdventures::LispParser: trailing content at line %d col %d: "
          . "unexpected %s ('%s')",
            $t->{line}, $t->{col}, $t->{type}, $t->{value}
        );
    }

    return $ast;
}

# ============================================================================
# Internal helpers
# ============================================================================

# --- _peek() ------------------------------------------------------------------
#
# Return the current token without consuming it.
# When past the end of the token list, returns the last token (EOF).

sub _peek {
    return $_tokens->[$_pos] // $_tokens->[-1];
}

# --- _advance() ---------------------------------------------------------------
#
# Consume and return the current token, advancing the position.

sub _advance {
    my $t = _peek();
    $_pos++;
    return $t;
}

# --- _expect($type) -----------------------------------------------------------
#
# Assert that the current token has type `$type`, consume it, and return it.
# Dies with a descriptive message if the type doesn't match.

sub _expect {
    my ($type) = @_;
    my $t = _peek();
    unless ($t->{type} eq $type) {
        die sprintf(
            "CodingAdventures::LispParser: Expected %s, got %s ('%s') "
          . "at line %d col %d",
            $type, $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _advance();
}

# --- _is($type) ---------------------------------------------------------------
#
# Return true if the current token has the given type, without consuming it.

sub _is {
    my ($type) = @_;
    return _peek()->{type} eq $type;
}

# --- _node($rule_name, $children_aref) ----------------------------------------
#
# Construct an internal (non-leaf) ASTNode.

sub _node {
    my ($rule, $children) = @_;
    return CodingAdventures::LispParser::ASTNode->new(
        rule_name => $rule,
        children  => $children,
        is_leaf   => 0,
    );
}

# --- _leaf($token) ------------------------------------------------------------
#
# Construct a leaf ASTNode wrapping a single token.

sub _leaf {
    my ($tok) = @_;
    return CodingAdventures::LispParser::ASTNode->new(
        rule_name => 'token',
        children  => [],
        is_leaf   => 1,
        token     => $tok,
    );
}

# ============================================================================
# Recursive descent productions
# ============================================================================

# --- _parse_program() ---------------------------------------------------------
#
# Grammar: program = { sexpr }
#
# A program is a sequence of zero or more S-expressions.  We keep parsing
# sexprs until we hit EOF (or an unexpected RPAREN, which is a parse error
# handled by _parse_sexpr).
#
# In Lisp, top-level expressions are usually:
#   (define ...) — variable/function definitions
#   (require ...) — module imports
#   bare atoms   — evaluated for their side effects
#
# A valid empty program (empty file or whitespace/comment-only) produces
# a program node with no children.

sub _parse_program {
    my @children;

    while ( !_is('EOF') ) {
        # Any token other than EOF can start a sexpr.
        # If it's RPAREN, _parse_sexpr will die with a helpful message.
        push @children, _parse_sexpr();
    }

    return _node('program', \@children);
}

# --- _parse_sexpr() -----------------------------------------------------------
#
# Grammar: sexpr = atom | list | quoted
#
# Decision table (current token type → production):
#
#   LPAREN  → _parse_list()
#   QUOTE   → _parse_quoted()
#   NUMBER  → _parse_atom()
#   SYMBOL  → _parse_atom()
#   STRING  → _parse_atom()
#   EOF     → die (expected an expression, got end-of-file)
#   RPAREN  → die (unmatched close paren)
#   else    → die (unexpected token)

sub _parse_sexpr {
    my $t    = _peek();
    my $type = $t->{type};

    # A list starts with LPAREN
    if ($type eq 'LPAREN') {
        return _node('sexpr', [_parse_list()]);
    }

    # A quoted form starts with QUOTE (the ' character)
    if ($type eq 'QUOTE') {
        return _node('sexpr', [_parse_quoted()]);
    }

    # An atom is a NUMBER, SYMBOL, or STRING
    if ($type eq 'NUMBER' || $type eq 'SYMBOL' || $type eq 'STRING') {
        return _node('sexpr', [_parse_atom()]);
    }

    # Everything else is an error.
    die sprintf(
        "CodingAdventures::LispParser: Unexpected token %s ('%s') "
      . "at line %d col %d — expected an S-expression",
        $type, $t->{value}, $t->{line}, $t->{col}
    );
}

# --- _parse_atom() ------------------------------------------------------------
#
# Grammar: atom = NUMBER | SYMBOL | STRING
#
# Atoms are self-evaluating values (numbers, strings) or symbolic references
# (symbols).  We consume the current token and wrap it in a leaf node.

sub _parse_atom {
    my $t = _peek();
    my $type = $t->{type};

    unless ($type eq 'NUMBER' || $type eq 'SYMBOL' || $type eq 'STRING') {
        die sprintf(
            "CodingAdventures::LispParser: Expected atom (NUMBER|SYMBOL|STRING), "
          . "got %s ('%s') at line %d col %d",
            $type, $t->{value}, $t->{line}, $t->{col}
        );
    }

    my $tok = _advance();
    return _node('atom', [_leaf($tok)]);
}

# --- _parse_list() ------------------------------------------------------------
#
# Grammar: list = LPAREN list_body RPAREN
#
# A list is a parenthesized sequence of S-expressions.
# In Lisp, (f a b c) calls function f with arguments a, b, c.
# The empty list () is Lisp's nil — the terminator of every proper list.

sub _parse_list {
    my @children;

    my $open = _expect('LPAREN');
    push @children, _leaf($open);

    push @children, _parse_list_body();

    my $close = _expect('RPAREN');
    push @children, _leaf($close);

    return _node('list', \@children);
}

# --- _parse_list_body() -------------------------------------------------------
#
# Grammar: list_body = [ sexpr { sexpr } [ DOT sexpr ] ]
#
# A list body is optionally:
#   - One sexpr (the head/car of the list)
#   - Followed by zero or more additional sexprs
#   - Optionally terminated by DOT and one more sexpr (the explicit cdr)
#
# Examples:
#   ()          → empty body: no sexprs, no dot
#   (1 2 3)     → body with three sexprs: 1, 2, 3
#   (a . b)     → body: sexpr "a", DOT, sexpr "b"
#   (1 2 . 3)   → body: sexprs 1, 2, then DOT, sexpr 3

sub _parse_list_body {
    my @children;

    # Empty body: the next token is RPAREN — nothing to parse.
    if ( _is('RPAREN') ) {
        return _node('list_body', \@children);
    }

    # Parse the first sexpr (always present in a non-empty body)
    push @children, _parse_sexpr();

    # Parse remaining sexprs and optional DOT
    while ( !_is('RPAREN') && !_is('EOF') ) {
        if ( _is('DOT') ) {
            # DOT sexpr — the explicit cdr value.
            # After this, we MUST see RPAREN.  No more sexprs are allowed.
            my $dot = _advance();    # consume DOT
            push @children, _leaf($dot);
            push @children, _parse_sexpr();   # the cdr value
            # The RPAREN is consumed by _parse_list(), not here.
            last;
        } else {
            push @children, _parse_sexpr();
        }
    }

    return _node('list_body', \@children);
}

# --- _parse_quoted() ----------------------------------------------------------
#
# Grammar: quoted = QUOTE sexpr
#
# The tick character ' is syntactic sugar for (quote ...).
# 'x  ≡  (quote x)
# '(1 2 3)  ≡  (quote (1 2 3))
#
# The Lisp evaluator will see a `quoted` node and return its sexpr child
# without evaluation — this is the foundation of Lisp's data/code duality.

sub _parse_quoted {
    my @children;

    my $quote_tok = _expect('QUOTE');
    push @children, _leaf($quote_tok);

    push @children, _parse_sexpr();

    return _node('quoted', \@children);
}

1;

__END__

=head1 NAME

CodingAdventures::LispParser - Hand-written recursive-descent Lisp parser

=head1 SYNOPSIS

    use CodingAdventures::LispParser;

    my $ast = CodingAdventures::LispParser->parse('(define x 42)');
    print $ast->rule_name;    # "program"

    # Walk the tree
    sub walk {
        my ($node, $depth) = @_;
        my $indent = '  ' x $depth;
        if ($node->is_leaf) {
            printf "%sToken(%s, %s)\n",
                $indent, $node->token->{type}, $node->token->{value};
        } else {
            printf "%s%s\n", $indent, $node->rule_name;
            walk($_, $depth + 1) for @{ $node->children };
        }
    }
    walk($ast, 0);

=head1 DESCRIPTION

A hand-written recursive-descent parser for Lisp/Scheme.  Tokenizes source
text using C<CodingAdventures::LispLexer> and constructs an AST using
C<CodingAdventures::LispParser::ASTNode>.

Implements all six Lisp grammar rules:

    program   = { sexpr }
    sexpr     = atom | list | quoted
    atom      = NUMBER | SYMBOL | STRING
    list      = LPAREN list_body RPAREN
    list_body = [ sexpr { sexpr } [ DOT sexpr ] ]
    quoted    = QUOTE sexpr

=head1 METHODS

=head2 parse($source)

Parse a Lisp/Scheme string.  Returns the root C<ASTNode> with
C<rule_name == "program">.  Dies on lexer or parser errors.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
