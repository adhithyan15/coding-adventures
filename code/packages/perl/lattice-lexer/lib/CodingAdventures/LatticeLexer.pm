package CodingAdventures::LatticeLexer;

# ============================================================================
# CodingAdventures::LatticeLexer — Grammar-driven Lattice tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools. It reads the shared `lattice.tokens`
# grammar file, compiles the token definitions into Perl regexes, and applies
# them in priority order to tokenize Lattice (CSS superset) source code.
#
# # What is Lattice?
# ==================
#
# Lattice is a CSS superset language that adds powerful features on top of
# standard CSS:
#   - Variables:           $color, $font-size
#   - Mixins:              @mixin, @include
#   - Control flow:        @if, @else, @for, @each
#   - Functions:           @function, @return
#   - Modules:             @use
#   - Nesting:             .parent { .child { ... } }
#   - Placeholder selectors: %button-base (used with @extend)
#   - Single-line comments: // to end of line
#   - Comparison operators: ==, !=, >=, <= (in @if conditions)
#   - Variable flags:      !default, !global
#
# Every valid CSS file is also valid Lattice.
#
# # What is Lattice tokenization?
# ================================
#
# Given the input:  $color: #ff0000;
#
# The tokenizer produces a flat list of token hashrefs:
#
#   { type => "VARIABLE",  value => "$color",  line => 1, col => 1  }
#   { type => "COLON",     value => ":",       line => 1, col => 7  }
#   { type => "HASH",      value => "#ff0000", line => 1, col => 9  }
#   { type => "SEMICOLON", value => ";",       line => 1, col => 16 }
#   { type => "EOF",       value => "",        line => 1, col => 17 }
#
# Whitespace and comments (// and /* */) are consumed silently.
#
# # Escape mode
# =============
#
# lattice.tokens declares `escapes: none`. This means STRING token values
# include the surrounding quote characters and any backslash sequences as
# raw text. CSS escape sequences (\26 for &, \A9 for ©) use a different
# format from JSON (\n, \t) and are a semantic concern handled post-parse,
# not at the lexer level.
#
# In practice for this Perl implementation: string values are simply the
# full matched text including quotes, with no processing of backslash
# sequences.
#
# # Architecture
# ==============
#
# 1. **Grammar loading** — `_grammar()` opens `lattice.tokens`, parses it
#    with `CodingAdventures::GrammarTools::parse_token_grammar`, and caches
#    the result for the lifetime of the process.
#
# 2. **Pattern compilation** — `_build_rules()` converts every TokenDefinition
#    in the grammar into a `{ name => str, pat => qr/\G.../ }` hashref.
#    Skip patterns (COMMENT, LINE_COMMENT, WHITESPACE) become `qr/\G.../`
#    entries in a separate list.
#
# 3. **Tokenization** — `tokenize()` walks the source string using Perl's
#    `\G` + `pos()` mechanism, trying skip patterns first and then token
#    patterns in definition order. First match wins. On no match, dies with
#    position info.
#
# # Path navigation
# =================
#
# `__FILE__` resolves to `lib/CodingAdventures/LatticeLexer.pm`.
# `dirname(__FILE__)` → `lib/CodingAdventures`
#
# From there we climb to the repo root (`code/`) then descend into
# `grammars/lattice.tokens`:
#
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → lattice-lexer/   (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/            ← repo root
#   + /grammars/lattice.tokens
#
# # Token types
# =============
#
# Lattice-specific tokens:
#   VARIABLE        — $color, $font-size
#   PLACEHOLDER     — %button-base, %flex-center
#   EQUALS_EQUALS   — ==
#   NOT_EQUALS      — !=
#   GREATER_EQUALS  — >=
#   LESS_EQUALS     — <=
#   BANG_DEFAULT    — !default
#   BANG_GLOBAL     — !global
#
# Shared with CSS (numeric priority: DIMENSION > PERCENTAGE > NUMBER):
#   STRING, DIMENSION, PERCENTAGE, NUMBER, HASH, AT_KEYWORD, URL_TOKEN,
#   FUNCTION, CDO, CDC, UNICODE_RANGE, CUSTOM_PROPERTY, IDENT
#
# CSS attribute selector operators:
#   COLON_COLON, TILDE_EQUALS, PIPE_EQUALS, CARET_EQUALS,
#   DOLLAR_EQUALS, STAR_EQUALS
#
# Single-character delimiters and operators:
#   LBRACE, RBRACE, LPAREN, RPAREN, LBRACKET, RBRACKET,
#   SEMICOLON, COLON, COMMA, DOT,
#   PLUS, GREATER, LESS, TILDE, STAR, PIPE,
#   BANG, SLASH, EQUALS, AMPERSAND, MINUS
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use File::Basename qw(dirname);
use File::Spec;
use CodingAdventures::GrammarTools qw(parse_token_grammar);

# ============================================================================
# Grammar loading and caching
# ============================================================================
#
# Reading and parsing the grammar file on every tokenize() call would be
# wasteful. We cache the TokenGrammar object and compiled rule lists in
# package-level variables. They are populated on the first call and reused.

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
    # __FILE__ = .../code/packages/perl/lattice-lexer/lib/CodingAdventures/LatticeLexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels: CodingAdventures/ → lib/ → lattice-lexer/ → perl/ → packages/ → code/
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

# --- _grammar() ---------------------------------------------------------------
#
# Load and parse `lattice.tokens`, caching the result.
# Returns a CodingAdventures::GrammarTools::TokenGrammar object.

sub _grammar {
    return $_grammar if $_grammar;

    my $tokens_file = File::Spec->catfile( _grammars_dir(), 'lattice.tokens' );
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::LatticeLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = parse_token_grammar($content);
    die "CodingAdventures::LatticeLexer: failed to parse lattice.tokens: $err"
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
#                     This is critical for CSS operators like ==, |=, etc.
#
# The `\G` anchor forces the match to start exactly at `pos($source)`,
# preventing the regex engine from skipping ahead.
#
# Alias resolution: definitions with `-> ALIAS` emit the alias as type name.
# For example:  STRING_DQ = /.../ -> STRING  emits type "STRING".
#
# lattice.tokens has no `mode:` directive (unlike starlark.tokens), so
# there is no indentation tracking — all whitespace including newlines
# is handled by the WHITESPACE skip pattern.

sub _build_rules {
    return if $_rules;    # already built

    my $grammar = _grammar();
    my (@rules, @skip_rules);

    # Build skip patterns.
    # lattice.tokens declares:
    #   skip:
    #     LINE_COMMENT = /\/\/[^\n]*/      -- // to end of line
    #     COMMENT      = /\/\*[\s\S]*?\*/  -- /* block comment */
    #     WHITESPACE   = /[ \t\r\n]+/      -- all whitespace
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

    # If no skip rules exist in the grammar (safety net), add a default
    # whitespace skip so that bare spaces/tabs/newlines are consumed silently.
    unless (@skip_rules) {
        push @skip_rules, qr/\G[ \t\r\n]+/;
    }

    # Build token patterns.
    for my $defn ( @{ $grammar->definitions } ) {
        my $pat;
        if ( $defn->is_regex ) {
            $pat = qr/\G(?:${\$defn->pattern})/;
        } else {
            my $lit = $defn->pattern;
            $pat = qr/\G\Q$lit\E/;
        }
        # Emit the alias if one exists, otherwise use the definition name.
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
# Tokenize a Lattice source string.
#
# Algorithm:
#
#   1. Ensure grammar and compiled rules are loaded (_build_rules).
#   2. Walk the source from position 0 to end using pos($source).
#   3. At each position, try each skip pattern with /gc.
#      If a skip pattern matches, update line/col tracking and continue.
#   4. If no skip matched, try each token pattern in definition order.
#      First match: record token, advance pos, update tracking, continue.
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
# String escape handling:
#
#   Because lattice.tokens declares `escapes: none`, this implementation
#   does NOT process backslash sequences in string token values. The full
#   matched text (including quotes and raw escape sequences) is stored as
#   the token value. This is intentional: CSS escape sequences (\26, \A9)
#   are decoded at the semantic level, not the lexer level.
#
# Return value:
#
#   An arrayref of hashrefs, each with keys: type, value, line, col.
#   The last element always has type 'EOF'.
#
# Raises:
#
#   `die` with a "LexerError" message on unexpected input.

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
        # Lattice skip patterns consume whitespace (including newlines, since
        # CSS/Lattice whitespace is insignificant), // line comments, and
        # /* block comments */ without emitting any tokens.

        my $skipped = 0;
        for my $spat (@$_skip_rules) {
            pos($source) = $pos;
            if ($source =~ /$spat/gc) {
                my $matched = $&;

                # Count newlines to update line/col accurately.
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

        # ---- No match — unexpected character ---------------------------------
        #
        # A valid Lattice/CSS source should rarely reach here. We emit a
        # descriptive error including position and the offending character.

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::LatticeLexer: LexerError at line %d col %d: "
              . "unexpected character '%s'",
                $line, $col, $ch
            );
        }
    }

    # Sentinel EOF token — always present as the last element.
    push @tokens, { type => 'EOF', value => '', line => $line, col => $col };

    return \@tokens;
}

1;

__END__

=head1 NAME

CodingAdventures::LatticeLexer - Grammar-driven Lattice (CSS superset) tokenizer

=head1 SYNOPSIS

    use CodingAdventures::LatticeLexer;

    my $tokens = CodingAdventures::LatticeLexer->tokenize('$color: #ff0000;');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<lattice.tokens> file, compiles token definitions to Perl regexes,
and tokenizes Lattice source into a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Whitespace (including newlines), C<//> line comments, and C</* block comments */>
are silently consumed. The last token is always C<EOF>.

Lattice-specific token types: VARIABLE, PLACEHOLDER, EQUALS_EQUALS, NOT_EQUALS,
GREATER_EQUALS, LESS_EQUALS, BANG_DEFAULT, BANG_GLOBAL.

Because C<lattice.tokens> declares C<escapes: none>, STRING values include their
surrounding quotes and any backslash sequences as raw text (CSS escape decoding
is a semantic post-parse concern).

=head1 METHODS

=head2 tokenize($source)

Tokenize a Lattice string. Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
