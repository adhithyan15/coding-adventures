package CodingAdventures::JsonLexer;

# ============================================================================
# CodingAdventures::JsonLexer — Grammar-driven JSON tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools and CodingAdventures::Lexer.  It reads
# the shared `json.tokens` grammar file, compiles the token definitions into
# Perl regexes, and applies them in priority order to tokenize JSON source.
#
# # What is JSON tokenization?
# =============================
#
# Given the input: {"key": 42, "active": true}
#
# The tokenizer produces a flat list of token hashrefs:
#
#   { type => "LBRACE",  value => "{",       line => 1, col => 1  }
#   { type => "STRING",  value => '"key"',   line => 1, col => 2  }
#   { type => "COLON",   value => ":",       line => 1, col => 7  }
#   { type => "NUMBER",  value => "42",      line => 1, col => 9  }
#   { type => "COMMA",   value => ",",       line => 1, col => 11 }
#   { type => "STRING",  value => '"active"',line => 1, col => 13 }
#   { type => "COLON",   value => ":",       line => 1, col => 21 }
#   { type => "TRUE",    value => "true",    line => 1, col => 23 }
#   { type => "RBRACE",  value => "}",       line => 1, col => 27 }
#   { type => "EOF",     value => "",        line => 1, col => 28 }
#
# Whitespace is consumed silently — it appears in `json.tokens` as a `skip:`
# pattern and is never emitted as a token.
#
# # Architecture
# ==============
#
# 1. **Grammar loading** — `_grammar()` opens `json.tokens`, parses it with
#    `CodingAdventures::GrammarTools::parse_token_grammar`, and caches the
#    resulting `TokenGrammar` object for the lifetime of the process.
#
# 2. **Pattern compilation** — `_build_rules()` converts every `TokenDefinition`
#    in the grammar into a `{ name => ..., pat => qr/\G.../ }` hashref.
#    Regex definitions get `qr/\G<pattern>/` directly; literal definitions get
#    `qr/\G\Q<literal>\E/` so that Perl interprets them as plain text.
#
# 3. **Tokenization** — `tokenize()` walks the source string character by
#    character using Perl's `\G` + `pos()` mechanism, trying skip patterns
#    first and then token patterns in definition order.  The first match wins.
#    On a match, a token hashref is pushed and position is advanced.
#    On no match, a `die` is raised with position info.
#
# # Path navigation
# =================
#
# `__FILE__` resolves to `lib/CodingAdventures/JsonLexer.pm`.
# `dirname(__FILE__)` → `lib/CodingAdventures`
#
# From there we need to climb to the repo root (`code/`) then descend
# into `grammars/json.tokens`:
#
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → json-lexer/       (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/             ← repo root
#   + /grammars/json.tokens
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
# Reading and parsing the grammar file on every tokenize() call would be
# wasteful.  We cache the TokenGrammar object and the compiled rule list
# in package-level variables.  They are populated on the first call and
# reused thereafter.

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
    # __FILE__ = .../code/packages/perl/json-lexer/lib/CodingAdventures/JsonLexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels: CodingAdventures/ → lib/ → json-lexer/ → perl/ → packages/ → code/
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

# --- _grammar() ---------------------------------------------------------------
#
# Load and parse `json.tokens`, caching the result.
# Returns a CodingAdventures::GrammarTools::TokenGrammar object.

sub _grammar {
    return $_grammar if $_grammar;

    my $tokens_file = File::Spec->catfile( _grammars_dir(), 'json.tokens' );
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::JsonLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_token_grammar($content);
    die "CodingAdventures::JsonLexer: failed to parse json.tokens: $err"
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
#                     Wrap in qr/\G(?:<pattern>)/ so the match is anchored
#                     to the current position.
#
#   is_regex == 0  →  treat `$defn->pattern` as a literal string.
#                     Use `\Q...\E` to disable regex metacharacters.
#
# The `\G` anchor is crucial: it forces the match to start exactly at
# `pos($source)`, preventing the regex engine from skipping ahead.
#
# Token type resolution:  if a definition has an alias (the `-> ALIAS` syntax
# in .tokens files), we emit the alias as the token type; otherwise we emit
# the definition name.

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

    $_skip_rules = \@skip_rules;
    $_rules      = \@rules;
}

# ============================================================================
# Public API
# ============================================================================

# --- tokenize($source) --------------------------------------------------------
#
# Tokenize a JSON source string.
#
# Algorithm:
#
#   1. Ensure the grammar and compiled rules are loaded (_build_rules).
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
        # Skip patterns are tried before token patterns.  Whitespace in JSON
        # is insignificant between tokens; we simply advance the position
        # without emitting anything.  We update line/col tracking so that
        # token positions after whitespace are still accurate.

        my $skipped = 0;
        for my $spat (@$_skip_rules) {
            pos($source) = $pos;
            if ($source =~ /$spat/gc) {
                my $matched = $&;

                # Count newlines to update line/col
                my $nl_count = () = $matched =~ /\n/g;
                if ($nl_count) {
                    $line += $nl_count;
                    # Column resets to 1 plus whatever follows the last newline
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
        # failure, anchored to \G).  The first match wins.

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

                # Advance position
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
        # A well-formed JSON document should never reach here.  We emit a
        # descriptive error that includes the position and the offending
        # character to aid debugging.

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::JsonLexer: LexerError at line %d col %d: "
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

CodingAdventures::JsonLexer - Grammar-driven JSON tokenizer

=head1 SYNOPSIS

    use CodingAdventures::JsonLexer;

    my $tokens = CodingAdventures::JsonLexer->tokenize('{"x": 1}');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<json.tokens> file, compiles token definitions to Perl regexes,
and tokenizes JSON source into a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Whitespace is silently consumed.  The last token is always C<EOF>.

=head1 METHODS

=head2 tokenize($source)

Tokenize a JSON string.  Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
