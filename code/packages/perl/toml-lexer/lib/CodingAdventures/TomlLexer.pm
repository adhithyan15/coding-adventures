package CodingAdventures::TomlLexer;

# ============================================================================
# CodingAdventures::TomlLexer — Grammar-driven TOML tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools and CodingAdventures::Lexer.  It reads
# the shared `toml.tokens` grammar file, compiles the token definitions into
# Perl regexes, and applies them in priority order to tokenize TOML source.
#
# # What is TOML tokenization?
# =============================
#
# TOML (Tom's Obvious, Minimal Language) is a configuration file format
# designed to be easy to read. Given the input:
#
#   [server]
#   host = "localhost"
#   port = 8080
#
# The tokenizer produces a flat list of token hashrefs:
#
#   { type => "LBRACKET",     value => "[",           line => 1, col => 1 }
#   { type => "BARE_KEY",     value => "server",      line => 1, col => 2 }
#   { type => "RBRACKET",     value => "]",           line => 1, col => 8 }
#   { type => "BARE_KEY",     value => "host",        line => 2, col => 1 }
#   { type => "EQUALS",       value => "=",           line => 2, col => 6 }
#   { type => "BASIC_STRING", value => '"localhost"', line => 2, col => 8 }
#   { type => "BARE_KEY",     value => "port",        line => 3, col => 1 }
#   { type => "EQUALS",       value => "=",           line => 3, col => 6 }
#   { type => "INTEGER",      value => "8080",        line => 3, col => 8 }
#   { type => "EOF",          value => "",            line => 4, col => 1 }
#
# Horizontal whitespace (spaces and tabs) and TOML comments (#...) are
# consumed silently via the skip patterns in `toml.tokens`. Newlines are
# NOT in the skip list — TOML is newline-sensitive.
#
# # TOML-specific concerns
# =========================
#
# **Pattern ordering** — The grammar orders more-specific patterns first:
#   - Multi-line strings (""", ''') before single-line strings
#   - Date/time patterns before bare keys and integers
#   - Floats (FLOAT_DEC, FLOAT_EXP, FLOAT_SPECIAL) before integers
#   - Boolean literals (true, false) before BARE_KEY
#
# **Aliases** — Multiple grammar names map to single token types:
#   FLOAT_SPECIAL, FLOAT_EXP, FLOAT_DEC  → FLOAT
#   HEX_INTEGER, OCT_INTEGER, BIN_INTEGER → INTEGER
#
# # Architecture
# ==============
#
# 1. **Grammar loading** — `_grammar()` opens `toml.tokens`, parses it with
#    `CodingAdventures::GrammarTools::parse_token_grammar`, and caches the
#    resulting `TokenGrammar` object for the lifetime of the process.
#
# 2. **Pattern compilation** — `_build_rules()` converts every `TokenDefinition`
#    in the grammar into a `{ name => ..., pat => qr/\G.../ }` hashref.
#    Regex definitions get `qr/\G(?:<pattern>)/` directly; literal definitions
#    get `qr/\G\Q<literal>\E/` so that Perl interprets them as plain text.
#
# 3. **Tokenization** — `tokenize()` walks the source string using Perl's
#    `\G` + `pos()` mechanism, trying skip patterns first (horizontal
#    whitespace and comments) and then token patterns in definition order.
#    The first match wins. On a match, a token hashref is pushed and position
#    is advanced. On no match, a `die` is raised with position info.
#
# # Path navigation
# =================
#
# `__FILE__` resolves to `lib/CodingAdventures/TomlLexer.pm`.
# `dirname(__FILE__)` → `lib/CodingAdventures`
#
# From there we need to climb to the repo root (`code/`) then descend
# into `grammars/toml.tokens`:
#
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → toml-lexer/       (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/             ← repo root
#   + /grammars/toml.tokens
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
# wasteful. We cache the TokenGrammar object and the compiled rule lists in
# package-level variables. They are populated on the first call and reused
# thereafter.

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
    # __FILE__ = .../code/packages/perl/toml-lexer/lib/CodingAdventures/TomlLexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels: CodingAdventures/ → lib/ → toml-lexer/ → perl/ → packages/ → code/
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

# --- _grammar() ---------------------------------------------------------------
#
# Load and parse `toml.tokens`, caching the result.
# Returns a CodingAdventures::GrammarTools::TokenGrammar object.

sub _grammar {
    return $_grammar if $_grammar;

    my $tokens_file = File::Spec->catfile( _grammars_dir(), 'toml.tokens' );
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::TomlLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = parse_token_grammar($content);
    die "CodingAdventures::TomlLexer: failed to parse toml.tokens: $err"
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
# Token type resolution: if a definition has an alias (the `-> ALIAS` syntax
# in .tokens files), we emit the alias as the token type; otherwise we emit
# the definition name.
#
# TOML-specific: FLOAT_SPECIAL, FLOAT_EXP, FLOAT_DEC all alias to FLOAT;
# HEX_INTEGER, OCT_INTEGER, BIN_INTEGER all alias to INTEGER.

sub _build_rules {
    return if $_rules;    # already built

    my $grammar = _grammar();
    my (@rules, @skip_rules);

    # Build skip patterns — horizontal whitespace and TOML comments.
    # Note: newlines are NOT in the skip list; TOML is newline-sensitive.
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

    # Build token patterns in grammar definition order.
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
# Tokenize a TOML source string.
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
# Note on newlines in TOML: the skip patterns in `toml.tokens` do NOT include
# newlines. Therefore, a bare `\n` in the source will reach the token-matching
# phase and fail to match any token rule — raising a LexerError. A TOML parser
# that uses this lexer should handle newlines at the grammar level, or the
# calling code should strip/normalize trailing newlines before tokenizing.
# The grammar infrastructure itself does not emit NEWLINE tokens for TOML in
# the same way it might for a language with explicit NEWLINE rules.
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
        # Skip patterns are tried before token patterns.  TOML horizontal
        # whitespace (spaces and tabs) and comments (#...) are skipped silently.
        # Newlines are NOT skipped — they are significant in TOML.

        my $skipped = 0;
        for my $spat (@$_skip_rules) {
            pos($source) = $pos;
            if ($source =~ /$spat/gc) {
                my $matched = $&;

                # Count newlines to update line/col tracking.
                # In practice, TOML skip patterns don't match newlines, but
                # we handle the general case for correctness.
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
        # failure, anchored to \G). The first match wins.

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

                # Advance position.
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
        # A well-formed TOML document should never reach here.  We emit a
        # descriptive error that includes the position and the offending
        # character to aid debugging.

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::TomlLexer: LexerError at line %d col %d: "
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

CodingAdventures::TomlLexer - Grammar-driven TOML tokenizer

=head1 SYNOPSIS

    use CodingAdventures::TomlLexer;

    my $tokens = CodingAdventures::TomlLexer->tokenize('key = "value"');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<toml.tokens> file, compiles token definitions to Perl regexes,
and tokenizes TOML source into a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Horizontal whitespace and comments are silently consumed.  The last token is
always C<EOF>.

Token types include: C<BARE_KEY>, C<BASIC_STRING>, C<LITERAL_STRING>,
C<ML_BASIC_STRING>, C<ML_LITERAL_STRING>, C<INTEGER>, C<FLOAT>, C<TRUE>,
C<FALSE>, C<OFFSET_DATETIME>, C<LOCAL_DATETIME>, C<LOCAL_DATE>, C<LOCAL_TIME>,
C<EQUALS>, C<DOT>, C<COMMA>, C<LBRACKET>, C<RBRACKET>, C<LBRACE>, C<RBRACE>,
C<EOF>.

=head1 METHODS

=head2 tokenize($source)

Tokenize a TOML string.  Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
