package CodingAdventures::Lexer;

# ============================================================================
# CodingAdventures::Lexer — General-purpose tokenizer / lexer
# ============================================================================
#
# A lexer (also called a tokenizer or scanner) is the first phase of any
# language implementation.  It reads raw source text and groups characters
# into *tokens* — the smallest meaningful units of the language.
#
# For example, the source text "x = 42 + y" becomes five tokens:
#
#   Token(IDENT,  "x",  line=1, col=1)
#   Token(SYMBOL, "=",  line=1, col=3)
#   Token(NUMBER, "42", line=1, col=5)
#   Token(SYMBOL, "+",  line=1, col=8)
#   Token(IDENT,  "y",  line=1, col=10)
#
# === ARCHITECTURE ===
#
# The lexer is rule-driven.  You provide a list of rules, each of which pairs
# a token type (string) with a Perl regex.  The lexer tries rules in order at
# the current position; the first match wins.
#
# Built-in rule sets are provided for common use cases.
#
# === TOKEN TYPES ===
#
#   NUMBER     — integer or float literal: 42, 3.14
#   STRING     — quoted string: "hello"
#   IDENT      — identifier: foo, myVar
#   KEYWORD    — reserved word: if, while, let
#   SYMBOL     — operator/punctuation: +, -, =, (, )
#   WHITESPACE — spaces and tabs (optionally skipped)
#   NEWLINE    — newline character \n
#   EOF        — end of input
#   ERROR      — unrecognized character
#
# === OOP PATTERN ===
#
#   my $lexer  = CodingAdventures::Lexer->new($source, \@rules);
#   my $token  = $lexer->next_token();   # one token
#   my @tokens = $lexer->tokenize();     # all tokens (including EOF)
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

use Exporter 'import';
our @EXPORT_OK = qw(default_rules expression_rules);

# ============================================================================
# Token type constants
# ============================================================================

use constant {
    TT_NUMBER     => 'NUMBER',
    TT_STRING     => 'STRING',
    TT_IDENT      => 'IDENT',
    TT_KEYWORD    => 'KEYWORD',
    TT_SYMBOL     => 'SYMBOL',
    TT_WHITESPACE => 'WHITESPACE',
    TT_NEWLINE    => 'NEWLINE',
    TT_EOF        => 'EOF',
    TT_ERROR      => 'ERROR',
};

# ============================================================================
# Default keyword list
# ============================================================================
#
# These identifiers are classified as KEYWORD instead of IDENT when
# encountered.  You can override the keyword set by providing a custom rule
# that checks the matched text.

my @DEFAULT_KEYWORDS = qw(
    if else then while for let var fn return true false nil and or not
);

# ============================================================================
# Built-in rule sets
# ============================================================================
#
# A rule is a hashref:
#
#   { type => "NUMBER", pattern => qr/\G[0-9]+(?:\.[0-9]+)?/ }
#
# Rules are tried in order; use \G to anchor to the current position.
# The pattern must NOT capture (use non-capturing groups where needed).

# expression_rules — suitable for a simple arithmetic/assignment language
sub expression_rules {
    my @keywords = @DEFAULT_KEYWORDS;
    my $kw_pat   = join('|', map { quotemeta($_) } @keywords);
    return [
        { type => TT_WHITESPACE, pattern => qr/\G[ \t]+/              },
        { type => TT_NEWLINE,    pattern => qr/\G\n/                   },
        { type => TT_NUMBER,     pattern => qr/\G[0-9]+(?:\.[0-9]+)?/  },
        { type => TT_STRING,     pattern => qr/\G"(?:[^"\\]|\\.)*"/    },
        { type => TT_KEYWORD,    pattern => qr/\G(?:$kw_pat)(?!\w)/    },
        { type => TT_IDENT,      pattern => qr/\G[A-Za-z_]\w*/         },
        { type => TT_SYMBOL,     pattern => qr/\G(?:==|!=|<=|>=|[+\-*\/=<>!(),;:{}\[\].])/ },
        { type => TT_ERROR,      pattern => qr/\G./                     },
    ];
}

# default_rules — alias for expression_rules
sub default_rules { expression_rules() }

# ============================================================================
# Lexer constructor
# ============================================================================
#
# === Parameters ===
#
#   $source  — the source string to tokenize
#   $rules   — arrayref of rule hashrefs (see expression_rules above)
#
# Internal state:
#   _source  — the full source string
#   _pos     — current character position (0-indexed)
#   _line    — current line number (1-indexed)
#   _col     — current column number (1-indexed)
#   _rules   — the rules array
#   _done    — true once EOF has been emitted

sub new {
    my ($class, $source, $rules) = @_;
    $rules //= expression_rules();
    return bless {
        _source => $source,
        _pos    => 0,
        _line   => 1,
        _col    => 1,
        _rules  => $rules,
        _done   => 0,
    }, $class;
}

# ============================================================================
# next_token — return the next token
# ============================================================================
#
# The algorithm:
#
#   1. If we are at end-of-input, return an EOF token.
#   2. Try each rule's pattern at the current position (using \G + pos()).
#   3. The first rule that matches produces a token with that rule's type.
#   4. Advance _pos, update _line/_col.
#   5. If no rule matches (shouldn't happen with an ERROR catch-all), return ERROR.
#
# We use Perl's \G and pos() mechanism:
#
#   pos($str) = $current_pos;
#   if ($str =~ /\G(pattern)/gc) { ... matched $1 ... }
#
# The /gc flags mean: keep pos() even if the match fails (/g), and don't
# reset pos() on failure (/c).

sub next_token {
    my ($self) = @_;

    # Once EOF has been emitted, keep returning EOF
    if ($self->{_pos} >= length($self->{_source})) {
        $self->{_done} = 1;
        return {
            type  => TT_EOF,
            value => '',
            line  => $self->{_line},
            col   => $self->{_col},
        };
    }

    my $source = $self->{_source};

    # Set Perl's pos() to our current position so \G anchors work
    pos($source) = $self->{_pos};

    for my $rule (@{ $self->{_rules} }) {
        my $pat = $rule->{pattern};
        if ($source =~ /$pat/gc) {
            my $matched = $&;    # $& is the entire match string

            my $tok = {
                type  => $rule->{type},
                value => $matched,
                line  => $self->{_line},
                col   => $self->{_col},
            };

            # Advance position
            $self->{_pos} = pos($source);

            # Update line/column tracking
            my $newlines = ($matched =~ tr/\n//);
            if ($newlines > 0) {
                $self->{_line} += $newlines;
                # Column resets to the length of the last line in the match + 1
                my $last_line = $matched;
                $last_line =~ s/.*\n//s;
                $self->{_col} = length($last_line) + 1;
            } else {
                $self->{_col} += length($matched);
            }

            return $tok;
        }
    }

    # Fallback — should not happen if rules include an ERROR catch-all
    my $char = substr($source, $self->{_pos}, 1);
    $self->{_pos}++;
    $self->{_col}++;
    return {
        type  => TT_ERROR,
        value => $char,
        line  => $self->{_line},
        col   => $self->{_col} - 1,
    };
}

# ============================================================================
# tokenize — collect all tokens into an array (including EOF)
# ============================================================================
#
# Useful when you want the full token list upfront (e.g. for a parser that
# does lookahead).  The EOF token is included as the last element.

sub tokenize {
    my ($self) = @_;
    my @tokens;
    while (1) {
        my $tok = $self->next_token();
        push @tokens, $tok;
        last if $tok->{type} eq TT_EOF;
    }
    return @tokens;
}

# ============================================================================
# Convenience: tokenize a string in one call
# ============================================================================

sub tokenize_string {
    my ($class, $source, $rules) = @_;
    my $lex = $class->new($source, $rules);
    return $lex->tokenize();
}

1;

__END__

=head1 NAME

CodingAdventures::Lexer - General-purpose tokenizer

=head1 SYNOPSIS

    use CodingAdventures::Lexer;

    my $lexer = CodingAdventures::Lexer->new("x = 42 + y");
    my @tokens = $lexer->tokenize();
    # Returns: IDENT("x"), WHITESPACE, SYMBOL("="), ...

=head1 DESCRIPTION

A rule-driven lexer.  Pass a source string and an array of C<{type, pattern}>
rules.  The lexer tries rules in order at each position; the first match wins.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
