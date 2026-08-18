package CodingAdventures::TypescriptLexer;

# ============================================================================
# CodingAdventures::TypescriptLexer — Grammar-driven TypeScript tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools and CodingAdventures::Lexer. It loads
# the shared TypeScript grammar (or a versioned variant) from a compiled
# native Perl module checked into git, compiles the token definitions into
# Perl regexes, and applies them in priority order to tokenize TypeScript
# source code.
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
#   undef / "" — Generic TypeScript (uses the default grammar).
#
# # Architecture
# ==============
#
# 1. **Grammar loading** — `_grammar($version)` looks up the compiled
#    grammar module for `$version` in `%GRAMMAR_MODULE` and calls its
#    `token_grammar()` sub, caching the result per-version. Each module
#    (`_Grammar.pm`, `_Grammar_ts1_0.pm`, ...) was generated at dev time
#    via `grammar-tools.pl compile-tokens ... -p <Package::Name>` from the
#    corresponding `.tokens` file under `code/grammars/typescript/` and is
#    checked into git — no runtime disk reads outside the installed package.
#
# 2. **Pattern compilation** — `_build_rules($version)` converts every
#    TokenDefinition in the grammar into a `{ name => str, pat => qr/\G.../ }`
#    hashref, cached per-version.
#
# 3. **Tokenization** — `tokenize()` walks the source string using Perl's
#    `\G` + `pos()` mechanism, trying skip patterns first and then token
#    patterns in definition order. First match wins.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.03';

use CodingAdventures::GrammarTools;

require CodingAdventures::TypescriptLexer::_Grammar;        # generic default
require CodingAdventures::TypescriptLexer::_Grammar_ts1_0;
require CodingAdventures::TypescriptLexer::_Grammar_ts2_0;
require CodingAdventures::TypescriptLexer::_Grammar_ts3_0;
require CodingAdventures::TypescriptLexer::_Grammar_ts4_0;
require CodingAdventures::TypescriptLexer::_Grammar_ts5_0;
require CodingAdventures::TypescriptLexer::_Grammar_ts5_8;

# ============================================================================
# Valid TypeScript versions
# ============================================================================

my %VALID_VERSIONS = map { $_ => 1 } qw(ts1.0 ts2.0 ts3.0 ts4.0 ts5.0 ts5.8);

# ============================================================================
# Grammar module dispatch table
# ============================================================================
#
# Each compiled grammar module was generated at dev time from a .tokens file
# via `grammar-tools.pl compile-tokens ... -p <Package::Name>` and is checked
# into git under lib/CodingAdventures/TypescriptLexer/_Grammar*.pm. This
# avoids reading code/grammars/ off disk at runtime — a real CPAN install of
# this package would not ship that directory.

my %GRAMMAR_MODULE = (
    ''      => 'CodingAdventures::TypescriptLexer::_Grammar',
    'ts1.0' => 'CodingAdventures::TypescriptLexer::_Grammar_ts1_0',
    'ts2.0' => 'CodingAdventures::TypescriptLexer::_Grammar_ts2_0',
    'ts3.0' => 'CodingAdventures::TypescriptLexer::_Grammar_ts3_0',
    'ts4.0' => 'CodingAdventures::TypescriptLexer::_Grammar_ts4_0',
    'ts5.0' => 'CodingAdventures::TypescriptLexer::_Grammar_ts5_0',
    'ts5.8' => 'CodingAdventures::TypescriptLexer::_Grammar_ts5_8',
);

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
# Grammar loading
# ============================================================================

# --- _grammar($version) -------------------------------------------------------
#
# Look up the compiled grammar for `$version`, caching the result.
#
#   undef / "" → CodingAdventures::TypescriptLexer::_Grammar        (generic)
#   "ts5.0"    → CodingAdventures::TypescriptLexer::_Grammar_ts5_0

sub _grammar {
    my ($class, $version) = @_;
    $version //= '';

    return $_grammar_cache{$version} if $_grammar_cache{$version};

    if ($version ne '' && !$VALID_VERSIONS{$version}) {
        die "CodingAdventures::TypescriptLexer: unknown TypeScript version '$version'. "
          . "Valid versions: ts1.0 ts2.0 ts3.0 ts4.0 ts5.0 ts5.8";
    }

    my $module = $GRAMMAR_MODULE{$version};
    no strict 'refs';
    my $grammar = &{"${module}::token_grammar"}();

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

0.03

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
