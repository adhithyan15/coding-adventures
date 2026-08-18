package CodingAdventures::RubyLexer;

# ============================================================================
# CodingAdventures::RubyLexer — Grammar-driven Ruby tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools. It loads the `ruby.tokens` grammar
# (compiled ahead of time into CodingAdventures::RubyLexer::_Grammar — see
# "Grammar loading" below), compiles the token definitions into Perl regexes,
# and applies them in priority order to tokenize Ruby source code.
#
# # What is Ruby tokenization?
# ================================
#
# Given the input:  def greet(name)
#
# The tokenizer produces a flat list of token hashrefs:
#
#   { type => "DEF",    value => "def",   line => 1, col => 1  }
#   { type => "NAME",   value => "greet", line => 1, col => 5  }
#   { type => "LPAREN", value => "(",     line => 1, col => 10 }
#   { type => "NAME",   value => "name",  line => 1, col => 11 }
#   { type => "RPAREN", value => ")",     line => 1, col => 15 }
#   { type => "EOF",    value => "",      line => 1, col => 16 }
#
# Whitespace is consumed silently — skip patterns in `ruby.tokens`
# match whitespace and it is never emitted as a token.
#
# # Architecture
# ==============
#
# 1. **Grammar loading** — `_grammar()` requires the generated
#    `CodingAdventures::RubyLexer::_Grammar` module and calls its
#    `token_grammar()` sub, caching the result for the lifetime of the
#    process. That module is produced ahead of time from `ruby.tokens` by
#    `code/programs/perl/grammar-tools/grammar-tools.pl compile-tokens` and
#    checked into git alongside this package — no disk I/O against
#    `code/grammars/` happens at runtime, so the package works the same way
#    whether it's used in this monorepo checkout or installed standalone
#    from CPAN.
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
# # Token types
# =============
#
# NAME        — identifiers; promoted to keyword type if in keywords section
# NUMBER      — integer literals (e.g. 42, 0)
# STRING      — double-quoted string literals
#
# Keyword tokens (promoted from NAME):
#   IF, ELSE, ELSIF, END, WHILE, FOR, DO, DEF, RETURN, CLASS, MODULE,
#   REQUIRE, PUTS, TRUE, FALSE, NIL, AND, OR, NOT, THEN, UNLESS, UNTIL,
#   YIELD, BEGIN, RESCUE, ENSURE
#
# Multi-char operators (matched before single-char ones):
#   EQUALS_EQUALS, DOT_DOT, HASH_ROCKET, NOT_EQUALS,
#   LESS_EQUALS, GREATER_EQUALS
#
# Single-char operators:
#   EQUALS, PLUS, MINUS, STAR, SLASH, LESS_THAN, GREATER_THAN
#
# Delimiters:
#   LPAREN, RPAREN, COMMA, COLON
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use CodingAdventures::GrammarTools;

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
my $_keyword_map;  # hashref mapping keyword string → promoted token type

# --- _grammar() ---------------------------------------------------------------
#
# Load `ruby.tokens`, caching the result. The grammar is no longer read from
# disk at runtime: `code/programs/perl/grammar-tools/grammar-tools.pl` compiles
# `ruby.tokens` once, at dev time, into the checked-in
# `CodingAdventures::RubyLexer::_Grammar` module (see
# lib/CodingAdventures/RubyLexer/_Grammar.pm). That generated module defines
# `token_grammar()`, which rebuilds the same `TokenGrammar` object that used
# to come from parsing the `.tokens` file directly. This means a real CPAN
# install of this package works without shipping `code/grammars/`.
# Returns a CodingAdventures::GrammarTools::TokenGrammar object.

sub _grammar {
    return $_grammar if $_grammar;

    require CodingAdventures::RubyLexer::_Grammar;
    $_grammar = CodingAdventures::RubyLexer::_Grammar::token_grammar();

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
#                     This is critical for operators like ==, =>, .., etc.
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
            # Security: reject patterns containing Perl code-execution constructs.
            # (?{ ... }) and (??{ ... }) allow arbitrary Perl code to run inside
            # a regex match. These constructs should never appear in a grammar
            # file from disk. Die early rather than silently execute injected code.
            # Fixed: 2026-04-10 security review.
            my $raw_pat = $defn->pattern;
            if ( $raw_pat =~ /\(\?{|\(\?\?{/ ) {
                die "Security error: unsafe Perl regex code construct in grammar pattern '$raw_pat'
";
            }
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
            # Security: reject patterns containing Perl code-execution constructs.
            # (?{ ... }) and (??{ ... }) allow arbitrary Perl code to run inside
            # a regex match. These constructs should never appear in a grammar
            # file from disk. Die early rather than silently execute injected code.
            # Fixed: 2026-04-10 security review.
            my $raw_pat = $defn->pattern;
            if ( $raw_pat =~ /\(\?{|\(\?\?{/ ) {
                die "Security error: unsafe Perl regex code construct in grammar pattern '$raw_pat'
";
            }
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

    # If the grammar has no skip definitions (e.g. ruby.tokens has none),
    # add a default whitespace skip so that spaces, tabs, carriage returns, and
    # newlines between tokens are silently consumed — matching the behaviour of
    # the Lua GrammarLexer which applies a built-in whitespace skip when the
    # grammar contains no skip: section.
    unless (@skip_rules) {
        push @skip_rules, qr/\G[ \t\r\n]+/;
    }

    # Build keyword lookup map from the grammar keywords section.
    my %kw_map;
    $kw_map{$_} = uc($_) for @{ $grammar->keywords };
    $_keyword_map = \%kw_map;

    $_skip_rules = \@skip_rules;
    $_rules      = \@rules;
}

# ============================================================================
# Public API
# ============================================================================

# --- tokenize($source) --------------------------------------------------------
#
# Tokenize a Ruby source string.
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
        # Whitespace in Ruby is insignificant between tokens (unlike Python,
        # Ruby uses `end` keywords rather than indentation for blocks).
        # We advance position without emitting anything, but still update
        # line/col so that token positions after whitespace are accurate.

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

                my $tok_type = $rule->{name};
                if ($tok_type eq 'NAME' && exists $_keyword_map->{$value}) {
                    $tok_type = $_keyword_map->{$value};
                }
                push @tokens, {
                    type  => $tok_type,
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
        # A well-formed Ruby source should rarely reach here. We emit a
        # descriptive error including position and the offending character.

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::RubyLexer: LexerError at line %d col %d: "
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

CodingAdventures::RubyLexer - Grammar-driven Ruby tokenizer

=head1 SYNOPSIS

    use CodingAdventures::RubyLexer;

    my $tokens = CodingAdventures::RubyLexer->tokenize('def greet(name)');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<ruby.tokens> file, compiles token definitions to Perl regexes,
and tokenizes Ruby source into a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Whitespace is silently consumed. The last token is always C<EOF>.

Token types include: NAME, NUMBER, STRING; keyword types: DEF, END, CLASS,
MODULE, IF, ELSIF, ELSE, UNLESS, WHILE, UNTIL, FOR, DO, RETURN, BEGIN,
RESCUE, ENSURE, REQUIRE, PUTS, YIELD, THEN, TRUE, FALSE, NIL, AND, OR, NOT;
multi-char operator types: EQUALS_EQUALS, DOT_DOT, HASH_ROCKET, NOT_EQUALS,
LESS_EQUALS, GREATER_EQUALS; single-char operator types: EQUALS, PLUS, MINUS,
STAR, SLASH, LESS_THAN, GREATER_THAN; delimiter types: LPAREN, RPAREN,
COMMA, COLON.

=head1 METHODS

=head2 tokenize($source)

Tokenize a Ruby string. Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
