package CodingAdventures::JavaLexer;

# ============================================================================
# CodingAdventures::JavaLexer — Grammar-driven Java tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools and CodingAdventures::Lexer. It reads
# the shared `java<version>.tokens` grammar file, compiles the token
# definitions into Perl regexes, and applies them in priority order to
# tokenize Java source code.
#
# # What is Java tokenization?
# ============================
#
# Given the input:  int x = 42;
#
# The tokenizer produces a flat list of token hashrefs:
#
#   { type => "INT",       value => "int",  line => 1, col => 1  }
#   { type => "NAME",      value => "x",    line => 1, col => 5  }
#   { type => "EQUALS",    value => "=",    line => 1, col => 7  }
#   { type => "NUMBER",    value => "42",   line => 1, col => 9  }
#   { type => "SEMICOLON", value => ";",    line => 1, col => 11 }
#   { type => "EOF",       value => "",     line => 1, col => 12 }
#
# Whitespace is consumed silently — skip patterns in the grammar file
# match whitespace and it is never emitted as a token.
#
# # Version-aware tokenization
# =============================
#
# Pass an optional `$version` argument to `tokenize()`:
#
#   "1.0" — Java 1.0  (1996): the original release.
#   "1.1" — Java 1.1  (1997): inner classes.
#   "1.4" — Java 1.4  (2002): assert keyword.
#   "5"   — Java 5    (2004): generics, annotations, enums.
#   "7"   — Java 7    (2011): try-with-resources, diamond operator.
#   "8"   — Java 8    (2014): lambdas, default methods, streams.
#   "10"  — Java 10   (2018): local variable type inference (var).
#   "14"  — Java 14   (2020): switch expressions, records (preview).
#   "17"  — Java 17   (2021): sealed classes, pattern matching.
#   "21"  — Java 21   (2023): virtual threads, record patterns.
#   undef / "" — defaults to Java 21 (latest).
#
# Version grammar files live under:
#   code/grammars/java/java<version>.tokens
#
# # Architecture
# ==============
#
# 1. **Grammar loading** — `_grammar($version)` opens the correct .tokens
#    file, parses it with `CodingAdventures::GrammarTools::parse_token_grammar`,
#    and caches the result per-version.
#
# 2. **Pattern compilation** — `_build_rules($version)` converts every
#    TokenDefinition in the grammar into a `{ name => str, pat => qr/\G.../ }`
#    hashref, cached per-version.
#
# 3. **Tokenization** — `tokenize()` walks the source string using Perl's
#    `\G` + `pos()` mechanism, trying skip patterns first and then token
#    patterns in definition order. First match wins.
#
# # Path navigation
# =================
#
# `__FILE__` resolves to `lib/CodingAdventures/JavaLexer.pm`.
# `dirname(__FILE__)` → `lib/CodingAdventures`
#
# From there we climb to the repo root (`code/`) then descend into
# `grammars/`:
#
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → java-lexer/     (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/                 ← repo root
#   + /grammars/java/java<version>.tokens
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use File::Basename qw(dirname);
use File::Spec;
use CodingAdventures::GrammarTools;

# ============================================================================
# Valid Java versions
# ============================================================================

my %VALID_VERSIONS = map { $_ => 1 } qw(
    1.0 1.1 1.4 5 7 8 10 14 17 21
);

# ============================================================================
# Default version
# ============================================================================
#
# When no version is specified, we default to Java 21 (the latest LTS release).

my $DEFAULT_VERSION = '21';

# ============================================================================
# Per-version caches
# ============================================================================
#
# Each cache is a hashref keyed by version string.

my %_grammar_cache;     # version => TokenGrammar
my %_rules_cache;       # version => arrayref of { name => str, pat => qr// }
my %_skip_rules_cache;  # version => arrayref of qr//
my %_keyword_map_cache; # version => hashref  keyword => type

# ============================================================================
# Path helpers
# ============================================================================

sub _grammars_dir {
    # __FILE__ = .../code/packages/perl/java-lexer/lib/CodingAdventures/JavaLexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels: CodingAdventures/ → lib/ → java-lexer/ → perl/ → packages/ → code/
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

# --- _resolve_tokens_path($version) ------------------------------------------
#
# Return the absolute path to the correct .tokens grammar file.
#
#   undef / "" → grammars/java/java21.tokens             (default)
#   "1.0"      → grammars/java/java1.0.tokens
#   "8"        → grammars/java/java8.tokens
#   "21"       → grammars/java/java21.tokens

sub _resolve_tokens_path {
    my ($class, $version) = @_;
    my $grammars = _grammars_dir();

    # Default to Java 21 when no version specified
    $version = $DEFAULT_VERSION unless $version;

    die "CodingAdventures::JavaLexer: unknown Java version '$version'. "
      . "Valid versions: 1.0 1.1 1.4 5 7 8 10 14 17 21"
        unless $VALID_VERSIONS{$version};

    return File::Spec->catfile($grammars, 'java', "java$version.tokens");
}

# --- _grammar($version) -------------------------------------------------------
#
# Load and parse the grammar for `$version`, caching the result.

sub _grammar {
    my ($class, $version) = @_;
    $version //= $DEFAULT_VERSION;

    return $_grammar_cache{$version} if $_grammar_cache{$version};

    my $tokens_file = $class->_resolve_tokens_path($version);
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::JavaLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_token_grammar($content);
    die "CodingAdventures::JavaLexer: failed to parse '$tokens_file': $err"
        unless $grammar;

    $_grammar_cache{$version} = $grammar;
    return $grammar;
}

# --- _build_rules($version) ---------------------------------------------------
#
# Convert TokenGrammar definitions into compiled Perl pattern lists,
# cached per version.

sub _build_rules {
    my ($class, $version) = @_;
    $version //= $DEFAULT_VERSION;

    return if $_rules_cache{$version};    # already built for this version

    my $grammar = $class->_grammar($version);
    my (@rules, @skip_rules);

    # Build skip patterns
    for my $defn ( @{ $grammar->skip_definitions } ) {
        my $pat;
        if ( $defn->is_regex ) {
            # Security: reject patterns containing Perl code-execution constructs.
            # (?{ ... }) and (??{ ... }) allow arbitrary Perl code to run inside
            # a regex match. These constructs should never appear in a grammar
            # file from disk. Die early rather than silently execute injected code.
            my $raw_pat = $defn->pattern;
            if ( $raw_pat =~ /\(\?\{|\(\?\?\{/ ) {
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
            my $raw_pat = $defn->pattern;
            if ( $raw_pat =~ /\(\?\{|\(\?\?\{/ ) {
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

    # Add default whitespace skip if grammar has no skip: section.
    unless (@skip_rules) {
        push @skip_rules, qr/\G[ \t\r\n]+/;
    }

    # Build keyword lookup map from the grammar keywords section.
    my %kw_map;
    $kw_map{$_} = uc($_) for @{ $grammar->keywords };

    $_skip_rules_cache{$version}  = \@skip_rules;
    $_rules_cache{$version}       = \@rules;
    $_keyword_map_cache{$version} = \%kw_map;
}

# ============================================================================
# Public API
# ============================================================================

# --- tokenize($source, $version) ----------------------------------------------
#
# Tokenize a Java source string.
#
# $version is optional. Valid values: "1.0", "1.1", "1.4", "5", "7", "8",
# "10", "14", "17", "21", or undef/"" for Java 21 (default).
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
# Return value:
#
#   An arrayref of hashrefs, each with keys: type, value, line, col.
#   The last element always has type 'EOF'.
#
# Raises:
#
#   `die` with a "LexerError" message on unexpected input.
#   `die` on unknown version string.

sub tokenize {
    my ($class_or_self, $source, $version) = @_;
    $version //= $DEFAULT_VERSION;

    $class_or_self->_build_rules($version);

    my $rules       = $_rules_cache{$version};
    my $skip_rules  = $_skip_rules_cache{$version};
    my $keyword_map = $_keyword_map_cache{$version};

    my @tokens;
    my $line = 1;
    my $col  = 1;
    my $pos  = 0;
    my $len  = length($source);

    while ($pos < $len) {
        pos($source) = $pos;

        # ---- Try skip patterns -----------------------------------------------
        #
        # Whitespace in Java is insignificant between tokens.
        # We advance position without emitting anything, but still update
        # line/col tracking so that token positions after whitespace are accurate.

        my $skipped = 0;
        for my $spat (@$skip_rules) {
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
        for my $rule (@$rules) {
            pos($source) = $pos;
            if ($source =~ /$rule->{pat}/gc) {
                my $value = $&;

                my $tok_type = $rule->{name};
                if ($tok_type eq 'NAME' && exists $keyword_map->{$value}) {
                    $tok_type = $keyword_map->{$value};
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
        # A well-formed Java source should rarely reach here. We emit a
        # descriptive error including position and the offending character.

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::JavaLexer: LexerError at line %d col %d: "
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

CodingAdventures::JavaLexer - Grammar-driven Java tokenizer

=head1 SYNOPSIS

    use CodingAdventures::JavaLexer;

    # Default (Java 21 grammar)
    my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;');

    # Version-specific
    my $tokens = CodingAdventures::JavaLexer->tokenize('int x = 1;', '1.0');

    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<java/java<version>.tokens> file, compiles token definitions to
Perl regexes, and tokenizes Java source into a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Whitespace is silently consumed. The last token is always C<EOF>.

=head1 METHODS

=head2 tokenize($source, $version)

Tokenize a Java string. C<$version> is optional; valid values are
C<"1.0">, C<"1.1">, C<"1.4">, C<"5">, C<"7">, C<"8">, C<"10">,
C<"14">, C<"17">, C<"21">,
or C<undef>/C<""> for Java 21 (default).

Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.
Dies on unknown version string.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
