package CodingAdventures::SqlLexer;

# ============================================================================
# CodingAdventures::SqlLexer — Grammar-driven SQL tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools and CodingAdventures::Lexer.  It reads
# the shared `sql.tokens` grammar file, compiles the token definitions into
# Perl regexes, and applies them in priority order to tokenize SQL source.
#
# # What is SQL tokenization?
# =============================
#
# Given the input: SELECT * FROM users WHERE id = 1
#
# The tokenizer produces a flat list of token hashrefs:
#
#   { type => "SELECT",  value => "SELECT",  line => 1, col => 1  }
#   { type => "STAR",    value => "*",       line => 1, col => 8  }
#   { type => "FROM",    value => "FROM",    line => 1, col => 10 }
#   { type => "NAME",    value => "users",   line => 1, col => 15 }
#   { type => "WHERE",   value => "WHERE",   line => 1, col => 21 }
#   { type => "NAME",    value => "id",      line => 1, col => 27 }
#   { type => "EQUALS",  value => "=",       line => 1, col => 30 }
#   { type => "NUMBER",  value => "1",       line => 1, col => 32 }
#   { type => "EOF",     value => "",        line => 1, col => 33 }
#
# Whitespace, line comments (-- ...), and block comments (/* ... */) are
# consumed silently — they appear in `sql.tokens` as skip patterns and are
# never emitted as tokens.
#
# # SQL-specific concerns
# ========================
#
# **Case-insensitive keywords** — The `sql.tokens` grammar has the directive
# `@case_insensitive true`. Keywords like SELECT, select, and Select all
# produce a SELECT token.  The GrammarTools infrastructure compiles keyword
# patterns case-insensitively.
#
# **Keywords vs identifiers** — Keywords are listed in the grammar's
# `keywords:` block and match before the generic `NAME` pattern (identifiers).
# Because we try patterns in order, keywords take priority.
#
# **Operator ordering** — Longer operators match before shorter ones:
#   `<=` before `<`, `>=` before `>`, `!=` before nothing.
# The grammar enforces this via definition order.
#
# **NEQ_ANSI alias** — `<>` is aliased to NOT_EQUALS so a parser handles only
# one token type for both `!=` and `<>`.
#
# **STRING alias** — Single-quoted strings (`STRING_SQ`) are aliased to
# STRING. Backtick-quoted identifiers (`QUOTED_ID`) are aliased to NAME.
#
# # Architecture
# ==============
#
# 1. **Grammar loading** — `_grammar()` opens `sql.tokens`, parses it with
#    `CodingAdventures::GrammarTools::parse_token_grammar`, and caches the
#    resulting `TokenGrammar` object for the lifetime of the process.
#
# 2. **Pattern compilation** — `_build_rules()` converts every `TokenDefinition`
#    in the grammar into a `{ name => ..., pat => qr/\G.../ }` hashref.
#    Regex definitions get `qr/\G(?:<pattern>)/`; literal definitions get
#    `qr/\G\Q<literal>\E/`.  For case-insensitive grammars, the GrammarTools
#    layer already handles case folding in keyword patterns.
#
# 3. **Tokenization** — `tokenize()` walks the source string using Perl's
#    `\G` + `pos()` mechanism, trying skip patterns first and then token
#    patterns in definition order.  The first match wins.
#
# # Path navigation
# =================
#
# `__FILE__` resolves to `lib/CodingAdventures/SqlLexer.pm`.
# `dirname(__FILE__)` → `lib/CodingAdventures`
#
# From there we need to climb to the repo root (`code/`) then descend
# into `grammars/sql.tokens`:
#
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → sql-lexer/        (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/             ← repo root
#   + /grammars/sql.tokens
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

sub _grammars_dir {
    # __FILE__ = .../code/packages/perl/sql-lexer/lib/CodingAdventures/SqlLexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels: CodingAdventures/ → lib/ → sql-lexer/ → perl/ → packages/ → code/
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

# --- _grammar() ---------------------------------------------------------------
#
# Load and parse `sql.tokens`, caching the result.
# Returns a CodingAdventures::GrammarTools::TokenGrammar object.

sub _grammar {
    return $_grammar if $_grammar;

    my $tokens_file = File::Spec->catfile( _grammars_dir(), 'sql.tokens' );
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::SqlLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = parse_token_grammar($content);
    die "CodingAdventures::SqlLexer: failed to parse sql.tokens: $err"
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
#   is_regex == 1  →  wrap `$defn->pattern` in qr/\G(?:<pattern>)/
#   is_regex == 0  →  literal: qr/\G\Q<literal>\E/
#
# Token type resolution: emit the alias if present, else the definition name.
# SQL aliases: STRING_SQ → STRING, QUOTED_ID → NAME, NEQ_ANSI → NOT_EQUALS.

sub _build_rules {
    return if $_rules;    # already built

    my $grammar = _grammar();
    my (@rules, @skip_rules);

    # Build skip patterns — whitespace, line comments, block comments.
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
# Tokenize a SQL source string.
#
# Algorithm:
#
#   1. Ensure the grammar and compiled rules are loaded (_build_rules).
#   2. Walk the source from position 0 to end.
#   3. At each position, try each skip pattern with /gc.
#      If a skip pattern matches, update line/col tracking and continue.
#   4. If no skip matched, try each token pattern in order.
#      The first match: record token, advance pos, update tracking, continue.
#   5. If nothing matched, die with a descriptive error message.
#   6. After exhausting the input, push an EOF sentinel and return.
#
# Line/column tracking:
#
#   - `$line` starts at 1, incremented for each '\n' in matched text.
#   - `$col` starts at 1; if match contains newlines, resets to length
#     of text after the last newline + 1.
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
        # SQL whitespace includes spaces, tabs, carriage returns, and newlines.
        # SQL line comments (-- ...) and block comments (/* ... */) are also
        # skipped silently. None of these appear in the token output.

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
        # A well-formed SQL statement should never reach here. We emit a
        # descriptive error that includes the position and the offending
        # character to aid debugging.

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::SqlLexer: LexerError at line %d col %d: "
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

CodingAdventures::SqlLexer - Grammar-driven SQL tokenizer

=head1 SYNOPSIS

    use CodingAdventures::SqlLexer;

    my $tokens = CodingAdventures::SqlLexer->tokenize('SELECT * FROM users');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<sql.tokens> file, compiles token definitions to Perl regexes,
and tokenizes SQL source into a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Whitespace and SQL comments are silently consumed.  The last token is always C<EOF>.

Keywords are case-insensitive: C<select>, C<SELECT>, and C<Select> all produce
a C<SELECT> token.

=head1 METHODS

=head2 tokenize($source)

Tokenize a SQL string.  Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
