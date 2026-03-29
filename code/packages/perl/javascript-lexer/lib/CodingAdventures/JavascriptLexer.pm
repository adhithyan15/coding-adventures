package CodingAdventures::JavascriptLexer;

# ============================================================================
# CodingAdventures::JavascriptLexer — Grammar-driven JavaScript tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools and CodingAdventures::Lexer. It reads
# the shared `javascript.tokens` grammar file, compiles the token definitions
# into Perl regexes, and applies them in priority order to tokenize JavaScript
# source code.
#
# # What is JavaScript tokenization?
# =====================================
#
# Given the input:  const x = 42;
#
# The tokenizer produces a flat list of token hashrefs:
#
#   { type => "CONST",     value => "const", line => 1, col => 1  }
#   { type => "NAME",      value => "x",     line => 1, col => 7  }
#   { type => "EQUALS",    value => "=",     line => 1, col => 9  }
#   { type => "NUMBER",    value => "42",    line => 1, col => 11 }
#   { type => "SEMICOLON", value => ";",     line => 1, col => 13 }
#   { type => "EOF",       value => "",      line => 1, col => 14 }
#
# Whitespace is consumed silently — skip patterns in `javascript.tokens`
# match whitespace and it is never emitted as a token.
#
# # Architecture
# ==============
#
# 1. **Grammar loading** — `_grammar()` opens `javascript.tokens`, parses it
#    with `CodingAdventures::GrammarTools::parse_token_grammar`, and caches
#    the result for the lifetime of the process.
#
# 2. **Pattern compilation** — `_build_rules()` converts every TokenDefinition
#    in the grammar into a `{ name => str, pat => qr/\G.../ }` hashref.
#    Regex definitions use `qr/\G(?:<pattern>)/`; literal definitions use
#    `qr/\G\Q<literal>\E/` to disable metacharacter interpretation.
#
# 3. **Tokenization** — `tokenize()` walks the source string using Perl's
#    `\G` + `pos()` mechanism, trying skip patterns first and then token
#    patterns in definition order. First match wins. On no match, dies with
#    position info.
#
# # Path navigation
# =================
#
# `__FILE__` resolves to `lib/CodingAdventures/JavascriptLexer.pm`.
# `dirname(__FILE__)` → `lib/CodingAdventures`
#
# From there we climb to the repo root (`code/`) then descend into
# `grammars/javascript.tokens`:
#
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → javascript-lexer/     (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/                 ← repo root
#   + /grammars/javascript.tokens
#
# # Token types
# =============
#
# NAME        — identifiers; promoted to keyword type if in keywords section
# NUMBER      — integer literals (e.g. 42, 0)
# STRING      — double-quoted string literals
#
# Keyword tokens (promoted from NAME):
#   LET, CONST, VAR, IF, ELSE, WHILE, FOR, DO, FUNCTION, RETURN,
#   CLASS, IMPORT, EXPORT, FROM, AS, NEW, THIS, TYPEOF, INSTANCEOF,
#   TRUE, FALSE, NULL, UNDEFINED
#
# Multi-char operators (matched before single-char ones):
#   STRICT_EQUALS, STRICT_NOT_EQUALS, EQUALS_EQUALS, NOT_EQUALS,
#   LESS_EQUALS, GREATER_EQUALS, ARROW
#
# Single-char operators:
#   EQUALS, PLUS, MINUS, STAR, SLASH, LESS_THAN, GREATER_THAN, BANG
#
# Delimiters:
#   LPAREN, RPAREN, LBRACE, RBRACE, LBRACKET, RBRACKET,
#   COMMA, COLON, SEMICOLON, DOT
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
    # __FILE__ = .../code/packages/perl/javascript-lexer/lib/CodingAdventures/JavascriptLexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels: CodingAdventures/ → lib/ → javascript-lexer/ → perl/ → packages/ → code/
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

# --- _grammar() ---------------------------------------------------------------
#
# Load and parse `javascript.tokens`, caching the result.
# Returns a CodingAdventures::GrammarTools::TokenGrammar object.

sub _grammar {
    return $_grammar if $_grammar;

    my $tokens_file = File::Spec->catfile( _grammars_dir(), 'javascript.tokens' );
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::JavascriptLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = parse_token_grammar($content);
    die "CodingAdventures::JavascriptLexer: failed to parse javascript.tokens: $err"
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
#                     This is critical for operators like ===, !==, =>, etc.
#
# The `\G` anchor forces the match to start exactly at `pos($source)`,
# preventing the regex engine from skipping ahead.
#
# Alias resolution: definitions with `-> ALIAS` emit the alias as type name.

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
# Tokenize a JavaScript source string.
#
# Algorithm:
#
#   1. Ensure grammar and compiled rules are loaded (_build_rules).
#   2. Walk the source from position 0 to end.
#   3. At each position, set pos($source) and try each skip pattern with /gc.
#      If a skip pattern matches, update line/col tracking and continue.
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
        # Whitespace in JavaScript is insignificant between tokens.
        # We advance position without emitting anything, but still update
        # line/col tracking so that token positions after whitespace are accurate.

        my $skipped = 0;
        for my $spat (@$_skip_rules) {
            pos($source) = $pos;
            if ($source =~ /$spat/gc) {
                my $matched = $&;

                # Count newlines to update line/col
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
        # A well-formed JavaScript source should rarely reach here. We emit a
        # descriptive error including position and the offending character.

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::JavascriptLexer: LexerError at line %d col %d: "
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

CodingAdventures::JavascriptLexer - Grammar-driven JavaScript tokenizer

=head1 SYNOPSIS

    use CodingAdventures::JavascriptLexer;

    my $tokens = CodingAdventures::JavascriptLexer->tokenize('const x = 1;');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<javascript.tokens> file, compiles token definitions to Perl regexes,
and tokenizes JavaScript source into a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Whitespace is silently consumed. The last token is always C<EOF>.

Token types include: NAME, NUMBER, STRING; keyword types: LET, CONST, VAR,
IF, ELSE, WHILE, FOR, DO, FUNCTION, RETURN, CLASS, IMPORT, EXPORT, FROM, AS,
NEW, THIS, TYPEOF, INSTANCEOF, TRUE, FALSE, NULL, UNDEFINED; operator types:
STRICT_EQUALS, STRICT_NOT_EQUALS, EQUALS_EQUALS, NOT_EQUALS, LESS_EQUALS,
GREATER_EQUALS, ARROW, EQUALS, PLUS, MINUS, STAR, SLASH, LESS_THAN,
GREATER_THAN, BANG; delimiter types: LPAREN, RPAREN, LBRACE, RBRACE,
LBRACKET, RBRACKET, COMMA, COLON, SEMICOLON, DOT.

=head1 METHODS

=head2 tokenize($source)

Tokenize a JavaScript string. Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
