package CodingAdventures::ExcelLexer;

# ============================================================================
# CodingAdventures::ExcelLexer — Grammar-driven Excel formula tokenizer
# ============================================================================
#
# This module tokenizes Excel formula strings into a flat list of typed
# token hashrefs.  It is a thin wrapper around the grammar infrastructure
# provided by CodingAdventures::GrammarTools and CodingAdventures::Lexer.
#
# It reads the shared `excel.tokens` grammar file, compiles each token
# definition into a `\G`-anchored Perl regex, and applies them in priority
# order to scan the formula source.
#
# # What is an Excel formula?
#
# Excel formulas are the mini-language embedded in spreadsheet cells.  They
# begin with "=" and describe a computation:
#
#   =A1+B2                     → add two cell values
#   =SUM(A1:B10)               → sum a range of cells
#   =IF(A1>0, "pos", "neg")    → conditional expression
#   =Sheet1!A1 * 1.1           → cross-sheet reference, scaled
#   =A1*100%                   → postfix percent operator
#
# # Token stream example: =SUM(A1:B10)
#
#   { type=>"EQUALS",  value=>"=",    line=>1, col=>1 }
#   { type=>"NAME",    value=>"sum",  line=>1, col=>2 }  (case-normalized)
#   { type=>"LPAREN",  value=>"(",    line=>1, col=>5 }
#   { type=>"CELL",    value=>"a1",   line=>1, col=>6 }
#   { type=>"COLON",   value=>":",    line=>1, col=>8 }
#   { type=>"CELL",    value=>"b10",  line=>1, col=>9 }
#   { type=>"RPAREN",  value=>")",    line=>1, col=>12 }
#   { type=>"EOF",     value=>"",     line=>1, col=>13 }
#
# # Excel's case-insensitivity (historical background)
#
# Excel has been case-insensitive since its origins as Multiplan (1982) on
# the early IBM PC.  The design decisions were:
#
#   1. Early spreadsheet users were accountants, not programmers.  They
#      did not expect case to be significant in formulas.
#   2. The IBM PC keyboard made shift-lock awkward during formula entry.
#   3. =SUM(a1:b10) must behave identically to =SUM(A1:B10).
#
# The `excel.tokens` grammar declares `@case_insensitive true`.  Because
# Perl regexes are case-sensitive by default and the GrammarTools Perl
# implementation does not automatically add `/i`, we handle case-
# insensitivity by **lowercasing the source** before tokenizing.  The
# returned token values therefore appear in lowercase.
#
# # A1 vs R1C1 reference styles
#
# Excel supports two reference styles:
#
#   A1 style (default, handled here):
#     Column = letter(s) A–XFD, Row = integer 1–1,048,576.
#     Dollar signs make a reference absolute: $A$1, $A1, A$1.
#
#   R1C1 style (optional, used in VBA macros):
#     R1C1 = row 1 column 1.  R[-1]C[2] = relative offset.
#     This style is NOT covered by the current grammar.
#
# # Space as the intersection operator
#
# In Excel, a space between two range references is the INTERSECTION
# operator, e.g.: =SUM(A1:B10 B5:C15) yields the sum of the overlapping
# cells.  Therefore the excel.tokens grammar emits a SPACE token for
# literal spaces rather than silently consuming them.  Only non-space
# whitespace (tabs, carriage return, newline) is silently skipped.
#
# # Path navigation
#
# __FILE__ = .../code/packages/perl/excel-lexer/lib/CodingAdventures/ExcelLexer.pm
# dirname(__FILE__)  = .../lib/CodingAdventures
#
# From there, climb 5 levels:
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → excel-lexer/       (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/              ← repo root
#   + /grammars/excel.tokens
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use File::Basename qw(dirname);
use File::Spec;
use CodingAdventures::GrammarTools;

# ============================================================================
# Grammar loading and caching
# ============================================================================
#
# The grammar file is parsed once and the resulting TokenGrammar object and
# compiled pattern lists are cached in package-level variables.  This avoids
# repeated file I/O and regex compilation on every call to tokenize().

my $_grammar;      # CodingAdventures::GrammarTools::TokenGrammar
my $_rules;        # arrayref of { name => str, pat => qr/\G.../ }
my $_skip_rules;   # arrayref of qr/\G.../ for skip definitions

# --- _grammars_dir() ----------------------------------------------------------
#
# Return the absolute path to the shared `grammars/` directory in the
# monorepo, computed relative to this module's file path.

sub _grammars_dir {
    # __FILE__ = .../code/packages/perl/excel-lexer/lib/CodingAdventures/ExcelLexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels to reach code/
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

# --- _grammar() ---------------------------------------------------------------
#
# Load and parse `excel.tokens`, caching the TokenGrammar object.
# Returns a CodingAdventures::GrammarTools::TokenGrammar.

sub _grammar {
    return $_grammar if $_grammar;

    my $tokens_file = File::Spec->catfile( _grammars_dir(), 'excel.tokens' );
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::ExcelLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_token_grammar($content);
    die "CodingAdventures::ExcelLexer: failed to parse excel.tokens: $err"
        unless $grammar;

    $_grammar = $grammar;
    return $_grammar;
}

# --- _build_rules() -----------------------------------------------------------
#
# Convert TokenGrammar definitions into compiled Perl patterns:
#
#   $_rules      — token definitions, each { name => str, pat => qr/\G.../ }
#   $_skip_rules — skip definitions, each qr/\G.../
#
# Pattern compilation strategy:
#
#   is_regex == 1  →  wrap raw pattern string in qr/\G(?:<pattern>)/
#   is_regex == 0  →  wrap literal in qr/\G\Q<literal>\E/
#
# The \G anchor ensures matching is pinned to pos($source), preventing the
# regex engine from skipping ahead.
#
# Token type: emit the alias (if defined) or the definition name.

sub _build_rules {
    return if $_rules;    # already built

    my $grammar = _grammar();
    my (@rules, @skip_rules);

    # Build skip patterns
    for my $defn ( @{ $grammar->skip_definitions } ) {
        my $pat;
        if ( $defn->is_regex ) {
            $pat = qr/\G(?:${\$defn->pattern})/;
        } else {
            my $lit = $defn->pattern;
            $pat = qr/\G\Q$lit\E/;
        }
        push @skip_rules, $pat;
    }

    # Build token patterns
    for my $defn ( @{ $grammar->definitions } ) {
        my $pat;
        if ( $defn->is_regex ) {
            $pat = qr/\G(?:${\$defn->pattern})/;
        } else {
            my $lit = $defn->pattern;
            $pat = qr/\G\Q$lit\E/;
        }
        # Use alias if defined, otherwise use the definition name.
        my $type = ( $defn->alias && $defn->alias ne '' )
                    ? $defn->alias
                    : $defn->name;
        push @rules, { name => $type, pat => $pat };
    }

    $_skip_rules = \@skip_rules;
    $_rules      = \@rules;
}

# ============================================================================
# Public API
# ============================================================================

# --- tokenize($source) --------------------------------------------------------
#
# Tokenize an Excel formula source string.
#
# # Case normalization
#
# Excel formulas are case-insensitive (see module preamble).  We normalize
# the source to lowercase before tokenizing.  All returned token values are
# therefore lowercase.
#
# # Algorithm
#
#   1. Ensure grammar and compiled rules are loaded.
#   2. Lowercase the source.
#   3. Walk the source from position 0 to end using pos() / \G.
#   4. At each position, try skip patterns first (non-space whitespace only).
#   5. If no skip matched, try token patterns in definition order.
#      First match wins: record token hashref, advance pos, update line/col.
#   6. If nothing matches, die with a descriptive error.
#   7. After exhausting input, push EOF sentinel and return.
#
# # Return value
#
# Arrayref of hashrefs, each with keys: type, value, line, col.
# The last element always has type 'EOF'.
#
# # Note on SPACE tokens
#
# The excel.tokens grammar does NOT declare space as a skip pattern.
# Spaces are emitted as SPACE tokens because the space character is the
# range-intersection operator in Excel.  Only tabs, CR, and LF are skipped.
#
# @param  $source  string  The Excel formula text to tokenize.
# @return arrayref         Array of token hashrefs (type, value, line, col).
# @die                     On unexpected input, with line/col info.

sub tokenize {
    my ($class_or_self, $source) = @_;

    _build_rules();

    # Normalize to lowercase for case-insensitive matching
    $source = lc($source);

    my @tokens;
    my $line = 1;
    my $col  = 1;
    my $pos  = 0;
    my $len  = length($source);

    while ($pos < $len) {
        pos($source) = $pos;

        # ---- Try skip patterns -----------------------------------------------
        #
        # Skip patterns in excel.tokens only include non-space whitespace
        # (tabs, CR, LF).  Spaces are significant (intersection operator) and
        # will be handled by the token patterns below.

        my $skipped = 0;
        for my $spat (@$_skip_rules) {
            pos($source) = $pos;
            if ($source =~ /$spat/gc) {
                my $matched = $&;

                my $nl_count = () = $matched =~ /\n/g;
                if ($nl_count) {
                    $line += $nl_count;
                    my $after_last_nl = $matched;
                    $after_last_nl =~ s/.*\n//s;
                    $col = length($after_last_nl) + 1;
                } else {
                    $col += length($matched);
                }

                $pos = pos($source);
                $skipped = 1;
                last;
            }
        }
        next if $skipped;

        # ---- Try token patterns ----------------------------------------------
        #
        # Patterns are tried in the order they appear in excel.tokens.
        # Order matters: multi-character operators (<>, <=, >=) must appear
        # before single-character ones (<, >, =).  Error constants (#DIV/0!)
        # must appear before NAME.  The grammar file already encodes this.

        my $matched_tok = 0;
        for my $rule (@$_rules) {
            pos($source) = $pos;
            if ($source =~ /$rule->{pat}/gc) {
                my $value = $&;

                push @tokens, {
                    type  => $rule->{name},
                    value => $value,
                    line  => $line,
                    col   => $col,
                };

                $pos = pos($source);

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

        # ---- No match — unexpected character ---------------------------------
        #
        # A syntactically correct Excel formula should never reach here.
        # We raise a descriptive error including position information.

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::ExcelLexer: LexerError at line %d col %d: "
              . "unexpected character '%s'",
                $line, $col, $ch
            );
        }
    }

    # Sentinel EOF token — always the last element in the returned list.
    push @tokens, { type => 'EOF', value => '', line => $line, col => $col };

    return \@tokens;
}

1;

__END__

=head1 NAME

CodingAdventures::ExcelLexer - Grammar-driven Excel formula tokenizer

=head1 SYNOPSIS

    use CodingAdventures::ExcelLexer;

    my $tokens = CodingAdventures::ExcelLexer->tokenize('=SUM(A1:B10)');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }
    # EQUALS  =
    # NAME    sum
    # LPAREN  (
    # CELL    a1
    # COLON   :
    # CELL    b10
    # RPAREN  )
    # EOF

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in
C<CodingAdventures::GrammarTools>.  Reads the shared C<excel.tokens>
grammar file, compiles token definitions to C<\G>-anchored Perl regexes,
and tokenizes Excel formula source into a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Excel formulas are case-insensitive: the source is lowercased before
tokenization so all returned values are lowercase.

Space characters are emitted as C<SPACE> tokens because the space is the
range-intersection operator in Excel (e.g. C<=SUM(A1:B10 B5:C15)>).
Only tabs, carriage returns, and newlines are silently consumed.

The last token is always C<EOF>.

=head1 TOKEN TYPES

    EQUALS CELL NAME NUMBER STRING TRUE FALSE ERROR_CONSTANT
    REF_PREFIX STRUCTURED_KEYWORD STRUCTURED_COLUMN
    NOT_EQUALS LESS_EQUALS GREATER_EQUALS
    PLUS MINUS STAR SLASH CARET AMP PERCENT
    LESS_THAN GREATER_THAN BANG DOLLAR
    LPAREN RPAREN LBRACE RBRACE LBRACKET RBRACKET
    COMMA SEMICOLON COLON AT SPACE EOF

=head1 METHODS

=head2 tokenize($source)

Tokenize an Excel formula string.  Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
