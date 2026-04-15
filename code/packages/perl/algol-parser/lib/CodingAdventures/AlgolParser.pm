package CodingAdventures::AlgolParser;

# ============================================================================
# CodingAdventures::AlgolParser — Hand-written recursive-descent ALGOL 60 parser
# ============================================================================
#
# This module parses ALGOL 60 source text into an Abstract Syntax Tree (AST).
# It uses a hand-written recursive-descent approach: each grammar rule becomes
# a Perl subroutine that consumes tokens and returns ASTNodes.
#
# # Why hand-written?
# --------------------
# The Perl CodingAdventures::GrammarTools module only provides
# `parse_token_grammar`, not `parse_parser_grammar`.  There is no grammar-
# driven parser generator in the Perl layer, so we implement the parser by
# hand, following the ALGOL 60 grammar specified in `algol.grammar`.
#
# # ALGOL 60 grammar (simplified)
#
#   program      = block
#   block        = BEGIN {declaration SEMICOLON} statement {SEMICOLON statement} END
#   declaration  = type_decl | array_decl | switch_decl | procedure_decl
#   type_decl    = type ident_list
#   type         = INTEGER | REAL | BOOLEAN | STRING
#   ident_list   = IDENT {COMMA IDENT}
#   statement    = [label COLON] unlabeled_stmt | [label COLON] cond_stmt
#   unlabeled_stmt = assign_stmt | goto_stmt | proc_stmt | compound_stmt
#                  | block | for_stmt | empty_stmt
#   cond_stmt    = IF bool_expr THEN unlabeled_stmt [ELSE statement]
#   assign_stmt  = left_part {left_part} expression
#   left_part    = variable ASSIGN
#   expression   = arith_expr | bool_expr
#   arith_expr   = IF bool_expr THEN simple_arith ELSE arith_expr | simple_arith
#   simple_arith = [PLUS|MINUS] term {(PLUS|MINUS) term}
#   term         = factor {(STAR|SLASH|DIV|MOD) factor}
#   factor       = primary {(CARET|POWER) primary}
#   primary      = INTEGER_LIT | REAL_LIT | STRING_LIT | variable | proc_call
#                | LPAREN arith_expr RPAREN
#   bool_expr    = IF bool_expr THEN simple_bool ELSE bool_expr | simple_bool
#   simple_bool  = implication {EQV implication}
#   implication  = bool_term {IMPL bool_term}
#   bool_term    = bool_factor {OR bool_factor}
#   bool_factor  = bool_secondary {AND bool_secondary}
#   bool_secondary = NOT bool_secondary | bool_primary
#   bool_primary = TRUE | FALSE | variable | proc_call
#                | LPAREN bool_expr RPAREN | relation
#   relation     = simple_arith (EQ|NEQ|LT|LEQ|GT|GEQ) simple_arith
#   variable     = IDENT [LBRACKET subscripts RBRACKET]
#   proc_call    = IDENT LPAREN actual_params RPAREN
#
# # LL(1) parsing
# ---------------
#
# ALGOL 60 is largely LL(1): at each parse point, one token of lookahead
# determines which production to apply.  Key lookahead decisions:
#
#   BEGIN     → block or compound_stmt
#   IF        → cond_stmt or conditional expression
#   FOR       → for_stmt
#   GOTO      → goto_stmt
#   INTEGER | REAL | BOOLEAN | STRING → type_decl (in declaration context)
#   ARRAY     → array_decl
#   SWITCH    → switch_decl
#   PROCEDURE → procedure_decl
#   IDENT     → assign_stmt, proc_stmt, or label (with COLON lookahead)
#
# # Ambiguity: declarations vs. statements
# -----------------------------------------
#
# In a block, declarations precede statements.  The parser peeks at the
# current token to distinguish:
#   - INTEGER, REAL, BOOLEAN, STRING, ARRAY, SWITCH, PROCEDURE → declaration
#   - anything else → statement
#
# This keeps the parser simple without backtracking.
#
# # Ambiguity: assign_stmt vs. proc_stmt vs. label
# --------------------------------------------------
#
# When we see IDENT at statement position:
#   - If IDENT ASSIGN follows → assign_stmt
#   - If IDENT LPAREN follows → proc_stmt (procedure call)
#   - If IDENT COLON follows → label, then parse the labeled statement
#   - Otherwise → proc_stmt with no arguments
#
# We use two-token lookahead (_peek2) to distinguish these cases.
#
# # AST structure
#
# Internal nodes:
#   { rule_name => "program",   children => [...], is_leaf => 0 }
#   { rule_name => "block",     children => [...], is_leaf => 0 }
#   { rule_name => "statement", children => [...], is_leaf => 0 }
#   etc.
#
# Leaf nodes (token wrappers):
#   { rule_name => "token", children => [], is_leaf => 1,
#     token => { type => "NAME", value => "x", line => 1, col => 5 } }
#
# # Parse state
#
# The parser keeps two package-level variables:
#   $tokens_ref — arrayref of token hashrefs from CodingAdventures::AlgolLexer
#   $pos        — current position (0-based index into @$tokens_ref)
#
# These are set by `parse()` on each call.  The parser is not re-entrant,
# but ALGOL parsing is synchronous so that's fine in practice.

use strict;
use warnings;

use CodingAdventures::AlgolLexer;
use CodingAdventures::AlgolParser::ASTNode;

our $VERSION = '0.01';

# Package-level parse state.
# Set at the start of each `parse()` call.
my ($tokens_ref, $pos);

# ============================================================================
# Public API
# ============================================================================

# --- parse($class, $source) ---------------------------------------------------
#
# Parse an ALGOL 60 source string and return the root ASTNode.
#
# Steps:
#   1. Tokenize `$source` using CodingAdventures::AlgolLexer.
#   2. Initialize parser state ($tokens_ref, $pos).
#   3. Parse the top-level `program` production.
#   4. Assert that we consumed all tokens (only EOF remains).
#   5. Return the root ASTNode.
#
# Dies on lexer errors or parse errors.
#
# @param  $source  string  The ALGOL 60 text to parse.
# @return ASTNode          Root node with rule_name "program".

sub parse {
    my ($class, $source, %opts) = @_;
    my $version = delete($opts{version});
    $version = 'algol60' if !defined($version) || $version eq '';
    die "CodingAdventures::AlgolParser: unknown ALGOL version '$version' (valid: algol60)"
        unless $version eq 'algol60';
    die "CodingAdventures::AlgolParser: unknown options: " . join(", ", sort keys %opts)
        if %opts;

    # Tokenize
    my $toks = CodingAdventures::AlgolLexer->tokenize($source, version => $version);
    $tokens_ref = $toks;
    $pos = 0;

    # Parse the top-level program
    my $ast = _parse_program();

    # Verify we consumed everything — only EOF should remain
    my $t = _peek();
    if ($t->{type} ne 'EOF') {
        die sprintf(
            "CodingAdventures::AlgolParser: trailing content at line %d col %d: "
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
    return $tokens_ref->[$pos] // $tokens_ref->[-1];
}

# --- _peek2() -----------------------------------------------------------------
#
# Return the NEXT token (one ahead of current) without consuming either.
# Used for two-token lookahead to disambiguate assign_stmt / proc_stmt / label.

sub _peek2 {
    return $tokens_ref->[$pos + 1] // $tokens_ref->[-1];
}

# --- _advance() ---------------------------------------------------------------
#
# Consume and return the current token, advancing the position.

sub _advance {
    my $t = _peek();
    $pos++;
    return $t;
}

# --- _expect($type) -----------------------------------------------------------
#
# Assert that the current token has type `$type`, consume it, and return it.
# Dies with a descriptive message if the type doesn't match.
#
# Example:
#   _expect('ASSIGN')  — dies "Expected ASSIGN, got SEMICOLON (';') at line 1 col 3"

sub _expect {
    my ($type) = @_;
    my $t = _peek();
    unless ($t->{type} eq $type) {
        die sprintf(
            "CodingAdventures::AlgolParser: Expected %s, got %s ('%s') "
          . "at line %d col %d",
            $type, $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _advance();
}

# --- _node($rule_name, $children_aref) ----------------------------------------
#
# Construct an internal (non-leaf) ASTNode.

sub _node {
    my ($rule, $children) = @_;
    return CodingAdventures::AlgolParser::ASTNode->new(
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
    return CodingAdventures::AlgolParser::ASTNode->new(
        rule_name => 'token',
        children  => [],
        is_leaf   => 1,
        token     => $tok,
    );
}

# ============================================================================
# Top-level productions
# ============================================================================

# --- _parse_program() ---------------------------------------------------------
#
# Grammar: program = block
#
# Every ALGOL 60 program is a single block.

sub _parse_program {
    my $block = _parse_block();
    return _node('program', [$block]);
}

# --- _parse_block() -----------------------------------------------------------
#
# Grammar: block = BEGIN {declaration SEMICOLON} statement {SEMICOLON statement} END
#
# A block opens a new lexical scope.  All declarations must precede all
# statements.  Both sections are optional; the minimal block is `begin end`.
#
# Decision: is the next item a declaration or statement?
#   Declaration starters: INTEGER, REAL, BOOLEAN, STRING, ARRAY, SWITCH, PROCEDURE
#   Statement: everything else
#
# Note: OWN and type+PROCEDURE also start declarations; we handle them here too.

sub _parse_block {
    my @children;

    my $open = _expect('BEGIN');
    push @children, _leaf($open);

    # Declaration phase: consume while next token looks like a declaration start.
    while (_is_declaration_start(_peek()->{type})) {
        push @children, _parse_declaration();
        my $semi = _expect('SEMICOLON');
        push @children, _leaf($semi);
    }

    # Statement phase: consume one or more statements separated by semicolons.
    # We require at least one statement (even if empty).
    push @children, _parse_statement();

    while (_peek()->{type} eq 'SEMICOLON' && _peek2()->{type} ne 'END') {
        push @children, _leaf(_advance());    # consume SEMICOLON
        push @children, _parse_statement();
    }

    # Consume optional trailing semicolon before END
    if (_peek()->{type} eq 'SEMICOLON' && _peek2()->{type} eq 'END') {
        push @children, _leaf(_advance());
    }

    my $close = _expect('END');
    push @children, _leaf($close);

    return _node('block', \@children);
}

# --- _is_declaration_start($type) ---------------------------------------------
#
# Return true if the token type can begin a declaration.
#
# Declaration starters:
#   INTEGER, REAL, BOOLEAN, STRING  → type_decl
#   ARRAY                           → array_decl (possibly untyped)
#   SWITCH                          → switch_decl
#   PROCEDURE                       → procedure_decl
#   OWN                             → own declaration (type follows)
#
# This is an LL(1) predicate: one token of lookahead is sufficient.

my %DECL_STARTERS = map { $_ => 1 }
    qw(INTEGER REAL BOOLEAN STRING ARRAY SWITCH PROCEDURE OWN);

sub _is_declaration_start {
    my ($type) = @_;
    return $DECL_STARTERS{$type} // 0;
}

# ============================================================================
# Declarations
# ============================================================================

# --- _parse_declaration() -----------------------------------------------------
#
# Grammar: declaration = type_decl | array_decl | switch_decl | procedure_decl
#
# Dispatch on current token.

sub _parse_declaration {
    my $t = _peek();
    my $type = $t->{type};

    # Array declaration may be preceded by a type or bare
    if ($type eq 'ARRAY') {
        return _parse_array_decl();
    }

    if ($type eq 'SWITCH') {
        return _parse_switch_decl();
    }

    if ($type eq 'PROCEDURE') {
        return _parse_procedure_decl();
    }

    # Type keywords: INTEGER REAL BOOLEAN STRING
    # Check if followed by PROCEDURE (type procedure) or ARRAY (typed array)
    # or IDENT (type_decl)
    if ($type =~ /^(INTEGER|REAL|BOOLEAN|STRING)$/) {
        my $next = _peek2()->{type};
        if ($next eq 'PROCEDURE') {
            return _parse_procedure_decl();
        } elsif ($next eq 'ARRAY') {
            return _parse_array_decl();
        } else {
            return _parse_type_decl();
        }
    }

    # OWN: e.g. "own integer x"
    if ($type eq 'OWN') {
        my @ch;
        push @ch, _leaf(_advance());   # consume OWN
        push @ch, _parse_declaration();
        return _node('own_decl', \@ch);
    }

    die sprintf(
        "CodingAdventures::AlgolParser: Expected declaration, got %s ('%s') "
      . "at line %d col %d",
        $type, $t->{value}, $t->{line}, $t->{col}
    );
}

# --- _parse_type_decl() -------------------------------------------------------
#
# Grammar: type_decl = type ident_list
#
# Examples:
#   integer x, y, z
#   real sum

sub _parse_type_decl {
    my @children;
    push @children, _parse_type();
    push @children, _parse_ident_list();
    return _node('type_decl', \@children);
}

# --- _parse_type() ------------------------------------------------------------
#
# Grammar: type = INTEGER | REAL | BOOLEAN | STRING

sub _parse_type {
    my $t = _peek();
    if ($t->{type} =~ /^(INTEGER|REAL|BOOLEAN|STRING)$/) {
        return _leaf(_advance());
    }
    die sprintf(
        "CodingAdventures::AlgolParser: Expected type keyword, got %s ('%s') "
      . "at line %d col %d",
        $t->{type}, $t->{value}, $t->{line}, $t->{col}
    );
}

# --- _parse_ident_list() ------------------------------------------------------
#
# Grammar: ident_list = IDENT {COMMA IDENT}

sub _parse_ident_list {
    my @children;
    push @children, _leaf( _expect('NAME') );
    while (_peek()->{type} eq 'COMMA') {
        push @children, _leaf(_advance());    # consume COMMA
        push @children, _leaf( _expect('NAME') );
    }
    return _node('ident_list', \@children);
}

# --- _parse_array_decl() ------------------------------------------------------
#
# Grammar: array_decl = [type] ARRAY array_segment {COMMA array_segment}
#
# The type prefix is optional; if present we consume it first.

sub _parse_array_decl {
    my @children;
    # Optional type prefix
    if (_peek()->{type} =~ /^(INTEGER|REAL|BOOLEAN|STRING)$/) {
        push @children, _parse_type();
    }
    push @children, _leaf( _expect('ARRAY') );
    push @children, _parse_array_segment();
    while (_peek()->{type} eq 'COMMA') {
        push @children, _leaf(_advance());
        push @children, _parse_array_segment();
    }
    return _node('array_decl', \@children);
}

# --- _parse_array_segment() ---------------------------------------------------
#
# Grammar: array_segment = ident_list LBRACKET bound_pair {COMMA bound_pair} RBRACKET

sub _parse_array_segment {
    my @children;
    push @children, _parse_ident_list();
    push @children, _leaf( _expect('LBRACKET') );
    push @children, _parse_bound_pair();
    while (_peek()->{type} eq 'COMMA') {
        push @children, _leaf(_advance());
        push @children, _parse_bound_pair();
    }
    push @children, _leaf( _expect('RBRACKET') );
    return _node('array_segment', \@children);
}

# --- _parse_bound_pair() ------------------------------------------------------
#
# Grammar: bound_pair = arith_expr COLON arith_expr

sub _parse_bound_pair {
    my @children;
    push @children, _parse_arith_expr();
    push @children, _leaf( _expect('COLON') );
    push @children, _parse_arith_expr();
    return _node('bound_pair', \@children);
}

# --- _parse_switch_decl() -----------------------------------------------------
#
# Grammar: switch_decl = SWITCH IDENT ASSIGN switch_list
# where: switch_list = desig_expr {COMMA desig_expr}

sub _parse_switch_decl {
    my @children;
    push @children, _leaf( _expect('SWITCH') );
    push @children, _leaf( _expect('NAME')  );
    push @children, _leaf( _expect('ASSIGN') );
    # switch_list: one or more designational expressions separated by COMMA
    push @children, _parse_desig_expr();
    while (_peek()->{type} eq 'COMMA') {
        push @children, _leaf(_advance());
        push @children, _parse_desig_expr();
    }
    return _node('switch_decl', \@children);
}

# --- _parse_procedure_decl() --------------------------------------------------
#
# Grammar:
#   procedure_decl = [type] PROCEDURE IDENT [formal_params] SEMICOLON
#                    [value_part] {spec_part} proc_body
#
# This is the most complex declaration in ALGOL 60.  The optional type prefix,
# the optional formal_params, the optional value_part, zero-or-more spec_parts,
# and the proc_body (block or statement) all need to be handled.

sub _parse_procedure_decl {
    my @children;

    # Optional type prefix
    if (_peek()->{type} =~ /^(INTEGER|REAL|BOOLEAN|STRING)$/) {
        push @children, _parse_type();
    }

    push @children, _leaf( _expect('PROCEDURE') );
    push @children, _leaf( _expect('NAME')     );

    # Optional formal parameters
    if (_peek()->{type} eq 'LPAREN') {
        push @children, _parse_formal_params();
    }

    push @children, _leaf( _expect('SEMICOLON') );

    # Optional value_part: VALUE ident_list SEMICOLON
    if (_peek()->{type} eq 'VALUE') {
        push @children, _parse_value_part();
    }

    # Zero or more spec_parts: specifier ident_list SEMICOLON
    while (_is_specifier(_peek()->{type})) {
        push @children, _parse_spec_part();
    }

    # Procedure body: block or statement
    push @children, _parse_proc_body();

    return _node('procedure_decl', \@children);
}

# --- _is_specifier($type) -----------------------------------------------------
#
# Return true if the type can begin a specifier (spec_part).

my %SPECIFIERS = map { $_ => 1 }
    qw(INTEGER REAL BOOLEAN STRING ARRAY LABEL SWITCH PROCEDURE);

sub _is_specifier {
    my ($type) = @_;
    return $SPECIFIERS{$type} // 0;
}

# --- _parse_formal_params() ---------------------------------------------------
#
# Grammar: formal_params = LPAREN ident_list RPAREN

sub _parse_formal_params {
    my @children;
    push @children, _leaf( _expect('LPAREN') );
    push @children, _parse_ident_list();
    push @children, _leaf( _expect('RPAREN') );
    return _node('formal_params', \@children);
}

# --- _parse_value_part() ------------------------------------------------------
#
# Grammar: value_part = VALUE ident_list SEMICOLON

sub _parse_value_part {
    my @children;
    push @children, _leaf( _expect('VALUE') );
    push @children, _parse_ident_list();
    push @children, _leaf( _expect('SEMICOLON') );
    return _node('value_part', \@children);
}

# --- _parse_spec_part() -------------------------------------------------------
#
# Grammar: spec_part = specifier ident_list SEMICOLON

sub _parse_spec_part {
    my @children;
    my $t = _peek();
    unless (_is_specifier($t->{type})) {
        die sprintf(
            "CodingAdventures::AlgolParser: Expected specifier, got %s ('%s') "
          . "at line %d col %d",
            $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    push @children, _leaf(_advance());   # the specifier keyword
    push @children, _parse_ident_list();
    push @children, _leaf( _expect('SEMICOLON') );
    return _node('spec_part', \@children);
}

# --- _parse_proc_body() -------------------------------------------------------
#
# Grammar: proc_body = block | statement

sub _parse_proc_body {
    if (_peek()->{type} eq 'BEGIN') {
        return _node('proc_body', [_parse_block()]);
    } else {
        return _node('proc_body', [_parse_statement()]);
    }
}

# ============================================================================
# Statements
# ============================================================================

# --- _parse_statement() -------------------------------------------------------
#
# Grammar:
#   statement = [label COLON] unlabeled_stmt
#             | [label COLON] cond_stmt
#
# Decision:
#   - IF → cond_stmt
#   - IDENT COLON → labeled statement (consume label+colon, parse body)
#   - anything else → unlabeled_stmt

sub _parse_statement {
    my @children;

    # Optional label: IDENT COLON or INTEGER_LIT COLON
    if ( (_peek()->{type} eq 'NAME' || _peek()->{type} eq 'INTEGER_LIT')
          && _peek2()->{type} eq 'COLON' ) {
        push @children, _leaf(_advance());    # label (IDENT or INTEGER_LIT)
        push @children, _leaf(_advance());    # COLON
    }

    if (_peek()->{type} eq 'IF') {
        push @children, _parse_cond_stmt();
    } else {
        push @children, _parse_unlabeled_stmt();
    }

    return _node('statement', \@children);
}

# --- _parse_cond_stmt() -------------------------------------------------------
#
# Grammar: cond_stmt = IF bool_expr THEN unlabeled_stmt [ELSE statement]
#
# Dangling else resolution: the then-branch is unlabeled_stmt (which cannot
# itself be a conditional), so the else always belongs to the nearest IF.
# begin/end is required to nest conditionals in the then-branch.

sub _parse_cond_stmt {
    my @children;
    push @children, _leaf( _expect('IF') );
    push @children, _parse_bool_expr();
    push @children, _leaf( _expect('THEN') );
    push @children, _parse_unlabeled_stmt();
    if (_peek()->{type} eq 'ELSE') {
        push @children, _leaf(_advance());    # consume ELSE
        push @children, _parse_statement();
    }
    return _node('cond_stmt', \@children);
}

# --- _parse_unlabeled_stmt() --------------------------------------------------
#
# Grammar:
#   unlabeled_stmt = assign_stmt | goto_stmt | proc_stmt | compound_stmt
#                  | block | for_stmt | empty_stmt
#
# Dispatch on current token:
#   BEGIN → block (has declarations) or compound_stmt (no declarations)
#           We always call _parse_block() which handles both cases.
#   FOR   → for_stmt
#   GOTO  → goto_stmt
#   IDENT with next=ASSIGN → assign_stmt
#   IDENT with next=LPAREN → proc_stmt (call with args)
#   IDENT                  → proc_stmt (call without args)
#   SEMICOLON or END       → empty_stmt
#
# Note: compound_stmt vs block distinction:
#   compound_stmt = BEGIN statement {SEMICOLON statement} END  (no declarations)
#   block         = BEGIN {declaration SEMICOLON} statement ... END
# We use _parse_block() for both — it handles zero declarations naturally.

sub _parse_unlabeled_stmt {
    my $t    = _peek();
    my $type = $t->{type};

    return _node('unlabeled_stmt', [_parse_block()])    if $type eq 'BEGIN';
    return _node('unlabeled_stmt', [_parse_for_stmt()]) if $type eq 'FOR';
    return _node('unlabeled_stmt', [_parse_goto_stmt()]) if $type eq 'GOTO';

    if ($type eq 'NAME') {
        my $next = _peek2()->{type};
        if ($next eq 'ASSIGN') {
            return _node('unlabeled_stmt', [_parse_assign_stmt()]);
        } elsif ($next eq 'LBRACKET') {
            # Could be subscripted variable assignment: A[i] := ...
            # We need deeper lookahead, but for now: parse assign_stmt
            # which handles left_part = variable ASSIGN.
            return _node('unlabeled_stmt', [_parse_assign_stmt()]);
        } else {
            return _node('unlabeled_stmt', [_parse_proc_stmt()]);
        }
    }

    # Empty statement
    return _node('unlabeled_stmt', [_node('empty_stmt', [])]);
}

# --- _parse_assign_stmt() -----------------------------------------------------
#
# Grammar: assign_stmt = left_part {left_part} expression
#
# left_part = variable ASSIGN
#
# Multiple left parts assign the same value right-to-left:
#   x := y := 0   assigns 0 to y, then x.
#
# We parse one or more left_parts and then the expression.

sub _parse_assign_stmt {
    my @children;
    # First left_part (required)
    push @children, _parse_left_part();
    # Additional left_parts: IDENT [LBRACKET ...] ASSIGN
    # Lookahead: if current position after consuming left_part starts another IDENT
    # followed eventually by ASSIGN, it's another left_part.
    # Simple heuristic: if next token is IDENT and the one after that is ASSIGN
    # (or LBRACKET ...), it's another left_part.
    while (_peek()->{type} eq 'NAME' && _peek2()->{type} eq 'ASSIGN') {
        push @children, _parse_left_part();
    }
    push @children, _parse_expression();
    return _node('assign_stmt', \@children);
}

# --- _parse_left_part() -------------------------------------------------------
#
# Grammar: left_part = variable ASSIGN

sub _parse_left_part {
    my @children;
    push @children, _parse_variable();
    push @children, _leaf( _expect('ASSIGN') );
    return _node('left_part', \@children);
}

# --- _parse_goto_stmt() -------------------------------------------------------
#
# Grammar: goto_stmt = GOTO desig_expr

sub _parse_goto_stmt {
    my @children;
    push @children, _leaf( _expect('GOTO') );
    push @children, _parse_desig_expr();
    return _node('goto_stmt', \@children);
}

# --- _parse_proc_stmt() -------------------------------------------------------
#
# Grammar: proc_stmt = IDENT [LPAREN actual_params RPAREN]

sub _parse_proc_stmt {
    my @children;
    push @children, _leaf( _expect('NAME') );
    if (_peek()->{type} eq 'LPAREN') {
        push @children, _leaf(_advance());    # LPAREN
        push @children, _parse_actual_params();
        push @children, _leaf( _expect('RPAREN') );
    }
    return _node('proc_stmt', \@children);
}

# --- _parse_actual_params() ---------------------------------------------------
#
# Grammar: actual_params = expression {COMMA expression}

sub _parse_actual_params {
    my @children;
    push @children, _parse_expression();
    while (_peek()->{type} eq 'COMMA') {
        push @children, _leaf(_advance());
        push @children, _parse_expression();
    }
    return _node('actual_params', \@children);
}

# --- _parse_for_stmt() --------------------------------------------------------
#
# Grammar: for_stmt = FOR IDENT ASSIGN for_list DO statement
# where: for_list = for_elem {COMMA for_elem}
# and: for_elem = arith_expr STEP arith_expr UNTIL arith_expr
#               | arith_expr WHILE bool_expr
#               | arith_expr

sub _parse_for_stmt {
    my @children;
    push @children, _leaf( _expect('FOR')   );
    push @children, _leaf( _expect('NAME') );
    push @children, _leaf( _expect('ASSIGN') );
    # for_list
    push @children, _parse_for_elem();
    while (_peek()->{type} eq 'COMMA') {
        push @children, _leaf(_advance());
        push @children, _parse_for_elem();
    }
    push @children, _leaf( _expect('DO') );
    push @children, _parse_statement();
    return _node('for_stmt', \@children);
}

# --- _parse_for_elem() --------------------------------------------------------
#
# Grammar: for_elem = arith_expr STEP arith_expr UNTIL arith_expr
#                   | arith_expr WHILE bool_expr
#                   | arith_expr
#
# We parse arith_expr first, then check the next token to decide the form.

sub _parse_for_elem {
    my @children;
    push @children, _parse_arith_expr();
    if (_peek()->{type} eq 'STEP') {
        push @children, _leaf(_advance());    # STEP
        push @children, _parse_arith_expr();
        push @children, _leaf( _expect('UNTIL') );
        push @children, _parse_arith_expr();
        return _node('for_elem_step', \@children);
    } elsif (_peek()->{type} eq 'WHILE') {
        push @children, _leaf(_advance());    # WHILE
        push @children, _parse_bool_expr();
        return _node('for_elem_while', \@children);
    }
    return _node('for_elem', \@children);
}

# ============================================================================
# Expressions
# ============================================================================

# --- _parse_expression() ------------------------------------------------------
#
# Grammar: expression = arith_expr | bool_expr
#
# ALGOL 60 does not have a unified expression type with typed operators —
# arithmetic and boolean are separate syntactic categories.
#
# Disambiguation: we try arith_expr first.  Boolean expressions begin with
# TRUE, FALSE, NOT, IF (conditional boolean), or a relation (which starts
# as an arith_expr anyway).  Since arith_expr can consume the left side of
# a relation, we parse arith_expr and then peek: if we see a relational
# operator (EQ, NEQ, LT, LEQ, GT, GEQ) we are in a relation / bool_expr.
# For simplicity, we parse as arith_expr which covers most expression uses
# in simple programs.

sub _parse_expression {
    my $t = _peek()->{type};

    # Clearly boolean starters
    if ($t eq 'TRUE' || $t eq 'FALSE' || $t eq 'NOT') {
        return _node('expression', [_parse_bool_expr()]);
    }

    # Try arith_expr (which may become part of a relation → bool_expr)
    return _node('expression', [_parse_arith_expr()]);
}

# --- _parse_arith_expr() ------------------------------------------------------
#
# Grammar:
#   arith_expr = IF bool_expr THEN simple_arith ELSE arith_expr
#              | simple_arith
#
# The conditional form is ALGOL's "conditional expression": the result of the
# if/then/else is an arithmetic value.

sub _parse_arith_expr {
    if (_peek()->{type} eq 'IF') {
        my @ch;
        push @ch, _leaf(_advance());    # IF
        push @ch, _parse_bool_expr();
        push @ch, _leaf( _expect('THEN') );
        push @ch, _parse_simple_arith();
        push @ch, _leaf( _expect('ELSE') );
        push @ch, _parse_arith_expr();
        return _node('arith_expr', \@ch);
    }
    return _node('arith_expr', [_parse_simple_arith()]);
}

# --- _parse_simple_arith() ----------------------------------------------------
#
# Grammar: simple_arith = [PLUS|MINUS] term {(PLUS|MINUS) term}
#
# The optional leading sign handles unary plus and minus.
# Subsequent +/- operators are left-to-right (left-associative).

sub _parse_simple_arith {
    my @children;

    # Optional leading sign (unary + or -)
    if (_peek()->{type} =~ /^(PLUS|MINUS)$/) {
        push @children, _leaf(_advance());
    }

    push @children, _parse_term();

    while (_peek()->{type} =~ /^(PLUS|MINUS)$/) {
        push @children, _leaf(_advance());
        push @children, _parse_term();
    }

    return _node('simple_arith', \@children);
}

# --- _parse_term() ------------------------------------------------------------
#
# Grammar: term = factor {(STAR|SLASH|DIV|MOD) factor}

sub _parse_term {
    my @children;
    push @children, _parse_factor();
    while (_peek()->{type} =~ /^(STAR|SLASH|DIV|MOD)$/) {
        push @children, _leaf(_advance());
        push @children, _parse_factor();
    }
    return _node('term', \@children);
}

# --- _parse_factor() ----------------------------------------------------------
#
# Grammar: factor = primary {(CARET|POWER) primary}
#
# Exponentiation in ALGOL 60 is LEFT-associative per the report:
#   2^3^4 = (2^3)^4 = 8^4 = 4096
# This differs from mathematical convention (right-associative).
# The `{...}` repetition in the grammar implements left-associativity.

sub _parse_factor {
    my @children;
    push @children, _parse_primary();
    while (_peek()->{type} =~ /^(CARET|POWER)$/) {
        push @children, _leaf(_advance());
        push @children, _parse_primary();
    }
    return _node('factor', \@children);
}

# --- _parse_primary() ---------------------------------------------------------
#
# Grammar:
#   primary = INTEGER_LIT | REAL_LIT | STRING_LIT
#           | variable | proc_call | LPAREN arith_expr RPAREN
#
# Disambiguation between variable and proc_call:
#   IDENT LPAREN → proc_call (function call)
#   IDENT ...    → variable (possibly subscripted: IDENT[...])

sub _parse_primary {
    my $t    = _peek();
    my $type = $t->{type};

    if ($type eq 'INTEGER_LIT' || $type eq 'REAL_LIT' || $type eq 'STRING_LIT') {
        return _node('primary', [_leaf(_advance())]);
    }

    if ($type eq 'LPAREN') {
        my @ch;
        push @ch, _leaf(_advance());    # LPAREN
        push @ch, _parse_arith_expr();
        push @ch, _leaf( _expect('RPAREN') );
        return _node('primary', \@ch);
    }

    if ($type eq 'NAME') {
        # Peek ahead: IDENT LPAREN → proc_call; otherwise variable
        if (_peek2()->{type} eq 'LPAREN') {
            return _node('primary', [_parse_proc_call()]);
        } else {
            return _node('primary', [_parse_variable()]);
        }
    }

    die sprintf(
        "CodingAdventures::AlgolParser: Expected primary expression, got %s ('%s') "
      . "at line %d col %d",
        $type, $t->{value}, $t->{line}, $t->{col}
    );
}

# --- _parse_bool_expr() -------------------------------------------------------
#
# Grammar:
#   bool_expr = IF bool_expr THEN simple_bool ELSE bool_expr
#             | simple_bool
#
# The conditional form is ALGOL's boolean conditional expression.

sub _parse_bool_expr {
    if (_peek()->{type} eq 'IF') {
        my @ch;
        push @ch, _leaf(_advance());    # IF
        push @ch, _parse_bool_expr();
        push @ch, _leaf( _expect('THEN') );
        push @ch, _parse_simple_bool();
        push @ch, _leaf( _expect('ELSE') );
        push @ch, _parse_bool_expr();
        return _node('bool_expr', \@ch);
    }
    return _node('bool_expr', [_parse_simple_bool()]);
}

# --- _parse_simple_bool() -----------------------------------------------------
#
# Grammar: simple_bool = implication {EQV implication}

sub _parse_simple_bool {
    my @children;
    push @children, _parse_implication();
    while (_peek()->{type} eq 'EQV') {
        push @children, _leaf(_advance());
        push @children, _parse_implication();
    }
    return _node('simple_bool', \@children);
}

# --- _parse_implication() -----------------------------------------------------
#
# Grammar: implication = bool_term {IMPL bool_term}

sub _parse_implication {
    my @children;
    push @children, _parse_bool_term();
    while (_peek()->{type} eq 'IMPL') {
        push @children, _leaf(_advance());
        push @children, _parse_bool_term();
    }
    return _node('implication', \@children);
}

# --- _parse_bool_term() -------------------------------------------------------
#
# Grammar: bool_term = bool_factor {OR bool_factor}

sub _parse_bool_term {
    my @children;
    push @children, _parse_bool_factor();
    while (_peek()->{type} eq 'OR') {
        push @children, _leaf(_advance());
        push @children, _parse_bool_factor();
    }
    return _node('bool_term', \@children);
}

# --- _parse_bool_factor() -----------------------------------------------------
#
# Grammar: bool_factor = bool_secondary {AND bool_secondary}

sub _parse_bool_factor {
    my @children;
    push @children, _parse_bool_secondary();
    while (_peek()->{type} eq 'AND') {
        push @children, _leaf(_advance());
        push @children, _parse_bool_secondary();
    }
    return _node('bool_factor', \@children);
}

# --- _parse_bool_secondary() --------------------------------------------------
#
# Grammar: bool_secondary = NOT bool_secondary | bool_primary
#
# NOT is right-associative here: `not not x` = `not (not x)`.

sub _parse_bool_secondary {
    if (_peek()->{type} eq 'NOT') {
        my @ch;
        push @ch, _leaf(_advance());    # NOT
        push @ch, _parse_bool_secondary();
        return _node('bool_secondary', \@ch);
    }
    return _node('bool_secondary', [_parse_bool_primary()]);
}

# --- _parse_bool_primary() ----------------------------------------------------
#
# Grammar:
#   bool_primary = TRUE | FALSE | variable | proc_call
#                | LPAREN bool_expr RPAREN | relation
#
# A relation starts with simple_arith, so we look ahead: parse simple_arith
# and check if a relational operator follows.  If so, finish the relation.
# If not, the simple_arith was actually a standalone boolean primary.

sub _parse_bool_primary {
    my $t    = _peek();
    my $type = $t->{type};

    if ($type eq 'TRUE' || $type eq 'FALSE') {
        return _node('bool_primary', [_leaf(_advance())]);
    }

    if ($type eq 'LPAREN') {
        my @ch;
        push @ch, _leaf(_advance());    # LPAREN
        push @ch, _parse_bool_expr();
        push @ch, _leaf( _expect('RPAREN') );
        return _node('bool_primary', \@ch);
    }

    # Relation: simple_arith relop simple_arith
    # We parse simple_arith, then check for a relational operator.
    my $lhs = _parse_simple_arith();
    if (_peek()->{type} =~ /^(EQ|NEQ|LT|LEQ|GT|GEQ)$/) {
        my @ch;
        push @ch, $lhs;
        push @ch, _leaf(_advance());    # relop
        push @ch, _parse_simple_arith();
        return _node('bool_primary', [_node('relation', \@ch)]);
    }

    # Not a relation — the simple_arith is a standalone primary
    return _node('bool_primary', [_node('arith_primary', [$lhs])]);
}

# ============================================================================
# Variables, calls, designational expressions
# ============================================================================

# --- _parse_variable() --------------------------------------------------------
#
# Grammar: variable = IDENT [LBRACKET subscripts RBRACKET]
#
# Examples:
#   x          scalar variable
#   A[i]       one-dimensional array element
#   B[i, j]    two-dimensional array element

sub _parse_variable {
    my @children;
    push @children, _leaf( _expect('NAME') );
    if (_peek()->{type} eq 'LBRACKET') {
        push @children, _leaf(_advance());    # LBRACKET
        # subscripts: arith_expr {COMMA arith_expr}
        push @children, _parse_arith_expr();
        while (_peek()->{type} eq 'COMMA') {
            push @children, _leaf(_advance());
            push @children, _parse_arith_expr();
        }
        push @children, _leaf( _expect('RBRACKET') );
    }
    return _node('variable', \@children);
}

# --- _parse_proc_call() -------------------------------------------------------
#
# Grammar: proc_call = IDENT LPAREN actual_params RPAREN
#
# Used when a procedure call appears inside an expression (the result is used).

sub _parse_proc_call {
    my @children;
    push @children, _leaf( _expect('NAME')  );
    push @children, _leaf( _expect('LPAREN') );
    push @children, _parse_actual_params();
    push @children, _leaf( _expect('RPAREN') );
    return _node('proc_call', \@children);
}

# --- _parse_desig_expr() ------------------------------------------------------
#
# Grammar:
#   desig_expr = IF bool_expr THEN simple_desig ELSE desig_expr
#              | simple_desig
#
# A designational expression evaluates to a label (jump target).

sub _parse_desig_expr {
    if (_peek()->{type} eq 'IF') {
        my @ch;
        push @ch, _leaf(_advance());    # IF
        push @ch, _parse_bool_expr();
        push @ch, _leaf( _expect('THEN') );
        push @ch, _parse_simple_desig();
        push @ch, _leaf( _expect('ELSE') );
        push @ch, _parse_desig_expr();
        return _node('desig_expr', \@ch);
    }
    return _node('desig_expr', [_parse_simple_desig()]);
}

# --- _parse_simple_desig() ----------------------------------------------------
#
# Grammar:
#   simple_desig = IDENT LBRACKET arith_expr RBRACKET   (switch subscript)
#                | LPAREN desig_expr RPAREN              (parenthesized)
#                | label                                  (IDENT or INTEGER_LIT)

sub _parse_simple_desig {
    my $t    = _peek();
    my $type = $t->{type};

    if ($type eq 'LPAREN') {
        my @ch;
        push @ch, _leaf(_advance());    # LPAREN
        push @ch, _parse_desig_expr();
        push @ch, _leaf( _expect('RPAREN') );
        return _node('simple_desig', \@ch);
    }

    if ($type eq 'NAME' && _peek2()->{type} eq 'LBRACKET') {
        my @ch;
        push @ch, _leaf(_advance());    # IDENT
        push @ch, _leaf(_advance());    # LBRACKET
        push @ch, _parse_arith_expr();
        push @ch, _leaf( _expect('RBRACKET') );
        return _node('simple_desig', \@ch);
    }

    # Label: IDENT or INTEGER_LIT
    if ($type eq 'NAME' || $type eq 'INTEGER_LIT') {
        return _node('simple_desig', [_leaf(_advance())]);
    }

    die sprintf(
        "CodingAdventures::AlgolParser: Expected designational expression, got %s ('%s') "
      . "at line %d col %d",
        $type, $t->{value}, $t->{line}, $t->{col}
    );
}

1;

__END__

=head1 NAME

CodingAdventures::AlgolParser - Hand-written recursive-descent ALGOL 60 parser

=head1 SYNOPSIS

    use CodingAdventures::AlgolParser;

    my $ast = CodingAdventures::AlgolParser->parse('begin integer x; x := 42 end');
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

A hand-written recursive-descent parser for ALGOL 60.  Tokenizes source text
using C<CodingAdventures::AlgolLexer> and constructs an AST using
C<CodingAdventures::AlgolParser::ASTNode>.

Implements the ALGOL 60 grammar from C<algol.grammar>:

    program      = block
    block        = BEGIN {declaration SEMICOLON} statement {SEMICOLON statement} END
    declaration  = type_decl | array_decl | switch_decl | procedure_decl
    type_decl    = type ident_list
    statement    = [label COLON] unlabeled_stmt | [label COLON] cond_stmt
    cond_stmt    = IF bool_expr THEN unlabeled_stmt [ELSE statement]
    assign_stmt  = left_part {left_part} expression
    for_stmt     = FOR IDENT ASSIGN for_list DO statement
    expression   = arith_expr | bool_expr

=head1 METHODS

=head2 parse($source)

Parse an ALGOL 60 string.  Returns the root C<ASTNode> with
C<rule_name == "program">.  Dies on lexer or parser errors.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
