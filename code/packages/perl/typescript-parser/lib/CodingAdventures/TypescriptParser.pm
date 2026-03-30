package CodingAdventures::TypescriptParser;

# ============================================================================
# CodingAdventures::TypescriptParser — Hand-written recursive-descent TS parser
# ============================================================================
#
# This module parses a subset of TypeScript into an Abstract Syntax Tree (AST).
# The parser is hand-written using the recursive-descent technique: each grammar
# rule is encoded as one Perl method, and rules call each other recursively.
#
# TypeScript is a strict superset of JavaScript that adds:
#   - Type annotations:   `let x: number = 1`
#   - Interfaces:         `interface Foo { bar: string }`
#   - Generics:           `Array<number>`
#   - Access modifiers:   `public`, `private`, `protected`
#   - Enums:              `enum Color { Red, Green, Blue }`
#   - Type aliases:       `type Pair = [string, number]`
#   - `readonly`, `abstract`, `declare`, `namespace`, `module`
#   - More primitive types: `any`, `void`, `never`, `unknown`
#
# For the grammar subset we support, TypeScript and JavaScript share the
# same grammar rules. The TypeScript-specific keywords appear as KEYWORD
# tokens during lexing — our grammar handles them the same way as
# var/let/const (the `var_declaration` rule accepts any KEYWORD token).
#
# # What language do we parse?
# =============================
#
# A practical subset covering the core statement forms:
#
#   let x = 5;                          — variable declaration (var/let/const)
#   x = 10;                             — assignment
#   42;                                 — expression statement
#   function add(a, b) { return a+b; }  — function declaration
#   if (x > 0) { … } else { … }        — if/else statement
#   for (let i = 0; i < 10; i++) { … } — for loop
#   return x + 1;                       — return statement
#   (x) => x + 1                        — arrow function expression
#   f(a, b)                             — function call
#   1 + 2 * 3                           — binary expression (precedence)
#
# TypeScript-specific keywords are recognized by the TypescriptLexer and
# become KEYWORD tokens, flowing through the grammar naturally.
#
# # Token types from CodingAdventures::TypescriptLexer
# =====================================================
#
# The TypescriptLexer emits specific keyword type names (like the JS lexer):
#
#   VAR, LET, CONST, FUNCTION, IF, ELSE, FOR, WHILE, DO, RETURN,
#   TRUE, FALSE, NULL, UNDEFINED, NEW, THIS, TYPEOF, INSTANCEOF, CLASS
#
# TypeScript-specific keyword tokens emitted as KEYWORD (or specific types):
#   INTERFACE, TYPE, ENUM, ABSTRACT, READONLY, DECLARE, NAMESPACE,
#   MODULE, KEYOF, INFER, NEVER, UNKNOWN, ANY, VOID, NUMBER_TYPE,
#   STRING_TYPE, BOOLEAN, OBJECT, SYMBOL, BIGINT, PUBLIC, PRIVATE,
#   PROTECTED, STATIC, IMPLEMENTS, EXTENDS, OVERRIDE
#
# Operators:
#   STRICT_EQUALS (===), STRICT_NOT_EQUALS (!==), EQUALS_EQUALS (==),
#   NOT_EQUALS (!=), EQUALS (=), LESS_THAN (<), GREATER_THAN (>),
#   LESS_EQUALS (<=), GREATER_EQUALS (>=), ARROW (=>),
#   PLUS, MINUS, STAR, SLASH, BANG, COLON
#
# Punctuation:
#   LPAREN, RPAREN, LBRACE, RBRACE, SEMICOLON, COMMA, EOF
#
# # Operator precedence (lowest to highest)
# ==========================================
#
#   Equality      →  ===  !==  ==  !=
#   Comparison    →  <  >  <=  >=
#   Additive      →  +  -
#   Multiplicative →  *  /
#   Unary          →  !  -  (unary)
#   Primary        →  literals, identifiers, (expr), call, arrow
#
# # AST node types (rule_name values)
# ====================================
#
#   program          — root; contains a list of statement nodes
#   statement        — wrapper for one statement
#   var_declaration  — var/let/const name = expr ;
#   assignment_stmt  — name = expr ;
#   expression_stmt  — expr ;
#   function_decl    — function name(params) { body }
#   if_stmt          — if (cond) block [else block/if_stmt]
#   for_stmt         — for (init; cond; update) block
#   return_stmt      — return [expr] ;
#   block            — { statement* }
#   expression       — full expression (lowest-precedence entry point)
#   binary_expr      — left op right
#   unary_expr       — op expr  (unary ! or -)
#   call_expr        — callee(args)
#   arrow_expr       — (params) => body
#   primary          — literal, identifier, grouped expression
#   param_list       — comma-separated parameter names
#   arg_list         — comma-separated argument expressions
#   for_init         — initializer part of for header
#   for_condition    — condition part of for header
#   for_update       — update part of for header
#   token            — leaf node wrapping a single lexer token
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::TypescriptLexer;
use CodingAdventures::TypescriptParser::ASTNode;

# ============================================================================
# Constructor
# ============================================================================

# --- new($source) -------------------------------------------------------------
#
# Tokenize `$source` with TypescriptLexer and return a ready-to-parse parser.
# The lexer handles all the TypeScript-specific keyword recognition, so by
# the time we see the token stream, keywords like `interface` and `enum`
# already have their proper token types.

sub new {
    my ($class, $source) = @_;
    my $tokens = CodingAdventures::TypescriptLexer->tokenize($source);
    return bless {
        _tokens => $tokens,
        _pos    => 0,
    }, $class;
}

# ============================================================================
# Token helpers
# ============================================================================

# Peek at the current token without consuming it.
# Returns a synthetic EOF token if we're past the end of the stream.
sub _peek {
    my ($self) = @_;
    return $self->{_tokens}[ $self->{_pos} ]
        // { type => 'EOF', value => '', line => 0, col => 0 };
}

# Peek N positions ahead from the current position (0 = current token).
# Used for lookahead in ambiguous situations (e.g. NAME EQUALS vs NAME LPAREN).
sub _peek_ahead {
    my ($self, $n) = @_;
    $n //= 0;
    return $self->{_tokens}[ $self->{_pos} + $n ]
        // { type => 'EOF', value => '', line => 0, col => 0 };
}

# Consume and return the current token, advancing the position counter.
# Never advances past EOF.
sub _advance {
    my ($self) = @_;
    my $tok = $self->_peek();
    $self->{_pos}++ unless $tok->{type} eq 'EOF';
    return $tok;
}

# Expect a specific token type; die with a helpful message on mismatch.
# This is called when the grammar requires exactly one token type at a
# given position — any mismatch is a syntax error.
sub _expect {
    my ($self, $type) = @_;
    my $tok = $self->_peek();
    unless ($tok->{type} eq $type) {
        die sprintf(
            "CodingAdventures::TypescriptParser: parse error at line %d col %d: "
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

# Return 1 if the current token is a declaration keyword (var/let/const).
# TypeScript uses the same declaration keywords as JavaScript.
sub _is_decl_kw {
    my ($self) = @_;
    my $type = $self->_peek()->{type};
    return $type eq 'VAR' || $type eq 'LET' || $type eq 'CONST';
}

# Return 1 if the current token is any keyword-like token that can begin
# an expression (for recognizing keyword-type tokens in TypeScript).
# The TypescriptLexer emits many specific keyword types; this checks a subset
# that can appear as "declaration" keywords.
sub _is_ts_keyword {
    my ($self) = @_;
    my $type = $self->_peek()->{type};
    # TypeScript-specific declaration forms that we handle as KEYWORD tokens
    return $type eq 'INTERFACE' || $type eq 'TYPE' || $type eq 'ENUM'
        || $type eq 'ABSTRACT' || $type eq 'DECLARE' || $type eq 'NAMESPACE'
        || $type eq 'MODULE';
}

# Wrap a token as a leaf ASTNode.
# Leaf nodes represent a single terminal in the grammar — they hold the
# actual token data (type, value, line, col) from the lexer.
sub _leaf {
    my ($self, $tok) = @_;
    return CodingAdventures::TypescriptParser::ASTNode->new_leaf($tok);
}

# Create an inner (non-leaf) ASTNode.
# Inner nodes represent grammar rules — they hold a rule_name and children.
sub _node {
    my ($self, $rule_name, @children) = @_;
    return CodingAdventures::TypescriptParser::ASTNode->new($rule_name, \@children);
}

# ============================================================================
# Public API
# ============================================================================

# --- parse() ------------------------------------------------------------------
#
# Parse the tokenized source and return the root AST node (rule_name "program").
# Dies on parse error with a descriptive message including line and column.

sub parse {
    my ($self) = @_;
    return $self->_parse_program();
}

# ============================================================================
# Grammar rules — each method parses one grammar rule
# ============================================================================

# program = { statement } ;
#
# The TypeScript grammar (for our subset) allows an empty program.
# We keep parsing statements until we hit EOF.
sub _parse_program {
    my ($self) = @_;
    my @children;
    while (!$self->_check('EOF')) {
        push @children, $self->_parse_statement();
    }
    return $self->_node('program', @children);
}

# statement = var_declaration | function_decl | if_stmt | for_stmt
#           | return_stmt | block | assignment_stmt | expression_stmt ;
#
# Statement dispatch: we inspect the current token to select the right rule.
# For the NAME EQUALS vs NAME LPAREN ambiguity, we look one token ahead.
# TypeScript-specific keywords that open statements are handled as expression_stmt.
sub _parse_statement {
    my ($self) = @_;
    my $tok  = $self->_peek();
    my $type = $tok->{type};

    # var / let / const — variable declaration
    if ($self->_is_decl_kw()) {
        return $self->_node('statement', $self->_parse_var_declaration());
    }

    # function declaration
    if ($type eq 'FUNCTION') {
        return $self->_node('statement', $self->_parse_function_decl());
    }

    # if statement
    if ($type eq 'IF') {
        return $self->_node('statement', $self->_parse_if_stmt());
    }

    # for loop
    if ($type eq 'FOR') {
        return $self->_node('statement', $self->_parse_for_stmt());
    }

    # return statement
    if ($type eq 'RETURN') {
        return $self->_node('statement', $self->_parse_return_stmt());
    }

    # block
    if ($type eq 'LBRACE') {
        return $self->_node('statement', $self->_parse_block());
    }

    # assignment: NAME EQUALS expr ;
    # Lookahead: NAME followed immediately by EQUALS (not EQUALS_EQUALS etc.)
    if ($type eq 'NAME') {
        my $next = $self->_peek_ahead(1);
        if ($next->{type} eq 'EQUALS') {
            return $self->_node('statement', $self->_parse_assignment_stmt());
        }
    }

    # expression statement (default — catches function calls, literals, etc.)
    return $self->_node('statement', $self->_parse_expression_stmt());
}

# var_declaration = ( VAR | LET | CONST ) NAME EQUALS expression SEMICOLON ;
#
# Example:  var x = 5;   let y = "hello";   const z = true;
# TypeScript also allows:  let x: number = 5;  — but we skip the type annotation
# in this grammar subset and treat it as an expression statement if encountered.
sub _parse_var_declaration {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_advance());          # VAR / LET / CONST
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('EQUALS'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('var_declaration', @ch);
}

# assignment_stmt = NAME EQUALS expression SEMICOLON ;
#
# Example:  x = 10;
sub _parse_assignment_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('EQUALS'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('assignment_stmt', @ch);
}

# expression_stmt = expression SEMICOLON ;
#
# Example:  42;   f(x);
sub _parse_expression_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('expression_stmt', @ch);
}

# function_decl = FUNCTION NAME LPAREN param_list RPAREN block ;
#
# Example:  function add(a, b) { return a + b; }
sub _parse_function_decl {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('FUNCTION'));
    push @ch, $self->_leaf($self->_expect('NAME'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    push @ch, $self->_parse_param_list();
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    push @ch, $self->_parse_block();
    return $self->_node('function_decl', @ch);
}

# param_list = [ NAME { COMMA NAME } ] ;
#
# Function parameters are just names in this subset.
# TypeScript parameters can have type annotations (name: type), but we
# parse only the name here for simplicity.
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

# if_stmt = IF LPAREN expression RPAREN block [ ELSE ( if_stmt | block ) ] ;
#
# Example:  if (x > 0) { return x; } else { return 0; }
# The else clause can chain into another if_stmt (else if).
sub _parse_if_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('IF'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));
    push @ch, $self->_parse_expression();
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    push @ch, $self->_parse_block();
    if ($self->_check('ELSE')) {
        push @ch, $self->_leaf($self->_advance());
        if ($self->_check('IF')) {
            push @ch, $self->_parse_if_stmt();
        } else {
            push @ch, $self->_parse_block();
        }
    }
    return $self->_node('if_stmt', @ch);
}

# for_stmt = FOR LPAREN for_header RPAREN block ;
# for_header = for_init SEMICOLON for_condition SEMICOLON for_update
#
# Example:  for (let i = 0; i < 10; i = i + 1) { }
sub _parse_for_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('FOR'));
    push @ch, $self->_leaf($self->_expect('LPAREN'));

    # --- Initializer ---
    if ($self->_is_decl_kw()) {
        # var/let/const declaration without trailing semicolon
        my @init_ch;
        push @init_ch, $self->_leaf($self->_advance());     # VAR/LET/CONST
        push @init_ch, $self->_leaf($self->_expect('NAME'));
        push @init_ch, $self->_leaf($self->_expect('EQUALS'));
        push @init_ch, $self->_parse_expression();
        push @ch, $self->_node('for_init', @init_ch);
    } elsif (!$self->_check('SEMICOLON')) {
        push @ch, $self->_node('for_init', $self->_parse_expression());
    } else {
        push @ch, $self->_node('for_init');
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));

    # --- Condition ---
    if (!$self->_check('SEMICOLON')) {
        push @ch, $self->_node('for_condition', $self->_parse_expression());
    } else {
        push @ch, $self->_node('for_condition');
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));

    # --- Update expression ---
    # The update is typically an increment: i = i + 1 or i++.
    # Since ++ is not a single token in our lexer, i++ parses as NAME PLUS PLUS.
    # Assignment (i = i + 1) requires lookahead: NAME EQUALS (not EQUALS_EQUALS).
    if (!$self->_check('RPAREN')) {
        my $next = $self->_peek_ahead(1);
        if ($self->_check('NAME') && $next && $next->{type} eq 'EQUALS') {
            my @upd_ch;
            push @upd_ch, $self->_leaf($self->_advance());    # NAME
            push @upd_ch, $self->_leaf($self->_advance());    # EQUALS
            push @upd_ch, $self->_parse_expression();
            push @ch, $self->_node('for_update', $self->_node('assign_expr', @upd_ch));
        } else {
            push @ch, $self->_node('for_update', $self->_parse_expression());
        }
    } else {
        push @ch, $self->_node('for_update');
    }

    push @ch, $self->_leaf($self->_expect('RPAREN'));
    push @ch, $self->_parse_block();
    return $self->_node('for_stmt', @ch);
}

# return_stmt = RETURN [ expression ] SEMICOLON ;
#
# Example:  return x + 1;   return;
sub _parse_return_stmt {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('RETURN'));
    if (!$self->_check('SEMICOLON') && !$self->_check('RBRACE') && !$self->_check('EOF')) {
        push @ch, $self->_parse_expression();
    }
    push @ch, $self->_leaf($self->_expect('SEMICOLON'));
    return $self->_node('return_stmt', @ch);
}

# block = LBRACE { statement } RBRACE ;
#
# Blocks are used by function bodies, if/else branches, and for loops.
# In TypeScript, blocks create a new variable scope (especially important
# for `let` and `const`, which are block-scoped, unlike `var`).
sub _parse_block {
    my ($self) = @_;
    my @ch;
    push @ch, $self->_leaf($self->_expect('LBRACE'));
    while (!$self->_check('RBRACE') && !$self->_check('EOF')) {
        push @ch, $self->_parse_statement();
    }
    push @ch, $self->_leaf($self->_expect('RBRACE'));
    return $self->_node('block', @ch);
}

# ============================================================================
# Expression parsing — precedence climbing
# ============================================================================
#
# We implement operator precedence by nesting rules, each level handling one
# tier of operators and delegating to the next higher tier for operands.
#
# expression → equality → comparison → additive → multiplicative → unary → primary
#
# This guarantees:
#   1 + 2 * 3  parses as  1 + (2 * 3)   — multiplication tighter
#   !x == y    parses as  (!x) == y      — unary tighter than equality

# expression = equality ;
#
# The top-level expression rule delegates to the equality tier.
sub _parse_expression {
    my ($self) = @_;
    my $inner = $self->_parse_equality();
    return $self->_node('expression', $inner);
}

# equality = comparison { ( STRICT_EQUALS | STRICT_NOT_EQUALS
#                          | EQUALS_EQUALS | NOT_EQUALS ) comparison } ;
#
# TypeScript has both JS equality operators (== !=) and strict variants (=== !==).
# TypeScript's type system makes strict equality preferred:
#   === checks both value AND type (no implicit coercion)
#   ==  may coerce types (1 == "1" is true in JS/TS, 1 === "1" is false)
sub _parse_equality {
    my ($self) = @_;
    my $left = $self->_parse_comparison();
    while ($self->_check('STRICT_EQUALS') || $self->_check('STRICT_NOT_EQUALS')
           || $self->_check('EQUALS_EQUALS') || $self->_check('NOT_EQUALS')) {
        my $op    = $self->_leaf($self->_advance());
        my $right = $self->_parse_comparison();
        $left = $self->_node('binary_expr', $left, $op, $right);
    }
    return $left;
}

# comparison = additive { ( LESS_THAN | GREATER_THAN
#                          | LESS_EQUALS | GREATER_EQUALS ) additive } ;
sub _parse_comparison {
    my ($self) = @_;
    my $left = $self->_parse_additive();
    while ($self->_check('LESS_THAN') || $self->_check('GREATER_THAN')
           || $self->_check('LESS_EQUALS') || $self->_check('GREATER_EQUALS')) {
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

# unary = ( BANG | MINUS ) unary | primary ;
#
# Unary minus:  -x   -1
# Logical not:  !flag
# TypeScript adds the `typeof` unary operator, but we handle it as a NAME.
sub _parse_unary {
    my ($self) = @_;
    if ($self->_check('BANG') || $self->_check('MINUS')) {
        my $op   = $self->_leaf($self->_advance());
        my $expr = $self->_parse_unary();
        return $self->_node('unary_expr', $op, $expr);
    }
    return $self->_parse_primary();
}

# primary = NUMBER | STRING | TRUE | FALSE | NULL
#         | NAME [ LPAREN arg_list RPAREN ]
#         | LPAREN ( arrow_params | expression ) RPAREN [ ARROW body ]
#
# This is the highest-precedence rule.  It handles literals, identifiers,
# function calls, grouped expressions, and arrow functions.
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

    # --- Boolean literals ---
    if ($type eq 'TRUE' || $type eq 'FALSE') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }

    # --- null / undefined ---
    if ($type eq 'NULL' || $type eq 'UNDEFINED') {
        return $self->_node('primary', $self->_leaf($self->_advance()));
    }

    # --- Identifier or function call ---
    # If the name is immediately followed by LPAREN, it is a function call.
    if ($type eq 'NAME') {
        my $name_leaf = $self->_leaf($self->_advance());
        if ($self->_check('LPAREN')) {
            my @ch = ($name_leaf);
            push @ch, $self->_leaf($self->_advance());       # (
            push @ch, $self->_parse_arg_list();
            push @ch, $self->_leaf($self->_expect('RPAREN'));
            return $self->_node('call_expr', @ch);
        }
        return $self->_node('primary', $name_leaf);
    }

    # --- Parenthesized expression or arrow function ---
    #
    # We need to decide between:
    #   (a + b)        → grouped expression
    #   (x) => x + 1   → arrow function
    #   (a, b) => a+b  → arrow function with two params
    #
    # We use _is_arrow_params() lookahead to detect the => token after ).
    if ($type eq 'LPAREN') {
        my $lparen = $self->_leaf($self->_advance());

        if ($self->_is_arrow_params()) {
            return $self->_parse_arrow_from_lparen($lparen);
        }

        # Grouped expression
        my $inner  = $self->_parse_expression();
        my $rparen = $self->_leaf($self->_expect('RPAREN'));

        # Check if this grouped expression is immediately followed by =>
        if ($self->_check('ARROW')) {
            my $arrow = $self->_leaf($self->_advance());
            my $body;
            if ($self->_check('LBRACE')) {
                $body = $self->_parse_block();
            } else {
                $body = $self->_parse_expression();
            }
            return $self->_node('arrow_expr', $lparen, $inner, $rparen, $arrow, $body);
        }

        return $self->_node('primary', $lparen, $inner, $rparen);
    }

    die sprintf(
        "CodingAdventures::TypescriptParser: unexpected token '%s' (type %s) "
      . "at line %d col %d\n",
        $tok->{value}, $tok->{type}, $tok->{line}, $tok->{col}
    );
}

# _is_arrow_params — lookahead to detect whether (…) is an arrow param list.
#
# We scan forward from the current position (after the opening LPAREN) looking
# for a pattern like: [NAME [COMMA NAME]*] RPAREN ARROW.
# If found, return 1.  If not, return 0.
sub _is_arrow_params {
    my ($self) = @_;
    my $pos = $self->{_pos};   # current pos is right after LPAREN
    my $i   = 0;
    while (1) {
        my $t = $self->{_tokens}[$pos + $i] // { type => 'EOF' };
        if ($t->{type} eq 'RPAREN') {
            my $after = $self->{_tokens}[$pos + $i + 1] // { type => 'EOF' };
            return ($after->{type} eq 'ARROW') ? 1 : 0;
        }
        if ($t->{type} eq 'NAME' || $t->{type} eq 'COMMA') {
            $i++;
            next;
        }
        return 0;  # unexpected token — not a simple arrow param list
    }
}

# _parse_arrow_from_lparen($lparen_leaf)
#
# Parse an arrow function, having already consumed the opening LPAREN.
# Grammar:  LPAREN param_list RPAREN ARROW ( block | expression )
sub _parse_arrow_from_lparen {
    my ($self, $lparen) = @_;
    my @ch = ($lparen);

    # Collect parameter names
    my @params;
    if ($self->_check('NAME')) {
        push @params, $self->_leaf($self->_advance());
        while ($self->_check('COMMA')) {
            push @params, $self->_leaf($self->_advance());
            push @params, $self->_leaf($self->_expect('NAME'));
        }
    }
    push @ch, $self->_node('param_list', @params);
    push @ch, $self->_leaf($self->_expect('RPAREN'));
    push @ch, $self->_leaf($self->_expect('ARROW'));

    # Arrow body: either a block or a concise expression
    if ($self->_check('LBRACE')) {
        push @ch, $self->_parse_block();
    } else {
        push @ch, $self->_parse_expression();
    }
    return $self->_node('arrow_expr', @ch);
}

# arg_list = [ expression { COMMA expression } ] ;
#
# The argument list for a function call. Zero or more expressions
# separated by commas.
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

# --- parse_ts($source) --------------------------------------------------------
#
# Convenience class method: tokenize and parse in one call.
# Returns the root ASTNode. Dies on error.

sub parse_ts {
    my ($class, $source) = @_;
    my $parser = $class->new($source);
    return $parser->parse();
}

1;

__END__

=head1 NAME

CodingAdventures::TypescriptParser - Hand-written recursive-descent TypeScript parser

=head1 SYNOPSIS

    use CodingAdventures::TypescriptParser;

    # Object-oriented
    my $parser = CodingAdventures::TypescriptParser->new("let x = 5;");
    my $ast    = $parser->parse();
    print $ast->rule_name;   # "program"

    # Convenience class method
    my $ast = CodingAdventures::TypescriptParser->parse_ts("const y = x + 1;");

=head1 DESCRIPTION

A hand-written recursive-descent parser for a practical subset of TypeScript.
Tokenizes input with C<CodingAdventures::TypescriptLexer> and builds an
Abstract Syntax Tree (AST) of C<CodingAdventures::TypescriptParser::ASTNode>
nodes.

Supported constructs: variable declarations (var/let/const), assignments,
function declarations, if/else statements, for loops, return statements,
blocks, function calls, arrow functions, and full expression parsing with
correct operator precedence (equality > comparison > additive > multiplicative
> unary > primary).

TypeScript-specific keywords (interface, type, enum, etc.) are recognized
by the lexer and appear as specific token types; the parser grammar handles
them naturally through the existing keyword token mechanisms.

=head1 METHODS

=head2 new($source)

Tokenize C<$source> with C<TypescriptLexer> and return a parser instance.

=head2 parse()

Parse and return the root AST node (rule_name C<"program">). Dies on error.

=head2 parse_ts($source)

Class method — tokenize and parse in one call. Returns the root ASTNode.

=head1 AST NODE FORMAT

Each node is a C<CodingAdventures::TypescriptParser::ASTNode>:

    $node->rule_name   # e.g. "var_declaration", "if_stmt", "binary_expr"
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
