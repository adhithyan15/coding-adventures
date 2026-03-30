package CodingAdventures::CssLexer;

# ============================================================================
# CodingAdventures::CssLexer — Grammar-driven CSS tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools. It reads the shared `css.tokens`
# grammar file, compiles the token definitions into Perl regexes, and applies
# them in priority order to tokenize CSS3 source text.
#
# # What is CSS tokenization?
# ============================
#
# CSS tokenization is significantly harder than most languages because of
# *compound tokens* — single lexical units that span character classes that
# would be separate tokens in other languages.
#
# Consider:
#
#   10px     → DIMENSION   (number + unit identifier — ONE token)
#   50%      → PERCENTAGE  (number + percent sign — ONE token)
#   rgba(    → FUNCTION    (identifier + opening paren — ONE token)
#   #333     → HASH        (hash + hex digits — ONE token)
#   @media   → AT_KEYWORD  (at-sign + identifier — ONE token)
#
# If the lexer applied NUMBER before DIMENSION, "10px" would tokenize as:
#   NUMBER("10")  IDENT("px")  — two tokens — WRONG!
#
# The `css.tokens` grammar prevents this with first-match-wins ordering:
#   DIMENSION before PERCENTAGE before NUMBER.
#   URL_TOKEN before FUNCTION.
#   FUNCTION before IDENT.
#   COLON_COLON before COLON.
#   CUSTOM_PROPERTY before IDENT.
#
# # Architecture
# ==============
#
# 1. **Grammar loading** — `_grammar()` opens `css.tokens`, parses it
#    with `CodingAdventures::GrammarTools::parse_token_grammar`, and caches
#    the result for the lifetime of the process.
#
# 2. **Pattern compilation** — `_build_rules()` converts every TokenDefinition
#    in the grammar into a `{ name => str, pat => qr/\G.../ }` hashref.
#    The `\G` anchor forces matches to start exactly at `pos($source)`.
#
# 3. **Tokenization** — `tokenize()` walks the source string using Perl's
#    `\G` + `pos()` mechanism, trying skip patterns first (whitespace and
#    CSS comments), then token patterns in definition order. First match wins.
#
# # Path navigation
# =================
#
# `__FILE__` resolves to `lib/CodingAdventures/CssLexer.pm`.
# `dirname(__FILE__)` → `lib/CodingAdventures`
#
# From there we climb to the repo root (`code/`) then descend into
# `grammars/css.tokens`:
#
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → css-lexer/        (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/             ← repo root
#   + /grammars/css.tokens
#
# # escapes: none
# ================
#
# The `css.tokens` grammar declares `escapes: none`. CSS uses a different
# escape format (\26 for hex code points, not JSON-style \uXXXX). The
# tokenizer preserves escape sequences as raw text in token values.
# CSS escape decoding is a semantic concern handled post-parse.
#
# # Token types produced
# =======================
#
# Compound tokens (unique to CSS):
#   DIMENSION       — number + unit: 10px, 1.5em, 100vh, 360deg
#   PERCENTAGE      — number + %: 50%, 0.5%, 100%
#   AT_KEYWORD      — @identifier: @media, @import, @keyframes, @charset
#   HASH            — #word: #333, #ff0000, #header, #nav
#   FUNCTION        — identifier(: rgba(, calc(, linear-gradient(, var(
#   URL_TOKEN       — url(unquoted): url(./img.png), url(data:...)
#   CUSTOM_PROPERTY — --name: --main-color, --bg
#
# Other value tokens:
#   NUMBER          — bare number: 42, 3.14, -0.5, 1e10
#   STRING          — quoted string (either delim): "hello", 'world'
#   IDENT           — identifier: color, sans-serif, -webkit-transform
#   UNICODE_RANGE   — U+XXXX: U+0025-00FF, U+4??
#   CDO, CDC        — legacy HTML comment delimiters: <!--, -->
#
# Structural delimiters:
#   COLON_COLON, TILDE_EQUALS, PIPE_EQUALS, CARET_EQUALS,
#   DOLLAR_EQUALS, STAR_EQUALS,
#   LBRACE, RBRACE, LPAREN, RPAREN, LBRACKET, RBRACKET,
#   SEMICOLON, COLON, COMMA, DOT, PLUS, GREATER, TILDE,
#   STAR, PIPE, BANG, SLASH, EQUALS, AMPERSAND, MINUS
#
# Error tokens (graceful degradation):
#   BAD_STRING      — unclosed string: "hello (no closing ")
#   BAD_URL         — unclosed url(): url(./path (no closing ))
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
# CSS grammars have many token types (30+), so caching the compiled rule list
# is important. We populate these package-level variables on the first call
# to tokenize() and reuse them on every subsequent call.

my $_grammar;      # CodingAdventures::GrammarTools::TokenGrammar
my $_rules;        # arrayref of { name => str, pat => qr// }
my $_skip_rules;   # arrayref of qr// patterns for skip definitions

# --- _grammars_dir() ----------------------------------------------------------
#
# Return the absolute path to the shared `grammars/` directory in the
# monorepo, computed relative to this module file.
#
# We use File::Spec for cross-platform path construction and
# File::Basename::dirname to strip the filename component.

sub _grammars_dir {
    # __FILE__ = .../code/packages/perl/css-lexer/lib/CodingAdventures/CssLexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels: CodingAdventures/ → lib/ → css-lexer/ → perl/ → packages/ → code/
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

# --- _grammar() ---------------------------------------------------------------
#
# Load and parse `css.tokens`, caching the result.
# Returns a CodingAdventures::GrammarTools::TokenGrammar object.

sub _grammar {
    return $_grammar if $_grammar;

    my $tokens_file = File::Spec->catfile( _grammars_dir(), 'css.tokens' );
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::CssLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_token_grammar($content);
    die "CodingAdventures::CssLexer: failed to parse css.tokens: $err"
        unless $grammar;

    $_grammar = $grammar;
    return $_grammar;
}

# --- _build_rules() -----------------------------------------------------------
#
# Convert TokenGrammar definitions into two lists of compiled Perl patterns:
#
#   $_rules      — token definitions, each { name => str, pat => qr/\G.../ }
#   $_skip_rules — skip definitions, each qr/\G.../
#
# Pattern compilation strategy:
#
#   is_regex == 1  →  treat `$defn->pattern` as a raw regex string.
#                     Wrap in qr/\G(?:<pattern>)/ to anchor at current pos.
#
#   is_regex == 0  →  treat `$defn->pattern` as a literal string.
#                     Use `\Q...\E` to disable regex metacharacters.
#                     Critical for operators like ::, ~=, etc.
#
# The `\G` anchor is essential — it forces the match to start exactly at
# pos($source), preventing the regex engine from skipping ahead. Without \G,
# a pattern like /[0-9]+/ would match the next digit anywhere in the string
# instead of at the current position.
#
# Priority ordering: definitions in the grammar are ordered for correctness.
# Since we iterate $_rules in order and take the first match, the ordering
# in css.tokens is preserved:
#   DIMENSION before PERCENTAGE before NUMBER
#   URL_TOKEN before FUNCTION
#   FUNCTION before IDENT
#   COLON_COLON before COLON
#   CUSTOM_PROPERTY before IDENT

sub _build_rules {
    return if $_rules;    # already built

    my $grammar = _grammar();
    my (@rules, @skip_rules);

    # Build skip patterns
    # CSS has two skip types: whitespace and /* ... */ comments.
    # Both are declared in the skip: section of css.tokens.
    for my $defn ( @{ $grammar->skip_definitions } ) {
        my $pat;
        if ( $defn->is_regex ) {
            $pat = qr/\G(?:${\$defn->pattern})/s;
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
            $pat = qr/\G(?:${\$defn->pattern})/s;
        } else {
            my $lit = $defn->pattern;
            $pat = qr/\G\Q$lit\E/;
        }
        # Emit the alias if one exists, otherwise use the definition name.
        # In css.tokens, STRING_DQ and STRING_SQ both use -> STRING.
        my $type = ( $defn->alias && $defn->alias ne '' )
                    ? $defn->alias
                    : $defn->name;
        push @rules, { name => $type, pat => $pat };
    }

    # css.tokens declares explicit skip: patterns, so we should have them.
    # If somehow we don't, fall back to a default whitespace skip.
    unless (@skip_rules) {
        push @skip_rules, qr/\G[ \t\r\n]+/;
    }

    $_skip_rules = \@skip_rules;
    $_rules      = \@rules;
}

# ============================================================================
# Public API
# ============================================================================

# --- tokenize($source) --------------------------------------------------------
#
# Tokenize a CSS3 source string.
#
# Algorithm:
#
#   1. Ensure grammar and compiled rules are loaded (_build_rules).
#   2. Walk the source from position 0 to end.
#   3. At each position, set pos($source) and try each skip pattern with /gc.
#      If a skip pattern matches (whitespace or /* comment */), update
#      line/col tracking and continue without emitting a token.
#   4. If no skip matched, try each token pattern in order.
#      The first match: record token, advance pos, update tracking, continue.
#   5. If nothing matched, die with a descriptive error message.
#   6. After exhausting the input, push an EOF sentinel and return.
#
# Line/column tracking:
#
#   - `$line` starts at 1, incremented for each '\n' in matched text.
#   - `$col`  starts at 1:
#       - If the match contains no newlines: col += length(match).
#       - If the match contains newlines: col = length of text after last '\n'.
#
# Return value:
#
#   An arrayref of hashrefs, each with keys: type, value, line, col.
#   The last element always has type 'EOF'.
#
# Raises:
#
#   `die` with a "LexerError" message on unexpected input.
#   CSS has BAD_STRING and BAD_URL error tokens, so for well-formed stylesheets
#   this should rarely trigger. Malformed CSS will produce BAD_STRING / BAD_URL
#   tokens rather than dying.

sub tokenize {
    my ($class_or_self, $source) = @_;

    _build_rules();

    my @tokens;
    my $line = 1;
    my $col  = 1;
    my $pos  = 0;
    my $len  = length($source);

    while ($pos < $len) {
        pos($source) = $pos;

        # ---- Try skip patterns -----------------------------------------------
        #
        # CSS whitespace (spaces, tabs, carriage returns, newlines) and
        # /* ... */ comments are declared as skip patterns in css.tokens.
        # They are silently consumed without emitting tokens.
        #
        # Multi-line comments use /s modifier so . matches newlines.
        # The \G anchor ensures we only match at the current position.

        my $skipped = 0;
        for my $spat (@$_skip_rules) {
            pos($source) = $pos;
            if ($source =~ /$spat/gc) {
                my $matched = $&;

                # Update line/col tracking for skipped text
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
        # Each pattern is tried at the current pos() using /gc (keep pos on
        # failure, anchored to \G). First match wins.
        #
        # The ordering of $_rules matches the ordering in css.tokens, which
        # ensures compound tokens win over their component tokens:
        #   DIMENSION("10px") wins over NUMBER("10") + IDENT("px")
        #   FUNCTION("rgba(") wins over IDENT("rgba") + LPAREN("(")
        #   etc.

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

                # Update line/col tracking
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
        # Well-formed CSS should rarely reach here because css.tokens provides
        # BAD_STRING and BAD_URL error tokens for graceful degradation.
        # Characters that don't match any token pattern (e.g., non-ASCII
        # code points not covered by IDENT) will trigger this error.

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::CssLexer: LexerError at line %d col %d: "
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

CodingAdventures::CssLexer - Grammar-driven CSS3 tokenizer

=head1 SYNOPSIS

    use CodingAdventures::CssLexer;

    my $tokens = CodingAdventures::CssLexer->tokenize('h1 { color: red; }');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<css.tokens> file, compiles token definitions to Perl regexes,
and tokenizes CSS3 source into a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Whitespace and C</* ... */> comments are silently consumed. The last token is
always C<EOF>.

=head2 CSS tokenization challenges

CSS uses compound tokens — single lexical units from multiple character classes:

  10px     → DIMENSION   (not NUMBER + IDENT)
  50%      → PERCENTAGE  (not NUMBER + literal)
  rgba(    → FUNCTION    (not IDENT + LPAREN)
  #333     → HASH        (not HASH_CHAR + IDENT)
  @media   → AT_KEYWORD  (not AT + IDENT)

The C<css.tokens> grammar handles this with first-match-wins ordering. This
module preserves that ordering when compiling patterns.

=head2 Token types

Compound: DIMENSION, PERCENTAGE, AT_KEYWORD, HASH, FUNCTION, URL_TOKEN,
CUSTOM_PROPERTY.

Values: NUMBER, STRING, IDENT, UNICODE_RANGE, CDO, CDC.

Operators: COLON_COLON, TILDE_EQUALS, PIPE_EQUALS, CARET_EQUALS, DOLLAR_EQUALS,
STAR_EQUALS.

Delimiters: LBRACE, RBRACE, LPAREN, RPAREN, LBRACKET, RBRACKET, SEMICOLON,
COLON, COMMA, DOT, PLUS, GREATER, TILDE, STAR, PIPE, BANG, SLASH, EQUALS,
AMPERSAND, MINUS.

Error: BAD_STRING, BAD_URL.

=head1 METHODS

=head2 tokenize($source)

Tokenize a CSS3 string. Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
