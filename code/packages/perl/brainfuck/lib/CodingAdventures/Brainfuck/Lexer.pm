package CodingAdventures::Brainfuck::Lexer;

# ============================================================================
# CodingAdventures::Brainfuck::Lexer — Grammar-driven Brainfuck tokenizer
# ============================================================================
#
# This module is a thin wrapper around the grammar infrastructure provided
# by CodingAdventures::GrammarTools and CodingAdventures::Lexer. It reads
# the shared `brainfuck.tokens` grammar file, compiles the token definitions
# into Perl regexes, and applies them in priority order to tokenize Brainfuck
# source code.
#
# # What is Brainfuck tokenization?
# ====================================
#
# Given the input: ++[>+<-] move 2 to cell 1
#
# The tokenizer produces a flat list of token hashrefs:
#
#   { type => "INC",        value => "+",  line => 1, col => 1 }
#   { type => "INC",        value => "+",  line => 1, col => 2 }
#   { type => "LOOP_START", value => "[",  line => 1, col => 3 }
#   { type => "RIGHT",      value => ">",  line => 1, col => 4 }
#   { type => "INC",        value => "+",  line => 1, col => 5 }
#   { type => "LEFT",       value => "<",  line => 1, col => 6 }
#   { type => "DEC",        value => "-",  line => 1, col => 7 }
#   { type => "LOOP_END",   value => "]",  line => 1, col => 8 }
#   { type => "EOF",        value => "",   line => 1, col => 9 }
#
# The text " move 2 to cell 1" is entirely comment — no Brainfuck commands.
# Whitespace and non-command characters are consumed silently via the
# `brainfuck.tokens` skip: section.
#
# # Architecture
# ==============
#
# 1. **Grammar loading** — `_grammar()` opens `brainfuck.tokens`, parses it
#    with `CodingAdventures::GrammarTools::parse_token_grammar`, and caches
#    the resulting `TokenGrammar` object for the lifetime of the process.
#
# 2. **Pattern compilation** — `_build_rules()` converts every `TokenDefinition`
#    in the grammar into a `{ name => ..., pat => qr/\G.../ }` hashref.
#    Skip definitions are compiled to plain qr/\G.../  patterns.
#
# 3. **Tokenization** — `tokenize()` walks the source string using Perl's
#    `\G` + `pos()` mechanism, trying skip patterns first and then token
#    patterns in definition order. The first match wins.
#
# # Path navigation
# =================
#
# `__FILE__` resolves to `lib/CodingAdventures/Brainfuck/Lexer.pm`.
# `dirname(__FILE__)` → `lib/CodingAdventures/Brainfuck`
#
# From there we need to climb to the repo root (`code/`) then descend
# into `grammars/brainfuck.tokens`:
#
#   lib/CodingAdventures/Brainfuck  (dirname of __FILE__)
#      ↑ up 1 → lib/CodingAdventures/
#      ↑ up 2 → lib/
#      ↑ up 3 → brainfuck/       (package directory)
#      ↑ up 4 → perl/
#      ↑ up 5 → packages/
#      ↑ up 6 → code/             ← repo root
#   + /grammars/brainfuck.tokens
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
# wasteful. We cache the TokenGrammar object and the compiled rule list
# in package-level variables. They are populated on the first call and
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
    # __FILE__ = .../code/packages/perl/brainfuck/lib/CodingAdventures/Brainfuck/Lexer.pm
    my $dir = File::Spec->rel2abs( dirname(__FILE__) );
    # Climb 6 levels: Brainfuck/ → CodingAdventures/ → lib/ → brainfuck/ → perl/ → packages/ → code/
    for (1..6) {
        $dir = dirname($dir);
    }
    return File::Spec->catdir($dir, 'grammars');
}

# --- _grammar() ---------------------------------------------------------------
#
# Load and parse `brainfuck.tokens`, caching the result.
# Returns a CodingAdventures::GrammarTools::TokenGrammar object.

sub _grammar {
    return $_grammar if $_grammar;

    my $tokens_file = File::Spec->catfile( _grammars_dir(), 'brainfuck.tokens' );
    open my $fh, '<', $tokens_file
        or die "CodingAdventures::Brainfuck::Lexer: cannot open '$tokens_file': $!";
    my $content = do { local $/; <$fh> };
    close $fh;

    my ($grammar, $err) = CodingAdventures::GrammarTools->parse_token_grammar($content);
    die "CodingAdventures::Brainfuck::Lexer: failed to parse brainfuck.tokens: $err"
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
#                     (Brainfuck commands are single chars like > < + - . , [ ])
#
# The `\G` anchor is crucial: it forces the match to start exactly at
# `pos($source)`, preventing the regex engine from skipping ahead.
#
# Token type resolution: if a definition has an alias (the `-> ALIAS` syntax
# in .tokens files), we emit the alias as the token type; otherwise we emit
# the definition name.

sub _build_rules {
    return if $_rules;    # already built

    my $grammar = _grammar();
    my (@rules, @skip_rules);

    # Build skip patterns — whitespace and comment characters.
    # In Brainfuck, any character that is not `><+-.,[]` is a comment.
    # The brainfuck.tokens grammar has two skip patterns:
    #   WHITESPACE = /[ \t\r\n]+/   — tracks newlines for line/col counting
    #   COMMENT    = /[^><+\-.,\[\] \t\r\n]+/  — non-whitespace, non-command chars
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

    # Build token patterns — one per command character.
    # Brainfuck command chars are all literals (no regex needed), but
    # the grammar infrastructure handles both via is_regex.
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
# Tokenize a Brainfuck source string.
#
# Algorithm:
#
#   1. Ensure the grammar and compiled rules are loaded (_build_rules).
#   2. Walk the source from position 0 to end.
#   3. At each position, set pos($source) and try each skip pattern with /gc.
#      If a skip pattern matches, update line/col tracking and continue.
#   4. If no skip matched, try each token pattern in order.
#      The first match: record token, advance pos, update tracking, continue.
#   5. Since every non-command character is covered by skip patterns,
#      we should never reach the "no match" case. But just in case, we
#      die with a descriptive error.
#   6. After exhausting the input, push an EOF sentinel and return.
#
# Unlike JSON, Brainfuck tokenization should NEVER fail on valid input
# because the skip patterns handle all non-command characters. The "no match"
# branch below is a safety net for implementation bugs.
#
# Line/column tracking works the same as the JSON lexer.
#
# Return value:
#
#   An arrayref of hashrefs, each with keys: type, value, line, col.
#   The last element always has type 'EOF'.

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
        # Brainfuck's skip patterns consume:
        #   1. WHITESPACE: spaces, tabs, carriage returns, newlines
        #      (newlines advance the line counter)
        #   2. COMMENT: any run of non-command, non-whitespace characters
        #      (e.g., letters, digits, punctuation other than command chars)
        #
        # This means "++ increment twice [loop] done" correctly produces
        # tokens: INC INC LOOP_START LOOP_END EOF
        # with "increment twice", " ", and " done" all silently consumed.

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
        # Each of the 8 Brainfuck command characters has a token pattern.
        # We try them in order; the first match wins.

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

        # ---- No match — should not happen in Brainfuck ----------------------
        #
        # Because the skip patterns cover all non-command characters, reaching
        # here indicates a bug in the grammar or the implementation.

        unless ($matched_tok) {
            my $ch = substr($source, $pos, 1);
            die sprintf(
                "CodingAdventures::Brainfuck::Lexer: LexerError at line %d col %d: "
              . "unexpected character '%s' (this is a bug — all chars should be handled)",
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

CodingAdventures::Brainfuck::Lexer - Grammar-driven Brainfuck tokenizer

=head1 SYNOPSIS

    use CodingAdventures::Brainfuck::Lexer;

    my $tokens = CodingAdventures::Brainfuck::Lexer->tokenize('++[>+<-]');
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A thin wrapper around the grammar infrastructure in CodingAdventures::GrammarTools.
Reads the shared C<brainfuck.tokens> file, compiles token definitions to Perl
regexes, and tokenizes Brainfuck source into a flat list of token hashrefs.

Each token hashref has four keys: C<type>, C<value>, C<line>, C<col>.

Whitespace and comment characters (all non-command chars) are silently consumed.
The last token is always C<EOF>.

Brainfuck has 8 command tokens: C<RIGHT> (>), C<LEFT> (<), C<INC> (+),
C<DEC> (-), C<OUTPUT> (.), C<INPUT> (,), C<LOOP_START> ([), C<LOOP_END> (]).

=head1 METHODS

=head2 tokenize($source)

Tokenize a Brainfuck string. Returns an arrayref of token hashrefs.
Should never die on valid (or even invalid) Brainfuck source since all
non-command characters are treated as comments.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
