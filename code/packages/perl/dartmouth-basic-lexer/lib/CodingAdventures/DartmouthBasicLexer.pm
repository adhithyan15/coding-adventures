package CodingAdventures::DartmouthBasicLexer;

# ============================================================================
# CodingAdventures::DartmouthBasicLexer — Grammar-driven Dartmouth BASIC tokenizer
# ============================================================================
#
# This module tokenizes 1964 Dartmouth BASIC source text into a flat list of
# token hashrefs.  It is a thin wrapper around the grammar infrastructure in
# CodingAdventures::GrammarTools.  It reads the shared `dartmouth_basic.tokens`
# grammar file, compiles token definitions into Perl regexes, and applies
# them in priority order.
#
# # What is Dartmouth BASIC?
# ==========================
#
# BASIC (Beginner's All-purpose Symbolic Instruction Code) was designed in
# 1964 by John Kemeny and Thomas Kurtz at Dartmouth College.  Their goal was
# to give humanities and social-science students — not just mathematicians —
# access to the college's GE-225 mainframe.
#
# Key design choices:
#
#   - LINE-NUMBERED: every statement begins with a line number.
#     `10 LET X = 5` names this line "10".  Lines are sorted by number
#     before execution, so you can type them in any order.
#
#   - TELETYPE-FRIENDLY: the GE-225 used uppercase-only teletypes.
#     The entire language is uppercase.  `print`, `Print`, and `PRINT`
#     are identical.  We normalise all source to uppercase before lexing.
#
#   - FLAT STRUCTURE: no blocks, no functions except DEF.
#     Control flow is entirely by GOTO and GOSUB with line-number targets.
#
#   - PRE-INITIALIZED VARIABLES: every variable is 0 until assigned.
#     No declaration needed — just use it.
#
# Example program:
#
#   10 LET X = 1
#   20 PRINT X
#   30 LET X = X + 1
#   40 IF X <= 10 THEN 20
#   50 END
#
# This produces the token stream (simplified):
#
#   LINE_NUM(10) KEYWORD(LET) NAME(X) EQ(=) NUMBER(1) NEWLINE
#   LINE_NUM(20) KEYWORD(PRINT) NAME(X) NEWLINE
#   LINE_NUM(30) KEYWORD(LET) NAME(X) EQ(=) NAME(X) PLUS(+) NUMBER(1) NEWLINE
#   LINE_NUM(40) KEYWORD(IF) NAME(X) LE(<=) NUMBER(10) KEYWORD(THEN) NUMBER(20) NEWLINE
#   LINE_NUM(50) KEYWORD(END) NEWLINE
#   EOF
#
# # Token types
# =============
#
# LINE_NUM   — digits at the very start of a source line (relabelled from NUMBER)
# NUMBER     — numeric literal: 42, 3.14, 1.5E3, .5
# STRING     — double-quoted string: "HELLO WORLD" (no escape sequences)
# KEYWORD    — reserved word: LET, PRINT, IF, THEN, GOTO, etc.
# BUILTIN_FN — one of the 11 built-in math functions: SIN, COS, LOG, etc.
# USER_FN    — user-defined function name: FNA, FNB, ..., FNZ
# NAME       — variable name: one letter + optional digit (X, A1, Z9)
# PLUS       — +
# MINUS      — -
# STAR       — *
# SLASH      — /
# CARET      — ^ (exponentiation)
# EQ         — = (assignment in LET; equality in IF)
# LT         — <
# GT         — >
# LE         — <= (must match before LT and EQ)
# GE         — >= (must match before GT and EQ)
# NE         — <> (not-equal; must match before LT and GT)
# LPAREN     — (
# RPAREN     — )
# COMMA      — ,
# SEMICOLON  — ; (PRINT separator: no space)
# NEWLINE    — \n or \r\n (statement terminator; significant)
# EOF        — sentinel at end of token stream
# UNKNOWN    — unrecognized character (error recovery)
#
# # Two post-tokenize transformations
# =====================================
#
# The grammar file cannot express two context-sensitive rules, so the module
# applies them manually after the base tokenization pass:
#
# ## 1. LINE_NUM disambiguation
#
# A bare integer like `10` is grammatically a NUMBER, but when it appears at
# the very start of a source line (position 0, or immediately after a NEWLINE
# token) it is a LINE_NUM — the line label.
#
# Algorithm (_relabel_line_numbers):
#   Walk the token list maintaining an "at_line_start" flag.
#   Flag starts as true.
#   When flag is true AND token type is NUMBER: relabel to LINE_NUM.
#   Flag becomes false after any token.
#   Flag resets to true after a NEWLINE token.
#
# ## 2. REM comment suppression
#
# `REM` introduces a comment that runs to the end of the line.  Everything
# after `REM` on the same line should be discarded.
#
# Algorithm (_suppress_rem_content):
#   Walk the token list maintaining a "suppressing" flag.
#   When a KEYWORD("REM") is encountered: set suppressing = true.
#   When a NEWLINE is encountered: set suppressing = false.
#   Tokens with suppressing = true are dropped from output.
#   (The REM token itself and the NEWLINE are kept.)
#
# Result for "10 REM THIS IS A COMMENT":
#   LINE_NUM(10) KEYWORD(REM) NEWLINE EOF
#
# # Architecture
# ==============
#
# 1. Grammar loading — _grammar() opens `dartmouth_basic.tokens`, parses it
#    with CodingAdventures::GrammarTools::parse_token_grammar, and caches the
#    result in a package-level variable.
#
# 2. Pattern compilation — _build_rules() converts each TokenDefinition into
#    a compiled qr/\G.../ pattern.
#
# 3. Case normalisation — the grammar uses `@case_insensitive true` which means
#    all source text should be uppercased before matching.  We apply uc() in
#    tokenize() before running the base lexer loop.
#
# 4. Base tokenization — the standard \G + pos() loop, identical to AlgolLexer.
#
# 5. Post-processing — _relabel_line_numbers and _suppress_rem_content are
#    applied to the token list in order.
#
# # Path navigation
# =================
#
# __FILE__ resolves to:
#   .../code/packages/perl/dartmouth-basic-lexer/lib/CodingAdventures/DartmouthBasicLexer.pm
#
# dirname(__FILE__) → .../lib/CodingAdventures
#
# We climb 5 levels to reach the repo root (code/):
#
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → dartmouth-basic-lexer/  (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/                  ← repo root
#   + /grammars/dartmouth_basic.tokens
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use File::Basename qw(dirname);
use File::Spec;
use CodingAdventures::GrammarTools;

# ============================================================================
# Grammar caching
# ============================================================================
#
# Parsing the grammar file on every tokenize() call would be wasteful.
# We cache the TokenGrammar object, the compiled rule list, the skip-rule
# list, and the keyword table in package-level variables.  They are
# populated on the first call and reused for the lifetime of the process.

my $_grammar;      # CodingAdventures::GrammarTools::TokenGrammar object
my $_rules;        # arrayref of { name => str, pat => qr/\G.../ }
my $_skip_rules;   # arrayref of qr/\G.../ patterns for skip definitions
my $_keywords;     # hashref of uppercase_word => TOKEN_TYPE

# --- _grammars_dir() ----------------------------------------------------------
#
# Return the absolute path to the shared grammars/ directory, computed
# relative to this module file.  Uses File::Spec for cross-platform
# path construction and File::Basename::dirname to strip filename components.

sub _grammars_dir {
    # __FILE__ = .../code/packages/perl/dartmouth-basic-lexer/lib/CodingAdventures/DartmouthBasicLexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels: CodingAdventures/ → lib/ → dartmouth-basic-lexer/ → perl/ → packages/ → code/
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

# --- _grammar() ---------------------------------------------------------------
#
# Load and parse dartmouth_basic.tokens, caching the result.
# Returns a CodingAdventures::GrammarTools::TokenGrammar object.

sub _grammar {
    return $_grammar if $_grammar;

    my $tokens_file = File::Spec->catfile( _grammars_dir(), 'dartmouth_basic.tokens' );
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::DartmouthBasicLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_token_grammar($content);
    die "CodingAdventures::DartmouthBasicLexer: failed to parse dartmouth_basic.tokens: $err"
        unless $grammar;

    $_grammar = $grammar;
    return $_grammar;
}

# --- _build_rules() -----------------------------------------------------------
#
# Convert TokenGrammar definitions into compiled Perl patterns and a keyword
# lookup table.
#
# $_rules      — token definitions, each { name => str, pat => qr/\G.../ }
# $_skip_rules — skip definitions, each qr/\G.../
# $_keywords   — hashref: uppercase_word → KEYWORD (all keywords map to KEYWORD type)
#
# Pattern compilation:
#   is_regex == 1  →  wrap in qr/\G(?:<pattern>)/  (raw regex string)
#   is_regex == 0  →  use qr/\G\Q<literal>\E/       (literal text)
#
# The \G anchor forces each match to start at pos($source), preventing the
# regex engine from skipping ahead.
#
# Keyword table:
#   The grammar uses a keywords: section.  The grammar engine classifies NAME
#   tokens that fully match a keyword entry as KEYWORD tokens.  We build a
#   hashtable for O(1) lookup.  Because @case_insensitive is true, the source
#   has been uppercased before matching, so all lookups use uppercase keys.
#
# Token type resolution:
#   If a definition has an alias (the `-> ALIAS` syntax in .tokens files),
#   we emit the alias as the token type; otherwise we emit the definition name.

sub _build_rules {
    return if $_rules;    # already built

    my $grammar = _grammar();
    my (@rules, @skip_rules);

    # Build skip patterns.
    # In Dartmouth BASIC, only horizontal whitespace (spaces, tabs) is skipped.
    # Newlines are kept in the token stream because BASIC is line-oriented.
    #
    # Pattern compilation note:
    #   The grammar file uses `case_sensitive: false` and writes all patterns
    #   in lowercase (e.g. KEYWORD = /let|print|.../, NAME = /[a-z][0-9]?/).
    #   We uppercase the source in tokenize() to normalise case, so we add
    #   the (?i) modifier to all regex patterns to make them match the
    #   uppercased source.  Literal patterns ("=", "+", etc.) are not affected
    #   by case — punctuation has no case — so they compile normally.
    for my $defn ( @{ $grammar->skip_definitions } ) {
        my $pat;
        if ( $defn->is_regex ) {
            $pat = qr/\G(?i:${\$defn->pattern})/;
        } else {
            my $lit = $defn->pattern;
            $pat = qr/\G\Q$lit\E/;
        }
        push @skip_rules, $pat;
    }

    # Build token patterns in definition order.
    # First match wins, so the order in dartmouth_basic.tokens is significant:
    #   LE / GE / NE before LT / GT / EQ
    #   LINE_NUM before NUMBER
    #   BUILTIN_FN before NAME (BUILTIN_FN must match before NAME consumes first char)
    #   KEYWORD before NAME (so PRINT is KEYWORD not NAME("P") + junk)
    #   USER_FN before NAME (so FNA is USER_FN not NAME("F"))
    #   NAME last among identifier-like patterns
    for my $defn ( @{ $grammar->definitions } ) {
        my $pat;
        if ( $defn->is_regex ) {
            # Use (?i) so that the uppercase source matches the lowercase patterns
            # defined in the grammar (e.g., /let/ matches "LET" in the source).
            $pat = qr/\G(?i:${\$defn->pattern})/;
        } else {
            my $lit = $defn->pattern;
            $pat = qr/\G\Q$lit\E/;
        }
        # Emit the alias if one exists, otherwise use the definition name.
        # Example: STRING_BODY -> STRING emits type "STRING".
        my $type = ( $defn->alias && $defn->alias ne '' )
                    ? $defn->alias
                    : $defn->name;
        push @rules, { name => $type, pat => $pat };
    }

    # Add error-recovery patterns at the end.
    #
    # The `errors: UNKNOWN = /./` catch-all in dartmouth_basic.tokens ensures
    # any unrecognized character is emitted as an UNKNOWN token rather than
    # causing the lexer to die.  We add error definitions after regular rules
    # so they only fire when nothing else matches.
    for my $defn ( @{ $grammar->error_definitions } ) {
        my $pat;
        if ( $defn->is_regex ) {
            $pat = qr/\G(?i:${\$defn->pattern})/;
        } else {
            my $lit = $defn->pattern;
            $pat = qr/\G\Q$lit\E/;
        }
        my $type = ( $defn->alias && $defn->alias ne '' )
                    ? $defn->alias
                    : $defn->name;
        push @rules, { name => $type, pat => $pat };
    }

    # Build keyword promotion table from the grammar's keywords: section.
    # The grammar uses NAME + keyword promotion (not a direct KEYWORD regex).
    # After a NAME token is matched, we check if its uppercased value is in
    # this table and reclassify it as KEYWORD if so.
    my %kw_map;
    for my $kw (@{ $grammar->keywords }) {
        $kw_map{uc($kw)} = 1;
    }
    $_keywords = \%kw_map;

    $_skip_rules = \@skip_rules;
    $_rules      = \@rules;
}

# ============================================================================
# Post-tokenize transformations
# ============================================================================
#
# These two functions implement the context-sensitive rules that cannot be
# expressed in the data-driven grammar file.  They are applied in order
# after the base tokenization pass.

# --- _relabel_line_numbers(\@tokens) ------------------------------------------
#
# Walk the token list and relabel the first NUMBER token on each line as
# LINE_NUM.
#
# Dartmouth BASIC requires every statement to begin with a line number:
#
#   10 LET X = 5
#   20 GOTO 10
#
# Here the leading `10` and `20` are LINE_NUM tokens, not NUMBER tokens.
# But within a statement — in `GOTO 10` — the target `10` is a NUMBER.
#
# The grammar cannot distinguish these by regex alone; we need position.
# The rule: any NUMBER immediately following a NEWLINE (or at position 0
# in the source, i.e., the very first token) is a LINE_NUM.
#
# State machine:
#   at_line_start = 1  (starts true: the first token in a program is at
#                       the beginning of a line)
#   On NUMBER token, if at_line_start: emit LINE_NUM, clear flag
#   On any other token: clear flag
#   On NEWLINE token: set flag (next token begins a new line)
#
# Returns: new arrayref of tokens

sub _relabel_line_numbers {
    my ($tokens) = @_;
    my $at_line_start = 1;
    my @result;
    for my $tok (@$tokens) {
        if ($at_line_start && $tok->{type} eq 'NUMBER') {
            # This NUMBER is in line-number position — relabel it.
            push @result, { %$tok, type => 'LINE_NUM' };
            $at_line_start = 0;
        } else {
            $at_line_start = 0 if $at_line_start;
            push @result, $tok;
        }
        # After a NEWLINE, the next token begins a new line.
        $at_line_start = 1 if $tok->{type} eq 'NEWLINE';
    }
    return \@result;
}

# --- _suppress_rem_content(\@tokens) ------------------------------------------
#
# Remove all tokens between a REM keyword and the following NEWLINE.
#
# `REM` marks the rest of the line as a programmer remark.  The 1964 spec
# says the interpreter ignores everything after REM on the same line.
# We implement this by discarding tokens in the suppression window:
#
#   Input:  LINE_NUM(10) KEYWORD(REM) NAME(THIS) NAME(IS) NAME(A) NAME(COMMENT) NEWLINE
#   Output: LINE_NUM(10) KEYWORD(REM) NEWLINE
#
# The comment text (THIS IS A COMMENT) never reaches the parser.
# The NEWLINE terminates suppression so that subsequent lines are unaffected.
#
# State machine:
#   suppressing = 0 initially
#   On KEYWORD("REM"): emit token, set suppressing = 1
#   On NEWLINE:        emit token, set suppressing = 0
#   Otherwise:         emit token only if !suppressing
#
# Note: NEWLINE is emitted unconditionally (it ends the REM line cleanly).
#
# Returns: new arrayref of tokens

sub _suppress_rem_content {
    my ($tokens) = @_;
    my @result;
    my $suppressing = 0;
    for my $tok (@$tokens) {
        # Reset suppression on NEWLINE before deciding whether to emit.
        # This ensures the NEWLINE that ends a REM line is kept in the output,
        # giving the parser a clean statement terminator after the comment.
        if ($tok->{type} eq 'NEWLINE') {
            $suppressing = 0;
        }

        push @result, $tok unless $suppressing;

        # Start suppressing after we have emitted the REM keyword itself.
        # Tokens following REM on the same line are comment text and are dropped.
        if ($tok->{type} eq 'KEYWORD' && $tok->{value} eq 'REM') {
            $suppressing = 1;
        }
    }
    return \@result;
}

# ============================================================================
# Public API
# ============================================================================

# --- tokenize($source) --------------------------------------------------------
#
# Tokenize a Dartmouth BASIC source string.
#
# Algorithm:
#
#   1. Ensure grammar and compiled rules are loaded (_build_rules).
#   2. Normalise the source to uppercase (because @case_insensitive true).
#      This is the authentic behaviour: the 1964 GE-225 teletypes only had
#      uppercase characters, so `print` and `PRINT` are identical.
#   3. Walk the source from position 0 to end using \G + pos() matching.
#      a. Try each skip pattern (horizontal whitespace).
#         If matched, advance pos and continue without emitting a token.
#      b. Try each token pattern in priority order.
#         First match: check if NAME needs keyword reclassification.
#         Emit token, advance pos, update line/col tracking.
#      c. If nothing matched, the source contains an unexpected character.
#         Because dartmouth_basic.tokens has an `errors: UNKNOWN = /./`
#         catch-all, this should never die — UNKNOWN tokens are emitted instead.
#   4. Push an EOF sentinel token.
#   5. Apply _relabel_line_numbers to distinguish LINE_NUM from NUMBER.
#   6. Apply _suppress_rem_content to drop comment text after REM.
#   7. Return the final token arrayref.
#
# Keyword reclassification (Dartmouth BASIC specifics):
#   The grammar has a keywords: section listing the 20 BASIC reserved words.
#   After uppercasing the source, NAME tokens whose value matches a keyword
#   entry are reclassified as type "KEYWORD".  This is why `let` becomes
#   KEYWORD("LET") — the source is uppercased to "LET", which matches the
#   keyword table entry, so type becomes "KEYWORD".
#
# Line/column tracking:
#   - $line starts at 1, incremented for each \n in matched text.
#   - $col  starts at 1:
#       - No newlines in match: col += length(match)
#       - Newlines in match: col = length of text after last \n
#
# Return value:
#   An arrayref of hashrefs, each with keys: type, value, line, col.
#   The last element always has type 'EOF'.
#
# Never dies (unlike AlgolLexer) because BASIC grammar has UNKNOWN catch-all.

sub tokenize {
    my ($class_or_self, $source) = @_;

    _build_rules();

    # ---- Case normalisation --------------------------------------------------
    #
    # @case_insensitive true in the grammar means the entire source text is
    # treated as uppercase.  We do this once up front rather than per-character.
    # This means `print` and `PRINT` both produce KEYWORD("PRINT").
    #
    # Historical note: the original Dartmouth BASIC ran on uppercase-only
    # teletypes.  There were no lowercase characters.  Supporting lowercase
    # input is a convenience extension, implemented by this normalisation step.
    my $upcased = uc($source);

    # ---- Implicit trailing newline -------------------------------------------
    #
    # Dartmouth BASIC is line-oriented: every statement ends with NEWLINE.
    # To ensure the last line always has a NEWLINE terminator in the token
    # stream (even if the source string does not end with '\n'), we append
    # one if the source contains non-whitespace content and does not already
    # end with a newline character.
    #
    # We check for non-whitespace content (not just length > 0) so that
    # whitespace-only input (spaces, tabs) does not get a synthesized NEWLINE
    # token — it should produce only EOF.
    #
    # This matches the spec's behaviour: "10 LET X = 5" (no trailing \n)
    # should produce [..., NUMBER("5"), NEWLINE, EOF].
    if ( $upcased =~ /\S/ && $upcased !~ /\n\z/ ) {
        $upcased .= "\n";
    }

    my @tokens;
    my $line = 1;
    my $col  = 1;
    my $pos  = 0;
    my $len  = length($upcased);

    while ($pos < $len) {
        pos($upcased) = $pos;

        # ---- Try skip patterns -----------------------------------------------
        #
        # Only horizontal whitespace (spaces, tabs) is skipped.  Newlines are
        # kept as NEWLINE tokens because BASIC is line-oriented — the parser
        # needs NEWLINEs to know where each statement ends.

        my $skipped = 0;
        for my $spat (@$_skip_rules) {
            pos($upcased) = $pos;
            if ($upcased =~ /$spat/gc) {
                my $matched = $&;

                # Count newlines to update line/col.
                my $nl_count = () = $matched =~ /\n/g;
                if ($nl_count) {
                    $line += $nl_count;
                    my $after_last_nl = $matched;
                    $after_last_nl =~ s/.*\n//s;
                    $col = length($after_last_nl) + 1;
                } else {
                    $col += length($matched);
                }

                $pos = pos($upcased);
                $skipped = 1;
                last;
            }
        }
        next if $skipped;

        # ---- Try token patterns ----------------------------------------------
        #
        # Each pattern is tried at the current pos() using /gc.
        # The first match wins.
        #
        # After matching a NAME, we check if the uppercased value is a BASIC
        # keyword.  If so, the token type is reclassified to "KEYWORD".
        #
        # Example:
        #   Source: "let"  → uppercased: "LET"
        #   NAME regex matches "LET"
        #   Keyword table: LET → KEYWORD
        #   Token emitted: { type => "KEYWORD", value => "LET", ... }

        my $matched_tok = 0;
        for my $rule (@$_rules) {
            pos($upcased) = $pos;
            if ($upcased =~ /$rule->{pat}/gc) {
                my $value = $&;
                my $type  = $rule->{name};

                # Keyword promotion: if a NAME token's uppercased value appears
                # in the keyword table, reclassify it as KEYWORD. This mirrors
                # the grammar's keywords: section, which promotes NAME → KEYWORD.
                if ($type eq 'NAME' && exists $_keywords->{uc($value)}) {
                    $type  = 'KEYWORD';
                    $value = uc($value);
                }

                push @tokens, {
                    type  => $type,
                    value => $value,
                    line  => $line,
                    col   => $col,
                };

                # Advance source position.
                $pos = pos($upcased);

                # Update line/col tracking.
                my $nl_count = () = $value =~ /\n/g;
                if ($nl_count) {
                    $line += $nl_count;
                    my $after_last_nl = $value;
                    $after_last_nl =~ s/.*\n//s;
                    $col = length($after_last_nl) + 1;
                } else {
                    $col += length($value);
                }

                $matched_tok = 1;
                last;
            }
        }

        # ---- Unrecognized character ------------------------------------------
        #
        # The grammar's `errors: UNKNOWN = /./` catch-all means this should
        # never be reached for well-formed grammar loading.  But we guard
        # against a missing catch-all just in case.

        unless ($matched_tok) {
            my $ch = substr($upcased, $pos, 1);
            die sprintf(
                "CodingAdventures::DartmouthBasicLexer: LexerError at line %d col %d: "
              . "unexpected character '%s'",
                $line, $col, $ch
            );
        }
    }

    # Sentinel EOF token — always the last element.
    push @tokens, { type => 'EOF', value => '', line => $line, col => $col };

    # ---- Post-tokenize transformations ---------------------------------------
    #
    # Apply both transformations in order:
    #   1. Relabel line-number positions: NUMBER → LINE_NUM at start of each line.
    #   2. Suppress comment content: drop tokens after KEYWORD("REM") until NEWLINE.

    my $result = _relabel_line_numbers(\@tokens);
    $result    = _suppress_rem_content($result);

    return $result;
}

1;

__END__

=head1 NAME

CodingAdventures::DartmouthBasicLexer - Grammar-driven Dartmouth BASIC 1964 tokenizer

=head1 SYNOPSIS

    use CodingAdventures::DartmouthBasicLexer;

    my $tokens = CodingAdventures::DartmouthBasicLexer->tokenize('10 LET X = 5');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }
    # LINE_NUM  10
    # KEYWORD   LET
    # NAME      X
    # EQ        =
    # NUMBER    5
    # NEWLINE
    # EOF

=head1 DESCRIPTION

A thin wrapper around CodingAdventures::GrammarTools that tokenizes 1964
Dartmouth BASIC source text.  Reads the shared C<dartmouth_basic.tokens>
grammar file, compiles definitions to Perl C<qr//> regexes, and returns
a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

The entire source is normalised to uppercase before tokenizing (C<@case_insensitive true>),
so C<print>, C<Print>, and C<PRINT> all produce C<KEYWORD("PRINT")>.

Newlines are kept in the token stream (Dartmouth BASIC is line-oriented).

Two post-tokenize transformations are applied:

=over 4

=item 1. B<LINE_NUM relabelling>

The first NUMBER token on each line is relabelled as LINE_NUM.  This
distinguishes C<10 LET X = 5> (where C<10> is a label) from
C<GOTO 10> (where C<10> is a branch target, and is left as NUMBER).

=item 2. B<REM suppression>

All tokens between a KEYWORD("REM") and the next NEWLINE are dropped,
implementing BASIC's line comment syntax.

=back

The last token is always C<EOF>.

=head1 METHODS

=head2 tokenize($source)

Tokenize a Dartmouth BASIC string.  Returns an arrayref of token hashrefs.
The grammar's C<UNKNOWN = /./>  catch-all means unrecognized characters produce
C<UNKNOWN> tokens rather than dying.

=head1 TOKEN TYPES

    LINE_NUM   — line-number label at start of each program line
    NUMBER     — numeric literal (integer, decimal, scientific)
    STRING     — double-quoted string literal
    KEYWORD    — reserved word: LET PRINT INPUT IF THEN GOTO GOSUB RETURN
                               FOR TO STEP NEXT END STOP REM READ DATA
                               RESTORE DIM DEF
    BUILTIN_FN — built-in function: SIN COS TAN ATN EXP LOG ABS SQR INT RND SGN
    USER_FN    — user-defined function: FNA..FNZ
    NAME       — variable name: one letter + optional digit
    PLUS MINUS STAR SLASH CARET EQ LT GT LE GE NE
    LPAREN RPAREN COMMA SEMICOLON
    NEWLINE    — statement terminator (significant, not skipped)
    EOF        — end of input
    UNKNOWN    — unrecognized character (error recovery)

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
