package CodingAdventures::LispLexer;

# ============================================================================
# CodingAdventures::LispLexer — Grammar-driven Lisp tokenizer
# ============================================================================
#
# This module tokenizes Lisp/Scheme source text into a flat list of token
# hashrefs.  It uses the grammar infrastructure from CodingAdventures::GrammarTools
# to read the shared `lisp.tokens` file, and applies those rules to scan
# the source using Perl's `\G` anchor and `pos()` mechanism.
#
# # A Very Short History of Lisp
#
# In 1958, John McCarthy at MIT invented Lisp (LISt Processing) as a practical
# implementation of Church's lambda calculus.  He wanted a language where
# programs and data had the same representation — lists.  The result was a
# language so powerful that its core ideas (garbage collection, higher-order
# functions, closures, macros, the REPL) took other languages decades to adopt.
#
# Lisp's syntax is built from S-expressions (symbolic expressions).  An
# S-expression is either:
#
#   • An atom:   42,  define,  "hello"
#   • A list:    (operator arg1 arg2 ...)
#
# Because lists can contain other lists, Lisp programs form trees — and those
# trees are themselves valid Lisp data.  A Lisp macro is a function that
# receives a syntax tree and returns a new syntax tree.  This is the deepest
# form of metaprogramming.
#
# # Token types
#
# From `lisp.tokens` (skip rules are consumed silently):
#
#   skip: WHITESPACE = /[ \t\r\n]+/    — spaces, tabs, carriage returns, newlines
#   skip: COMMENT    = /;[^\n]*/       — ; to end-of-line (never emitted)
#
#   NUMBER = /-?[0-9]+/                — integer literals: 42, -7, 0
#   SYMBOL = /[a-zA-Z_+\-*\/=<>!?&][a-zA-Z0-9_+\-*\/=<>!?&]*/
#            — identifiers and operators: define, lambda, +, car, null?, set!
#   STRING = /"([^"\\]|\\.)*"/         — quoted strings: "hello"
#   LPAREN = "("                       — open list
#   RPAREN = ")"                       — close list
#   QUOTE  = "'"                       — shorthand for (quote x)
#   DOT    = "."                       — cons cell separator: (a . b)
#
# # What makes these token types special?
#
# SYMBOL is the most unusual token type compared to conventional languages.
# In Lisp, symbols can contain many punctuation characters:
#
#   +  -  *  /  =  <  >  !  ?  &
#
# This means `null?`, `set!`, `<=`, `string->number`, and even just `+` are
# all valid symbol names.  Functions and operators are stored in the same
# symbol table.  (+ 1 2) looks up the symbol `+`, finds the addition function,
# and calls it.
#
# DOT is used for cons cell notation:
#   (a . b)    → cons(a, b)
#   (1 2 . 3)  → cons(1, cons(2, 3))  — improper list
#
# QUOTE is a reader macro: 'x expands to (quote x), preventing evaluation.
#   'foo        → the symbol foo (not its value)
#   '(1 2 3)    → the list (1 2 3) as data (not a function call)
#
# # Architecture
#
# 1. **Grammar loading** — `_grammar()` requires
#    `CodingAdventures::LispLexer::_Grammar`, a checked-in generated module
#    produced ahead of time by
#    `grammar-tools.pl compile-tokens code/grammars/lisp/lisp.tokens`, and
#    calls its `token_grammar()` sub to reconstruct the TokenGrammar object
#    graph natively. The result is cached for the lifetime of the process.
#    This avoids reading `lisp.tokens` off disk at runtime: a real CPAN
#    install of this package does not ship `code/grammars/`, so the old
#    disk-read approach would fail with "cannot open ... No such file or
#    directory" outside this monorepo checkout.
#
# 2. **Pattern compilation** — `_build_rules()` converts every TokenDefinition
#    in the grammar into either a skip pattern (`qr/\G.../`) or a token rule
#    `{ name => str, pat => qr/\G.../ }`.
#
# 3. **Tokenization** — `tokenize()` walks the source using Perl's `pos()` +
#    `\G` anchoring.  Skip patterns are tried first; then token patterns in
#    definition order.  The first match wins.
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.02';

use CodingAdventures::GrammarTools;

# ============================================================================
# Grammar loading and caching
# ============================================================================
#
# Reading and parsing the grammar file on every tokenize() call would be
# wasteful.  We cache the TokenGrammar object and the compiled rule lists
# in package-level variables.  They are populated on the first call and
# reused thereafter for the lifetime of the process.

my $_grammar;      # CodingAdventures::GrammarTools::TokenGrammar
my $_rules;        # arrayref of { name => str, pat => qr/\G.../ }
my $_skip_rules;   # arrayref of qr/\G.../ patterns for skip definitions

# --- _grammar() ---------------------------------------------------------------
#
# Load the pre-compiled `lisp.tokens` grammar, caching the result.
# Returns a CodingAdventures::GrammarTools::TokenGrammar object.
#
# The grammar is compiled once at dev time via
# `grammar-tools.pl compile-tokens code/grammars/lisp/lisp.tokens` into
# CodingAdventures::LispLexer::_Grammar, a checked-in generated module that
# reconstructs the TokenGrammar object graph natively — no disk I/O or
# grammar re-parsing at runtime, and no reliance on `code/grammars/` being
# present outside the monorepo checkout.

sub _grammar {
    return $_grammar if $_grammar;

    require CodingAdventures::LispLexer::_Grammar;
    $_grammar = CodingAdventures::LispLexer::_Grammar::token_grammar();

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

    # Build skip patterns (WHITESPACE, COMMENT)
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

    # Build token patterns (NUMBER, SYMBOL, STRING, LPAREN, RPAREN, QUOTE, DOT)
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
# Tokenize a Lisp/Scheme source string.
#
# Algorithm:
#
#   1. Ensure the grammar and compiled rules are loaded (_build_rules).
#   2. Walk the source from position 0 to end.
#   3. At each position, try each skip pattern with /gc (keeping pos on miss).
#      If a skip pattern matches, update line/col tracking and continue.
#   4. If no skip matched, try each token pattern in definition order.
#      First match: record token, advance pos, update tracking, continue.
#   5. If nothing matched, die with a descriptive error message including
#      the line, column, and the offending character.
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
        # WHITESPACE and COMMENT are tried before token patterns.
        # Lisp uses the semicolon for line comments — one of the earliest
        # uses of a dedicated comment syntax in a programming language.
        # Emacs Lisp conventions use ; for inline, ;; for block, ;;; for headers.

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
        # A well-formed Lisp source file should never reach here.
        # Characters like @, #, ` (backquote is not in this grammar), etc.
        # will trigger this error.

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::LispLexer: LexerError at line %d col %d: "
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

CodingAdventures::LispLexer - Grammar-driven Lisp/Scheme tokenizer

=head1 SYNOPSIS

    use CodingAdventures::LispLexer;

    my $tokens = CodingAdventures::LispLexer->tokenize('(define x 42)');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }
    # LPAREN   (
    # SYMBOL   define
    # SYMBOL   x
    # NUMBER   42
    # RPAREN   )
    # EOF

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in
L<CodingAdventures::GrammarTools>.  Reads the shared C<lisp.tokens> file,
compiles token definitions to Perl regexes, and tokenizes Lisp/Scheme source
into a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Whitespace and C<;> line comments are silently consumed.  The last token is
always C<EOF>.

Token types emitted: C<NUMBER>, C<SYMBOL>, C<STRING>, C<LPAREN>, C<RPAREN>,
C<QUOTE>, C<DOT>, C<EOF>.

=head1 METHODS

=head2 tokenize($source)

Tokenize a Lisp/Scheme string.  Returns an arrayref of token hashrefs.
Dies on unexpected input with a descriptive message.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
