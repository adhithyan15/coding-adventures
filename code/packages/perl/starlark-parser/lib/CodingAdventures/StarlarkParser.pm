package CodingAdventures::StarlarkParser;

# ============================================================================
# CodingAdventures::StarlarkParser — Hand-written recursive-descent Starlark parser
# ============================================================================
#
# This module parses a subset of Starlark into an Abstract Syntax Tree (AST).
# The parser is hand-written using the recursive-descent technique: each grammar
# rule is encoded as one Perl method, and rules call each other recursively.
#
# # What is Starlark?
# ====================
#
# Starlark is a deterministic subset of Python used for configuration files —
# most famously in Bazel BUILD files. It is syntactically very close to Python
# but with key constraints that guarantee termination and determinism:
#
#   x = 1                         — assignment
#   cc_library(name="foo")        — function call (BUILD rule)
#   def greet(name): return name  — function definition
#   if score >= 90: grade = "A"   — if/elif/else
#   for item in items: use(item)  — for loop (NO while loop!)
#   load("//rules.star", "sym")   — import from another Starlark file
#   [x*2 for x in range(10)]      — list comprehension
#   {k: v for k, v in d.items()}  — dict comprehension
#   lambda x, y: x + y            — anonymous function
#
# # What Starlark does NOT have (vs Python)
# ==========================================
#
#   - while loops           (removed for termination guarantees)
#   - classes               (removed for simplicity)
#   - try/except/raise      (removed; errors are fatal)
#   - global/nonlocal       (removed; no mutable shared state)
#   - recursion             (disabled at runtime; not parse-time)
#   - most import machinery (replaced by load())
#
# # Token types from CodingAdventures::StarlarkLexer
# ====================================================
#
# Keywords (each is its own token type):
#
#   DEF, IF, ELIF, ELSE, FOR, IN, RETURN, BREAK, CONTINUE, PASS, LOAD
#   AND, OR, NOT, LAMBDA
#   TRUE, FALSE, NONE
#
# Operators:
#
#   EQUALS              — =
#   EQUALS_EQUALS       — ==
#   NOT_EQUALS          — !=
#   LESS_THAN           — <
#   GREATER_THAN        — >
#   LESS_EQUALS         — <=
#   GREATER_EQUALS      — >=
#   PLUS                — +
#   MINUS               — -
#   STAR                — *
#   SLASH               — /
#   FLOOR_DIV           — //
#   PERCENT             — %
#   DOUBLE_STAR         — **
#   AMP                 — &
#   PIPE                — |
#   CARET               — ^
#   TILDE               — ~
#   LEFT_SHIFT          — <<
#   RIGHT_SHIFT         — >>
#   PLUS_EQUALS         — +=
#   MINUS_EQUALS        — -=
#   STAR_EQUALS         — *=
#   SLASH_EQUALS        — /=
#   FLOOR_DIV_EQUALS    — //=
#   PERCENT_EQUALS      — %=
#   AMP_EQUALS          — &=
#   PIPE_EQUALS         — |=
#   CARET_EQUALS        — ^=
#   LEFT_SHIFT_EQUALS   — <<=
#   RIGHT_SHIFT_EQUALS  — >>=
#   DOUBLE_STAR_EQUALS  — **=
#
# Punctuation:
#
#   LPAREN, RPAREN      — ( )
#   LBRACKET, RBRACKET  — [ ]
#   LBRACE, RBRACE      — { }
#   COMMA               — ,
#   COLON               — :
#   DOT                 — .
#   SEMICOLON           — ;
#
# Literals:
#
#   INT     — integer literal: 42, 0, -1
#   FLOAT   — float literal: 3.14, 1e10
#   STRING  — string literal: "hello", 'world', """multi"""
#   NAME    — identifier: foo, bar, _private
#
# Structural (indentation mode):
#
#   NEWLINE — end of logical line
#   INDENT  — increase in indentation level
#   DEDENT  — decrease in indentation level
#   EOF     — end of input
#
# # Operator precedence (lowest to highest)
# ==========================================
#
#   1. lambda               lambda x: x + 1
#   2. if-else (ternary)    a if cond else b
#   3. or                   a or b
#   4. and                  a and b
#   5. not                  not a
#   6. comparisons          a == b, a in lst, a not in lst
#   7. bitwise |            a | b
#   8. bitwise ^            a ^ b
#   9. bitwise &            a & b
#  10. shifts               a << 1, a >> 1
#  11. additive             a + b, a - b
#  12. multiplicative       a * b, a // b, a % b
#  13. unary                -a, +a, ~a
#  14. power                a ** b
#  15. primary              x.attr, x[i], f(args)
#
# # AST node types (rule_name values)
# ====================================
#
#   program         — root; the whole file
#   statement       — wrapper for one statement
#   simple_stmt     — one or more small_stmt separated by ;
#   small_stmt      — one of: assign_stmt, return_stmt, …
#   assign_stmt     — assignment or expression statement
#   return_stmt     — return [expr]
#   break_stmt      — break
#   continue_stmt   — continue
#   pass_stmt       — pass
#   load_stmt       — load("module", "symbol")
#   compound_stmt   — if_stmt | for_stmt | def_stmt
#   if_stmt         — if/elif/else
#   for_stmt        — for vars in expr: suite
#   def_stmt        — def name(params): suite
#   suite           — indented block or inline simple_stmt
#   parameters      — parameter list in def
#   parameter       — one parameter (plain, default, *args, **kwargs)
#   expression_list — comma-separated expressions
#   expression      — top-level expression
#   lambda_expr     — lambda params: expr
#   or_expr         — a or b or c
#   and_expr        — a and b and c
#   not_expr        — not a
#   comparison      — a == b, a < b, a in lst
#   bitwise_or      — a | b
#   bitwise_xor     — a ^ b
#   bitwise_and     — a & b
#   shift           — a << b, a >> b
#   arith           — a + b, a - b
#   term            — a * b, a / b, a // b, a % b
#   factor          — unary +a, -a, ~a, or power
#   power           — a ** b
#   primary         — atom with optional suffixes (.attr, [i], (args))
#   suffix          — .NAME, [subscript], (arguments)
#   atom            — INT, FLOAT, STRING, NAME, True, False, None, list, dict, paren
#   list_expr       — [expr, ...]  or  [expr for ... in ...]
#   dict_expr       — {key: val, ...}  or  {k: v for ... in ...}
#   paren_expr      — (expr) or (expr, ...) tuple
#   arguments       — call arguments
#   argument        — one call argument
#   comp_clause     — for ... in ... [if ...]
#   comp_for        — for vars in expr
#   comp_if         — if expr
#   loop_vars       — names in for-loop header
#   lambda_params   — parameter list in lambda
#   lambda_param    — one lambda parameter
#   token           — leaf node wrapping a single lexer token
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::StarlarkLexer;
use CodingAdventures::StarlarkParser::ASTNode;

# ============================================================================
# Constructor
# ============================================================================

# --- new($source) -------------------------------------------------------------
#
# Tokenize `$source` with StarlarkLexer and return a ready-to-parse parser.

sub new {
    my ($class, $source) = @_;
    my $tokens = CodingAdventures::StarlarkLexer->tokenize($source);
    return bless {
        _tokens => $tokens,
        _pos    => 0,
    }, $class;
}

# ============================================================================
# Token helpers
# ============================================================================

# Peek at the current token without consuming it.
# Returns a synthetic EOF token if past the end of the stream.
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
            "CodingAdventures::StarlarkParser: parse error at line %d col %d: "
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

# Skip any NEWLINE tokens at the current position.
# Used to consume blank lines between top-level statements.
sub _skip_newlines {
    my ($self) = @_;
    while ($self->_check('NEWLINE')) {
        $self->_advance();
    }
}

# Wrap a token as a leaf ASTNode.
sub _leaf {
    my ($self, $tok) = @_;
    return CodingAdventures::StarlarkParser::ASTNode->new_leaf($tok);
}

# Create an inner ASTNode.
sub _node {
    my ($self, $rule_name, @children) = @_;
    return CodingAdventures::StarlarkParser::ASTNode->new($rule_name, \@children);
}

# Return 1 if the current token is an augmented assignment operator.
# Augmented operators: +=, -=, *=, /=, //=, %=, &=, |=, ^=, <<=, >>=, **=
sub _is_augmented_assign {
    my ($self) = @_;
    my $t = $self->_peek()->{type};
    return grep { $t eq $_ } qw(
        PLUS_EQUALS MINUS_EQUALS STAR_EQUALS SLASH_EQUALS
        FLOOR_DIV_EQUALS PERCENT_EQUALS AMP_EQUALS PIPE_EQUALS
        CARET_EQUALS LEFT_SHIFT_EQUALS RIGHT_SHIFT_EQUALS DOUBLE_STAR_EQUALS
    );
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
#
# The grammar is defined in code/grammars/starlark.grammar. We implement
# the same rules here as a recursive-descent parser. Each _parse_RULENAME
# method corresponds to one rule in the grammar.

# program = { NEWLINE | statement } ;
#
# The top-level "file" in the grammar. A Starlark file is a sequence of
# statements, possibly separated by blank lines (which appear as NEWLINE tokens).
# The program node is the root of the AST.
sub _parse_program {
    my ($self) = @_;
    my @children;
    while (!$self->_check('EOF')) {
        # Blank lines produce NEWLINE tokens; skip them between statements.
        if ($self->_check('NEWLINE')) {
            $self->_advance();
            next;
        }
        push @children, $self->_parse_statement();
    }
    return $self->_node('program', @children);
}

# statement = compound_stmt | simple_stmt ;
#
# Dispatch: compound statements start with keywords DEF, IF, FOR.
# Everything else is a simple statement.
sub _parse_statement {
    my ($self) = @_;
    my $type = $self->_peek()->{type};

    # Compound statements begin with a specific keyword.
    if ($type eq 'DEF') {
        return $self->_node('statement', $self->_parse_def_stmt());
    }
    if ($type eq 'IF') {
        return $self->_node('statement', $self->_parse_if_stmt());
    }
    if ($type eq 'FOR') {
        return $self->_node('statement', $self->_parse_for_stmt());
    }

    # Everything else is a simple statement.
    return $self->_node('statement', $self->_parse_simple_stmt());
}

# simple_stmt = small_stmt { SEMICOLON small_stmt } NEWLINE ;
#
# A simple statement line can contain multiple small statements chained
# with semicolons, terminated by a NEWLINE:
#   x = 1; y = 2; z = 3
sub _parse_simple_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_small_stmt();

    while ($self->_check('SEMICOLON')) {
        push @ch, $self->_leaf($self->_advance());
        # If next is NEWLINE or EOF, we're done (trailing semicolon)
        last if $self->_check('NEWLINE') || $self->_check('EOF')
             || $self->_check('DEDENT');
        push @ch, $self->_parse_small_stmt();
    }

    # Consume the terminating NEWLINE (or EOF at end of file).
    if ($self->_check('NEWLINE')) {
        push @ch, $self->_leaf($self->_advance());
    }

    return $self->_node('simple_stmt', @ch);
}

# small_stmt = return_stmt | break_stmt | continue_stmt | pass_stmt
#            | load_stmt | assign_stmt ;
#
# Dispatch on the current token type.
sub _parse_small_stmt {
    my ($self) = @_;
    my $type = $self->_peek()->{type};

    if ($type eq 'RETURN') {
        return $self->_parse_return_stmt();
    }
    if ($type eq 'BREAK') {
        my $tok = $self->_advance();
        return $self->_node('break_stmt', $self->_leaf($tok));
    }
    if ($type eq 'CONTINUE') {
        my $tok = $self->_advance();
        return $self->_node('continue_stmt', $self->_leaf($tok));
    }
    if ($type eq 'PASS') {
        my $tok = $self->_advance();
        return $self->_node('pass_stmt', $self->_leaf($tok));
    }
    if ($type eq 'LOAD') {
        return $self->_parse_load_stmt();
    }

    # Default: assignment or expression statement.
    return $self->_parse_assign_stmt();
}

# return_stmt = "return" [ expression ] ;
#
# Return exits the current function. The expression is optional:
#   return      → returns None
#   return x+1  → returns the value of x+1
sub _parse_return_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('RETURN'));

    # An expression follows if the next token is not end-of-statement.
    unless ($self->_check('NEWLINE') || $self->_check('SEMICOLON')
         || $self->_check('EOF')     || $self->_check('DEDENT')) {
        push @ch, $self->_parse_expression();
    }

    return $self->_node('return_stmt', @ch);
}

# load_stmt = "load" LPAREN STRING { COMMA load_arg } [ COMMA ] RPAREN ;
# load_arg  = NAME EQUALS STRING | STRING ;
#
# The load statement imports symbols from another Starlark module.
# The first argument is the module path (a string literal).
# Subsequent arguments specify which symbols to import:
#
#   load("//rules/python.star", "py_library")            — plain import
#   load("//rules/python.star", lib = "py_library")      — aliased import
#
# The alias form: local_name = "original_symbol_name"
sub _parse_load_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LOAD'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    push @ch, $self->_leaf($self->_expect('STRING'));  # module path

    # Parse remaining arguments (symbol imports).
    while ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());      # ,

        # Trailing comma: stop if next is )
        last if $self->_check('RPAREN');

        # load_arg = NAME EQUALS STRING | STRING
        if ($self->_check('NAME') && $self->_peek_ahead(1)->{type} eq 'EQUALS') {
            push @ch, $self->_node('load_arg',
                $self->_leaf($self->_expect('NAME')),
                $self->_leaf($self->_expect('EQUALS')),
                $self->_leaf($self->_expect('STRING')),
            );
        } else {
            push @ch, $self->_node('load_arg',
                $self->_leaf($self->_expect('STRING')),
            );
        }
    }

    push @ch, $self->_leaf($self->_expect('RPAREN'));
    return $self->_node('load_stmt', @ch);
}

# assign_stmt = expression_list [ ( assign_op | augmented_assign_op ) expression_list ] ;
#
# This handles three things via a single rule:
#
# 1. Expression statement:   print("hello")    [no RHS]
# 2. Simple assignment:      x = 1             [RHS with EQUALS]
# 3. Augmented assignment:   x += 1            [RHS with +=, -=, etc.]
# 4. Tuple unpacking:        a, b = 1, 2       [LHS is comma-separated]
#
# Note: we parse the LHS as an expression_list in all cases and then
# check for an optional assignment operator.
sub _parse_assign_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_expression_list();

    if ($self->_check('EQUALS')) {
        push @ch, $self->_node('assign_op', $self->_leaf($self->_advance()));
        push @ch, $self->_parse_expression_list();
    } elsif ($self->_is_augmented_assign()) {
        my $op_tok = $self->_advance();
        push @ch, $self->_node('augmented_assign_op', $self->_leaf($op_tok));
        push @ch, $self->_parse_expression_list();
    }
    # If neither, this is a plain expression statement.

    return $self->_node('assign_stmt', @ch);
}

# ============================================================================
# Compound Statements
# ============================================================================

# if_stmt = "if" expression COLON suite
#           { "elif" expression COLON suite }
#           [ "else" COLON suite ] ;
#
# Starlark if/elif/else mirrors Python exactly. The elif chain can be any length.
# Example:
#   if score >= 90:
#       grade = "A"
#   elif score >= 80:
#       grade = "B"
#   else:
#       grade = "F"
sub _parse_if_stmt {
    my ($self) = @_;
    my @ch;

    # "if" expression COLON suite
    push @ch, $self->_leaf($self->_expect('IF'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('COLON'));
    push @ch, $self->_parse_suite();

    # { "elif" expression COLON suite }
    while ($self->_check('ELIF')) {
        push @ch, $self->_leaf($self->_advance());   # elif
        push @ch, $self->_parse_expression();
        push @ch, $self->_leaf($self->_expect('COLON'));
        push @ch, $self->_parse_suite();
    }

    # [ "else" COLON suite ]
    if ($self->_check('ELSE')) {
        push @ch, $self->_leaf($self->_advance());   # else
        push @ch, $self->_leaf($self->_expect('COLON'));
        push @ch, $self->_parse_suite();
    }

    return $self->_node('if_stmt', @ch);
}

# for_stmt = "for" loop_vars "in" expression COLON suite ;
#
# Starlark has for loops but NOT while loops. This is intentional:
# for-loops over finite collections guarantee termination. Without while,
# every BUILD file evaluation is guaranteed to complete.
#
# Examples:
#   for item in items:
#       process(item)
#
#   for key, value in d.items():
#       print(key, value)
sub _parse_for_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('FOR'));
    push @ch, $self->_parse_loop_vars();
    push @ch, $self->_leaf($self->_expect('IN'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('COLON'));
    push @ch, $self->_parse_suite();
    return $self->_node('for_stmt', @ch);
}

# loop_vars = NAME { COMMA NAME } ;
#
# The variable(s) that receive each iterated value.
# Single variable:  for x in lst:      → loop_vars = NAME("x")
# Tuple unpacking:  for k, v in d.items(): → loop_vars = NAME("k"), COMMA, NAME("v")
sub _parse_loop_vars {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NAME'));
    while ($self->_check('COMMA')) {
        # Only continue if the next token after COMMA is a NAME (not IN)
        my $next = $self->_peek_ahead(1);
        last unless $next->{type} eq 'NAME';
        push @ch, $self->_leaf($self->_advance());   # ,
        push @ch, $self->_leaf($self->_expect('NAME'));
    }
    return $self->_node('loop_vars', @ch);
}

# def_stmt = "def" NAME LPAREN [ parameters ] RPAREN COLON suite ;
#
# Function definitions in Starlark look exactly like Python.
# Constraints vs Python:
#   - Functions cannot call themselves (recursion disabled at runtime)
#   - Closures are read-only (cannot mutate outer scope)
#
# Examples:
#   def noop(): pass
#   def greet(name, greeting="Hello"): return greeting + ", " + name
#   def fold(lst, fn, acc): ...
sub _parse_def_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('DEF'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));

    if (!$self->_check('RPAREN')) {
        push @ch, $self->_parse_parameters();
    }

    push @ch, $self->_leaf($self->_expect('RPAREN'));
    push @ch, $self->_leaf($self->_expect('COLON'));
    push @ch, $self->_parse_suite();
    return $self->_node('def_stmt', @ch);
}

# suite = simple_stmt | NEWLINE INDENT { statement } DEDENT ;
#
# A suite is the body of a compound statement (if, for, def).
# Two forms:
#
# 1. Inline (compact):   if True: pass
#    The body is a simple_stmt on the same line, after the colon.
#
# 2. Indented block:     if True:
#                            x = 1
#                            y = 2
#    The body is a NEWLINE followed by an INDENT, then statements, then DEDENT.
sub _parse_suite {
    my ($self) = @_;
    my @ch;

    if ($self->_check('NEWLINE')) {
        push @ch, $self->_leaf($self->_advance());     # NEWLINE
        push @ch, $self->_leaf($self->_expect('INDENT'));

        while (!$self->_check('DEDENT') && !$self->_check('EOF')) {
            if ($self->_check('NEWLINE')) {
                $self->_advance();
                next;
            }
            push @ch, $self->_parse_statement();
        }

        push @ch, $self->_leaf($self->_expect('DEDENT'));
    } else {
        # Inline suite: single simple_stmt on the same line.
        push @ch, $self->_parse_simple_stmt();
    }

    return $self->_node('suite', @ch);
}

# ============================================================================
# Function Parameters
# ============================================================================
#
# parameters = parameter { COMMA parameter } [ COMMA ] ;
# parameter  = DOUBLE_STAR NAME | STAR NAME | NAME EQUALS expression | NAME ;
#
# The four parameter kinds:
#   def f(a, b=1, *args, **kwargs):
#         ↑  ↑↑    ↑↑     ↑↑
#         │  │     │      └─ keyword collector (**kwargs)
#         │  │     └─ positional collector (*args)
#         │  └─ default value (b=1)
#         └─ positional required (a)

sub _parse_parameters {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_parameter();

    while ($self->_check('COMMA')) {
        my $comma = $self->_peek_ahead(0);
        # Peek ahead: if the next non-comma token is RPAREN, trailing comma
        my $after = $self->_peek_ahead(1);
        last if $after->{type} eq 'RPAREN';
        push @ch, $self->_leaf($self->_advance());  # ,
        push @ch, $self->_parse_parameter();
    }

    # Consume optional trailing comma
    if ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
    }

    return $self->_node('parameters', @ch);
}

# parameter = DOUBLE_STAR NAME | STAR NAME | NAME EQUALS expression | NAME ;
sub _parse_parameter {
    my ($self) = @_;
    my @ch;

    # **kwargs — keyword collector
    if ($self->_check('DOUBLE_STAR')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_leaf($self->_expect('NAME'));
        return $self->_node('parameter', @ch);
    }

    # *args — positional collector
    if ($self->_check('STAR')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_leaf($self->_expect('NAME'));
        return $self->_node('parameter', @ch);
    }

    # NAME [ EQUALS expression ] — positional or default-value parameter
    push @ch, $self->_leaf($self->_expect('NAME'));
    if ($self->_check('EQUALS')) {
        push @ch, $self->_leaf($self->_advance());  # =
        push @ch, $self->_parse_expression();
    }
    return $self->_node('parameter', @ch);
}

# ============================================================================
# Expressions — precedence climbing
# ============================================================================
#
# Operator precedence is encoded by nesting rules. Each level handles
# one tier and delegates to the next tier for operands. Reading from
# top to bottom = decreasing precedence:
#
#   expression  → lambda_expr | conditional (if-else)
#   or_expr     → and_expr { "or" and_expr }
#   and_expr    → not_expr { "and" not_expr }
#   not_expr    → "not" not_expr | comparison
#   comparison  → bitwise_or { comp_op bitwise_or }
#   bitwise_or  → bitwise_xor { | bitwise_xor }
#   bitwise_xor → bitwise_and { ^ bitwise_and }
#   bitwise_and → shift { & shift }
#   shift       → arith { (<<|>>) arith }
#   arith       → term { (+|-) term }
#   term        → factor { (*|/|//|%) factor }
#   factor      → (+|-|~) factor | power
#   power       → primary [ ** factor ]
#   primary     → atom { suffix }

# expression_list = expression { COMMA expression } [ COMMA ] ;
#
# Used for multi-assignment (a, b = 1, 2) and tuple creation.
# The trailing comma is consumed but not stored.
sub _parse_expression_list {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_expression();

    while ($self->_check('COMMA')) {
        # Stop if next token after comma is an assignment or end-of-statement.
        # This handles "a, b = 1, 2" where the RHS also has commas.
        my $after = $self->_peek_ahead(1);
        last if grep { $after->{type} eq $_ }
            qw(NEWLINE SEMICOLON EOF DEDENT EQUALS
               PLUS_EQUALS MINUS_EQUALS STAR_EQUALS SLASH_EQUALS
               FLOOR_DIV_EQUALS PERCENT_EQUALS AMP_EQUALS PIPE_EQUALS
               CARET_EQUALS LEFT_SHIFT_EQUALS RIGHT_SHIFT_EQUALS DOUBLE_STAR_EQUALS);

        push @ch, $self->_leaf($self->_advance());  # ,
        push @ch, $self->_parse_expression();
    }

    # Trailing comma (single-element tuple etc.) — consume silently
    if ($self->_check('COMMA')) {
        $self->_advance();
    }

    return $self->_node('expression_list', @ch);
}

# expression = lambda_expr | or_expr [ "if" or_expr "else" expression ] ;
#
# Top-level expression. Handles:
#   - Lambda: lambda x: x + 1
#   - Ternary: a if cond else b
#   - Everything else delegates to or_expr
sub _parse_expression {
    my ($self) = @_;

    # lambda_expr: starts with "lambda" keyword
    if ($self->_check('LAMBDA')) {
        return $self->_parse_lambda_expr();
    }

    # or_expr [ "if" or_expr "else" expression ]
    my $left = $self->_parse_or_expr();

    if ($self->_check('IF')) {
        my $if_tok = $self->_leaf($self->_advance());    # if
        my $cond   = $self->_parse_or_expr();
        my $else   = $self->_leaf($self->_expect('ELSE'));
        my $alt    = $self->_parse_expression();
        return $self->_node('expression', $left, $if_tok, $cond, $else, $alt);
    }

    return $self->_node('expression', $left);
}

# lambda_expr = "lambda" [ lambda_params ] COLON expression ;
#
# Starlark lambdas are single-expression anonymous functions, the same as Python.
#   f = lambda x, y: x + y
#   g = lambda: 42
sub _parse_lambda_expr {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LAMBDA'));

    if (!$self->_check('COLON')) {
        push @ch, $self->_parse_lambda_params();
    }

    push @ch, $self->_leaf($self->_expect('COLON'));
    push @ch, $self->_parse_expression();
    return $self->_node('lambda_expr', @ch);
}

# lambda_params = lambda_param { COMMA lambda_param } [ COMMA ] ;
# lambda_param  = NAME [ EQUALS expression ] | STAR NAME | DOUBLE_STAR NAME ;
sub _parse_lambda_params {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_lambda_param();

    while ($self->_check('COMMA')) {
        my $after = $self->_peek_ahead(1);
        last if $after->{type} eq 'COLON';
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_lambda_param();
    }

    if ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());
    }

    return $self->_node('lambda_params', @ch);
}

sub _parse_lambda_param {
    my ($self) = @_;
    if ($self->_check('DOUBLE_STAR')) {
        return $self->_node('lambda_param',
            $self->_leaf($self->_advance()),
            $self->_leaf($self->_expect('NAME')),
        );
    }
    if ($self->_check('STAR')) {
        return $self->_node('lambda_param',
            $self->_leaf($self->_advance()),
            $self->_leaf($self->_expect('NAME')),
        );
    }
    my @ch = ($self->_leaf($self->_expect('NAME')));
    if ($self->_check('EQUALS')) {
        push @ch, $self->_leaf($self->_advance());
        push @ch, $self->_parse_expression();
    }
    return $self->_node('lambda_param', @ch);
}

# or_expr = and_expr { "or" and_expr } ;
#
# Boolean OR is short-circuit: if the left operand is truthy, the right is
# not evaluated. Returns the first truthy operand (or the last one if none).
#   x or default_value   →  x if x is truthy, else default_value
sub _parse_or_expr {
    my ($self) = @_;
    my $left = $self->_parse_and_expr();
    while ($self->_check('OR')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_and_expr();
        $left = $self->_node('or_expr', $left, $op, $right);
    }
    return $left;
}

# and_expr = not_expr { "and" not_expr } ;
#
# Boolean AND is short-circuit: if the left operand is falsy, the right is
# not evaluated. Returns the first falsy operand (or the last one if none).
#   x and y   →  y if x is truthy, else x
sub _parse_and_expr {
    my ($self) = @_;
    my $left = $self->_parse_not_expr();
    while ($self->_check('AND')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_not_expr();
        $left = $self->_node('and_expr', $left, $op, $right);
    }
    return $left;
}

# not_expr = "not" not_expr | comparison ;
#
# Logical NOT negates a boolean value:
#   not True  →  False
#   not []    →  True  (empty collections are falsy)
sub _parse_not_expr {
    my ($self) = @_;
    if ($self->_check('NOT')) {
        my $op   = $self->_leaf($self->_advance());
        my $expr = $self->_parse_not_expr();
        return $self->_node('not_expr', $op, $expr);
    }
    return $self->_parse_comparison();
}

# comparison = bitwise_or { comp_op bitwise_or } ;
#
# comp_op = EQUALS_EQUALS | NOT_EQUALS
#         | LESS_THAN | GREATER_THAN | LESS_EQUALS | GREATER_EQUALS
#         | "in" | "not" "in" ;
#
# Comparison operators include:  ==  !=  <  >  <=  >=  in  not in
# "not in" is a two-token operator: we detect it by seeing NOT followed by IN.
# Note: comparison chaining (a < b < c) is a syntax error in Starlark.
sub _parse_comparison {
    my ($self) = @_;
    my $left = $self->_parse_bitwise_or();

    while (1) {
        my $type = $self->_peek()->{type};

        # Standard comparison operators
        if (grep { $type eq $_ }
            qw(EQUALS_EQUALS NOT_EQUALS LESS_THAN GREATER_THAN LESS_EQUALS GREATER_EQUALS)) {
            my $op    = $self->_leaf($self->_advance());
            my $right = $self->_parse_bitwise_or();
            $left = $self->_node('comparison', $left, $op, $right);
            next;
        }

        # "in" membership test
        if ($type eq 'IN') {
            my $op    = $self->_leaf($self->_advance());
            my $right = $self->_parse_bitwise_or();
            $left = $self->_node('comparison', $left, $op, $right);
            next;
        }

        # "not in" — two-keyword operator
        if ($type eq 'NOT' && $self->_peek_ahead(1)->{type} eq 'IN') {
            my $not_tok = $self->_leaf($self->_advance());   # not
            my $in_tok  = $self->_leaf($self->_advance());   # in
            my $op      = $self->_node('comp_op', $not_tok, $in_tok);
            my $right   = $self->_parse_bitwise_or();
            $left = $self->_node('comparison', $left, $op, $right);
            next;
        }

        last;
    }

    return $left;
}

# bitwise_or = bitwise_xor { PIPE bitwise_xor } ;
#
# Bitwise OR: a | b — sets each bit that is set in either a or b.
# Useful for combining flag values:  READONLY | HIDDEN
sub _parse_bitwise_or {
    my ($self) = @_;
    my $left = $self->_parse_bitwise_xor();
    while ($self->_check('PIPE')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_bitwise_xor();
        $left = $self->_node('bitwise_or', $left, $op, $right);
    }
    return $left;
}

# bitwise_xor = bitwise_and { CARET bitwise_and } ;
#
# Bitwise XOR: a ^ b — sets each bit that differs between a and b.
sub _parse_bitwise_xor {
    my ($self) = @_;
    my $left = $self->_parse_bitwise_and();
    while ($self->_check('CARET')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_bitwise_and();
        $left = $self->_node('bitwise_xor', $left, $op, $right);
    }
    return $left;
}

# bitwise_and = shift { AMP shift } ;
#
# Bitwise AND: a & b — sets each bit that is set in both a and b.
sub _parse_bitwise_and {
    my ($self) = @_;
    my $left = $self->_parse_shift();
    while ($self->_check('AMP')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_shift();
        $left = $self->_node('bitwise_and', $left, $op, $right);
    }
    return $left;
}

# shift = arith { ( LEFT_SHIFT | RIGHT_SHIFT ) arith } ;
#
# Bit shifting:
#   a << n  →  a * 2^n  (left shift, multiplication by power of 2)
#   a >> n  →  a // 2^n (right shift, integer division by power of 2)
sub _parse_shift {
    my ($self) = @_;
    my $left = $self->_parse_arith();
    while ($self->_check('LEFT_SHIFT') || $self->_check('RIGHT_SHIFT')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_arith();
        $left = $self->_node('shift', $left, $op, $right);
    }
    return $left;
}

# arith = term { ( PLUS | MINUS ) term } ;
#
# Addition and subtraction. Also handles:
#   - String concatenation: "hello" + " " + "world"
#   - List concatenation: [1, 2] + [3, 4]
sub _parse_arith {
    my ($self) = @_;
    my $left = $self->_parse_term();
    while ($self->_check('PLUS') || $self->_check('MINUS')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_term();
        $left = $self->_node('arith', $left, $op, $right);
    }
    return $left;
}

# term = factor { ( STAR | SLASH | FLOOR_DIV | PERCENT ) factor } ;
#
# Multiplication and division:
#   a * b    → multiplication / list repetition: [0] * 10
#   a / b    → float division (even if both operands are ints)
#   a // b   → integer (floor) division: 7 // 2 = 3
#   a % b    → modulo / string formatting: "Hello, %s" % name
sub _parse_term {
    my ($self) = @_;
    my $left = $self->_parse_factor();
    while ($self->_check('STAR') || $self->_check('SLASH')
        || $self->_check('FLOOR_DIV') || $self->_check('PERCENT')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_factor();
        $left = $self->_node('term', $left, $op, $right);
    }
    return $left;
}

# factor = ( PLUS | MINUS | TILDE ) factor | power ;
#
# Unary operators:
#   -x    → arithmetic negation
#   +x    → no-op (identity, but type-checked)
#   ~x    → bitwise complement (flips all bits)
sub _parse_factor {
    my ($self) = @_;
    if ($self->_check('PLUS') || $self->_check('MINUS') || $self->_check('TILDE')) {
        my $op   = $self->_leaf($self->_advance());
        my $expr = $self->_parse_factor();
        return $self->_node('factor', $op, $expr);
    }
    return $self->_parse_power();
}

# power = primary [ DOUBLE_STAR factor ] ;
#
# Exponentiation: a ** b — raises a to the power b.
# It is RIGHT-associative: 2 ** 3 ** 2 = 2 ** (3 ** 2) = 512.
# Implemented by delegating the exponent to factor (not power), which
# can recurse back through factor → power.
sub _parse_power {
    my ($self) = @_;
    my $base = $self->_parse_primary();
    if ($self->_check('DOUBLE_STAR')) {
        my $op  = $self->_leaf($self->_advance());
        my $exp = $self->_parse_factor();
        return $self->_node('power', $base, $op, $exp);
    }
    return $base;
}

# ============================================================================
# Primary Expressions
# ============================================================================
#
# primary = atom { suffix } ;
#
# A primary is an atom (the innermost expression) followed by zero or more
# suffixes. Suffixes bind left-to-right, like Python:
#   f(x).attr[0]  =  ((f(x)).attr)[0]
#
# Suffixes:
#   .NAME        attribute access:   obj.method
#   [subscript]  indexing/slicing:   lst[0], d["key"], lst[1:3]
#   (args)       function call:      f(x, y), cc_library(name="foo")

sub _parse_primary {
    my ($self) = @_;
    my $base = $self->_parse_atom();

    while (1) {
        if ($self->_check('DOT')) {
            $self->_advance();  # .
            my $attr = $self->_leaf($self->_expect('NAME'));
            $base = $self->_node('primary', $base,
                $self->_node('suffix', $self->_leaf({ type => 'DOT', value => '.', line => 0, col => 0 }), $attr));
        } elsif ($self->_check('LBRACKET')) {
            my $lb      = $self->_leaf($self->_advance());
            my $sub     = $self->_parse_subscript();
            my $rb      = $self->_leaf($self->_expect('RBRACKET'));
            $base = $self->_node('primary', $base,
                $self->_node('suffix', $lb, $sub, $rb));
        } elsif ($self->_check('LPAREN')) {
            my $lp      = $self->_leaf($self->_advance());
            my $args;
            if (!$self->_check('RPAREN')) {
                $args = $self->_parse_arguments();
            }
            my $rp      = $self->_leaf($self->_expect('RPAREN'));
            my @suffix_ch = ($lp);
            push @suffix_ch, $args if defined $args;
            push @suffix_ch, $rp;
            $base = $self->_node('primary', $base,
                $self->_node('suffix', @suffix_ch));
        } else {
            last;
        }
    }

    return $base;
}

# subscript = expression | [ expression ] COLON [ expression ] [ COLON [ expression ] ] ;
#
# Indexing: lst[0], d["key"]
# Slicing:  lst[1:3], lst[::2], lst[::-1]
#
# All three slice components are optional. Some examples:
#   [:]      — copy entire list: start=0, stop=len, step=1
#   [1:]     — from index 1 to end
#   [:3]     — from start to index 3 (exclusive)
#   [1:3]    — from 1 to 3
#   [::2]    — every second element
#   [::-1]   — reversed
sub _parse_subscript {
    my ($self) = @_;
    my @ch;

    # If we see a COLON immediately, it's a slice with no start.
    if ($self->_check('COLON')) {
        push @ch, $self->_leaf($self->_advance());  # :
        push @ch, $self->_parse_expression() unless $self->_check('COLON') || $self->_check('RBRACKET');
        if ($self->_check('COLON')) {
            push @ch, $self->_leaf($self->_advance());  # second :
            push @ch, $self->_parse_expression() unless $self->_check('RBRACKET');
        }
        return $self->_node('subscript', @ch);
    }

    # Otherwise parse the first expression.
    push @ch, $self->_parse_expression();

    # If followed by COLON, it's a slice.
    if ($self->_check('COLON')) {
        push @ch, $self->_leaf($self->_advance());  # :
        push @ch, $self->_parse_expression() unless $self->_check('COLON') || $self->_check('RBRACKET');
        if ($self->_check('COLON')) {
            push @ch, $self->_leaf($self->_advance());  # second :
            push @ch, $self->_parse_expression() unless $self->_check('RBRACKET');
        }
    }

    return $self->_node('subscript', @ch);
}

# ============================================================================
# Atoms
# ============================================================================
#
# atom = INT | FLOAT | STRING { STRING } | NAME
#      | "True" | "False" | "None"
#      | list_expr | dict_expr | paren_expr ;
#
# Atoms are the indivisible leaves of the expression tree — the innermost values.

sub _parse_atom {
    my ($self) = @_;
    my $tok  = $self->_peek();
    my $type = $tok->{type};

    # --- Integer literal: 42, 0, 255 ---
    if ($type eq 'INT') {
        return $self->_node('atom', $self->_leaf($self->_advance()));
    }

    # --- Float literal: 3.14, 1e10 ---
    if ($type eq 'FLOAT') {
        return $self->_node('atom', $self->_leaf($self->_advance()));
    }

    # --- String literal(s): "hello" 'world' """multi-line"""
    #
    # Adjacent string literals are concatenated: "hello " "world" → "hello world"
    # This is useful for splitting long strings across lines.
    if ($type eq 'STRING') {
        my @strings = ($self->_leaf($self->_advance()));
        while ($self->_check('STRING')) {
            push @strings, $self->_leaf($self->_advance());
        }
        return $self->_node('atom', @strings);
    }

    # --- Boolean literals: True, False ---
    #
    # Note: In Starlark, True and False are capitalized (like Python).
    # The lexer emits them as token types TRUE and FALSE.
    if ($type eq 'TRUE' || $type eq 'FALSE') {
        return $self->_node('atom', $self->_leaf($self->_advance()));
    }

    # --- None literal ---
    if ($type eq 'NONE') {
        return $self->_node('atom', $self->_leaf($self->_advance()));
    }

    # --- Identifier: foo, bar, _private ---
    if ($type eq 'NAME') {
        return $self->_node('atom', $self->_leaf($self->_advance()));
    }

    # --- List literal or list comprehension: [1, 2, 3] or [x*2 for x in r] ---
    if ($type eq 'LBRACKET') {
        return $self->_parse_list_expr();
    }

    # --- Dict literal or dict comprehension: {"a": 1} ---
    if ($type eq 'LBRACE') {
        return $self->_parse_dict_expr();
    }

    # --- Parenthesized expression or tuple: (x), (x, y), () ---
    if ($type eq 'LPAREN') {
        return $self->_parse_paren_expr();
    }

    die sprintf(
        "CodingAdventures::StarlarkParser: unexpected token '%s' (type %s) "
      . "at line %d col %d\n",
        $tok->{value}, $tok->{type}, $tok->{line}, $tok->{col}
    );
}

# list_expr = LBRACKET [ list_body ] RBRACKET ;
# list_body = expression comp_clause | expression { COMMA expression } [ COMMA ] ;
#
# Two forms:
#   [1, 2, 3]           — list literal
#   [x * 2 for x in r]  — list comprehension
sub _parse_list_expr {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LBRACKET'));

    if (!$self->_check('RBRACKET')) {
        my $first = $self->_parse_expression();

        if ($self->_check('FOR')) {
            # List comprehension: [expr for ... in ...]
            push @ch, $first;
            push @ch, $self->_parse_comp_clause();
        } else {
            # List literal: [expr, ...]
            push @ch, $first;
            while ($self->_check('COMMA')) {
                my $after = $self->_peek_ahead(1);
                last if $after->{type} eq 'RBRACKET';
                push @ch, $self->_leaf($self->_advance());  # ,
                push @ch, $self->_parse_expression();
            }
            # Consume trailing comma if present
            if ($self->_check('COMMA')) {
                push @ch, $self->_leaf($self->_advance());
            }
        }
    }

    push @ch, $self->_leaf($self->_expect('RBRACKET'));
    return $self->_node('list_expr', @ch);
}

# dict_expr = LBRACE [ dict_body ] RBRACE ;
# dict_body = dict_entry comp_clause | dict_entry { COMMA dict_entry } [ COMMA ] ;
# dict_entry = expression COLON expression ;
sub _parse_dict_expr {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LBRACE'));

    if (!$self->_check('RBRACE')) {
        my $key   = $self->_parse_expression();
        push @ch, $self->_leaf($self->_expect('COLON'));
        my $value = $self->_parse_expression();
        my $entry = $self->_node('dict_entry', $key, $value);

        if ($self->_check('FOR')) {
            # Dict comprehension: {k: v for k, v in ...}
            push @ch, $entry;
            push @ch, $self->_parse_comp_clause();
        } else {
            # Dict literal: {k: v, ...}
            push @ch, $entry;
            while ($self->_check('COMMA')) {
                my $after = $self->_peek_ahead(1);
                last if $after->{type} eq 'RBRACE';
                push @ch, $self->_leaf($self->_advance());  # ,
                my $k = $self->_parse_expression();
                push @ch, $self->_leaf($self->_expect('COLON'));
                my $v = $self->_parse_expression();
                push @ch, $self->_node('dict_entry', $k, $v);
            }
            if ($self->_check('COMMA')) {
                push @ch, $self->_leaf($self->_advance());
            }
        }
    }

    push @ch, $self->_leaf($self->_expect('RBRACE'));
    return $self->_node('dict_expr', @ch);
}

# paren_expr = LPAREN [ paren_body ] RPAREN ;
#
# Parenthesized expressions and tuples:
#   ()      — empty tuple
#   (x)     — parenthesized expression (not a tuple)
#   (x,)    — single-element tuple (trailing comma distinguishes from grouping)
#   (x, y)  — two-element tuple
sub _parse_paren_expr {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LPAREN'));

    if (!$self->_check('RPAREN')) {
        my $first = $self->_parse_expression();

        if ($self->_check('FOR')) {
            # Parenthesized comprehension: (expr for ...)
            push @ch, $first;
            push @ch, $self->_parse_comp_clause();
        } elsif ($self->_check('COMMA')) {
            # Tuple: (x, y, z)
            push @ch, $first;
            while ($self->_check('COMMA')) {
                push @ch, $self->_leaf($self->_advance());  # ,
                last if $self->_check('RPAREN');
                push @ch, $self->_parse_expression();
            }
        } else {
            push @ch, $first;
        }
    }

    push @ch, $self->_leaf($self->_expect('RPAREN'));
    return $self->_node('paren_expr', @ch);
}

# ============================================================================
# Comprehensions
# ============================================================================
#
# comp_clause = comp_for { comp_for | comp_if } ;
# comp_for    = "for" loop_vars "in" or_expr ;
# comp_if     = "if" or_expr ;
#
# Comprehensions produce values by iterating over collections with optional
# filtering. They can be nested:
#
#   [x for x in lst if x > 0]
#   [x + y for x in row for y in col if x != y]

sub _parse_comp_clause {
    my ($self) = @_;
    my @ch;

    # First comp_for is mandatory
    push @ch, $self->_parse_comp_for();

    # Additional comp_for or comp_if clauses
    while ($self->_check('FOR') || $self->_check('IF')) {
        if ($self->_check('FOR')) {
            push @ch, $self->_parse_comp_for();
        } else {
            push @ch, $self->_parse_comp_if();
        }
    }

    return $self->_node('comp_clause', @ch);
}

# comp_for = "for" loop_vars "in" or_expr ;
#
# Note: uses or_expr (not expression) as the iterable, to avoid consuming
# a trailing "if" that belongs to a comp_if clause.
sub _parse_comp_for {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('FOR'));
    push @ch, $self->_parse_loop_vars();
    push @ch, $self->_leaf($self->_expect('IN'));
    push @ch, $self->_parse_or_expr();
    return $self->_node('comp_for', @ch);
}

# comp_if = "if" or_expr ;
#
# Note: uses or_expr to avoid ambiguity with the ternary "if".
sub _parse_comp_if {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('IF'));
    push @ch, $self->_parse_or_expr();
    return $self->_node('comp_if', @ch);
}

# ============================================================================
# Call Arguments
# ============================================================================
#
# arguments = argument { COMMA argument } [ COMMA ] ;
# argument  = DOUBLE_STAR expression | STAR expression
#           | NAME EQUALS expression | expression ;
#
# Four kinds of arguments:
#   f(1, 2)       → positional
#   f(key=value)  → keyword
#   f(*args)      → positional unpacking (splat)
#   f(**kwargs)   → keyword unpacking (double splat)

sub _parse_arguments {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_argument();

    while ($self->_check('COMMA')) {
        my $after = $self->_peek_ahead(1);
        last if $after->{type} eq 'RPAREN';
        push @ch, $self->_leaf($self->_advance());  # ,
        push @ch, $self->_parse_argument();
    }

    if ($self->_check('COMMA')) {
        push @ch, $self->_leaf($self->_advance());  # trailing comma
    }

    return $self->_node('arguments', @ch);
}

# argument = DOUBLE_STAR expression | STAR expression
#           | NAME EQUALS expression | expression ;
sub _parse_argument {
    my ($self) = @_;

    # **expr — keyword unpacking
    if ($self->_check('DOUBLE_STAR')) {
        return $self->_node('argument',
            $self->_leaf($self->_advance()),
            $self->_parse_expression(),
        );
    }

    # *expr — positional unpacking
    if ($self->_check('STAR')) {
        return $self->_node('argument',
            $self->_leaf($self->_advance()),
            $self->_parse_expression(),
        );
    }

    # NAME EQUALS expression — keyword argument
    # We need one token of lookahead to distinguish:
    #   f(key=value)  — keyword argument
    #   f(expr)       — positional argument where expr might start with NAME
    if ($self->_check('NAME') && $self->_peek_ahead(1)->{type} eq 'EQUALS') {
        return $self->_node('argument',
            $self->_leaf($self->_advance()),      # NAME
            $self->_leaf($self->_advance()),      # EQUALS
            $self->_parse_expression(),
        );
    }

    # Plain positional argument
    return $self->_node('argument', $self->_parse_expression());
}

# ============================================================================
# Class-method convenience wrapper
# ============================================================================

# --- parse_starlark($source) --------------------------------------------------
#
# Convenience class method: tokenize and parse in one call.
# Returns the root ASTNode (rule_name "program"). Dies on error.

sub parse_starlark {
    my ($class, $source) = @_;
    my $parser = $class->new($source);
    return $parser->parse();
}

1;

__END__

=head1 NAME

CodingAdventures::StarlarkParser - Hand-written recursive-descent Starlark parser

=head1 SYNOPSIS

    use CodingAdventures::StarlarkParser;

    # Object-oriented
    my $parser = CodingAdventures::StarlarkParser->new("x = 1\n");
    my $ast    = $parser->parse();
    print $ast->rule_name;   # "program"

    # Convenience class method
    my $ast = CodingAdventures::StarlarkParser->parse_starlark("x = 1\n");

=head1 DESCRIPTION

A hand-written recursive-descent parser for Starlark — the deterministic
Python subset used in Bazel BUILD files. Tokenizes input with
C<CodingAdventures::StarlarkLexer> and builds an Abstract Syntax Tree (AST)
of C<CodingAdventures::StarlarkParser::ASTNode> nodes.

Supported constructs: assignments, augmented assignments, function definitions,
if/elif/else, for loops, return/break/continue/pass, load statements, list
and dict literals, comprehensions, lambda expressions, ternary expressions,
and full expression parsing with correct operator precedence.

=head1 METHODS

=head2 new($source)

Tokenize C<$source> with C<StarlarkLexer> and return a parser instance.

=head2 parse()

Parse and return the root AST node (rule_name C<"program">). Dies on error.

=head2 parse_starlark($source)

Class method — tokenize and parse in one call. Returns the root ASTNode.

=head1 AST NODE FORMAT

Each node is a C<CodingAdventures::StarlarkParser::ASTNode>:

    $node->rule_name   # e.g. "assign_stmt", "if_stmt", "def_stmt"
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
