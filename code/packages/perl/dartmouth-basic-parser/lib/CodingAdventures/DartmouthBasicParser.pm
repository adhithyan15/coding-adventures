package CodingAdventures::DartmouthBasicParser;

# ============================================================================
# CodingAdventures::DartmouthBasicParser — Hand-written recursive-descent
#                                          Dartmouth BASIC parser
# ============================================================================
#
# This module parses 1964 Dartmouth BASIC source text into an Abstract Syntax
# Tree (AST). It uses a hand-written recursive-descent approach rather than
# the grammar-driven infrastructure used by the Lua implementation.
#
# # Why hand-written?
# -------------------
# The Perl CodingAdventures::GrammarTools module only provides
# `parse_token_grammar`, not `parse_parser_grammar`. There is no grammar-
# driven GrammarParser in the Perl layer, so we implement the parser by
# hand, following the same grammar rules as the Lua implementation.
#
# # What is Dartmouth BASIC?
# --------------------------
# Dartmouth BASIC was designed by John Kemeny and Thomas Kurtz at Dartmouth
# College in 1964. It was the world's first widely accessible programming
# language, running on the GE-225 time-sharing mainframe. The design goal
# was simple: humanities students — not just mathematicians — should be
# able to write programs.
#
# Key design choices:
#
#   - LINE-NUMBERED: every statement begins with a line number (10, 20, ...).
#     Programs are stored sorted by line number; you can type lines in any order.
#
#   - UPPERCASE ONLY: the original GE-225 teletypes had no lowercase.
#     The lexer normalises all input to uppercase, so LET, let, and Let
#     all produce the same KEYWORD("LET") token.
#
#   - FLAT STRUCTURE: no block structure, no nested functions. Control flow
#     is entirely via GOTO and GOSUB/RETURN with line-number targets.
#
#   - SIMPLE VARIABLES: A–Z (scalars) and A0–Z9 (scalars with numeric suffix).
#     Arrays are declared with DIM and accessed as A(I).
#
#   - ALL NUMBERS ARE FLOATS: 42 is stored as 42.0 internally. No integer type.
#
# Example program:
#
#   10 LET X = 1
#   20 PRINT X
#   30 LET X = X + 1
#   40 IF X <= 10 THEN 20
#   50 END
#
# # BASIC grammar (implemented by this module)
# --------------------------------------------
#
#   program      = { line }
#   line         = LINE_NUM [ statement ] NEWLINE
#   statement    = let_stmt | print_stmt | input_stmt | if_stmt | goto_stmt
#                | gosub_stmt | return_stmt | for_stmt | next_stmt | end_stmt
#                | stop_stmt | rem_stmt | read_stmt | data_stmt | restore_stmt
#                | dim_stmt | def_stmt
#
#   let_stmt     = LET variable EQ expr
#   print_stmt   = PRINT [ print_list ]
#   print_list   = print_item { (COMMA|SEMICOLON) print_item } [ COMMA|SEMICOLON ]
#   print_item   = STRING | expr
#   input_stmt   = INPUT variable { COMMA variable }
#   if_stmt      = IF expr relop expr THEN LINE_NUM
#   goto_stmt    = GOTO LINE_NUM
#   gosub_stmt   = GOSUB LINE_NUM
#   return_stmt  = RETURN
#   for_stmt     = FOR NAME EQ expr TO expr [ STEP expr ]
#   next_stmt    = NEXT NAME
#   end_stmt     = END
#   stop_stmt    = STOP
#   rem_stmt     = REM
#   read_stmt    = READ variable { COMMA variable }
#   data_stmt    = DATA NUMBER { COMMA NUMBER }
#   restore_stmt = RESTORE
#   dim_stmt     = DIM NAME LPAREN NUMBER RPAREN { COMMA NAME LPAREN NUMBER RPAREN }
#   def_stmt     = DEF USER_FN LPAREN NAME RPAREN EQ expr
#
#   variable     = NAME LPAREN expr RPAREN | NAME
#
#   expr         = term { (PLUS|MINUS) term }
#   term         = power { (STAR|SLASH) power }
#   power        = unary [ CARET power ]
#   unary        = MINUS primary | primary
#   primary      = NUMBER | BUILTIN_FN LPAREN expr RPAREN
#                | USER_FN LPAREN expr RPAREN | variable | LPAREN expr RPAREN
#
#   relop        = EQ | LT | GT | LE | GE | NE
#
# # How recursive descent works
# ------------------------------
# In recursive descent, each grammar rule becomes a subroutine. The subroutine
# reads tokens from the shared `$tokens_ref` array at position `$pos`.
#
#   Rule: let_stmt = LET variable EQ expr
#
#   sub _parse_let_stmt {
#       _expect_keyword('LET');    # consume KEYWORD("LET")
#       _parse_variable();         # recurse into variable rule
#       _expect('EQ');             # consume EQ
#       _parse_expr();             # recurse into expr rule
#   }
#
# When a rule has alternatives (|), we use _peek() to look at the current
# token and pick which alternative to apply. For example, `statement` has
# 17 alternatives, each starting with a distinct KEYWORD token value.
#
# When a rule has repetition ({ ... }), we use a `while` loop with a
# condition that checks whether the current token begins another repetition.
#
# When a rule has optionality ([ ... ]), we check the current token and
# skip the optional part if it doesn't match.
#
# # AST structure
# ---------------
# Internal nodes:
#   { rule_name => "program",  children => [...], is_leaf => 0 }
#   { rule_name => "line",     children => [...], is_leaf => 0 }
#   { rule_name => "let_stmt", children => [...], is_leaf => 0 }
#   ... etc.
#
# Leaf nodes (token wrappers):
#   { rule_name => "token", children => [], is_leaf => 1,
#     token => { type => "LINE_NUM", value => "10", line => 1, col => 1 } }
#
# # Parse state
# -------------
# The parser keeps two package-level variables:
#   $tokens_ref — arrayref of token hashrefs from DartmouthBasicLexer
#   $pos        — current position (0-based index into @$tokens_ref)
#
# These are set by `parse()` on each call. The parser is not re-entrant,
# but BASIC parsing is synchronous so that's fine in practice.

use strict;
use warnings;

use CodingAdventures::DartmouthBasicLexer;
use CodingAdventures::DartmouthBasicParser::ASTNode;

our $VERSION = '0.01';

# Package-level parse state.
# Set at the start of each `parse()` call.
my ($tokens_ref, $pos);

# ============================================================================
# Public API
# ============================================================================

# --- parse($class, $source) ---------------------------------------------------
#
# Parse a Dartmouth BASIC source string and return the root ASTNode.
#
# Steps:
#   1. Tokenize `$source` using CodingAdventures::DartmouthBasicLexer.
#      The lexer normalises to uppercase, relabels line-number positions,
#      and suppresses REM comment tokens.
#   2. Initialize parser state ($tokens_ref, $pos).
#   3. Parse the top-level `program` production.
#   4. Assert that we consumed all tokens (only EOF remains).
#   5. Return the root ASTNode with rule_name "program".
#
# Dies on lexer errors or parse errors.
#
# @param  $source  string  The Dartmouth BASIC text to parse.
# @return ASTNode          Root node with rule_name "program".

sub parse {
    my ($class, $source) = @_;

    # Tokenize — the lexer handles case normalisation, LINE_NUM relabelling,
    # and REM content suppression. By the time we see the token stream, all
    # context-sensitive issues are resolved.
    my $toks = CodingAdventures::DartmouthBasicLexer->tokenize($source);
    $tokens_ref = $toks;
    $pos = 0;

    # Parse the top-level program production
    my $ast = _parse_program();

    # Verify we consumed everything — only EOF should remain.
    # In BASIC this is normal: each line ends with NEWLINE, and the token
    # stream ends with EOF after the last line.
    my $t = _peek();
    if ($t->{type} ne 'EOF') {
        die sprintf(
            "CodingAdventures::DartmouthBasicParser: trailing content at "
          . "line %d col %d: unexpected %s ('%s')",
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
#   _expect('EQ')  — dies "Expected EQ, got NAME ('X') at line 1 col 11"

sub _expect {
    my ($type) = @_;
    my $t = _peek();
    unless ($t->{type} eq $type) {
        die sprintf(
            "CodingAdventures::DartmouthBasicParser: Expected %s, got %s ('%s') "
          . "at line %d col %d",
            $type, $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _advance();
}

# --- _expect_keyword($value) --------------------------------------------------
#
# Assert that the current token is KEYWORD with the given value.
# Dies with a descriptive message if it doesn't match.
#
# BASIC keywords are stored as: { type => "KEYWORD", value => "LET" }
# We need to check both the type and the value.
#
# Example:
#   _expect_keyword('LET')  — dies if current token is not KEYWORD("LET")

sub _expect_keyword {
    my ($value) = @_;
    my $t = _peek();
    unless ($t->{type} eq 'KEYWORD' && $t->{value} eq $value) {
        die sprintf(
            "CodingAdventures::DartmouthBasicParser: Expected KEYWORD('%s'), "
          . "got %s ('%s') at line %d col %d",
            $value, $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }
    return _advance();
}

# --- _peek_keyword($value) ----------------------------------------------------
#
# Return true if the current token is KEYWORD with the given value,
# without consuming it. Used for lookahead in optional clauses.

sub _peek_keyword {
    my ($value) = @_;
    my $t = _peek();
    return $t->{type} eq 'KEYWORD' && $t->{value} eq $value;
}

# --- _node($rule_name, $children_aref) ----------------------------------------
#
# Construct an internal (non-leaf) ASTNode.
#
# @param $rule_name  string    The grammar rule that produced this node.
# @param $children   arrayref  Child ASTNodes.
# @return ASTNode

sub _node {
    my ($rule, $children) = @_;
    return CodingAdventures::DartmouthBasicParser::ASTNode->new(
        rule_name => $rule,
        children  => $children,
        is_leaf   => 0,
    );
}

# --- _leaf($token) ------------------------------------------------------------
#
# Construct a leaf ASTNode wrapping a single token.
#
# @param $token  hashref  A token from CodingAdventures::DartmouthBasicLexer.
# @return ASTNode

sub _leaf {
    my ($tok) = @_;
    return CodingAdventures::DartmouthBasicParser::ASTNode->new(
        rule_name => 'token',
        children  => [],
        is_leaf   => 1,
        token     => $tok,
    );
}

# ============================================================================
# Recursive descent productions
# ============================================================================
#
# Each sub below corresponds to one rule in the BASIC grammar. The naming
# convention is _parse_<rule_name>().
#
# Reading guide:
#   _expect(TYPE)         → consume a token of that TYPE or die
#   _expect_keyword(VAL)  → consume KEYWORD(VAL) or die
#   _peek()->{type}       → look at current token type without consuming
#   _peek_keyword(VAL)    → check current token is KEYWORD(VAL)
#   _advance()            → consume current token and return it
#   _leaf(_advance())     → consume and wrap token as leaf ASTNode
#   _node('rule', \@kids) → create internal ASTNode with children

# --- _parse_program() ---------------------------------------------------------
#
# Grammar: program = { line }
#
# A program is zero or more lines. The loop stops when EOF is reached.
# An empty program (no lines, just EOF) is valid BASIC — it represents
# an empty file or a session with no stored program.
#
# The `{ line }` repetition in EBNF means "zero or more". We implement it
# as a while loop that continues as long as the current token is not EOF.
# Each iteration parses exactly one numbered line.

sub _parse_program {
    my @children;

    # Collect lines until we hit EOF.
    # Each line starts with LINE_NUM, so we keep going while LINE_NUM is next.
    # We stop at EOF (also handles empty input).
    while (_peek()->{type} ne 'EOF') {
        push @children, _parse_line();
    }

    return _node('program', \@children);
}

# --- _parse_line() ------------------------------------------------------------
#
# Grammar: line = LINE_NUM [ statement ] NEWLINE
#
# Every BASIC line begins with a line number (LINE_NUM), followed by an
# optional statement, and terminated by NEWLINE.
#
# The `[ statement ]` optional part: we look at the token after LINE_NUM.
# If it is NEWLINE or EOF, there is no statement (bare line number). This
# is valid BASIC — in interactive mode, a bare line number deletes that line.
# In a stored program, it's a no-op.
#
# Why EOF as a fallback? If the source ends without a trailing newline,
# the lexer adds a synthetic NEWLINE. But as extra safety we check EOF too.

sub _parse_line {
    my @children;

    # Consume the line number that begins this line.
    my $linenum = _expect('LINE_NUM');
    push @children, _leaf($linenum);

    # Optional statement: present unless the next token is NEWLINE.
    my $next_type = _peek()->{type};
    if ($next_type ne 'NEWLINE' && $next_type ne 'EOF') {
        push @children, _parse_statement();
    }

    # Each line ends with NEWLINE.
    # (The lexer ensures a trailing NEWLINE for the last line if missing.)
    my $nl = _expect('NEWLINE');
    push @children, _leaf($nl);

    return _node('line', \@children);
}

# --- _parse_statement() -------------------------------------------------------
#
# Grammar: statement = let_stmt | print_stmt | input_stmt | if_stmt
#                    | goto_stmt | gosub_stmt | return_stmt | for_stmt
#                    | next_stmt | end_stmt | stop_stmt | rem_stmt
#                    | read_stmt | data_stmt | restore_stmt | dim_stmt
#                    | def_stmt
#
# Decision table: the first token of every statement is KEYWORD, and each
# BASIC keyword maps to exactly one statement type. So we peek at the
# current KEYWORD value and dispatch directly.
#
# This is an LL(1) grammar for statements: one token of lookahead is enough
# to decide which production to use.

sub _parse_statement {
    my $t = _peek();

    # Only KEYWORD tokens introduce statements. Any other type here is a
    # parse error — line content must begin with a BASIC keyword.
    unless ($t->{type} eq 'KEYWORD') {
        die sprintf(
            "CodingAdventures::DartmouthBasicParser: expected KEYWORD to begin "
          . "statement, got %s ('%s') at line %d col %d",
            $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }

    my $kw = $t->{value};

    # Dispatch table: keyword → parser sub
    # The 17 BASIC statement types, in the order they appear in the grammar.
    my $stmt;
    if    ($kw eq 'LET')     { $stmt = _parse_let_stmt()     }
    elsif ($kw eq 'PRINT')   { $stmt = _parse_print_stmt()   }
    elsif ($kw eq 'INPUT')   { $stmt = _parse_input_stmt()   }
    elsif ($kw eq 'IF')      { $stmt = _parse_if_stmt()      }
    elsif ($kw eq 'GOTO')    { $stmt = _parse_goto_stmt()    }
    elsif ($kw eq 'GOSUB')   { $stmt = _parse_gosub_stmt()   }
    elsif ($kw eq 'RETURN')  { $stmt = _parse_return_stmt()  }
    elsif ($kw eq 'FOR')     { $stmt = _parse_for_stmt()     }
    elsif ($kw eq 'NEXT')    { $stmt = _parse_next_stmt()    }
    elsif ($kw eq 'END')     { $stmt = _parse_end_stmt()     }
    elsif ($kw eq 'STOP')    { $stmt = _parse_stop_stmt()    }
    elsif ($kw eq 'REM')     { $stmt = _parse_rem_stmt()     }
    elsif ($kw eq 'READ')    { $stmt = _parse_read_stmt()    }
    elsif ($kw eq 'DATA')    { $stmt = _parse_data_stmt()    }
    elsif ($kw eq 'RESTORE') { $stmt = _parse_restore_stmt() }
    elsif ($kw eq 'DIM')     { $stmt = _parse_dim_stmt()     }
    elsif ($kw eq 'DEF')     { $stmt = _parse_def_stmt()     }
    else {
        die sprintf(
            "CodingAdventures::DartmouthBasicParser: unknown BASIC keyword '%s' "
          . "at line %d col %d",
            $kw, $t->{line}, $t->{col}
        );
    }

    return _node('statement', [$stmt]);
}

# ============================================================================
# Statement parsers — one per grammar rule
# ============================================================================

# --- _parse_let_stmt() --------------------------------------------------------
#
# Grammar: let_stmt = LET variable EQ expr
#
# LET assigns a value to a scalar or array variable:
#   10 LET X = 5
#   20 LET A(3) = X + 1
#
# The = in LET is ALWAYS assignment — never comparison. Comparison is only
# in IF statements, handled by _parse_relop().

sub _parse_let_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('LET'));
    push @children, _parse_variable();
    push @children, _leaf(_expect('EQ'));
    push @children, _parse_expr();
    return _node('let_stmt', \@children);
}

# --- _parse_print_stmt() ------------------------------------------------------
#
# Grammar: print_stmt = PRINT [ print_list ]
#
# PRINT with no arguments outputs a blank line.
# PRINT with a list outputs each item, with:
#   COMMA     → advance to the next print zone (about 14 characters wide)
#   SEMICOLON → continue printing immediately (no space)
#
# In the token stream, PRINT is followed by NEWLINE if no arguments.
# We check for NEWLINE (or EOF) to decide whether to parse print_list.

sub _parse_print_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('PRINT'));

    # Optional print list: present unless we're at end of statement.
    my $next = _peek()->{type};
    if ($next ne 'NEWLINE' && $next ne 'EOF') {
        push @children, _parse_print_list();
    }

    return _node('print_stmt', \@children);
}

# --- _parse_print_list() ------------------------------------------------------
#
# Grammar: print_list = print_item { (COMMA|SEMICOLON) print_item } [ COMMA|SEMICOLON ]
#
# A print list is one or more print items separated by COMMA or SEMICOLON.
# A trailing separator (no final item) suppresses the newline at runtime.
#
# Examples:
#   PRINT X, Y        → two items, zone-separated
#   PRINT X; Y        → two items, concatenated
#   PRINT "HELLO",    → trailing comma, no newline at end of line

sub _parse_print_list {
    my @children;

    # First print item
    push @children, _parse_print_item();

    # Additional items: COMMA or SEMICOLON followed by a print item.
    # A trailing separator (COMMA or SEMICOLON at end of line) is allowed —
    # it suppresses the final newline. We check for that case.
    while (_peek()->{type} eq 'COMMA' || _peek()->{type} eq 'SEMICOLON') {
        push @children, _leaf(_advance());    # consume COMMA or SEMICOLON

        # Trailing separator: if next is NEWLINE, stop here (no item follows).
        my $next = _peek()->{type};
        last if $next eq 'NEWLINE' || $next eq 'EOF';

        push @children, _parse_print_item();
    }

    return _node('print_list', \@children);
}

# --- _parse_print_item() ------------------------------------------------------
#
# Grammar: print_item = STRING | expr
#
# A print item is either a string literal or an arithmetic expression.
# Strings are printed as-is; expressions are evaluated and printed.
#
# Decision: if the current token is STRING, consume it as a leaf.
# Otherwise, parse an expression.

sub _parse_print_item {
    my @children;
    if (_peek()->{type} eq 'STRING') {
        push @children, _leaf(_advance());
    } else {
        push @children, _parse_expr();
    }
    return _node('print_item', \@children);
}

# --- _parse_input_stmt() ------------------------------------------------------
#
# Grammar: input_stmt = INPUT variable { COMMA variable }
#
# INPUT reads values from the user (or from the :input_queue in tests).
# Multiple variables are separated by commas:
#   10 INPUT A, B, C    — reads three values

sub _parse_input_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('INPUT'));
    push @children, _parse_variable();

    while (_peek()->{type} eq 'COMMA') {
        push @children, _leaf(_advance());    # consume COMMA
        push @children, _parse_variable();
    }

    return _node('input_stmt', \@children);
}

# --- _parse_if_stmt() ---------------------------------------------------------
#
# Grammar: if_stmt = IF expr relop expr THEN LINE_NUM
#
# The 1964 Dartmouth BASIC IF has exactly this form — no ELSE, no IF...THEN
# statement (those came in later dialects). The branch target must be a
# literal line number, not an expression.
#
#   10 IF X > 0 THEN 100
#   20 IF A = B THEN 50
#   30 IF X <> Y THEN 70
#
# Note: the THEN token is a KEYWORD, and LINE_NUM is a special token type.
# Wait — in IF statements, the target number after THEN is NOT at the start
# of a line, so the lexer emits it as NUMBER (not LINE_NUM). We handle both
# to be safe, but the spec says it's NUMBER in the token stream here.
#
# Actually, re-reading the lexer: LINE_NUM is only set for the FIRST number
# token on each line. The number after THEN is not at line-start, so it is
# a regular NUMBER. The grammar uses LINE_NUM for the target. Let's check:
# The grammar file says: if_stmt = "IF" expr relop expr "THEN" LINE_NUM
# The lexer relabels at line-start only. So THEN 20 → KEYWORD(THEN) NUMBER(20).
# But the grammar expects LINE_NUM. This is a known quirk: in the grammar file,
# LINE_NUM is used consistently for "a line number reference", but in the token
# stream these are actually NUMBER after THEN, GOTO, GOSUB.
#
# To handle this correctly, we accept both NUMBER and LINE_NUM for the target.

sub _parse_if_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('IF'));
    push @children, _parse_expr();
    push @children, _parse_relop();
    push @children, _parse_expr();
    push @children, _leaf(_expect_keyword('THEN'));

    # The target line number: emitted as NUMBER (not at line start).
    my $t = _peek();
    if ($t->{type} eq 'NUMBER' || $t->{type} eq 'LINE_NUM') {
        push @children, _leaf(_advance());
    } else {
        die sprintf(
            "CodingAdventures::DartmouthBasicParser: expected line number after THEN, "
          . "got %s ('%s') at line %d col %d",
            $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }

    return _node('if_stmt', \@children);
}

# --- _parse_goto_stmt() -------------------------------------------------------
#
# Grammar: goto_stmt = GOTO LINE_NUM
#
# Unconditional jump to a named line:
#   10 GOTO 50
#
# The target is a NUMBER in the token stream (not at line-start).

sub _parse_goto_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('GOTO'));

    my $t = _peek();
    if ($t->{type} eq 'NUMBER' || $t->{type} eq 'LINE_NUM') {
        push @children, _leaf(_advance());
    } else {
        die sprintf(
            "CodingAdventures::DartmouthBasicParser: expected line number after GOTO, "
          . "got %s ('%s') at line %d col %d",
            $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }

    return _node('goto_stmt', \@children);
}

# --- _parse_gosub_stmt() ------------------------------------------------------
#
# Grammar: gosub_stmt = GOSUB LINE_NUM
#
# Call a subroutine by jumping to a line number and pushing the return address:
#   10 GOSUB 200

sub _parse_gosub_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('GOSUB'));

    my $t = _peek();
    if ($t->{type} eq 'NUMBER' || $t->{type} eq 'LINE_NUM') {
        push @children, _leaf(_advance());
    } else {
        die sprintf(
            "CodingAdventures::DartmouthBasicParser: expected line number after GOSUB, "
          . "got %s ('%s') at line %d col %d",
            $t->{type}, $t->{value}, $t->{line}, $t->{col}
        );
    }

    return _node('gosub_stmt', \@children);
}

# --- _parse_return_stmt() -----------------------------------------------------
#
# Grammar: return_stmt = RETURN
#
# Return from a subroutine to the address pushed by the matching GOSUB:
#   210 RETURN

sub _parse_return_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('RETURN'));
    return _node('return_stmt', \@children);
}

# --- _parse_for_stmt() --------------------------------------------------------
#
# Grammar: for_stmt = FOR NAME EQ expr TO expr [ STEP expr ]
#
# A counted loop. The loop variable must be a scalar NAME (not an array element).
# STEP defaults to 1 if omitted. A negative STEP counts downward.
#
#   10 FOR I = 1 TO 10
#   20 FOR I = 10 TO 1 STEP -1
#   30 FOR X = 0 TO 100 STEP 5

sub _parse_for_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('FOR'));
    push @children, _leaf(_expect('NAME'));
    push @children, _leaf(_expect('EQ'));
    push @children, _parse_expr();             # start value
    push @children, _leaf(_expect_keyword('TO'));
    push @children, _parse_expr();             # end value

    # Optional STEP clause
    if (_peek_keyword('STEP')) {
        push @children, _leaf(_expect_keyword('STEP'));
        push @children, _parse_expr();         # step value (may be negative)
    }

    return _node('for_stmt', \@children);
}

# --- _parse_next_stmt() -------------------------------------------------------
#
# Grammar: next_stmt = NEXT NAME
#
# End of a FOR loop. The NAME must match the loop variable from the
# corresponding FOR statement. The VM uses a stack to match FOR/NEXT pairs.
#
#   30 NEXT I

sub _parse_next_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('NEXT'));
    push @children, _leaf(_expect('NAME'));
    return _node('next_stmt', \@children);
}

# --- _parse_end_stmt() --------------------------------------------------------
#
# Grammar: end_stmt = END
#
# Normal program termination. The VM halts when it executes END.

sub _parse_end_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('END'));
    return _node('end_stmt', \@children);
}

# --- _parse_stop_stmt() -------------------------------------------------------
#
# Grammar: stop_stmt = STOP
#
# Halt with a "STOP IN LINE n" message in the original DTSS system.
# In our VM, STOP and END both terminate execution.

sub _parse_stop_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('STOP'));
    return _node('stop_stmt', \@children);
}

# --- _parse_rem_stmt() --------------------------------------------------------
#
# Grammar: rem_stmt = REM
#
# Programmer remark (comment). The lexer's _suppress_rem_content post-
# processing already dropped all tokens between REM and NEWLINE. So by
# the time the parser sees this token stream, a REM line looks like:
#   LINE_NUM(10) KEYWORD(REM) NEWLINE
#
# The rem_stmt rule has an empty body — just the REM keyword token.

sub _parse_rem_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('REM'));
    return _node('rem_stmt', \@children);
}

# --- _parse_read_stmt() -------------------------------------------------------
#
# Grammar: read_stmt = READ variable { COMMA variable }
#
# Read values sequentially from the DATA pool into variables:
#   10 READ X
#   20 READ A, B, C

sub _parse_read_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('READ'));
    push @children, _parse_variable();

    while (_peek()->{type} eq 'COMMA') {
        push @children, _leaf(_advance());    # consume COMMA
        push @children, _parse_variable();
    }

    return _node('read_stmt', \@children);
}

# --- _parse_data_stmt() -------------------------------------------------------
#
# Grammar: data_stmt = DATA NUMBER { COMMA NUMBER }
#
# Define a pool of numeric data values that READ statements consume:
#   10 DATA 1, 2, 3, 4, 5
#   20 DATA 3.14
#
# DATA values are always numeric in the 1964 spec. String DATA came later.

sub _parse_data_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('DATA'));
    push @children, _leaf(_expect('NUMBER'));

    while (_peek()->{type} eq 'COMMA') {
        push @children, _leaf(_advance());    # consume COMMA
        push @children, _leaf(_expect('NUMBER'));
    }

    return _node('data_stmt', \@children);
}

# --- _parse_restore_stmt() ----------------------------------------------------
#
# Grammar: restore_stmt = RESTORE
#
# Reset the DATA pool pointer to the beginning. The next READ will start
# from the first DATA value again.

sub _parse_restore_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('RESTORE'));
    return _node('restore_stmt', \@children);
}

# --- _parse_dim_stmt() --------------------------------------------------------
#
# Grammar: dim_stmt = DIM NAME LPAREN NUMBER RPAREN { COMMA NAME LPAREN NUMBER RPAREN }
#
# Declare one or more arrays with their maximum index:
#   10 DIM A(10)           — A can be used as A(0) through A(10)
#   20 DIM A(10), B(20)    — declare two arrays
#
# Without DIM, arrays default to size 10 (indices 0–10).
# The size must be a literal integer (not an expression).

sub _parse_dim_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('DIM'));

    # Parse first dim_decl
    push @children, _parse_dim_decl();

    # Parse additional dim_decls separated by COMMA
    while (_peek()->{type} eq 'COMMA') {
        push @children, _leaf(_advance());    # consume COMMA
        push @children, _parse_dim_decl();
    }

    return _node('dim_stmt', \@children);
}

# --- _parse_dim_decl() --------------------------------------------------------
#
# Grammar: dim_decl = NAME LPAREN NUMBER RPAREN
#
# A single array declaration: name and size.
# Example: A(10) means array A with maximum index 10.

sub _parse_dim_decl {
    my @children;
    push @children, _leaf(_expect('NAME'));
    push @children, _leaf(_expect('LPAREN'));
    push @children, _leaf(_expect('NUMBER'));
    push @children, _leaf(_expect('RPAREN'));
    return _node('dim_decl', \@children);
}

# --- _parse_def_stmt() --------------------------------------------------------
#
# Grammar: def_stmt = DEF USER_FN LPAREN NAME RPAREN EQ expr
#
# Define a user function with a single formal parameter:
#   10 DEF FNA(X) = X * X
#   20 DEF FNB(T) = SIN(T) / COS(T)
#
# Function names range from FNA through FNZ (26 functions total).
# The body can reference the formal parameter and global variables.
# User functions are called as: FNA(expression).

sub _parse_def_stmt {
    my @children;
    push @children, _leaf(_expect_keyword('DEF'));
    push @children, _leaf(_expect('USER_FN'));
    push @children, _leaf(_expect('LPAREN'));
    push @children, _leaf(_expect('NAME'));
    push @children, _leaf(_expect('RPAREN'));
    push @children, _leaf(_expect('EQ'));
    push @children, _parse_expr();
    return _node('def_stmt', \@children);
}

# ============================================================================
# Variable parser
# ============================================================================

# --- _parse_variable() --------------------------------------------------------
#
# Grammar: variable = NAME LPAREN expr RPAREN | NAME
#
# A variable is either:
#   - A scalar:  X, A1, Z9
#   - An array element: A(I), B(I+1)
#
# We must try the array form first, because the scalar form would match
# and leave "(I)" unparsed. We use 2-token lookahead:
#   if current is NAME and next is LPAREN → array form
#   otherwise → scalar form
#
# Lookahead: peek at $tokens_ref->[$pos+1] (the token after NAME) to see
# if it's LPAREN. This avoids consuming NAME before we know which branch.

sub _parse_variable {
    my @children;

    my $name_tok = _peek();
    unless ($name_tok->{type} eq 'NAME') {
        die sprintf(
            "CodingAdventures::DartmouthBasicParser: expected NAME for variable, "
          . "got %s ('%s') at line %d col %d",
            $name_tok->{type}, $name_tok->{value},
            $name_tok->{line}, $name_tok->{col}
        );
    }

    # Lookahead: is the token after NAME an LPAREN?
    my $next = $tokens_ref->[$pos + 1] // $tokens_ref->[-1];

    if ($next->{type} eq 'LPAREN') {
        # Array element: NAME LPAREN expr RPAREN
        push @children, _leaf(_advance());    # consume NAME
        push @children, _leaf(_advance());    # consume LPAREN
        push @children, _parse_expr();
        push @children, _leaf(_expect('RPAREN'));
    } else {
        # Scalar: NAME
        push @children, _leaf(_advance());    # consume NAME
    }

    return _node('variable', \@children);
}

# ============================================================================
# Relational operator parser
# ============================================================================

# --- _parse_relop() -----------------------------------------------------------
#
# Grammar: relop = EQ | LT | GT | LE | GE | NE
#
# The six relational operators used in IF statements.
# The lexer emits each as its own distinct token type.
#
# Precedence note: LE (<=), GE (>=), NE (<>) are multi-character operators
# that must be matched before LT (<), GT (>), EQ (=) by the lexer. This is
# handled by the lexer, not the parser — by the time we see the token stream,
# each operator is already the correct type.
#
# All six are valid relops in BASIC:
#   =   equal
#   <   less than
#   >   greater than
#   <=  less than or equal
#   >=  greater than or equal
#   <>  not equal

sub _parse_relop {
    my $t = _peek();
    my $type = $t->{type};

    if ($type =~ /^(EQ|LT|GT|LE|GE|NE)$/) {
        return _node('relop', [_leaf(_advance())]);
    }

    die sprintf(
        "CodingAdventures::DartmouthBasicParser: expected relational operator, "
      . "got %s ('%s') at line %d col %d",
        $type, $t->{value}, $t->{line}, $t->{col}
    );
}

# ============================================================================
# Expression parsers — arithmetic with standard precedence
# ============================================================================
#
# Precedence cascade (lowest binding to highest):
#
#   expr   → addition (+) and subtraction (−), left-associative
#     term → multiplication (*) and division (/), left-associative
#       power → exponentiation (^), RIGHT-associative
#         unary → unary minus (−)
#           primary → atoms: NUMBER, function calls, variables, (expr)
#
# Right-associativity of ^ means:  2 ^ 3 ^ 2 = 2 ^ (3^2) = 512
# This matches the 1964 Dartmouth BASIC specification.

# --- _parse_expr() ------------------------------------------------------------
#
# Grammar: expr = term { (PLUS|MINUS) term }
#
# Addition and subtraction, left-associative.
# The { ... } repetition means "zero or more".
# Example: A + B - C + D parses as ((A + B) - C) + D

sub _parse_expr {
    my @children;

    push @children, _parse_term();

    # Zero or more (PLUS|MINUS term) repetitions
    while (_peek()->{type} eq 'PLUS' || _peek()->{type} eq 'MINUS') {
        push @children, _leaf(_advance());    # consume PLUS or MINUS
        push @children, _parse_term();
    }

    return _node('expr', \@children);
}

# --- _parse_term() ------------------------------------------------------------
#
# Grammar: term = power { (STAR|SLASH) power }
#
# Multiplication and division, left-associative.
# Example: A * B / C * D parses as ((A * B) / C) * D

sub _parse_term {
    my @children;

    push @children, _parse_power();

    # Zero or more (STAR|SLASH power) repetitions
    while (_peek()->{type} eq 'STAR' || _peek()->{type} eq 'SLASH') {
        push @children, _leaf(_advance());    # consume STAR or SLASH
        push @children, _parse_power();
    }

    return _node('term', \@children);
}

# --- _parse_power() -----------------------------------------------------------
#
# Grammar: power = unary [ CARET power ]
#
# Exponentiation, RIGHT-associative.
# The [ CARET power ] optional part recurses on itself:
#   2 ^ 3 ^ 2  →  power(2, CARET, power(3, CARET, power(2)))
#                = 2 ^ (3 ^ 2) = 2 ^ 9 = 512
#
# Contrast with left-associative ((2^3)^2 = 8^2 = 64).
# The 1964 Dartmouth BASIC spec mandates right-associativity for ^.

sub _parse_power {
    my @children;

    push @children, _parse_unary();

    # Optional CARET followed by a recursive power (right-associative)
    if (_peek()->{type} eq 'CARET') {
        push @children, _leaf(_advance());    # consume CARET
        push @children, _parse_power();       # right-recursion for right-assoc
    }

    return _node('power', \@children);
}

# --- _parse_unary() -----------------------------------------------------------
#
# Grammar: unary = MINUS primary | primary
#
# Unary minus: -X, -3.14, -(X + 1).
# Unary PLUS is not in the 1964 spec (it was added in later dialects).
#
# Implementation note: for nested negation like --X, the parser would see
# MINUS primary where primary starts with MINUS again. This isn't valid
# 1964 BASIC, but the grammar is technically recursive: _parse_unary would
# only be called from _parse_power, not from within _parse_unary itself.
# So double negation would require an explicit: 0 - (0 - X).

sub _parse_unary {
    my @children;

    if (_peek()->{type} eq 'MINUS') {
        push @children, _leaf(_advance());    # consume MINUS
        push @children, _parse_primary();
        return _node('unary', \@children);
    }

    # No unary minus: delegate directly to primary
    push @children, _parse_primary();
    return _node('unary', \@children);
}

# --- _parse_primary() ---------------------------------------------------------
#
# Grammar: primary = NUMBER
#                  | BUILTIN_FN LPAREN expr RPAREN
#                  | USER_FN LPAREN expr RPAREN
#                  | variable
#                  | LPAREN expr RPAREN
#
# Decision table (current token type → production):
#
#   NUMBER     → leaf node wrapping the NUMBER token
#   BUILTIN_FN → leaf(BUILTIN_FN) leaf(LPAREN) parse_expr() leaf(RPAREN)
#   USER_FN    → leaf(USER_FN)    leaf(LPAREN) parse_expr() leaf(RPAREN)
#   NAME       → _parse_variable() (handles both scalar and array)
#   LPAREN     → leaf(LPAREN) parse_expr() leaf(RPAREN)
#   anything else → die
#
# BUILTIN_FN and USER_FN come before NAME because the lexer emits them as
# distinct token types (SIN is BUILTIN_FN, FNA is USER_FN, X is NAME).
# There is no ambiguity at the token level.

sub _parse_primary {
    my @children;
    my $t    = _peek();
    my $type = $t->{type};

    if ($type eq 'NUMBER') {
        # Numeric literal: 42, 3.14, 1.5E3
        push @children, _leaf(_advance());
        return _node('primary', \@children);
    }

    if ($type eq 'BUILTIN_FN') {
        # Built-in function call: SIN(X), ABS(Y-1), INT(X+0.5)
        push @children, _leaf(_advance());          # consume BUILTIN_FN
        push @children, _leaf(_expect('LPAREN'));
        push @children, _parse_expr();
        push @children, _leaf(_expect('RPAREN'));
        return _node('primary', \@children);
    }

    if ($type eq 'USER_FN') {
        # User-defined function call: FNA(X), FNZ(I*2)
        push @children, _leaf(_advance());          # consume USER_FN
        push @children, _leaf(_expect('LPAREN'));
        push @children, _parse_expr();
        push @children, _leaf(_expect('RPAREN'));
        return _node('primary', \@children);
    }

    if ($type eq 'NAME') {
        # Variable (scalar or array): X, A1, A(I)
        push @children, _parse_variable();
        return _node('primary', \@children);
    }

    if ($type eq 'LPAREN') {
        # Parenthesised sub-expression: (X + 1), (A * B + C)
        push @children, _leaf(_advance());          # consume LPAREN
        push @children, _parse_expr();
        push @children, _leaf(_expect('RPAREN'));
        return _node('primary', \@children);
    }

    die sprintf(
        "CodingAdventures::DartmouthBasicParser: unexpected token %s ('%s') "
      . "in expression at line %d col %d",
        $type, $t->{value}, $t->{line}, $t->{col}
    );
}

1;

__END__

=head1 NAME

CodingAdventures::DartmouthBasicParser - Hand-written recursive-descent 1964 Dartmouth BASIC parser

=head1 SYNOPSIS

    use CodingAdventures::DartmouthBasicParser;

    my $ast = CodingAdventures::DartmouthBasicParser->parse(
        "10 LET X = 5\n20 PRINT X\n30 END\n"
    );
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

A hand-written recursive-descent parser for 1964 Dartmouth BASIC. Tokenizes
source text using C<CodingAdventures::DartmouthBasicLexer> and constructs an
AST using C<CodingAdventures::DartmouthBasicParser::ASTNode>.

Implements all 17 BASIC statement types plus expression precedence rules.

=head1 GRAMMAR

    program      = { line }
    line         = LINE_NUM [ statement ] NEWLINE
    statement    = let_stmt | print_stmt | ... | def_stmt  (17 alternatives)

    let_stmt     = LET variable EQ expr
    print_stmt   = PRINT [ print_list ]
    input_stmt   = INPUT variable { COMMA variable }
    if_stmt      = IF expr relop expr THEN LINE_NUM
    goto_stmt    = GOTO LINE_NUM
    gosub_stmt   = GOSUB LINE_NUM
    return_stmt  = RETURN
    for_stmt     = FOR NAME EQ expr TO expr [ STEP expr ]
    next_stmt    = NEXT NAME
    end_stmt     = END
    stop_stmt    = STOP
    rem_stmt     = REM
    read_stmt    = READ variable { COMMA variable }
    data_stmt    = DATA NUMBER { COMMA NUMBER }
    restore_stmt = RESTORE
    dim_stmt     = DIM NAME LPAREN NUMBER RPAREN { COMMA NAME LPAREN NUMBER RPAREN }
    def_stmt     = DEF USER_FN LPAREN NAME RPAREN EQ expr

    variable     = NAME LPAREN expr RPAREN | NAME
    expr         = term { (PLUS|MINUS) term }
    term         = power { (STAR|SLASH) power }
    power        = unary [ CARET power ]
    unary        = MINUS primary | primary
    primary      = NUMBER | BUILTIN_FN LPAREN expr RPAREN
                 | USER_FN LPAREN expr RPAREN | variable | LPAREN expr RPAREN
    relop        = EQ | LT | GT | LE | GE | NE

=head1 METHODS

=head2 parse($source)

Parse a Dartmouth BASIC string. Returns the root C<ASTNode> with
C<rule_name == "program">. Dies on lexer or parser errors.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
