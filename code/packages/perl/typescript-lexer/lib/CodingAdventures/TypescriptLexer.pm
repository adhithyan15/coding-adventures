package CodingAdventures::TypescriptLexer;

# ============================================================================
# CodingAdventures::TypescriptLexer — Grammar-driven TypeScript tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools and CodingAdventures::Lexer. It reads
# the shared `typescript.tokens` grammar file, compiles the token definitions
# into Perl regexes, and applies them in priority order to tokenize TypeScript
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
# # What is TypeScript tokenization?
# =====================================
#
# Given the input:  interface Foo { bar: number; }
#
# The tokenizer produces a flat list of token hashrefs:
#
#   { type => "INTERFACE", value => "interface", line => 1, col => 1  }
#   { type => "NAME",      value => "Foo",       line => 1, col => 11 }
#   { type => "LBRACE",    value => "{",         line => 1, col => 15 }
#   { type => "NAME",      value => "bar",       line => 1, col => 17 }
#   { type => "COLON",     value => ":",         line => 1, col => 20 }
#   { type => "NUMBER",    value => "number",    line => 1, col => 22 }
#   { type => "SEMICOLON", value => ";",         line => 1, col => 28 }
#   { type => "RBRACE",    value => "}",         line => 1, col => 30 }
#   { type => "EOF",       value => "",          line => 1, col => 31 }
#
# Note: `number` the type keyword produces a NUMBER token with value "number";
# the literal `42` also produces a NUMBER token with value "42". The token
# type is the same; the value distinguishes them.
#
# # Architecture
# ==============
#
# 1. **Grammar loading** — `_grammar()` opens `typescript.tokens`, parses it
#    with `CodingAdventures::GrammarTools::parse_token_grammar`, and caches
#    the result for the lifetime of the process.
#
# 2. **Pattern compilation** — `_build_rules()` converts every TokenDefinition
#    in the grammar into a `{ name => str, pat => qr/\G.../ }` hashref.
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
# `grammars/typescript.tokens`:
#
#   lib/CodingAdventures  (dirname of __FILE__)
#      ↑ up 1 → lib/
#      ↑ up 2 → typescript-lexer/    (package directory)
#      ↑ up 3 → perl/
#      ↑ up 4 → packages/
#      ↑ up 5 → code/                ← repo root
#   + /grammars/typescript.tokens
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

my $_grammar;      # CodingAdventures::GrammarTools::TokenGrammar
my $_rules;        # arrayref of { name => str, pat => qr// }
my $_skip_rules;   # arrayref of qr// patterns for skip definitions

# --- _grammars_dir() ----------------------------------------------------------

sub _grammars_dir {
    # __FILE__ = .../code/packages/perl/typescript-lexer/lib/CodingAdventures/TypescriptLexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 5 levels: CodingAdventures/ → lib/ → typescript-lexer/ → perl/ → packages/ → code/
    for (1..5) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

# --- _grammar() ---------------------------------------------------------------
#
# Load and parse `typescript.tokens`, caching the result.

sub _grammar {
    return $_grammar if $_grammar;

    my $tokens_file = File::Spec->catfile( _grammars_dir(), 'typescript.tokens' );
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::TypescriptLexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = parse_token_grammar($content);
    die "CodingAdventures::TypescriptLexer: failed to parse typescript.tokens: $err"
        unless $grammar;

    $_grammar = $grammar;
    return $_grammar;
}

# --- _build_rules() -----------------------------------------------------------
#
# Convert TokenGrammar definitions into compiled Perl pattern lists.
#
# Skip patterns come first (whitespace, comments).
# Token patterns follow in grammar definition order.
#
# The \G anchor ensures all matches start at the current pos(), never ahead.

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
# Tokenize a TypeScript source string.
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
# Raises: `die` on unexpected input.

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
        # Whitespace in TypeScript is insignificant between tokens.
        # We advance position without emitting anything, updating line/col.

        my $skipped = 0;
        for my $spat (@$_skip_rules) {
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

    my $tokens = CodingAdventures::TypescriptLexer->tokenize('interface Foo { x: number }');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<typescript.tokens> file, compiles token definitions to Perl regexes,
and tokenizes TypeScript source into a flat list of token hashrefs.

TypeScript is a strict superset of JavaScript. This lexer recognizes all JavaScript
tokens plus TypeScript-specific keywords: INTERFACE, TYPE, ENUM, NAMESPACE, DECLARE,
READONLY, PUBLIC, PRIVATE, PROTECTED, ABSTRACT, IMPLEMENTS, EXTENDS, KEYOF, INFER,
NEVER, UNKNOWN, ANY, VOID, and type-keyword versions of NUMBER (C<number>), STRING
(C<string>), BOOLEAN, OBJECT, SYMBOL, BIGINT.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Whitespace is silently consumed. The last token is always C<EOF>.

=head1 METHODS

=head2 tokenize($source)

Tokenize a TypeScript string. Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
