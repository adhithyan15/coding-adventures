package CodingAdventures::LatticeParser;

# ============================================================================
# CodingAdventures::LatticeParser — Hand-written recursive-descent Lattice parser
# ============================================================================
#
# This module parses Lattice source text into an Abstract Syntax Tree (AST).
# It uses a hand-written recursive-descent approach because the Perl
# CodingAdventures::GrammarTools module only provides `parse_token_grammar`,
# not `parse_parser_grammar`.  There is no grammar-driven GrammarParser in
# the Perl layer — each grammar rule is a Perl subroutine.
#
# # What is Lattice?
#
# Lattice is a CSS superset language.  Every valid CSS file is valid Lattice.
# Lattice adds:
#
#   Variables           $primary: #4a90d9;
#   Mixins              @mixin button($bg) { background: $bg; }
#   @include            @include button(red);
#   Control flow        @if $dark { background: #1a1a1a; }
#                       @for $i from 1 through 12 { .col-#{$i} { ... } }
#                       @each $c in red, green { .text { color: $c; } }
#                       @while $i <= 10 { $i: $i + 1; }
#   Functions           @function spacing($n) { @return $n * 8px; }
#   Modules             @use "colors";
#   Nesting             .parent { .child { color: blue; } }
#   Placeholder selectors  %flex-center { display: flex; }
#   @extend             .hero { @extend %flex-center; }
#   @content            @mixin respond($bp) { @media ($bp) { @content; } }
#   @at-root            .parent { @at-root .child { color: red; } }
#   Map literals        $colors: (primary: #4a90d9, secondary: #e74c3c);
#
# # Grammar (implemented here)
#
#   stylesheet         = { rule }
#   rule               = lattice_rule | at_rule | qualified_rule
#   lattice_rule       = variable_declaration | mixin_definition
#                      | function_definition | use_directive | lattice_control
#   variable_declaration = VARIABLE COLON value_list [BANG_DEFAULT|BANG_GLOBAL] SEMICOLON
#   mixin_definition   = "@mixin" FUNCTION [mixin_params] RPAREN block
#                      | "@mixin" IDENT block
#   mixin_params       = mixin_param { COMMA mixin_param }
#   mixin_param        = VARIABLE [COLON mixin_value_list]
#   mixin_value_list   = mixin_value { mixin_value }
#   mixin_value        = DIMENSION|PERCENTAGE|NUMBER|STRING|IDENT|HASH
#                      | CUSTOM_PROPERTY|function_call|VARIABLE|SLASH|PLUS|MINUS
#   include_directive  = "@include" FUNCTION [include_args] RPAREN (SEMICOLON|block)
#                      | "@include" IDENT (SEMICOLON|block)
#   include_args       = include_arg { COMMA include_arg }
#   include_arg        = VARIABLE COLON value_list | value_list
#   lattice_control    = if_directive|for_directive|each_directive|while_directive
#   if_directive       = "@if" lattice_expression block
#                        { "@else" "if" lattice_expression block } ["@else" block]
#   for_directive      = "@for" VARIABLE "from" lattice_expression
#                        ("through"|"to") lattice_expression block
#   each_directive     = "@each" VARIABLE {COMMA VARIABLE} "in" each_list block
#   each_list          = value { COMMA value }
#   while_directive    = "@while" lattice_expression block
#   lattice_expression = lattice_or_expr
#   lattice_or_expr    = lattice_and_expr { "or" lattice_and_expr }
#   lattice_and_expr   = lattice_comparison { "and" lattice_comparison }
#   lattice_comparison = lattice_additive [comparison_op lattice_additive]
#   comparison_op      = EQUALS_EQUALS|NOT_EQUALS|GREATER|GREATER_EQUALS|LESS|LESS_EQUALS
#   lattice_additive   = lattice_multiplicative { (PLUS|MINUS) lattice_multiplicative }
#   lattice_multiplicative = lattice_unary { (STAR|SLASH) lattice_unary }
#   lattice_unary      = MINUS lattice_unary | lattice_primary
#   lattice_primary    = VARIABLE|NUMBER|DIMENSION|PERCENTAGE|STRING|IDENT|HASH
#                      | "true"|"false"|"null"|function_call
#                      | map_literal|LPAREN lattice_expression RPAREN
#   map_literal        = LPAREN map_entry COMMA map_entry {COMMA map_entry} RPAREN
#   map_entry          = (IDENT|STRING) COLON lattice_expression
#   function_definition = "@function" FUNCTION [mixin_params] RPAREN function_body
#                       | "@function" IDENT function_body
#   function_body      = LBRACE { function_body_item } RBRACE
#   function_body_item = variable_declaration | return_directive | lattice_control
#   return_directive   = "@return" lattice_expression SEMICOLON
#   use_directive      = "@use" STRING ["as" IDENT] SEMICOLON
#   at_rule            = AT_KEYWORD at_prelude (SEMICOLON|block)
#   qualified_rule     = selector_list block
#   selector_list      = complex_selector { COMMA complex_selector }
#   complex_selector   = compound_selector { [combinator] compound_selector }
#   combinator         = GREATER|PLUS|TILDE
#   compound_selector  = simple_selector { subclass_selector }
#                      | subclass_selector { subclass_selector }
#   simple_selector    = IDENT|STAR|AMPERSAND|VARIABLE|PERCENTAGE
#   subclass_selector  = class_selector|id_selector|placeholder_selector
#                      | attribute_selector|pseudo_class|pseudo_element
#   class_selector     = DOT IDENT
#   id_selector        = HASH
#   placeholder_selector = PLACEHOLDER
#   attribute_selector = LBRACKET IDENT [attr_matcher attr_value [IDENT]] RBRACKET
#   attr_matcher       = EQUALS|TILDE_EQUALS|PIPE_EQUALS|CARET_EQUALS
#                      | DOLLAR_EQUALS|STAR_EQUALS
#   attr_value         = IDENT|STRING
#   pseudo_class       = COLON FUNCTION pseudo_class_args RPAREN | COLON IDENT
#   pseudo_element     = COLON_COLON IDENT
#   block              = LBRACE block_contents RBRACE
#   block_contents     = { block_item }
#   block_item         = lattice_block_item | at_rule | declaration_or_nested
#   lattice_block_item = variable_declaration | include_directive | lattice_control
#                      | content_directive | extend_directive | at_root_directive
#   content_directive  = "@content" SEMICOLON
#   extend_directive   = "@extend" selector_list SEMICOLON
#   at_root_directive  = "@at-root" (selector_list block | block)
#   declaration_or_nested = declaration | qualified_rule
#   declaration        = property COLON value_list [priority] SEMICOLON
#                      | property COLON block
#   property           = IDENT|CUSTOM_PROPERTY
#   priority           = BANG "important"
#   value_list         = value { value }
#   value              = DIMENSION|PERCENTAGE|NUMBER|STRING|IDENT|HASH
#                      | CUSTOM_PROPERTY|UNICODE_RANGE|function_call
#                      | VARIABLE|SLASH|COMMA|PLUS|MINUS|map_literal
#   function_call      = FUNCTION function_args RPAREN | URL_TOKEN
#   function_args      = { function_arg }
#   function_arg       = DIMENSION|PERCENTAGE|NUMBER|STRING|IDENT|HASH
#                      | CUSTOM_PROPERTY|COMMA|SLASH|PLUS|MINUS|STAR|VARIABLE
#                      | FUNCTION function_args RPAREN
#
# # Parse state
#
# Two package-level variables hold the current parse state:
#   $tokens_ref — arrayref from CodingAdventures::LatticeLexer->tokenize
#   $pos        — 0-based current index into the token array
#
# The parser is not re-entrant (package-level state), but Lattice parsing is
# always synchronous so this is fine in practice.

use strict;
use warnings;

use CodingAdventures::LatticeLexer;
use CodingAdventures::LatticeParser::ASTNode;

our $VERSION = '0.01';

# Package-level parse state
my ($tokens_ref, $pos);

# ============================================================================
# Token-type sets used in parsing decisions
# ============================================================================
#
# Rather than hard-coding token types in conditionals scattered across the
# file, we group them into hashes.  This makes the grammar rules easier to
# read and update.

# Tokens that can appear as a mixin/function parameter value (everything
# except COMMA, which is the parameter separator).
my %MIXIN_VALUE_TYPES = map { $_ => 1 } qw(
    DIMENSION PERCENTAGE NUMBER STRING IDENT HASH
    CUSTOM_PROPERTY UNICODE_RANGE VARIABLE SLASH PLUS MINUS
);

# Tokens that can appear as a CSS/Lattice declaration value.
# COMMA is included here (e.g. "font-family: Helvetica, sans-serif;").
my %VALUE_TYPES = map { $_ => 1 } qw(
    DIMENSION PERCENTAGE NUMBER STRING IDENT HASH
    CUSTOM_PROPERTY UNICODE_RANGE VARIABLE
    SLASH COMMA PLUS MINUS
);

# Tokens valid inside a function argument list.
my %FUNCTION_ARG_TYPES = map { $_ => 1 } qw(
    DIMENSION PERCENTAGE NUMBER STRING IDENT HASH
    CUSTOM_PROPERTY COMMA SLASH PLUS MINUS STAR VARIABLE
);

# Token types that can start a simple_selector (type selector, universal,
# BEM parent reference, variable in selector, keyframe percentage).
my %SIMPLE_SELECTOR_START = map { $_ => 1 } qw(
    IDENT STAR AMPERSAND VARIABLE PERCENTAGE
);

# Token types that start a subclass_selector.
my %SUBCLASS_SELECTOR_START = map { $_ => 1 } qw(
    DOT HASH LBRACKET COLON COLON_COLON PLACEHOLDER
);

# Comparison operator token types.
my %COMPARISON_OPS = map { $_ => 1 } qw(
    EQUALS_EQUALS NOT_EQUALS GREATER GREATER_EQUALS LESS LESS_EQUALS
);

# Lattice expression primary token types (scalars and keywords).
my %PRIMARY_SCALAR_TYPES = map { $_ => 1 } qw(
    VARIABLE NUMBER DIMENSION PERCENTAGE STRING IDENT HASH
);

# AT_KEYWORD values that start Lattice-specific top-level constructs.
# These are matched by checking $t->{value}.  Anything else is a CSS at_rule.
my %LATTICE_AT_KEYWORDS = map { $_ => 1 } qw(
    @mixin @include @if @else @for @each @while
    @function @return @use @content @extend @at-root
);

# ============================================================================
# Public API
# ============================================================================

# --- parse($class, $source) ---------------------------------------------------
#
# Parse a Lattice source string and return the root ASTNode.
#
# @param  $source  string  The Lattice text to parse.
# @return ASTNode          Root with rule_name "stylesheet".
# @die                     On any lexer or parser error.

sub parse {
    my ($class, $source) = @_;

    my $toks = CodingAdventures::LatticeLexer->tokenize($source);
    $tokens_ref = $toks;
    $pos = 0;

    my $ast = _parse_stylesheet();

    # Verify we consumed everything
    my $t = _peek();
    if ($t->{type} ne 'EOF') {
        die sprintf(
            "CodingAdventures::LatticeParser: trailing content at line %d col %d: "
          . "unexpected %s ('%s')",
            $t->{line}, $t->{col}, $t->{type}, $t->{value}
        );
    }

    return $ast;
}

# ============================================================================
# Internal helpers
# ============================================================================

# _peek() — return the current token without consuming it.
# _peek_at($offset) — look $offset positions ahead.
# _advance() — consume and return the current token.
# _expect($type) — consume a token of the given type, or die.
# _node($rule, $children) — construct an internal AST node.
# _leaf($token) — construct a leaf AST node wrapping a token.

sub _peek    { $tokens_ref->[$pos] // $tokens_ref->[-1] }
sub _peek_at { $tokens_ref->[$pos + $_[0]] // $tokens_ref->[-1] }

sub _advance {
    my $t = _peek();
    $pos++;
    return $t;
}

sub _expect {
    my ($type) = @_;
    my $t = _peek();
    unless ($t->{type} eq $type) {
        die sprintf(
            "CodingAdventures::LatticeParser: Expected %s, got %s ('%s') "
          . "at line %d col %d",
            $type, $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _advance();
}

sub _node {
    my ($rule, $children) = @_;
    return CodingAdventures::LatticeParser::ASTNode->new(
        rule_name => $rule,
        children  => $children,
        is_leaf   => 0,
    );
}

sub _leaf {
    my ($tok) = @_;
    return CodingAdventures::LatticeParser::ASTNode->new(
        rule_name => 'token',
        children  => [],
        is_leaf   => 1,
        token     => $tok,
    );
}

# ============================================================================
# Top-level: stylesheet and rule
# ============================================================================

# --- _parse_stylesheet() ------------------------------------------------------
#
# Grammar: stylesheet = { rule }
#
# A Lattice stylesheet is a sequence of rules.  Unlike TOML or most languages
# with line-oriented syntax, CSS is not newline-sensitive.  We keep consuming
# rules until we hit EOF.

sub _parse_stylesheet {
    my @children;

    while (_peek()->{type} ne 'EOF') {
        push @children, _parse_rule();
    }

    return _node('stylesheet', \@children);
}

# --- _parse_rule() ------------------------------------------------------------
#
# Grammar: rule = lattice_rule | at_rule | qualified_rule
#
# Dispatch:
#   VARIABLE           → variable_declaration (lattice_rule)
#   AT_KEYWORD(@mixin,
#     @include, @if,
#     @for, @each,
#     @while, @function,
#     @return, @use)    → lattice_rule
#   AT_KEYWORD (other) → at_rule (standard CSS like @media, @keyframes)
#   anything else       → qualified_rule (selector { ... })

sub _parse_rule {
    my $t = _peek();

    # VARIABLE token → variable_declaration
    if ($t->{type} eq 'VARIABLE') {
        return _node('rule', [_node('lattice_rule', [_parse_variable_declaration()])]);
    }

    # AT_KEYWORD — split between Lattice constructs and CSS at-rules
    if ($t->{type} eq 'AT_KEYWORD') {
        my $kw = $t->{value};

        if ($kw eq '@mixin') {
            return _node('rule', [_node('lattice_rule', [_parse_mixin_definition()])]);
        }
        if ($kw eq '@function') {
            return _node('rule', [_node('lattice_rule', [_parse_function_definition()])]);
        }
        if ($kw eq '@use') {
            return _node('rule', [_node('lattice_rule', [_parse_use_directive()])]);
        }
        if ($kw eq '@if') {
            return _node('rule', [_node('lattice_rule', [_node('lattice_control', [_parse_if_directive()])])]);
        }
        if ($kw eq '@for') {
            return _node('rule', [_node('lattice_rule', [_node('lattice_control', [_parse_for_directive()])])]);
        }
        if ($kw eq '@each') {
            return _node('rule', [_node('lattice_rule', [_node('lattice_control', [_parse_each_directive()])])]);
        }
        if ($kw eq '@while') {
            return _node('rule', [_node('lattice_rule', [_node('lattice_control', [_parse_while_directive()])])]);
        }

        # All other AT_KEYWORD values are standard CSS at-rules
        return _node('rule', [_parse_at_rule()]);
    }

    # Everything else is a qualified rule (selector + block)
    return _node('rule', [_parse_qualified_rule()]);
}

# ============================================================================
# Lattice: Variables
# ============================================================================

# --- _parse_variable_declaration() -------------------------------------------
#
# Grammar: variable_declaration = VARIABLE COLON value_list
#                                  [BANG_DEFAULT|BANG_GLOBAL] SEMICOLON
#
# Variable declarations are the simplest Lattice construct.  They look
# exactly like CSS custom properties but use $name instead of --name:
#
#   $primary: #4a90d9;
#   $font-size: 16px !default;
#   $theme-color: $primary !global;
#
# The optional flags !default and !global are tokenized as BANG_DEFAULT
# and BANG_GLOBAL by the lexer (the leading '!' is part of the token).

sub _parse_variable_declaration {
    my @children;

    push @children, _leaf(_expect('VARIABLE'));
    push @children, _leaf(_expect('COLON'));
    push @children, _parse_value_list();

    # Optional !default or !global flag
    my $t = _peek();
    if ($t->{type} eq 'BANG_DEFAULT' || $t->{type} eq 'BANG_GLOBAL') {
        push @children, _leaf(_advance());
    }

    push @children, _leaf(_expect('SEMICOLON'));

    return _node('variable_declaration', \@children);
}

# ============================================================================
# Lattice: Mixins
# ============================================================================

# --- _parse_mixin_definition() -----------------------------------------------
#
# Grammar:
#   mixin_definition = "@mixin" FUNCTION [mixin_params] RPAREN block
#                    | "@mixin" IDENT block
#
# Mixins are reusable blocks of declarations.  Two syntactic forms:
#
#   FUNCTION form (name-with-parens tokenized as one FUNCTION token):
#     @mixin button($bg, $fg: white) { background: $bg; color: $fg; }
#     @mixin reset() { margin: 0; padding: 0; }   ← zero params, parens present
#
#   IDENT form (name without parens):
#     @mixin clearfix { &::after { content: ""; clear: both; display: table; } }
#
# We check the second token after @mixin to choose which form applies.

sub _parse_mixin_definition {
    my @children;

    push @children, _leaf(_expect_kw('@mixin'));

    my $t = _peek();

    if ($t->{type} eq 'FUNCTION') {
        # FUNCTION form: name includes the opening paren
        push @children, _leaf(_advance());  # FUNCTION token

        # Optional parameters (may be empty: @mixin reset())
        if (_peek()->{type} ne 'RPAREN') {
            push @children, _parse_mixin_params();
        }

        push @children, _leaf(_expect('RPAREN'));
    }
    elsif ($t->{type} eq 'IDENT') {
        # IDENT form: name without parens
        push @children, _leaf(_advance());  # IDENT token
    }
    else {
        die sprintf(
            "CodingAdventures::LatticeParser: Expected FUNCTION or IDENT after \@mixin, "
          . "got %s ('%s') at line %d col %d",
            $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }

    push @children, _parse_block();

    return _node('mixin_definition', \@children);
}

# --- _parse_mixin_params() ----------------------------------------------------
#
# Grammar: mixin_params = mixin_param { COMMA mixin_param }
#
# Parses a comma-separated list of mixin parameters.

sub _parse_mixin_params {
    my @children;

    push @children, _parse_mixin_param();

    while (_peek()->{type} eq 'COMMA') {
        push @children, _leaf(_advance());  # COMMA
        push @children, _parse_mixin_param();
    }

    return _node('mixin_params', \@children);
}

# --- _parse_mixin_param() -----------------------------------------------------
#
# Grammar: mixin_param = VARIABLE [COLON mixin_value_list]
#
# A single parameter with an optional default value.
# The default value list stops before COMMA or RPAREN.
#
# Example: $bg: #fff   →  VARIABLE("$bg") COLON HASH("#fff")
# Example: $radius     →  VARIABLE("$radius")

sub _parse_mixin_param {
    my @children;

    push @children, _leaf(_expect('VARIABLE'));

    if (_peek()->{type} eq 'COLON') {
        push @children, _leaf(_advance());  # COLON
        push @children, _parse_mixin_value_list();
    }

    return _node('mixin_param', \@children);
}

# --- _parse_mixin_value_list() ------------------------------------------------
#
# Grammar: mixin_value_list = mixin_value { mixin_value }
#
# Like value_list but excludes COMMA (which separates parameters).
# Stops at COMMA, RPAREN, or EOF.

sub _parse_mixin_value_list {
    my @children;

    while ($MIXIN_VALUE_TYPES{_peek()->{type}}) {
        my $t = _peek();

        if ($t->{type} eq 'FUNCTION') {
            push @children, _parse_function_call();
        }
        else {
            push @children, _node('mixin_value', [_leaf(_advance())]);
        }

        # Stop before parameter separator
        last if _peek()->{type} eq 'COMMA' || _peek()->{type} eq 'RPAREN';
    }

    return _node('mixin_value_list', \@children);
}

# --- _parse_include_directive() -----------------------------------------------
#
# Grammar:
#   include_directive = "@include" FUNCTION [include_args] RPAREN (SEMICOLON|block)
#                     | "@include" IDENT (SEMICOLON|block)
#
# @include expands a mixin at the call site.  Forms:
#
#   @include button(red);             ← FUNCTION form, args present
#   @include button();                ← FUNCTION form, no args
#   @include clearfix;                ← IDENT form
#   @include respond(768px) { ... }   ← IDENT form with content block

sub _parse_include_directive {
    my @children;

    push @children, _leaf(_expect_kw('@include'));

    my $t = _peek();

    if ($t->{type} eq 'FUNCTION') {
        push @children, _leaf(_advance());  # FUNCTION token

        # Optional arguments
        if (_peek()->{type} ne 'RPAREN') {
            push @children, _parse_include_args();
        }

        push @children, _leaf(_expect('RPAREN'));
    }
    elsif ($t->{type} eq 'IDENT') {
        push @children, _leaf(_advance());  # IDENT token
    }
    else {
        die sprintf(
            "CodingAdventures::LatticeParser: Expected FUNCTION or IDENT after \@include, "
          . "got %s ('%s') at line %d col %d",
            $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }

    # Terminator: semicolon or content block
    if (_peek()->{type} eq 'SEMICOLON') {
        push @children, _leaf(_advance());
    }
    elsif (_peek()->{type} eq 'LBRACE') {
        push @children, _parse_block();
    }
    else {
        die sprintf(
            "CodingAdventures::LatticeParser: Expected ';' or '{' after \@include, "
          . "got %s ('%s') at line %d col %d",
            _peek()->{type}, _peek()->{value}, _peek()->{line}, _peek()->{col}
        );
    }

    return _node('include_directive', \@children);
}

# --- _parse_include_args() ----------------------------------------------------
#
# Grammar: include_args = include_arg { COMMA include_arg }

sub _parse_include_args {
    my @children;

    push @children, _parse_include_arg();

    while (_peek()->{type} eq 'COMMA') {
        push @children, _leaf(_advance());  # COMMA
        push @children, _parse_include_arg();
    }

    return _node('include_args', \@children);
}

# --- _parse_include_arg() -----------------------------------------------------
#
# Grammar: include_arg = VARIABLE COLON value_list | value_list
#
# Named argument ($param: value) is tried first.  If the token is VARIABLE
# and the next token is COLON, it's a named argument; otherwise it's a
# positional value_list.
#
# Example named:    @include btn($color: red, $size: large)
# Example positional: @include btn(red, 14px)

sub _parse_include_arg {
    my @children;
    my $t = _peek();

    if ($t->{type} eq 'VARIABLE' && _peek_at(1)->{type} eq 'COLON') {
        # Named argument: VARIABLE COLON value_list
        push @children, _leaf(_advance());  # VARIABLE
        push @children, _leaf(_advance());  # COLON
        push @children, _parse_value_list_no_comma();
    }
    else {
        # Positional: value_list (but stop at COMMA and RPAREN)
        push @children, _parse_value_list_no_comma();
    }

    return _node('include_arg', \@children);
}

# ============================================================================
# Lattice: Control flow
# ============================================================================

# --- _parse_if_directive() ----------------------------------------------------
#
# Grammar: if_directive = "@if" lattice_expression block
#                         { "@else" "if" lattice_expression block }
#                         ["@else" block]
#
# Conditional inclusion.  The @else if and @else clauses are optional.
# Disambiguation: after each block, peek at the next token.
#   If it is AT_KEYWORD with value "@else", consume the @else.
#   Then peek again: if IDENT with value "if", consume "if" and parse another
#   lattice_expression + block (chained @else if).
#   Otherwise parse the bare @else block.

sub _parse_if_directive {
    my @children;

    push @children, _leaf(_expect_kw('@if'));
    push @children, _parse_lattice_expression();
    push @children, _parse_block();

    # Zero or more @else if chains
    while (_peek()->{type} eq 'AT_KEYWORD' && _peek()->{value} eq '@else') {
        # Look one further: is the next token IDENT("if")?
        if (_peek_at(1)->{type} eq 'IDENT' && _peek_at(1)->{value} eq 'if') {
            push @children, _leaf(_advance());              # @else
            push @children, _leaf(_advance());              # if
            push @children, _parse_lattice_expression();
            push @children, _parse_block();
        }
        else {
            # Bare @else — consume and parse block, then stop
            push @children, _leaf(_advance());              # @else
            push @children, _parse_block();
            last;
        }
    }

    return _node('if_directive', \@children);
}

# --- _parse_for_directive() ---------------------------------------------------
#
# Grammar: for_directive = "@for" VARIABLE "from" lattice_expression
#                          ("through"|"to") lattice_expression block
#
# Numeric iteration.  "through" is inclusive, "to" is exclusive.
#
#   @for $i from 1 through 12 { .col-#{$i} { width: calc(100% / 12); } }
#   @for $i from 0 to 5       { .step-#{$i} { opacity: $i * 0.2; } }
#
# The keywords "from", "through", and "to" are IDENT tokens matched
# by checking $t->{value}.

sub _parse_for_directive {
    my @children;

    push @children, _leaf(_expect_kw('@for'));
    push @children, _leaf(_expect('VARIABLE'));
    push @children, _leaf(_expect_ident('from'));
    push @children, _parse_lattice_expression();

    # "through" or "to"
    my $t = _peek();
    if ($t->{type} eq 'IDENT' && ($t->{value} eq 'through' || $t->{value} eq 'to')) {
        push @children, _leaf(_advance());
    }
    else {
        die sprintf(
            "CodingAdventures::LatticeParser: Expected 'through' or 'to' in \@for, "
          . "got %s ('%s') at line %d col %d",
            $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }

    push @children, _parse_lattice_expression();
    push @children, _parse_block();

    return _node('for_directive', \@children);
}

# --- _parse_each_directive() --------------------------------------------------
#
# Grammar: each_directive = "@each" VARIABLE {COMMA VARIABLE} "in" each_list block
#
# List iteration.  Multiple variables enable destructuring over maps:
#   @each $key, $value in $colors { ... }
#   @each $color in red, green, blue { .text { color: $color; } }

sub _parse_each_directive {
    my @children;

    push @children, _leaf(_expect_kw('@each'));
    push @children, _leaf(_expect('VARIABLE'));

    # Additional variables for destructuring
    while (_peek()->{type} eq 'COMMA' && _peek_at(1)->{type} eq 'VARIABLE') {
        push @children, _leaf(_advance());  # COMMA
        push @children, _leaf(_advance());  # VARIABLE
    }

    push @children, _leaf(_expect_ident('in'));
    push @children, _parse_each_list();
    push @children, _parse_block();

    return _node('each_directive', \@children);
}

# --- _parse_each_list() -------------------------------------------------------
#
# Grammar: each_list = value { COMMA value }
#
# The list after "in" in an @each directive.  Each element is a single value
# (IDENT, HASH, STRING, etc.).  We stop when we hit LBRACE (the block start).

sub _parse_each_list {
    my @children;

    push @children, _parse_single_value();

    while (_peek()->{type} eq 'COMMA' && _peek()->{type} ne 'LBRACE') {
        # Only continue if the next-next token is not LBRACE (end of list)
        last if _peek_at(1)->{type} eq 'LBRACE';
        push @children, _leaf(_advance());   # COMMA
        push @children, _parse_single_value();
    }

    return _node('each_list', \@children);
}

# --- _parse_while_directive() -------------------------------------------------
#
# Grammar: while_directive = "@while" lattice_expression block
#
# Loop until condition is false.  The transformer enforces a max-iteration
# guard; the grammar/parser does not limit iterations.
#
#   @while $i <= 12 { .col { width: $i * 8%; } $i: $i + 1; }

sub _parse_while_directive {
    my @children;

    push @children, _leaf(_expect_kw('@while'));
    push @children, _parse_lattice_expression();
    push @children, _parse_block();

    return _node('while_directive', \@children);
}

# ============================================================================
# Lattice: Expressions
# ============================================================================
#
# Expressions use a classic recursive-descent precedence climb.  We model
# operator precedence by nesting grammar rules from loosest to tightest:
#
#   loosest:   lattice_or_expr  (or)
#              lattice_and_expr (and)
#              lattice_comparison (==, !=, <, >, <=, >=)
#              lattice_additive (+, -)
#              lattice_multiplicative (*, /)
#   tightest:  lattice_unary (unary -)
#              lattice_primary (atom)
#
# Each level delegates to the next-tighter level and then optionally
# consumes operators at its precedence, looping for left-associativity.

# --- _parse_lattice_expression() ----------------------------------------------
#
# The top-level expression rule — delegates to lattice_or_expr.

sub _parse_lattice_expression {
    return _node('lattice_expression', [_parse_lattice_or_expr()]);
}

# --- _parse_lattice_or_expr() -------------------------------------------------
#
# Grammar: lattice_or_expr = lattice_and_expr { "or" lattice_and_expr }

sub _parse_lattice_or_expr {
    my @children;

    push @children, _parse_lattice_and_expr();

    while (_peek()->{type} eq 'IDENT' && _peek()->{value} eq 'or') {
        push @children, _leaf(_advance());  # "or"
        push @children, _parse_lattice_and_expr();
    }

    return _node('lattice_or_expr', \@children);
}

# --- _parse_lattice_and_expr() ------------------------------------------------
#
# Grammar: lattice_and_expr = lattice_comparison { "and" lattice_comparison }

sub _parse_lattice_and_expr {
    my @children;

    push @children, _parse_lattice_comparison();

    while (_peek()->{type} eq 'IDENT' && _peek()->{value} eq 'and') {
        push @children, _leaf(_advance());  # "and"
        push @children, _parse_lattice_comparison();
    }

    return _node('lattice_and_expr', \@children);
}

# --- _parse_lattice_comparison() ----------------------------------------------
#
# Grammar: lattice_comparison = lattice_additive [comparison_op lattice_additive]
#
# Comparison is non-associative: $a < $b > $c is not valid Lattice.
# We parse at most one comparison operator.

sub _parse_lattice_comparison {
    my @children;

    push @children, _parse_lattice_additive();

    if ($COMPARISON_OPS{_peek()->{type}}) {
        push @children, _parse_comparison_op();
        push @children, _parse_lattice_additive();
    }

    return _node('lattice_comparison', \@children);
}

# --- _parse_comparison_op() ---------------------------------------------------
#
# Grammar: comparison_op = EQUALS_EQUALS|NOT_EQUALS|GREATER|GREATER_EQUALS
#                        | LESS|LESS_EQUALS

sub _parse_comparison_op {
    my $t = _peek();
    unless ($COMPARISON_OPS{$t->{type}}) {
        die sprintf(
            "CodingAdventures::LatticeParser: Expected comparison operator, "
          . "got %s ('%s') at line %d col %d",
            $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _node('comparison_op', [_leaf(_advance())]);
}

# --- _parse_lattice_additive() ------------------------------------------------
#
# Grammar: lattice_additive = lattice_multiplicative
#                             { (PLUS|MINUS) lattice_multiplicative }

sub _parse_lattice_additive {
    my @children;

    push @children, _parse_lattice_multiplicative();

    while (_peek()->{type} eq 'PLUS' || _peek()->{type} eq 'MINUS') {
        push @children, _leaf(_advance());  # PLUS or MINUS
        push @children, _parse_lattice_multiplicative();
    }

    return _node('lattice_additive', \@children);
}

# --- _parse_lattice_multiplicative() ------------------------------------------
#
# Grammar: lattice_multiplicative = lattice_unary { (STAR|SLASH) lattice_unary }

sub _parse_lattice_multiplicative {
    my @children;

    push @children, _parse_lattice_unary();

    while (_peek()->{type} eq 'STAR' || _peek()->{type} eq 'SLASH') {
        push @children, _leaf(_advance());  # STAR or SLASH
        push @children, _parse_lattice_unary();
    }

    return _node('lattice_multiplicative', \@children);
}

# --- _parse_lattice_unary() ---------------------------------------------------
#
# Grammar: lattice_unary = MINUS lattice_unary | lattice_primary
#
# Handles unary minus: -$x, -16px.  Recursively applied so --$x is also valid
# (double negation, unusual but grammatically correct).

sub _parse_lattice_unary {
    my @children;

    if (_peek()->{type} eq 'MINUS') {
        push @children, _leaf(_advance());  # MINUS
        push @children, _parse_lattice_unary();
        return _node('lattice_unary', \@children);
    }

    return _node('lattice_unary', [_parse_lattice_primary()]);
}

# --- _parse_lattice_primary() -------------------------------------------------
#
# Grammar: lattice_primary = VARIABLE|NUMBER|DIMENSION|PERCENTAGE|STRING
#                          | IDENT|HASH|"true"|"false"|"null"
#                          | function_call|map_literal
#                          | LPAREN lattice_expression RPAREN
#
# The atom level of the expression hierarchy.
#
# Disambiguation for LPAREN:
#   If the next-next token (after LPAREN) is (IDENT|STRING) COLON, it's a
#   map_literal (key: value pairs).  Otherwise it's a parenthesized expression.
#
# Note: "true", "false", "null" are tokenized as IDENT tokens with those
# specific values, not as separate token types in Lattice.

sub _parse_lattice_primary {
    my @children;
    my $t = _peek();

    if ($PRIMARY_SCALAR_TYPES{$t->{type}}) {
        return _node('lattice_primary', [_leaf(_advance())]);
    }

    if ($t->{type} eq 'FUNCTION') {
        # function_call starts with FUNCTION token (name + opening paren)
        return _node('lattice_primary', [_parse_function_call()]);
    }

    if ($t->{type} eq 'LPAREN') {
        # Disambiguate: map_literal vs parenthesized expression
        my $t1 = _peek_at(1);
        my $t2 = _peek_at(2);
        if (($t1->{type} eq 'IDENT' || $t1->{type} eq 'STRING')
                && $t2->{type} eq 'COLON') {
            return _node('lattice_primary', [_parse_map_literal()]);
        }
        # Parenthesized expression: LPAREN expr RPAREN
        push @children, _leaf(_advance());  # LPAREN
        push @children, _parse_lattice_expression();
        push @children, _leaf(_expect('RPAREN'));
        return _node('lattice_primary', \@children);
    }

    die sprintf(
        "CodingAdventures::LatticeParser: Expected expression primary, "
      . "got %s ('%s') at line %d col %d",
        $t->{type}, $t->{value}, $t->{line}, $t->{col}
    );
}

# --- _parse_map_literal() -----------------------------------------------------
#
# Grammar: map_literal = LPAREN map_entry COMMA map_entry {COMMA map_entry} RPAREN
#
# Map literals hold ordered key-value pairs.  They must have at least two
# entries to distinguish them from parenthesized single expressions.
#
#   $colors: (primary: #4a90d9, secondary: #e74c3c);

sub _parse_map_literal {
    my @children;

    push @children, _leaf(_expect('LPAREN'));
    push @children, _parse_map_entry();

    while (_peek()->{type} eq 'COMMA') {
        push @children, _leaf(_advance());  # COMMA
        last if _peek()->{type} eq 'RPAREN';
        push @children, _parse_map_entry();
    }

    push @children, _leaf(_expect('RPAREN'));

    return _node('map_literal', \@children);
}

# --- _parse_map_entry() -------------------------------------------------------
#
# Grammar: map_entry = (IDENT|STRING) COLON lattice_expression

sub _parse_map_entry {
    my @children;
    my $t = _peek();

    unless ($t->{type} eq 'IDENT' || $t->{type} eq 'STRING') {
        die sprintf(
            "CodingAdventures::LatticeParser: Expected IDENT or STRING as map key, "
          . "got %s ('%s') at line %d col %d",
            $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }

    push @children, _leaf(_advance());  # key
    push @children, _leaf(_expect('COLON'));
    push @children, _parse_lattice_expression();

    return _node('map_entry', \@children);
}

# ============================================================================
# Lattice: Functions
# ============================================================================

# --- _parse_function_definition() ---------------------------------------------
#
# Grammar:
#   function_definition = "@function" FUNCTION [mixin_params] RPAREN function_body
#                       | "@function" IDENT function_body
#
# Like mixin_definition but the body is restricted to variable declarations,
# @return, and control flow.  No CSS rules inside a function body.
#
#   @function spacing($n) { @return $n * 8px; }
#   @function pi { @return 3.14159; }

sub _parse_function_definition {
    my @children;

    push @children, _leaf(_expect_kw('@function'));

    my $t = _peek();

    if ($t->{type} eq 'FUNCTION') {
        push @children, _leaf(_advance());  # FUNCTION token

        if (_peek()->{type} ne 'RPAREN') {
            push @children, _parse_mixin_params();
        }

        push @children, _leaf(_expect('RPAREN'));
    }
    elsif ($t->{type} eq 'IDENT') {
        push @children, _leaf(_advance());  # IDENT
    }
    else {
        die sprintf(
            "CodingAdventures::LatticeParser: Expected FUNCTION or IDENT after \@function, "
          . "got %s ('%s') at line %d col %d",
            $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }

    push @children, _parse_function_body();

    return _node('function_definition', \@children);
}

# --- _parse_function_body() ---------------------------------------------------
#
# Grammar: function_body = LBRACE { function_body_item } RBRACE
#
# A function body may only contain variable declarations, @return, and
# control flow — it cannot contain CSS rules or declarations.

sub _parse_function_body {
    my @children;

    push @children, _leaf(_expect('LBRACE'));

    while (_peek()->{type} ne 'RBRACE' && _peek()->{type} ne 'EOF') {
        push @children, _parse_function_body_item();
    }

    push @children, _leaf(_expect('RBRACE'));

    return _node('function_body', \@children);
}

# --- _parse_function_body_item() ----------------------------------------------
#
# Grammar: function_body_item = variable_declaration | return_directive
#                             | lattice_control

sub _parse_function_body_item {
    my $t = _peek();

    if ($t->{type} eq 'VARIABLE') {
        return _node('function_body_item', [_parse_variable_declaration()]);
    }

    if ($t->{type} eq 'AT_KEYWORD') {
        my $kw = $t->{value};

        if ($kw eq '@return') {
            return _node('function_body_item', [_parse_return_directive()]);
        }
        if ($kw eq '@if') {
            return _node('function_body_item', [_node('lattice_control', [_parse_if_directive()])]);
        }
        if ($kw eq '@for') {
            return _node('function_body_item', [_node('lattice_control', [_parse_for_directive()])]);
        }
        if ($kw eq '@each') {
            return _node('function_body_item', [_node('lattice_control', [_parse_each_directive()])]);
        }
        if ($kw eq '@while') {
            return _node('function_body_item', [_node('lattice_control', [_parse_while_directive()])]);
        }
    }

    die sprintf(
        "CodingAdventures::LatticeParser: Unexpected token in function body: "
      . "%s ('%s') at line %d col %d",
        $t->{type}, $t->{value}, $t->{line}, $t->{col}
    );
}

# --- _parse_return_directive() ------------------------------------------------
#
# Grammar: return_directive = "@return" lattice_expression SEMICOLON

sub _parse_return_directive {
    my @children;

    push @children, _leaf(_expect_kw('@return'));
    push @children, _parse_lattice_expression();
    push @children, _leaf(_expect('SEMICOLON'));

    return _node('return_directive', \@children);
}

# ============================================================================
# Lattice: Modules
# ============================================================================

# --- _parse_use_directive() ---------------------------------------------------
#
# Grammar: use_directive = "@use" STRING ["as" IDENT] SEMICOLON
#
# Imports another Lattice file's exports.  The string is a file path
# (without extension) resolved relative to the source file.
#
#   @use "colors";
#   @use "utils/mixins" as m;

sub _parse_use_directive {
    my @children;

    push @children, _leaf(_expect_kw('@use'));
    push @children, _leaf(_expect('STRING'));

    if (_peek()->{type} eq 'IDENT' && _peek()->{value} eq 'as') {
        push @children, _leaf(_advance());          # "as"
        push @children, _leaf(_expect('IDENT'));    # namespace alias
    }

    push @children, _leaf(_expect('SEMICOLON'));

    return _node('use_directive', \@children);
}

# ============================================================================
# CSS: At-rules
# ============================================================================

# --- _parse_at_rule() ---------------------------------------------------------
#
# Grammar: at_rule = AT_KEYWORD at_prelude (SEMICOLON|block)
#
# Handles all standard CSS at-rules that are NOT Lattice-specific.
# Examples: @media, @keyframes, @charset, @import, @supports, @font-face.
#
# The prelude is everything between the AT_KEYWORD and the terminating
# SEMICOLON or LBRACE.  We consume prelude tokens greedily.

sub _parse_at_rule {
    my @children;

    push @children, _leaf(_advance());  # AT_KEYWORD
    push @children, _parse_at_prelude();

    if (_peek()->{type} eq 'SEMICOLON') {
        push @children, _leaf(_advance());
    }
    elsif (_peek()->{type} eq 'LBRACE') {
        push @children, _parse_block();
    }
    else {
        die sprintf(
            "CodingAdventures::LatticeParser: Expected ';' or '{' after at-rule prelude, "
          . "got %s ('%s') at line %d col %d",
            _peek()->{type}, _peek()->{value}, _peek()->{line}, _peek()->{col}
        );
    }

    return _node('at_rule', \@children);
}

# --- _parse_at_prelude() ------------------------------------------------------
#
# Grammar: at_prelude = { at_prelude_token }
#
# Consumes prelude tokens until we reach SEMICOLON or LBRACE (neither of
# which can appear inside a prelude without LPAREN balancing).

sub _parse_at_prelude {
    my @children;

    while (_is_at_prelude_token(_peek())) {
        my $t = _peek();

        if ($t->{type} eq 'FUNCTION') {
            # function_in_prelude: FUNCTION at_prelude_tokens RPAREN
            my @fc;
            push @fc, _leaf(_advance());  # FUNCTION
            push @fc, _parse_at_prelude_tokens();
            push @fc, _leaf(_expect('RPAREN'));
            push @children, _node('at_prelude_token', \@fc);
        }
        elsif ($t->{type} eq 'LPAREN') {
            # paren_block: LPAREN at_prelude_tokens RPAREN
            my @pb;
            push @pb, _leaf(_advance());  # LPAREN
            push @pb, _parse_at_prelude_tokens();
            push @pb, _leaf(_expect('RPAREN'));
            push @children, _node('at_prelude_token', \@pb);
        }
        else {
            push @children, _node('at_prelude_token', [_leaf(_advance())]);
        }
    }

    return _node('at_prelude', \@children);
}

# --- _parse_at_prelude_tokens() -----------------------------------------------
#
# Helper: parse zero or more at_prelude_tokens (used inside FUNCTION/LPAREN).

sub _parse_at_prelude_tokens {
    my @children;
    while (_is_at_prelude_token(_peek())) {
        my $t = _peek();
        if ($t->{type} eq 'FUNCTION' || $t->{type} eq 'LPAREN') {
            # Recurse for nested parens (e.g. calc(100% - 2px))
            push @children, _node('at_prelude_token', [_leaf(_advance())]);
        }
        else {
            push @children, _leaf(_advance());
        }
    }
    return _node('at_prelude_tokens', \@children);
}

# --- _is_at_prelude_token($t) -------------------------------------------------
#
# Returns true if $t is a valid at_prelude_token.  Stops at SEMICOLON,
# LBRACE, RBRACE, RPAREN, and EOF.

sub _is_at_prelude_token {
    my ($t) = @_;
    my $type = $t->{type};
    return 0 if $type eq 'SEMICOLON' || $type eq 'LBRACE'
              || $type eq 'RBRACE'   || $type eq 'RPAREN'
              || $type eq 'EOF';
    return 1;
}

# ============================================================================
# CSS: Qualified rules and selectors
# ============================================================================

# --- _parse_qualified_rule() --------------------------------------------------
#
# Grammar: qualified_rule = selector_list block
#
# A standard CSS rule: one or more selectors followed by a declaration block.
# Examples:
#   h1 { color: red; }
#   .nav, .header { display: flex; }
#   .parent > .child + .sibling { margin: 0; }

sub _parse_qualified_rule {
    my @children;

    push @children, _parse_selector_list();
    push @children, _parse_block();

    return _node('qualified_rule', \@children);
}

# --- _parse_selector_list() ---------------------------------------------------
#
# Grammar: selector_list = complex_selector { COMMA complex_selector }

sub _parse_selector_list {
    my @children;

    push @children, _parse_complex_selector();

    # COMMA-separated selector list (e.g. "h1, h2, .header")
    # Only consume COMMA if followed by a selector token (not LBRACE)
    while (_peek()->{type} eq 'COMMA' && _peek_at(1)->{type} ne 'LBRACE') {
        last unless _is_selector_start(_peek_at(1));
        push @children, _leaf(_advance());  # COMMA
        push @children, _parse_complex_selector();
    }

    return _node('selector_list', \@children);
}

# --- _parse_complex_selector() ------------------------------------------------
#
# Grammar: complex_selector = compound_selector { [combinator] compound_selector }
#
# A complex selector is two or more compound selectors joined by combinators:
#   .parent > .child         — child combinator (GREATER)
#   .nav .item               — descendant (space, no explicit combinator)
#   h1 + h2                  — adjacent sibling (PLUS)
#   h1 ~ h2                  — general sibling (TILDE)

sub _parse_complex_selector {
    my @children;

    push @children, _parse_compound_selector();

    # Continue while the next token starts another compound selector
    while (1) {
        my $t = _peek();

        if ($t->{type} eq 'GREATER' || $t->{type} eq 'PLUS' || $t->{type} eq 'TILDE') {
            # Explicit combinator
            push @children, _node('combinator', [_leaf(_advance())]);
            push @children, _parse_compound_selector();
        }
        elsif (_is_selector_start($t)) {
            # Implicit descendant combinator (whitespace in source)
            push @children, _parse_compound_selector();
        }
        else {
            last;
        }
    }

    return _node('complex_selector', \@children);
}

# --- _parse_compound_selector() -----------------------------------------------
#
# Grammar: compound_selector = simple_selector { subclass_selector }
#                            | subclass_selector { subclass_selector }
#
# A compound selector is a type/universal/BEM selector optionally followed by
# class, id, attribute, pseudo-class, and pseudo-element selectors:
#   input.form-control[type="text"]:focus
#   .container.wide

sub _parse_compound_selector {
    my @children;
    my $t = _peek();

    if ($SIMPLE_SELECTOR_START{$t->{type}}) {
        push @children, _parse_simple_selector();
    }

    # Zero or more subclass selectors (DOT, HASH, LBRACKET, COLON, PLACEHOLDER)
    while ($SUBCLASS_SELECTOR_START{_peek()->{type}}) {
        push @children, _parse_subclass_selector();
    }

    unless (@children) {
        die sprintf(
            "CodingAdventures::LatticeParser: Expected selector, "
          . "got %s ('%s') at line %d col %d",
            $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }

    return _node('compound_selector', \@children);
}

# --- _parse_simple_selector() -------------------------------------------------
#
# Grammar: simple_selector = IDENT | STAR | AMPERSAND | VARIABLE | PERCENTAGE
#
# The type selector (element name), universal selector (*), BEM parent
# reference (&), variable reference ($var in selector), or keyframe percentage.

sub _parse_simple_selector {
    my $t = _peek();
    unless ($SIMPLE_SELECTOR_START{$t->{type}}) {
        die sprintf(
            "CodingAdventures::LatticeParser: Expected simple_selector, "
          . "got %s ('%s') at line %d col %d",
            $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _node('simple_selector', [_leaf(_advance())]);
}

# --- _parse_subclass_selector() -----------------------------------------------
#
# Grammar: subclass_selector = class_selector | id_selector | placeholder_selector
#                            | attribute_selector | pseudo_class | pseudo_element

sub _parse_subclass_selector {
    my $t = _peek();

    if ($t->{type} eq 'DOT') {
        return _parse_class_selector();
    }
    if ($t->{type} eq 'HASH') {
        return _parse_id_selector();
    }
    if ($t->{type} eq 'PLACEHOLDER') {
        return _parse_placeholder_selector();
    }
    if ($t->{type} eq 'LBRACKET') {
        return _parse_attribute_selector();
    }
    if ($t->{type} eq 'COLON_COLON') {
        return _parse_pseudo_element();
    }
    if ($t->{type} eq 'COLON') {
        return _parse_pseudo_class();
    }

    die sprintf(
        "CodingAdventures::LatticeParser: Expected subclass_selector, "
      . "got %s ('%s') at line %d col %d",
        $t->{type}, $t->{value}, $t->{line}, $t->{col}
    );
}

# --- _parse_class_selector() --------------------------------------------------
# Grammar: class_selector = DOT IDENT

sub _parse_class_selector {
    return _node('class_selector', [
        _leaf(_expect('DOT')),
        _leaf(_expect('IDENT')),
    ]);
}

# --- _parse_id_selector() -----------------------------------------------------
# Grammar: id_selector = HASH

sub _parse_id_selector {
    return _node('id_selector', [_leaf(_expect('HASH'))]);
}

# --- _parse_placeholder_selector() --------------------------------------------
# Grammar: placeholder_selector = PLACEHOLDER
# Placeholder selectors are used with @extend: %flex-center

sub _parse_placeholder_selector {
    return _node('placeholder_selector', [_leaf(_expect('PLACEHOLDER'))]);
}

# --- _parse_attribute_selector() ----------------------------------------------
#
# Grammar: attribute_selector = LBRACKET IDENT [attr_matcher attr_value [IDENT]] RBRACKET
#
# Attribute selectors target elements by their HTML attributes:
#   [type]                  — has the attribute
#   [type="text"]           — exact match
#   [class~="nav"]          — word in space-separated list
#   [href^="https"]         — starts with value
#   [lang|="en"]            — exactly "en" or starts with "en-"

sub _parse_attribute_selector {
    my @children;

    push @children, _leaf(_expect('LBRACKET'));
    push @children, _leaf(_expect('IDENT'));

    my %ATTR_MATCHERS = map { $_ => 1 } qw(
        EQUALS TILDE_EQUALS PIPE_EQUALS CARET_EQUALS DOLLAR_EQUALS STAR_EQUALS
    );

    if ($ATTR_MATCHERS{_peek()->{type}}) {
        push @children, _node('attr_matcher', [_leaf(_advance())]);

        my $av = _peek();
        if ($av->{type} eq 'IDENT' || $av->{type} eq 'STRING') {
            push @children, _node('attr_value', [_leaf(_advance())]);
        }

        # Optional case-sensitivity flag (IDENT "i" or "s")
        if (_peek()->{type} eq 'IDENT') {
            push @children, _leaf(_advance());
        }
    }

    push @children, _leaf(_expect('RBRACKET'));

    return _node('attribute_selector', \@children);
}

# --- _parse_pseudo_class() ----------------------------------------------------
#
# Grammar: pseudo_class = COLON FUNCTION pseudo_class_args RPAREN | COLON IDENT
#
# Pseudo-classes select elements based on their state or position:
#   :hover, :focus, :first-child, :nth-child(2n+1)

sub _parse_pseudo_class {
    my @children;

    push @children, _leaf(_expect('COLON'));

    if (_peek()->{type} eq 'FUNCTION') {
        push @children, _leaf(_advance());  # FUNCTION
        push @children, _parse_pseudo_class_args();
        push @children, _leaf(_expect('RPAREN'));
    }
    else {
        push @children, _leaf(_expect('IDENT'));
    }

    return _node('pseudo_class', \@children);
}

# --- _parse_pseudo_class_args() -----------------------------------------------
#
# Parses argument tokens for functional pseudo-classes like :nth-child(2n+1),
# :not(.visible), :is(h1, h2, h3).

sub _parse_pseudo_class_args {
    my @children;

    while (_peek()->{type} ne 'RPAREN' && _peek()->{type} ne 'EOF') {
        my $t = _peek();

        if ($t->{type} eq 'FUNCTION') {
            push @children, _leaf(_advance());
            push @children, _parse_pseudo_class_args();
            push @children, _leaf(_expect('RPAREN'));
        }
        elsif ($t->{type} eq 'LBRACKET') {
            push @children, _leaf(_advance());  # LBRACKET
            push @children, _parse_pseudo_class_args();
            push @children, _leaf(_expect('RBRACKET'));
        }
        else {
            push @children, _leaf(_advance());
        }
    }

    return _node('pseudo_class_args', \@children);
}

# --- _parse_pseudo_element() --------------------------------------------------
#
# Grammar: pseudo_element = COLON_COLON IDENT
#
# Pseudo-elements select part of an element: ::before, ::after, ::first-line.

sub _parse_pseudo_element {
    return _node('pseudo_element', [
        _leaf(_expect('COLON_COLON')),
        _leaf(_expect('IDENT')),
    ]);
}

# ============================================================================
# CSS: Declaration blocks
# ============================================================================

# --- _parse_block() -----------------------------------------------------------
#
# Grammar: block = LBRACE block_contents RBRACE

sub _parse_block {
    my @children;

    push @children, _leaf(_expect('LBRACE'));
    push @children, _parse_block_contents();
    push @children, _leaf(_expect('RBRACE'));

    return _node('block', \@children);
}

# --- _parse_block_contents() --------------------------------------------------
#
# Grammar: block_contents = { block_item }

sub _parse_block_contents {
    my @children;

    while (_peek()->{type} ne 'RBRACE' && _peek()->{type} ne 'EOF') {
        push @children, _parse_block_item();
    }

    return _node('block_contents', \@children);
}

# --- _parse_block_item() ------------------------------------------------------
#
# Grammar: block_item = lattice_block_item | at_rule | declaration_or_nested
#
# Dispatch inside a declaration block:
#   VARIABLE            → variable_declaration (lattice_block_item)
#   @include            → include_directive (lattice_block_item)
#   @if/@for/@each/@while → lattice_control (lattice_block_item)
#   @content            → content_directive (lattice_block_item)
#   @extend             → extend_directive (lattice_block_item)
#   @at-root            → at_root_directive (lattice_block_item)
#   AT_KEYWORD (other)  → at_rule
#   IDENT/STAR/./etc    → declaration_or_nested

sub _parse_block_item {
    my $t = _peek();

    if ($t->{type} eq 'VARIABLE') {
        return _node('block_item', [_node('lattice_block_item', [_parse_variable_declaration()])]);
    }

    if ($t->{type} eq 'AT_KEYWORD') {
        my $kw = $t->{value};

        if ($kw eq '@include') {
            return _node('block_item', [_node('lattice_block_item', [_parse_include_directive()])]);
        }
        if ($kw eq '@if') {
            return _node('block_item', [_node('lattice_block_item', [_node('lattice_control', [_parse_if_directive()])])]);
        }
        if ($kw eq '@for') {
            return _node('block_item', [_node('lattice_block_item', [_node('lattice_control', [_parse_for_directive()])])]);
        }
        if ($kw eq '@each') {
            return _node('block_item', [_node('lattice_block_item', [_node('lattice_control', [_parse_each_directive()])])]);
        }
        if ($kw eq '@while') {
            return _node('block_item', [_node('lattice_block_item', [_node('lattice_control', [_parse_while_directive()])])]);
        }
        if ($kw eq '@content') {
            return _node('block_item', [_node('lattice_block_item', [_parse_content_directive()])]);
        }
        if ($kw eq '@extend') {
            return _node('block_item', [_node('lattice_block_item', [_parse_extend_directive()])]);
        }
        if ($kw eq '@at-root') {
            return _node('block_item', [_node('lattice_block_item', [_parse_at_root_directive()])]);
        }

        # Standard CSS at-rule inside a block (e.g. @media nested inside .parent)
        return _node('block_item', [_parse_at_rule()]);
    }

    # Must be a declaration or a nested qualified rule
    return _node('block_item', [_parse_declaration_or_nested()]);
}

# --- _parse_content_directive() -----------------------------------------------
#
# Grammar: content_directive = "@content" SEMICOLON
#
# @content emits the caller-supplied block inside a mixin that accepts one.
# Example:
#   @mixin respond-to($bp) { @media (min-width: $bp) { @content; } }

sub _parse_content_directive {
    my @children;
    push @children, _leaf(_expect_kw('@content'));
    push @children, _leaf(_expect('SEMICOLON'));
    return _node('content_directive', \@children);
}

# --- _parse_extend_directive() ------------------------------------------------
#
# Grammar: extend_directive = "@extend" selector_list SEMICOLON
#
# @extend copies the extended rule's declarations into the extending selector.
# Placeholder selectors (%name) are the most common target:
#   %flex-center { display: flex; align-items: center; }
#   .hero { @extend %flex-center; }

sub _parse_extend_directive {
    my @children;
    push @children, _leaf(_expect_kw('@extend'));
    push @children, _parse_selector_list();
    push @children, _leaf(_expect('SEMICOLON'));
    return _node('extend_directive', \@children);
}

# --- _parse_at_root_directive() -----------------------------------------------
#
# Grammar: at_root_directive = "@at-root" (selector_list block | block)
#
# @at-root emits its contents at stylesheet root, escaping all nesting context.
#   .parent { @at-root .child { color: red; } }  →  .child { color: red; }

sub _parse_at_root_directive {
    my @children;

    push @children, _leaf(_expect_kw('@at-root'));

    if (_peek()->{type} eq 'LBRACE') {
        # bare block (no selector)
        push @children, _parse_block();
    }
    else {
        # selector_list followed by block
        push @children, _parse_selector_list();
        push @children, _parse_block();
    }

    return _node('at_root_directive', \@children);
}

# --- _parse_declaration_or_nested() -------------------------------------------
#
# Grammar: declaration_or_nested = declaration | qualified_rule
#
# Disambiguation: both start with an IDENT (property name) or selector token.
#
# Heuristic: if the token after the first IDENT is COLON, treat it as a
# declaration.  If it is anything selector-like, treat it as a nested rule.
#
# Edge case: CUSTOM_PROPERTY (--var) always starts a declaration.
# Edge case: class/id/attribute/pseudo selectors start a nested rule directly.

sub _parse_declaration_or_nested {
    my $t = _peek();

    # CUSTOM_PROPERTY is always a declaration property
    if ($t->{type} eq 'CUSTOM_PROPERTY') {
        return _node('declaration_or_nested', [_parse_declaration()]);
    }

    # Subclass selectors (., #, [, :, ::, %) start nested rules directly
    if ($SUBCLASS_SELECTOR_START{$t->{type}}) {
        return _node('declaration_or_nested', [_parse_qualified_rule()]);
    }

    # IDENT: peek at next token to distinguish property:value from selector{
    if ($t->{type} eq 'IDENT') {
        my $next = _peek_at(1);

        if ($next->{type} eq 'COLON') {
            return _node('declaration_or_nested', [_parse_declaration()]);
        }
        else {
            return _node('declaration_or_nested', [_parse_qualified_rule()]);
        }
    }

    # STAR (universal selector) and AMPERSAND (&) are selectors
    if ($t->{type} eq 'STAR' || $t->{type} eq 'AMPERSAND') {
        return _node('declaration_or_nested', [_parse_qualified_rule()]);
    }

    die sprintf(
        "CodingAdventures::LatticeParser: Expected declaration or nested rule, "
      . "got %s ('%s') at line %d col %d",
        $t->{type}, $t->{value}, $t->{line}, $t->{col}
    );
}

# --- _parse_declaration() -----------------------------------------------------
#
# Grammar: declaration = property COLON value_list [priority] SEMICOLON
#                      | property COLON block
#
# The second form ("property COLON block") handles nested declarations where
# the parent property name is implicitly prepended with a hyphen:
#   font: { size: 14px; weight: bold; }
#   → font-size: 14px; font-weight: bold;

sub _parse_declaration {
    my @children;

    push @children, _parse_property();
    push @children, _leaf(_expect('COLON'));

    # Disambiguate: block form vs value_list form
    if (_peek()->{type} eq 'LBRACE') {
        push @children, _parse_block();
    }
    else {
        push @children, _parse_value_list();

        # Optional !important priority
        if (_peek()->{type} eq 'BANG') {
            push @children, _parse_priority();
        }

        push @children, _leaf(_expect('SEMICOLON'));
    }

    return _node('declaration', \@children);
}

# --- _parse_property() --------------------------------------------------------
#
# Grammar: property = IDENT | CUSTOM_PROPERTY

sub _parse_property {
    my $t = _peek();
    unless ($t->{type} eq 'IDENT' || $t->{type} eq 'CUSTOM_PROPERTY') {
        die sprintf(
            "CodingAdventures::LatticeParser: Expected property name (IDENT or CUSTOM_PROPERTY), "
          . "got %s ('%s') at line %d col %d",
            $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _node('property', [_leaf(_advance())]);
}

# --- _parse_priority() --------------------------------------------------------
#
# Grammar: priority = BANG "important"
#
# !important in CSS gives a declaration the highest specificity weight.

sub _parse_priority {
    my @children;
    push @children, _leaf(_expect('BANG'));
    push @children, _leaf(_expect_ident('important'));
    return _node('priority', \@children);
}

# ============================================================================
# CSS: Values
# ============================================================================

# --- _parse_value_list() ------------------------------------------------------
#
# Grammar: value_list = value { value }
#
# A sequence of one or more values.  This includes all scalar types, function
# calls, variable references, and separators like COMMA and SLASH.
#
# Note: COMMA is a valid value token in CSS (e.g. "font-family: A, B, C" or
# "background: linear-gradient(to right, red, blue)"), so we don't stop at COMMA.
# We stop at SEMICOLON, RBRACE, BANG, and EOF.

sub _parse_value_list {
    my @children;

    my $first = _peek();
    unless ($VALUE_TYPES{$first->{type}} || $first->{type} eq 'FUNCTION'
                || $first->{type} eq 'URL_TOKEN' || $first->{type} eq 'LPAREN') {
        die sprintf(
            "CodingAdventures::LatticeParser: Expected value, "
          . "got %s ('%s') at line %d col %d",
            $first->{type}, $first->{value}, $first->{line}, $first->{col}
        );
    }

    while (1) {
        my $t = _peek();
        last if $t->{type} eq 'SEMICOLON' || $t->{type} eq 'RBRACE'
             || $t->{type} eq 'BANG'      || $t->{type} eq 'BANG_DEFAULT'
             || $t->{type} eq 'BANG_GLOBAL' || $t->{type} eq 'EOF'
             || $t->{type} eq 'RPAREN';

        if ($t->{type} eq 'FUNCTION') {
            push @children, _node('value', [_parse_function_call()]);
        }
        elsif ($t->{type} eq 'URL_TOKEN') {
            push @children, _node('value', [_parse_function_call()]);
        }
        elsif ($t->{type} eq 'LPAREN') {
            # map_literal or parenthesized expression in a value
            my $t1 = _peek_at(1);
            my $t2 = _peek_at(2);
            if (($t1->{type} eq 'IDENT' || $t1->{type} eq 'STRING')
                    && $t2->{type} eq 'COLON') {
                push @children, _node('value', [_parse_map_literal()]);
            }
            else {
                # Plain parenthesized group — consume as-is
                push @children, _node('value', [_leaf(_advance())]);
            }
        }
        elsif ($VALUE_TYPES{$t->{type}}) {
            push @children, _node('value', [_leaf(_advance())]);
        }
        else {
            last;
        }
    }

    return _node('value_list', \@children);
}

# --- _parse_value_list_no_comma() ---------------------------------------------
#
# Like _parse_value_list but stops at COMMA as well.  Used for include_arg
# and mixin_value_list so that commas separating arguments are not consumed.

sub _parse_value_list_no_comma {
    my @children;

    while (1) {
        my $t = _peek();
        last if $t->{type} eq 'SEMICOLON' || $t->{type} eq 'RBRACE'
             || $t->{type} eq 'COMMA'     || $t->{type} eq 'RPAREN'
             || $t->{type} eq 'BANG'      || $t->{type} eq 'EOF';

        if ($t->{type} eq 'FUNCTION') {
            push @children, _node('value', [_parse_function_call()]);
        }
        elsif ($t->{type} eq 'URL_TOKEN') {
            push @children, _node('value', [_leaf(_advance())]);
        }
        elsif ($VALUE_TYPES{$t->{type}}) {
            push @children, _node('value', [_leaf(_advance())]);
        }
        else {
            last;
        }
    }

    return _node('value_list', \@children);
}

# --- _parse_single_value() ----------------------------------------------------
#
# Parse exactly one value token (used in each_list).

sub _parse_single_value {
    my $t = _peek();

    if ($t->{type} eq 'FUNCTION') {
        return _node('value', [_parse_function_call()]);
    }
    if ($VALUE_TYPES{$t->{type}}) {
        return _node('value', [_leaf(_advance())]);
    }

    die sprintf(
        "CodingAdventures::LatticeParser: Expected a value, "
      . "got %s ('%s') at line %d col %d",
        $t->{type}, $t->{value}, $t->{line}, $t->{col}
    );
}

# --- _parse_function_call() ---------------------------------------------------
#
# Grammar: function_call = FUNCTION function_args RPAREN | URL_TOKEN
#
# CSS/Lattice function calls: color(red), calc(100% - 2px), url("img.png").
# The FUNCTION token includes the function name and opening paren: "calc(".
# URL_TOKEN is a special token for url("...") shorthand.

sub _parse_function_call {
    my @children;
    my $t = _peek();

    if ($t->{type} eq 'URL_TOKEN') {
        return _node('function_call', [_leaf(_advance())]);
    }

    push @children, _leaf(_expect('FUNCTION'));
    push @children, _parse_function_args();
    push @children, _leaf(_expect('RPAREN'));

    return _node('function_call', \@children);
}

# --- _parse_function_args() ---------------------------------------------------
#
# Grammar: function_args = { function_arg }
#
# Arguments inside a function call.  Stops at RPAREN or EOF.

sub _parse_function_args {
    my @children;

    while (_peek()->{type} ne 'RPAREN' && _peek()->{type} ne 'EOF') {
        my $t = _peek();

        if ($t->{type} eq 'FUNCTION') {
            # Nested function call (e.g. hsl(calc(180 + $hue), 50%, 50%))
            push @children, _node('function_arg', [_parse_function_call()]);
        }
        elsif ($FUNCTION_ARG_TYPES{$t->{type}}) {
            push @children, _node('function_arg', [_leaf(_advance())]);
        }
        else {
            last;
        }
    }

    return _node('function_args', \@children);
}

# ============================================================================
# Helper predicates and utilities
# ============================================================================

# --- _is_selector_start($t) ---------------------------------------------------
#
# Returns true if $t can start a compound_selector.

sub _is_selector_start {
    my ($t) = @_;
    return $SIMPLE_SELECTOR_START{$t->{type}}
        || $SUBCLASS_SELECTOR_START{$t->{type}};
}

# --- _expect_kw($keyword) -----------------------------------------------------
#
# Consume the current token if it is AT_KEYWORD with the given value.
# Dies with a helpful message otherwise.

sub _expect_kw {
    my ($keyword) = @_;
    my $t = _peek();
    unless ($t->{type} eq 'AT_KEYWORD' && $t->{value} eq $keyword) {
        die sprintf(
            "CodingAdventures::LatticeParser: Expected %s, "
          . "got %s ('%s') at line %d col %d",
            $keyword, $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _advance();
}

# --- _expect_ident($value) ----------------------------------------------------
#
# Consume the current token if it is IDENT with the given text value.
# Used for contextual keywords: "from", "through", "to", "in", "as", "important".

sub _expect_ident {
    my ($value) = @_;
    my $t = _peek();
    unless ($t->{type} eq 'IDENT' && $t->{value} eq $value) {
        die sprintf(
            "CodingAdventures::LatticeParser: Expected identifier '%s', "
          . "got %s ('%s') at line %d col %d",
            $value, $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _advance();
}

1;

__END__

=head1 NAME

CodingAdventures::LatticeParser - Hand-written recursive-descent Lattice CSS parser

=head1 SYNOPSIS

    use CodingAdventures::LatticeParser;

    my $ast = CodingAdventures::LatticeParser->parse(<<'LATTICE');
    $primary: #4a90d9;

    @mixin center {
      display: flex;
      align-items: center;
    }

    .hero {
      @include center;
      color: $primary;
    }
    LATTICE

    print $ast->rule_name;  # "stylesheet"

=head1 DESCRIPTION

A hand-written recursive-descent parser for Lattice, a CSS superset language
that adds variables, mixins, control flow, functions, modules, and nesting
to plain CSS.

Tokenizes source text using C<CodingAdventures::LatticeLexer> and constructs
an AST using C<CodingAdventures::LatticeParser::ASTNode>.

Every valid CSS file is valid Lattice.  Lattice constructs are tried first
during dispatch so they take precedence over CSS at-rules.

=head1 METHODS

=head2 parse($source)

Parse a Lattice/CSS string.  Returns the root C<ASTNode> with
C<rule_name == "stylesheet">.  Dies on lexer or parser errors.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
