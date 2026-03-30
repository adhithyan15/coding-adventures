package CodingAdventures::CssParser;

# ============================================================================
# CodingAdventures::CssParser — Hand-written recursive-descent CSS3 parser
# ============================================================================
#
# This module parses CSS3 source text into an Abstract Syntax Tree (AST).
# It uses a hand-written recursive-descent approach because the Perl
# CodingAdventures::GrammarTools module only provides `parse_token_grammar`,
# not `parse_parser_grammar`. There is no grammar-driven GrammarParser in
# the Perl layer.
#
# # CSS grammar (implemented here)
# ==================================
#
#   stylesheet         = { rule }
#   rule               = at_rule | qualified_rule
#   at_rule            = AT_KEYWORD at_prelude ( SEMICOLON | block )
#   at_prelude         = { at_prelude_token }
#   qualified_rule     = selector_list block
#   selector_list      = complex_selector { COMMA complex_selector }
#   complex_selector   = compound_selector { [ combinator ] compound_selector }
#   combinator         = GREATER | PLUS | TILDE
#   compound_selector  = simple_selector { subclass_selector }
#                      | subclass_selector { subclass_selector }
#   simple_selector    = IDENT | STAR | AMPERSAND
#   subclass_selector  = class_selector | id_selector | attribute_selector
#                      | pseudo_class | pseudo_element
#   class_selector     = DOT IDENT
#   id_selector        = HASH
#   attribute_selector = LBRACKET IDENT [ attr_matcher attr_value [ IDENT ] ] RBRACKET
#   attr_matcher       = EQUALS | TILDE_EQUALS | PIPE_EQUALS | CARET_EQUALS
#                      | DOLLAR_EQUALS | STAR_EQUALS
#   attr_value         = IDENT | STRING
#   pseudo_class       = COLON FUNCTION pseudo_class_args RPAREN
#                      | COLON IDENT
#   pseudo_class_args  = { any-token except RPAREN }
#   pseudo_element     = COLON_COLON IDENT
#   block              = LBRACE block_contents RBRACE
#   block_contents     = { block_item }
#   block_item         = at_rule | declaration_or_nested
#   declaration_or_nested = declaration | qualified_rule
#   declaration        = property COLON value_list [ priority ] SEMICOLON
#   property           = IDENT | CUSTOM_PROPERTY
#   priority           = BANG IDENT("important")
#   value_list         = value { value }
#   value              = DIMENSION | PERCENTAGE | NUMBER | STRING | IDENT
#                      | HASH | CUSTOM_PROPERTY | UNICODE_RANGE
#                      | function_call | SLASH | COMMA | PLUS | MINUS
#   function_call      = FUNCTION function_args RPAREN | URL_TOKEN
#   function_args      = { function_arg }
#   function_arg       = DIMENSION | PERCENTAGE | NUMBER | STRING | IDENT
#                      | HASH | CUSTOM_PROPERTY | COMMA | SLASH | PLUS | MINUS | STAR
#                      | FUNCTION function_args RPAREN
#
# # CSS parsing challenges
# ========================
#
# 1. **Declaration vs. nested rule disambiguation** — Both can start with IDENT.
#    A declaration:    `color: red;`
#    A nested rule:    `div { color: red; }`
#    Strategy: try declaration first. If the next token after IDENT is COLON,
#    it's a declaration. Otherwise, it's a qualified_rule (selector + block).
#    The `_try_declaration()` method uses backtracking to handle this.
#
# 2. **Compound tokens from the lexer** — By the time tokens reach the parser,
#    the lexer has already handled compound tokens: "10px" is already DIMENSION,
#    "rgba(" is already FUNCTION. The parser just consumes them.
#
# 3. **Flexible at-rule preludes** — @media can have complex preludes:
#    `@media screen and (min-width: 768px) { }`. We consume the prelude as a
#    sequence of "any token except { or ;" — then decide if the at-rule ends
#    with a SEMICOLON or a block.
#
# 4. **Selector list comma** — "h1, h2, h3" is a selector_list. Commas
#    separate selectors. But commas also appear in function args. We use
#    context to distinguish: top-level commas separate selectors; commas
#    inside FUNCTION ... RPAREN are function arguments.
#
# 5. **Nested rules (CSS Nesting)** — `.parent { & .child { } }` allows
#    qualified_rule inside a block. The block_item production handles this
#    by trying declaration first, then falling back to qualified_rule.
#
# # Parse state
#
# Two package-level variables hold the current parse state:
#   $tokens_ref — arrayref from CodingAdventures::CssLexer->tokenize
#   $pos        — 0-based current index
#
# The parser is not re-entrant (package-level state), but CSS parsing is
# always synchronous so this is fine in practice.

use strict;
use warnings;

use CodingAdventures::CssLexer;
use CodingAdventures::CssParser::ASTNode;

our $VERSION = '0.01';

# Package-level parse state
my ($tokens_ref, $pos);

# ============================================================================
# Token type sets
# ============================================================================
#
# These sets drive the parsing decisions. Perl hashes are ideal for O(1)
# membership tests.

# Value tokens — can appear as a single value in a declaration's value list.
my %VALUE_TYPES = map { $_ => 1 } qw(
    DIMENSION PERCENTAGE NUMBER STRING IDENT HASH
    CUSTOM_PROPERTY UNICODE_RANGE SLASH COMMA PLUS MINUS
);

# Function arg tokens — can appear inside a function's argument list.
# Includes FUNCTION (for nested functions like calc(var(--x))).
my %FUNCTION_ARG_TYPES = map { $_ => 1 } qw(
    DIMENSION PERCENTAGE NUMBER STRING IDENT HASH
    CUSTOM_PROPERTY COMMA SLASH PLUS MINUS STAR FUNCTION
);

# At-prelude tokens — can appear in an at-rule prelude.
# Anything except LBRACE and SEMICOLON (which terminate the prelude).
my %AT_PRELUDE_TYPES = map { $_ => 1 } qw(
    IDENT STRING NUMBER DIMENSION PERCENTAGE HASH CUSTOM_PROPERTY
    UNICODE_RANGE FUNCTION URL_TOKEN
    COLON COMMA SLASH DOT STAR PLUS MINUS GREATER TILDE PIPE
    EQUALS AMPERSAND CDO CDC LPAREN RPAREN LBRACKET RBRACKET
    COLON_COLON TILDE_EQUALS PIPE_EQUALS CARET_EQUALS
    DOLLAR_EQUALS STAR_EQUALS BANG
);

# Attribute matcher operators
my %ATTR_MATCHERS = map { $_ => 1 } qw(
    EQUALS TILDE_EQUALS PIPE_EQUALS CARET_EQUALS DOLLAR_EQUALS STAR_EQUALS
);

# Combinator tokens
my %COMBINATORS = map { $_ => 1 } qw(GREATER PLUS TILDE);

# ============================================================================
# Public API
# ============================================================================

# --- parse($class, $source) ---------------------------------------------------
#
# Parse a CSS3 source string and return the root ASTNode.
#
# @param  $source  string  The CSS text to parse.
# @return ASTNode          Root with rule_name "stylesheet".
# @die                     On any lexer or parser error.

sub parse {
    my ($class, $source) = @_;

    my $toks = CodingAdventures::CssLexer->tokenize($source);
    $tokens_ref = $toks;
    $pos = 0;

    my $ast = _parse_stylesheet();

    # Verify we consumed everything
    my $t = _peek();
    if ($t->{type} ne 'EOF') {
        die sprintf(
            "CodingAdventures::CssParser: trailing content at line %d col %d: "
          . "unexpected %s ('%s')",
            $t->{line}, $t->{col}, $t->{type}, $t->{value}
        );
    }

    return $ast;
}

# ============================================================================
# Internal helpers
# ============================================================================

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
            "CodingAdventures::CssParser: Expected %s, got %s ('%s') "
          . "at line %d col %d",
            $type, $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _advance();
}

sub _node {
    my ($rule, $children) = @_;
    return CodingAdventures::CssParser::ASTNode->new(
        rule_name => $rule,
        children  => $children,
        is_leaf   => 0,
    );
}

sub _leaf {
    my ($tok) = @_;
    return CodingAdventures::CssParser::ASTNode->new(
        rule_name => 'token',
        children  => [],
        is_leaf   => 1,
        token     => $tok,
    );
}

# ============================================================================
# Recursive descent productions
# ============================================================================

# --- _parse_stylesheet() -------------------------------------------------------
#
# Grammar: stylesheet = { rule }
#
# A CSS stylesheet is a sequence of rules. Each rule is either an at-rule
# (@media, @import, etc.) or a qualified rule (selector + declaration block).
# We stop at EOF.

sub _parse_stylesheet {
    my @children;

    while (_peek()->{type} ne 'EOF') {
        my $rule = _parse_rule();
        push @children, $rule;
    }

    return _node('stylesheet', \@children);
}

# --- _parse_rule() --------------------------------------------------------------
#
# Grammar: rule = at_rule | qualified_rule
#
# Disambiguation:
#   AT_KEYWORD  → at_rule
#   anything else → qualified_rule

sub _parse_rule {
    my $t = _peek();

    if ($t->{type} eq 'AT_KEYWORD') {
        return _node('rule', [_parse_at_rule()]);
    }

    return _node('rule', [_parse_qualified_rule()]);
}

# --- _parse_at_rule() -----------------------------------------------------------
#
# Grammar: at_rule = AT_KEYWORD at_prelude ( SEMICOLON | block )
#
# Examples:
#   @import "file.css";         — prelude + semicolon
#   @charset "UTF-8";           — prelude + semicolon
#   @media screen { }           — prelude + block
#   @keyframes name { }         — prelude + block
#   @font-face { }              — empty prelude + block
#
# The prelude is everything between the AT_KEYWORD and the terminator.
# We stop collecting prelude tokens when we see { or ;.

sub _parse_at_rule {
    my @children;

    # AT_KEYWORD token (@media, @import, etc.)
    push @children, _leaf(_expect('AT_KEYWORD'));

    # Prelude: zero or more tokens before { or ;
    push @children, _parse_at_prelude();

    # Terminator: SEMICOLON for simple at-rules, block for complex ones
    my $t = _peek();
    if ($t->{type} eq 'SEMICOLON') {
        push @children, _leaf(_advance());
    } elsif ($t->{type} eq 'LBRACE') {
        push @children, _parse_block();
    } else {
        die sprintf(
            "CodingAdventures::CssParser: at-rule expected { or ; at "
          . "line %d col %d, got %s ('%s')",
            $t->{line}, $t->{col}, $t->{type}, $t->{value}
        );
    }

    return _node('at_rule', \@children);
}

# --- _parse_at_prelude() --------------------------------------------------------
#
# Grammar: at_prelude = { at_prelude_token }
#
# Consumes tokens until LBRACE, SEMICOLON, or EOF.
# Parenthesized groups inside the prelude (like `(min-width: 768px)`)
# are consumed as a unit — we track paren depth to avoid stopping early.

sub _parse_at_prelude {
    my @children;
    my $depth = 0;  # paren depth tracker

    while (1) {
        my $t = _peek();

        # Stop at LBRACE or SEMICOLON (unless inside parens)
        last if $depth == 0 && ($t->{type} eq 'LBRACE'
                             || $t->{type} eq 'SEMICOLON'
                             || $t->{type} eq 'EOF');

        # Track parenthesis depth for (min-width: 768px) style preludes
        if ($t->{type} eq 'LPAREN') {
            $depth++;
        } elsif ($t->{type} eq 'RPAREN') {
            $depth--;
            # Allow going to depth -1 briefly (RPAREN closes FUNCTION token context)
            $depth = 0 if $depth < 0;
        }

        # FUNCTION token includes the opening paren, so increment depth
        if ($t->{type} eq 'FUNCTION') {
            $depth++;
        }

        push @children, _leaf(_advance());
    }

    return _node('at_prelude', \@children);
}

# --- _parse_qualified_rule() ----------------------------------------------------
#
# Grammar: qualified_rule = selector_list block
#
# A qualified rule is the most common CSS construct:
#   h1 { color: red; }
#   .class > p:hover { font-size: 16px; }

sub _parse_qualified_rule {
    my @children;

    push @children, _parse_selector_list();
    push @children, _parse_block();

    return _node('qualified_rule', \@children);
}

# --- _parse_selector_list() -------------------------------------------------------
#
# Grammar: selector_list = complex_selector { COMMA complex_selector }
#
# Comma-separated list: h1, h2, h3 { }
# The comma here is a selector separator, not a value separator.

sub _parse_selector_list {
    my @children;

    push @children, _parse_complex_selector();

    while (_peek()->{type} eq 'COMMA') {
        push @children, _leaf(_advance());   # COMMA
        push @children, _parse_complex_selector();
    }

    return _node('selector_list', \@children);
}

# --- _parse_complex_selector() ----------------------------------------------------
#
# Grammar: complex_selector = compound_selector { [ combinator ] compound_selector }
#
# Complex selectors join compound selectors with combinators:
#   div > p + span ~ a
#
# The descendant combinator (space) is implicit — since the lexer already
# consumed whitespace, adjacent compound selectors with no explicit combinator
# represent the descendant relationship. We handle this by checking if the
# next token starts another compound selector (without an explicit combinator).

sub _parse_complex_selector {
    my @children;

    push @children, _parse_compound_selector();

    # Keep consuming if we see a combinator OR if the next token can start
    # a compound selector (for the implicit descendant combinator case).
    while (1) {
        my $t = _peek();

        # Explicit combinator: >, +, ~
        if ($COMBINATORS{$t->{type}}) {
            push @children, _leaf(_advance());    # combinator
            push @children, _parse_compound_selector();
            next;
        }

        # If the next token can start a compound selector, it's an implicit
        # descendant combinator (the whitespace was consumed by the lexer).
        if (_can_start_compound_selector($t)) {
            push @children, _parse_compound_selector();
            next;
        }

        last;
    }

    return _node('complex_selector', \@children);
}

# --- _can_start_compound_selector($tok) ------------------------------------------
#
# Returns true if the given token can begin a compound selector.
# Used to detect the implicit descendant combinator.
#
# A compound selector can start with:
#   IDENT   — type selector (h1, div, p)
#   STAR    — universal selector (*)
#   AMPERSAND — nesting selector (&)
#   DOT     — class selector (.class)
#   HASH    — ID selector (#id)
#   LBRACKET — attribute selector ([attr])
#   COLON   — pseudo-class (:hover)
#   COLON_COLON — pseudo-element (::before)

sub _can_start_compound_selector {
    my ($t) = @_;
    return $t->{type} =~ /^(IDENT|STAR|AMPERSAND|DOT|HASH|LBRACKET|COLON|COLON_COLON)$/;
}

# --- _parse_compound_selector() ---------------------------------------------------
#
# Grammar: compound_selector = simple_selector { subclass_selector }
#                             | subclass_selector { subclass_selector }
#
# A compound selector is a chain of simple and subclass selectors with no
# whitespace (whitespace was already consumed by the lexer):
#   div.class#id[attr]:hover::before
#
# We try simple_selector first. If that fails, we try subclass_selector.
# If neither succeeds, we error.

sub _parse_compound_selector {
    my @children;
    my $t = _peek();

    # Try simple selector (IDENT, STAR, AMPERSAND)
    if ($t->{type} eq 'IDENT' || $t->{type} eq 'STAR' || $t->{type} eq 'AMPERSAND') {
        push @children, _parse_simple_selector();
        # Followed by zero or more subclass selectors (no whitespace in between)
        while (_can_start_subclass_selector(_peek())) {
            push @children, _parse_subclass_selector();
        }
    }
    # Otherwise must start with a subclass selector (.class, #id, [attr], :, ::)
    elsif (_can_start_subclass_selector($t)) {
        push @children, _parse_subclass_selector();
        while (_can_start_subclass_selector(_peek())) {
            push @children, _parse_subclass_selector();
        }
    }
    else {
        die sprintf(
            "CodingAdventures::CssParser: Expected selector at "
          . "line %d col %d, got %s ('%s')",
            $t->{line}, $t->{col}, $t->{type}, $t->{value}
        );
    }

    return _node('compound_selector', \@children);
}

# --- _can_start_subclass_selector($tok) ------------------------------------------
#
# Returns true if the token can begin a subclass selector.
# Subclass selectors: .class, #id, [attr], :pseudo-class, ::pseudo-element

sub _can_start_subclass_selector {
    my ($t) = @_;
    return $t->{type} =~ /^(DOT|HASH|LBRACKET|COLON|COLON_COLON)$/;
}

# --- _parse_simple_selector() -------------------------------------------------------
#
# Grammar: simple_selector = IDENT | STAR | AMPERSAND

sub _parse_simple_selector {
    my $t = _peek();
    unless ($t->{type} =~ /^(IDENT|STAR|AMPERSAND)$/) {
        die sprintf(
            "CodingAdventures::CssParser: Expected simple selector (IDENT/*/&) "
          . "at line %d col %d, got %s ('%s')",
            $t->{line}, $t->{col}, $t->{type}, $t->{value}
        );
    }
    return _node('simple_selector', [_leaf(_advance())]);
}

# --- _parse_subclass_selector() ------------------------------------------------------
#
# Grammar: subclass_selector = class_selector | id_selector
#                             | attribute_selector | pseudo_class
#                             | pseudo_element

sub _parse_subclass_selector {
    my $t = _peek();

    if ($t->{type} eq 'DOT') {
        return _node('subclass_selector', [_parse_class_selector()]);
    }
    if ($t->{type} eq 'HASH') {
        return _node('subclass_selector', [_parse_id_selector()]);
    }
    if ($t->{type} eq 'LBRACKET') {
        return _node('subclass_selector', [_parse_attribute_selector()]);
    }
    if ($t->{type} eq 'COLON_COLON') {
        return _node('subclass_selector', [_parse_pseudo_element()]);
    }
    if ($t->{type} eq 'COLON') {
        return _node('subclass_selector', [_parse_pseudo_class()]);
    }

    die sprintf(
        "CodingAdventures::CssParser: Expected subclass selector at "
      . "line %d col %d, got %s ('%s')",
        $t->{line}, $t->{col}, $t->{type}, $t->{value}
    );
}

# --- _parse_class_selector() -------------------------------------------------------
#
# Grammar: class_selector = DOT IDENT
# Example: .active, .btn-primary, .main-content

sub _parse_class_selector {
    my @children;
    push @children, _leaf(_expect('DOT'));
    push @children, _leaf(_expect('IDENT'));
    return _node('class_selector', \@children);
}

# --- _parse_id_selector() ----------------------------------------------------------
#
# Grammar: id_selector = HASH
# Example: #header, #nav
# The HASH token already includes the # character.

sub _parse_id_selector {
    return _node('id_selector', [_leaf(_expect('HASH'))]);
}

# --- _parse_attribute_selector() ---------------------------------------------------
#
# Grammar: attribute_selector = LBRACKET IDENT [ attr_matcher attr_value [ IDENT ] ] RBRACKET
#
# Examples:
#   [disabled]              — has attribute (no matcher)
#   [type="text"]           — exact match
#   [class~="warning"]      — word match
#   [href^="https"]         — starts with
#   [type="text" i]         — case-insensitive flag

sub _parse_attribute_selector {
    my @children;

    push @children, _leaf(_expect('LBRACKET'));
    push @children, _leaf(_expect('IDENT'));    # attribute name

    # Optional: matcher + value [ + flag ]
    if ($ATTR_MATCHERS{_peek()->{type}}) {
        push @children, _leaf(_advance());      # matcher operator

        # Attribute value: IDENT or STRING
        my $vt = _peek();
        if ($vt->{type} eq 'IDENT' || $vt->{type} eq 'STRING') {
            push @children, _leaf(_advance());
        } else {
            die sprintf(
                "CodingAdventures::CssParser: Expected attribute value at "
              . "line %d col %d, got %s ('%s')",
                $vt->{line}, $vt->{col}, $vt->{type}, $vt->{value}
            );
        }

        # Optional case-sensitivity flag: 'i' or 's'
        if (_peek()->{type} eq 'IDENT') {
            push @children, _leaf(_advance());
        }
    }

    push @children, _leaf(_expect('RBRACKET'));
    return _node('attribute_selector', \@children);
}

# --- _parse_pseudo_class() ----------------------------------------------------------
#
# Grammar: pseudo_class = COLON FUNCTION pseudo_class_args RPAREN
#                        | COLON IDENT
#
# Simple: :hover, :focus, :checked
# Functional: :nth-child(2n+1), :not(.class), :is(.a, .b)
#
# Disambiguation: after consuming COLON, if the next token is FUNCTION,
# it's a functional pseudo-class. Otherwise it's a simple one (IDENT).

sub _parse_pseudo_class {
    my @children;

    push @children, _leaf(_expect('COLON'));

    my $t = _peek();
    if ($t->{type} eq 'FUNCTION') {
        push @children, _leaf(_advance());           # FUNCTION (e.g., nth-child()
        push @children, _parse_pseudo_class_args();  # args
        push @children, _leaf(_expect('RPAREN'));
    } elsif ($t->{type} eq 'IDENT') {
        push @children, _leaf(_advance());           # simple pseudo-class name
    } else {
        die sprintf(
            "CodingAdventures::CssParser: Expected pseudo-class name at "
          . "line %d col %d, got %s ('%s')",
            $t->{line}, $t->{col}, $t->{type}, $t->{value}
        );
    }

    return _node('pseudo_class', \@children);
}

# --- _parse_pseudo_class_args() -------------------------------------------------------
#
# Grammar: pseudo_class_args = { any token except RPAREN }
#
# Pseudo-class arguments can be selector lists (:not(.class)),
# An+B expressions (:nth-child(2n+1)), or plain tokens (:lang(en-US)).
# We use a flexible approach: consume any token except the closing RPAREN.
# Track nested parens and functions to avoid stopping at inner RPARENs.

sub _parse_pseudo_class_args {
    my @children;
    my $depth = 0;

    while (1) {
        my $t = _peek();
        last if $t->{type} eq 'EOF';
        last if $t->{type} eq 'RPAREN' && $depth == 0;

        if ($t->{type} eq 'LPAREN' || $t->{type} eq 'FUNCTION') {
            $depth++;
        } elsif ($t->{type} eq 'RPAREN') {
            $depth--;
        }

        push @children, _leaf(_advance());
    }

    return _node('pseudo_class_args', \@children);
}

# --- _parse_pseudo_element() ---------------------------------------------------------
#
# Grammar: pseudo_element = COLON_COLON IDENT
# Examples: ::before, ::after, ::first-line, ::placeholder

sub _parse_pseudo_element {
    my @children;
    push @children, _leaf(_expect('COLON_COLON'));
    push @children, _leaf(_expect('IDENT'));
    return _node('pseudo_element', \@children);
}

# --- _parse_block() ----------------------------------------------------------------
#
# Grammar: block = LBRACE block_contents RBRACE
#
# A block is the { ... } container for declarations and nested rules.

sub _parse_block {
    my @children;
    push @children, _leaf(_expect('LBRACE'));
    push @children, _parse_block_contents();
    push @children, _leaf(_expect('RBRACE'));
    return _node('block', \@children);
}

# --- _parse_block_contents() -------------------------------------------------------
#
# Grammar: block_contents = { block_item }
#
# A block can contain declarations and nested rules.
# Stop at RBRACE or EOF.

sub _parse_block_contents {
    my @children;

    while (_peek()->{type} ne 'RBRACE' && _peek()->{type} ne 'EOF') {
        my $item = _parse_block_item();
        push @children, $item;
    }

    return _node('block_contents', \@children);
}

# --- _parse_block_item() -----------------------------------------------------------
#
# Grammar: block_item = at_rule | declaration_or_nested
#
# At-rules can appear inside blocks (e.g., @media inside another @media,
# @keyframes inside a @supports block).

sub _parse_block_item {
    my $t = _peek();

    if ($t->{type} eq 'AT_KEYWORD') {
        return _node('block_item', [_parse_at_rule()]);
    }

    return _node('block_item', [_parse_declaration_or_nested()]);
}

# --- _parse_declaration_or_nested() -------------------------------------------------
#
# Grammar: declaration_or_nested = declaration | qualified_rule
#
# Both can start with IDENT (property name vs. type selector). We try
# declaration first. If the current token is IDENT or CUSTOM_PROPERTY and
# the token *after* it is COLON, it's a declaration. Otherwise it's a
# qualified_rule.
#
# This peek-ahead avoids expensive backtracking: we look one or two tokens
# ahead to make the decision.
#
# Note: we also handle CUSTOM_PROPERTY as a property name (CSS variables).

sub _parse_declaration_or_nested {
    my $t  = _peek();
    my $t1 = _peek_at(1);

    # If current token is IDENT or CUSTOM_PROPERTY and next is COLON,
    # it's a declaration.
    if (($t->{type} eq 'IDENT' || $t->{type} eq 'CUSTOM_PROPERTY')
            && $t1->{type} eq 'COLON') {
        return _node('declaration_or_nested', [_parse_declaration()]);
    }

    # Otherwise it's a nested qualified rule (CSS Nesting).
    return _node('declaration_or_nested', [_parse_qualified_rule()]);
}

# --- _parse_declaration() ---------------------------------------------------------
#
# Grammar: declaration = property COLON value_list [ priority ] SEMICOLON
#
# Examples:
#   color: red;
#   font-size: 16px;
#   --custom-var: 42px;
#   margin: 10px 20px !important;

sub _parse_declaration {
    my @children;

    push @children, _parse_property();
    push @children, _leaf(_expect('COLON'));
    push @children, _parse_value_list();

    # Optional !important
    if (_peek()->{type} eq 'BANG') {
        push @children, _parse_priority();
    }

    push @children, _leaf(_expect('SEMICOLON'));

    return _node('declaration', \@children);
}

# --- _parse_property() ------------------------------------------------------------
#
# Grammar: property = IDENT | CUSTOM_PROPERTY
#
# Regular property: color, font-size, background-image
# Custom property: --main-color, --bg

sub _parse_property {
    my $t = _peek();
    unless ($t->{type} eq 'IDENT' || $t->{type} eq 'CUSTOM_PROPERTY') {
        die sprintf(
            "CodingAdventures::CssParser: Expected property name at "
          . "line %d col %d, got %s ('%s')",
            $t->{line}, $t->{col}, $t->{type}, $t->{value}
        );
    }
    return _node('property', [_leaf(_advance())]);
}

# --- _parse_priority() ------------------------------------------------------------
#
# Grammar: priority = BANG IDENT("important")
#
# The !important annotation. We consume BANG and then the IDENT "important".

sub _parse_priority {
    my @children;
    push @children, _leaf(_expect('BANG'));
    # The word "important" is tokenized as IDENT by the CSS lexer.
    # We just consume the next IDENT without checking its value.
    push @children, _leaf(_expect('IDENT'));
    return _node('priority', \@children);
}

# --- _parse_value_list() ----------------------------------------------------------
#
# Grammar: value_list = value { value }
#
# CSS values are extraordinarily diverse. A value list is a sequence of
# individual values (separated by spaces, commas, or slashes). We keep
# consuming values until we see something that can't be a value:
# SEMICOLON, RBRACE, BANG (for !important), EOF.

sub _parse_value_list {
    my @children;

    while (1) {
        my $t = _peek();

        # Stop conditions for value list
        last if $t->{type} eq 'SEMICOLON';
        last if $t->{type} eq 'RBRACE';
        last if $t->{type} eq 'BANG';
        last if $t->{type} eq 'EOF';

        # Try to parse a value
        my $v = _try_parse_value();
        last unless $v;

        push @children, $v;
    }

    if (!@children) {
        die sprintf(
            "CodingAdventures::CssParser: Expected at least one value at "
          . "line %d col %d, got %s ('%s')",
            _peek()->{line}, _peek()->{col}, _peek()->{type}, _peek()->{value}
        );
    }

    return _node('value_list', \@children);
}

# --- _try_parse_value() -----------------------------------------------------------
#
# Try to parse a single value. Returns undef if the current token can't
# start a value. This allows _parse_value_list to stop gracefully.

sub _try_parse_value {
    my $t = _peek();

    # Function call: FUNCTION ... RPAREN
    if ($t->{type} eq 'FUNCTION') {
        return _node('value', [_parse_function_call()]);
    }

    # Unquoted URL token: url(./path)
    if ($t->{type} eq 'URL_TOKEN') {
        return _node('value', [_leaf(_advance())]);
    }

    # Simple value tokens
    if ($VALUE_TYPES{$t->{type}}) {
        return _node('value', [_leaf(_advance())]);
    }

    return undef;
}

# --- _parse_function_call() -------------------------------------------------------
#
# Grammar: function_call = FUNCTION function_args RPAREN | URL_TOKEN
#
# FUNCTION token includes the opening paren: "rgba(" is one token.
# We consume the FUNCTION token, the arguments, and the closing RPAREN.

sub _parse_function_call {
    my @children;

    push @children, _leaf(_expect('FUNCTION'));
    push @children, _parse_function_args();
    push @children, _leaf(_expect('RPAREN'));

    return _node('function_call', \@children);
}

# --- _parse_function_args() -------------------------------------------------------
#
# Grammar: function_args = { function_arg }
#
# Function arguments can contain any value-like tokens, including nested
# function calls. Stop at RPAREN or EOF.

sub _parse_function_args {
    my @children;

    while (1) {
        my $t = _peek();
        last if $t->{type} eq 'RPAREN';
        last if $t->{type} eq 'EOF';

        # Nested function call: calc(100% - var(--x, 20px))
        if ($t->{type} eq 'FUNCTION') {
            push @children, _leaf(_advance());            # FUNCTION token
            push @children, _parse_function_args();       # recursive args
            push @children, _leaf(_expect('RPAREN'));      # closing paren
            next;
        }

        if ($FUNCTION_ARG_TYPES{$t->{type}}) {
            push @children, _leaf(_advance());
            next;
        }

        last;  # Unknown token — stop consuming args
    }

    return _node('function_args', \@children);
}

1;

__END__

=head1 NAME

CodingAdventures::CssParser - Hand-written recursive-descent CSS3 parser

=head1 SYNOPSIS

    use CodingAdventures::CssParser;

    my $ast = CodingAdventures::CssParser->parse(<<'CSS');
    h1 { color: red; }
    @media screen { p { font-size: 16px; } }
    CSS

    print $ast->rule_name;  # "stylesheet"

=head1 DESCRIPTION

A hand-written recursive-descent parser for CSS3. Tokenizes source text
using C<CodingAdventures::CssLexer> and constructs an AST using
C<CodingAdventures::CssParser::ASTNode>.

Implements the CSS grammar rules: stylesheet, rule, at_rule, at_prelude,
qualified_rule, selector_list, complex_selector, compound_selector,
simple_selector, subclass_selector, class_selector, id_selector,
attribute_selector, pseudo_class, pseudo_element, block, block_contents,
block_item, declaration_or_nested, declaration, property, priority,
value_list, value, function_call, function_args.

=head2 CSS parsing challenges

Declaration vs. nested rule disambiguation: both can start with IDENT.
The parser peeks one token ahead — if IDENT is followed by COLON, it's
a declaration; otherwise it's a nested qualified rule.

At-rule preludes: the sequence between AT_KEYWORD and { or ; is consumed
as a flexible token sequence with paren-depth tracking.

Function arguments: the FUNCTION token includes the opening paren ("rgba("),
so arguments are collected until the matching RPAREN, with nesting support.

=head1 METHODS

=head2 parse($source)

Parse a CSS3 string. Returns the root C<ASTNode> with
C<rule_name == "stylesheet">. Dies on lexer or parser errors.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
