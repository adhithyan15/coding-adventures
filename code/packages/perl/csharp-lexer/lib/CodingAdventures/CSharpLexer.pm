package CodingAdventures::CSharpLexer;

# ============================================================================
# CodingAdventures::CSharpLexer — Grammar-driven C# tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools and CodingAdventures::Lexer. It reads
# the shared `csharp/csharp<version>.tokens` grammar file, compiles the token
# definitions into Perl regexes, and applies them in priority order to
# tokenize C# source code.
#
# # What is C# tokenization?
# ===========================
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
#   "1.0"  — C# 1.0  (2002): the original release (.NET 1.0).
#   "2.0"  — C# 2.0  (2005): generics, nullable types, iterators.
#   "3.0"  — C# 3.0  (2007): LINQ, lambda expressions, extension methods.
#   "4.0"  — C# 4.0  (2010): dynamic keyword, named/optional arguments.
#   "5.0"  — C# 5.0  (2012): async/await, caller info attributes.
#   "6.0"  — C# 6.0  (2015): null-conditional (?.), string interpolation.
#   "7.0"  — C# 7.0  (2017): tuples, pattern matching, ref returns.
#   "8.0"  — C# 8.0  (2019): nullable reference types, switch expressions.
#   "9.0"  — C# 9.0  (2020): records, init-only setters, top-level programs.
#   "10.0" — C# 10.0 (2021): record structs, global using, file-scoped namespaces.
#   "11.0" — C# 11.0 (2022): raw string literals, list patterns, required members.
#   "12.0" — C# 12.0 (2023): primary constructors, collection expressions.
#   undef / "" — defaults to C# 12.0 (latest).
#
# Version grammar files live under:
#   code/grammars/csharp/csharp<version>.tokens
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
# # C#-specific token highlights
# ================================
#
# C# introduces several operators that Java lacks:
#
#   ??   — null-coalescing operator (C# 2.0+)
#          Returns the left operand if it is not null; otherwise the right.
#          Example:  string name = possiblyNull ?? "default";
#
#   ?.   — null-conditional member access (C# 6.0+)
#          Returns null if the left operand is null; otherwise accesses the member.
#          Example:  int? len = str?.Length;
#
#   ??=  — null-coalescing assignment (C# 8.0+)
#          Assigns the right operand only if the left operand is null.
#          Example:  list ??= new List<int>();
#
# These must be matched before simpler patterns like `?` or `.` because Perl's
# regex engine is greedy and will take the longest match when patterns are
# ordered correctly.
#
# # Path navigation
# =================
#
# `__FILE__` resolves to `lib/CodingAdventures/CSharpLexer.pm`.
# `dirname(__FILE__)` → `lib/CodingAdventures`
#
# From there we climb to the repo root (`code/`) then descend into
# `grammars/`:
#
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → csharp-lexer/   (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/                 ← repo root
#   + /grammars/csharp/csharp<version>.tokens
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use File::Basename qw(dirname);
use File::Spec;
use CodingAdventures::GrammarTools;

# ============================================================================
# Valid C# versions
# ============================================================================
#
# All 12 released C# versions from 1.0 through 12.0.

my %VALID_VERSIONS = map { $_ => 1 } qw(
    1.0 2.0 3.0 4.0 5.0 6.0 7.0 8.0 9.0 10.0 11.0 12.0
);

# ============================================================================
# Default version
# ============================================================================
#
# When no version is specified, we default to C# 12.0 (the latest release).

my $DEFAULT_VERSION = '12.0';

# ============================================================================
# Per-version caches
# ============================================================================
#
# Each cache is a hashref keyed by version string.
# Caching is important because parsing grammar files from disk and compiling
# regexes is expensive — we only do it once per version per process.

my %_grammar_cache;     # version => TokenGrammar
my %_rules_cache;       # version => arrayref of { name => str, pat => qr// }
my %_skip_rules_cache;  # version => arrayref of qr//
my %_keyword_map_cache; # version => hashref  keyword => type

# ============================================================================
# Path helpers
# ============================================================================

sub _grammars_dir {
    # __FILE__ = .../code/packages/perl/csharp-lexer/lib/CodingAdventures/CSharpLexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels: CodingAdventures/ → lib/ → csharp-lexer/ → perl/ → packages/ → code/
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

# --- _resolve_tokens_path($version) ------------------------------------------
#
# Return the absolute path to the correct .tokens grammar file.
#
#   undef / "" → grammars/csharp/csharp12.0.tokens             (default)
#   "1.0"      → grammars/csharp/csharp1.0.tokens
#   "8.0"      → grammars/csharp/csharp8.0.tokens
#   "12.0"     → grammars/csharp/csharp12.0.tokens

sub _resolve_tokens_path {
    my ($class, $version) = @_;
    my $grammars = _grammars_dir();

    # Default to C# 12.0 when no version specified
    $version = $DEFAULT_VERSION unless $version;

    die "CodingAdventures::CSharpLexer: unknown C# version '$version'. "
      . "Valid versions: 1.0 2.0 3.0 4.0 5.0 6.0 7.0 8.0 9.0 10.0 11.0 12.0"
        unless $VALID_VERSIONS{$version};

    return File::Spec->catfile($grammars, 'csharp', "csharp$version.tokens");
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
        or die "CodingAdventures::CSharpLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_token_grammar($content);
    die "CodingAdventures::CSharpLexer: failed to parse '$tokens_file': $err"
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
                die "Security error: unsafe Perl regex code construct in grammar pattern '$raw_pat'\n";
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
                die "Security error: unsafe Perl regex code construct in grammar pattern '$raw_pat'\n";
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
# Tokenize a C# source string.
#
# $version is optional. Valid values: "1.0", "2.0", "3.0", "4.0", "5.0",
# "6.0", "7.0", "8.0", "9.0", "10.0", "11.0", "12.0",
# or undef/"" for C# 12.0 (default).
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
        # Whitespace in C# is insignificant between tokens.
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
        # A well-formed C# source should rarely reach here. We emit a
        # descriptive error including position and the offending character.

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::CSharpLexer: LexerError at line %d col %d: "
              . "unexpected character '%s'",
                $line, $col, $ch
            );
        }
    }

    # Sentinel EOF token — always present as the last element.
    push @tokens, { type => 'EOF', value => '', line => $line, col => $col };

    return \@tokens;
}

# --- new_csharp_lexer($source, $version) --------------------------------------
#
# Convenience function: same as calling tokenize() as a class method.
# Returns the token arrayref.

sub new_csharp_lexer {
    my ($source, $version) = @_;
    return __PACKAGE__->tokenize($source, $version);
}

# --- tokenize_csharp($source, $version) ---------------------------------------
#
# Standalone convenience function (not a method).
# Import with: use CodingAdventures::CSharpLexer qw(tokenize_csharp);

sub tokenize_csharp {
    my ($source, $version) = @_;
    return __PACKAGE__->tokenize($source, $version);
}

1;

__END__

=head1 NAME

CodingAdventures::CSharpLexer - Grammar-driven C# tokenizer

=head1 SYNOPSIS

    use CodingAdventures::CSharpLexer;

    # Default (C# 12.0 grammar)
    my $tokens = CodingAdventures::CSharpLexer->tokenize('int x = 1;');

    # Version-specific
    my $tokens = CodingAdventures::CSharpLexer->tokenize('int x = 1;', '8.0');

    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

    # Convenience functions
    use CodingAdventures::CSharpLexer qw(tokenize_csharp new_csharp_lexer);
    my $tokens = tokenize_csharp('string s = "hello";', '6.0');

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<csharp/csharp<version>.tokens> file, compiles token definitions
to Perl regexes, and tokenizes C# source into a flat list of typed token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Whitespace is silently consumed. The last token is always C<EOF>.

C# introduces operators not found in Java, including:

=over 4

=item C<??>

The null-coalescing operator (C# 2.0+). Returns the left operand if non-null,
otherwise the right operand. Example: C<string s = possiblyNull ?? "default";>

=item C<?.>

The null-conditional member access operator (C# 6.0+). Returns C<null> if the
left operand is null; otherwise accesses the member. Example: C<int? len = str?.Length;>

=item C<??=>

The null-coalescing assignment operator (C# 8.0+). Assigns only if the left
operand is null. Example: C<list ??= new List<int>();>

=back

=head1 METHODS

=head2 tokenize($source, $version)

Tokenize a C# string. C<$version> is optional; valid values are
C<"1.0">, C<"2.0">, C<"3.0">, C<"4.0">, C<"5.0">, C<"6.0">, C<"7.0">,
C<"8.0">, C<"9.0">, C<"10.0">, C<"11.0">, C<"12.0">,
or C<undef>/C<""> for C# 12.0 (default).

Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.
Dies on unknown version string.

=head2 tokenize_csharp($source, $version)

Standalone convenience function (not a method). Same return value as C<tokenize>.

=head2 new_csharp_lexer($source, $version)

Convenience function synonym for C<tokenize>.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
