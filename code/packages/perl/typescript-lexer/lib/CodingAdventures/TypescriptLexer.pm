package CodingAdventures::TypescriptLexer;

# ============================================================================
# CodingAdventures::TypescriptLexer — Grammar-driven TypeScript tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools and CodingAdventures::Lexer. It reads
# the shared `typescript.tokens` grammar file (or a versioned variant),
# compiles the token definitions into Perl regexes, and applies them in
# priority order to tokenize TypeScript source code.
#
# TypeScript is a strict superset of JavaScript. Every valid JavaScript
# program is also valid TypeScript. TypeScript adds:
#   - Type annotations: `let x: number = 1`
#   - Interfaces: `interface Foo { bar: string }`
#   - Generics: `Array<number>`
#   - Access modifiers: `public`, `private`, `protected`
#   - `enum`, `type`, `namespace`, `declare`, `readonly`
#   - Abstract classes, `implements`, `extends`
#   - Type utilities: `keyof`, `infer`, `never`, `unknown`
#   - Primitive type keywords: `any`, `void`, `number`, `string`,
#     `boolean`, `object`, `symbol`, `bigint`
#
# # Version-aware tokenization
# =============================
#
# Pass an optional `$version` argument to `tokenize()`:
#
#   "ts1.0" — TypeScript 1.0 (April 2014): initial public release.
#   "ts2.0" — TypeScript 2.0 (September 2016): non-nullable types.
#   "ts3.0" — TypeScript 3.0 (July 2018): project references, tuples.
#   "ts4.0" — TypeScript 4.0 (August 2020): variadic tuple types.
#   "ts5.0" — TypeScript 5.0 (March 2023): decorators (Stage 3).
#   "ts5.8" — TypeScript 5.8 (February 2025): granular control-flow.
#   undef / "" — Generic TypeScript (uses typescript.tokens).
#
# Version grammar files live under:
#   code/grammars/typescript/<version>.tokens
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
# `__FILE__` resolves to `lib/CodingAdventures/TypescriptLexer.pm`.
# `dirname(__FILE__)` → `lib/CodingAdventures`
#
# From there we climb to the repo root (`code/`) then descend into
# `grammars/`:
#
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → typescript-lexer/    (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/                ← repo root
#   + /grammars/typescript.tokens  (or typescript/<version>.tokens)
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.02';

use File::Basename qw(dirname);
use File::Spec;
use CodingAdventures::GrammarTools;

# ============================================================================
# Valid TypeScript versions
# ============================================================================

my %VALID_VERSIONS = map { $_ => 1 } qw(ts1.0 ts2.0 ts3.0 ts4.0 ts5.0 ts5.8);

# ============================================================================
# Per-version caches
# ============================================================================
#
# Each cache is a hashref keyed by version string ("" = generic).
# This allows different versions to load different grammar files while
# sharing the common tokenization machinery.

my %_grammar_cache;    # version => TokenGrammar
my %_rules_cache;      # version => arrayref of { name => str, pat => qr// }
my %_skip_rules_cache; # version => arrayref of qr//
my %_keyword_map_cache; # version => hashref  keyword => type

# ============================================================================
# Path helpers
# ============================================================================

sub _grammars_dir {
    # __FILE__ = .../code/packages/perl/typescript-lexer/lib/CodingAdventures/TypescriptLexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels: CodingAdventures/ → lib/ → typescript-lexer/ → perl/ → packages/ → code/
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

# --- _resolve_tokens_path($version) ------------------------------------------
#
# Return the absolute path to the correct .tokens grammar file.
#
#   undef / "" → grammars/typescript.tokens         (generic)
#   "ts5.0"    → grammars/typescript/ts5.0.tokens

sub _resolve_tokens_path {
    my ($class, $version) = @_;
    my $grammars = _grammars_dir();

    return File::Spec->catfile($grammars, 'typescript.tokens')
        unless $version;

    die "CodingAdventures::TypescriptLexer: unknown TypeScript version '$version'. "
      . "Valid versions: ts1.0 ts2.0 ts3.0 ts4.0 ts5.0 ts5.8"
        unless $VALID_VERSIONS{$version};

    return File::Spec->catfile($grammars, 'typescript', "$version.tokens");
}

# --- _grammar($version) -------------------------------------------------------
#
# Load and parse the grammar for `$version`, caching the result.

sub _grammar {
    my ($class, $version) = @_;
    $version //= '';

    return $_grammar_cache{$version} if $_grammar_cache{$version};

    my $tokens_file = $class->_resolve_tokens_path($version);
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::TypescriptLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_token_grammar($content);
    die "CodingAdventures::TypescriptLexer: failed to parse '$tokens_file': $err"
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
    $version //= '';

    return if $_rules_cache{$version};    # already built for this version

    my $grammar = $class->_grammar($version);
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
# Tokenize a TypeScript source string.
#
# $version is optional. Valid values: "ts1.0", "ts2.0", "ts3.0", "ts4.0",
# "ts5.0", "ts5.8", or undef/"" for generic TypeScript.
#
# Recognizes all JavaScript tokens plus TypeScript-specific keywords:
# INTERFACE, TYPE, ENUM, NAMESPACE, DECLARE, READONLY, PUBLIC, PRIVATE,
# PROTECTED, ABSTRACT, IMPLEMENTS, EXTENDS, KEYOF, INFER, NEVER, UNKNOWN,
# ANY, VOID, and type-keyword variants of NUMBER, STRING, BOOLEAN, OBJECT,
# SYMBOL, BIGINT.
#
# Return value: arrayref of hashrefs {type, value, line, col}.
# Last element always has type 'EOF'.
#
# Raises: `die` on unexpected input or unknown version.

sub tokenize {
    my ($class_or_self, $source, $version) = @_;
    $version //= '';

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
        # Whitespace in TypeScript is insignificant between tokens.
        # We advance position without emitting anything, updating line/col.

        my $skipped = 0;
        for my $spat (@$skip_rules) {
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

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::TypescriptLexer: LexerError at line %d col %d: "
              . "unexpected character '%s'",
                $line, $col, $ch
            );
        }
    }

    push @tokens, { type => 'EOF', value => '', line => $line, col => $col };

    return \@tokens;
}

1;

__END__

=head1 NAME

CodingAdventures::TypescriptLexer - Grammar-driven TypeScript tokenizer

=head1 SYNOPSIS

    use CodingAdventures::TypescriptLexer;

    # Generic (latest grammar)
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('interface Foo { x: number }');

    # Version-specific
    my $tokens = CodingAdventures::TypescriptLexer->tokenize('let x = 1;', 'ts5.0');

    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<typescript.tokens> file (or a versioned variant), compiles token
definitions to Perl regexes, and tokenizes TypeScript source into a flat list of
token hashrefs.

TypeScript is a strict superset of JavaScript. This lexer recognizes all JavaScript
tokens plus TypeScript-specific keywords.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Whitespace is silently consumed. The last token is always C<EOF>.

=head1 METHODS

=head2 tokenize($source, $version)

Tokenize a TypeScript string. C<$version> is optional; valid values are
C<"ts1.0">, C<"ts2.0">, C<"ts3.0">, C<"ts4.0">, C<"ts5.0">, C<"ts5.8">,
or C<undef>/C<""> for generic TypeScript.

Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.
Dies on unknown version string.

=head1 VERSION

0.02

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
