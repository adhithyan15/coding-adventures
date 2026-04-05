package CodingAdventures::MosaicLexer;

# ============================================================================
# CodingAdventures::MosaicLexer — Hand-written Mosaic tokenizer
# ============================================================================
#
# Mosaic is a Component Description Language (CDL) for declaring UI component
# structure with named typed slots. A .mosaic file looks like this:
#
#   component ProfileCard {
#     slot avatar-url: image;
#     slot display-name: text;
#     slot count: number = 0;
#
#     Column {
#       Image { source: @avatar-url; }
#       Text  { content: @display-name; }
#       when @show-count {
#         Text { content: @count; }
#       }
#     }
#   }
#
# This hand-written lexer converts Mosaic source text into a flat list of
# token hashrefs, each with keys: type, value, line, col.
#
# Why hand-written?
# -----------------
# The task specifies: "IMPORTANT: Perl does NOT have grammar-tools. Lexer and
# parser are hand-written." Rather than depending on GrammarTools infrastructure,
# we implement the tokenizer directly for clarity and self-sufficiency.
#
# Token types emitted
# -------------------
#
#   KEYWORD      — reserved words: component, slot, import, from, as,
#                  text, number, bool, image, color, node, list,
#                  true, false, when, each
#   NAME         — identifiers: [a-zA-Z_][a-zA-Z0-9_-]*  (hyphen allowed)
#   STRING       — double-quoted string: "..."
#   HEX_COLOR    — hex color literal: #[0-9a-fA-F]{3,8}
#   DIMENSION    — number with unit suffix: 16dp, 50%, 1.5sp
#   NUMBER       — plain number: 42, -1.5, 3.14
#   LBRACE       — {
#   RBRACE       — }
#   LANGLE       — <
#   RANGLE       — >
#   COLON        — :
#   SEMICOLON    — ;
#   AT           — @
#   COMMA        — ,
#   DOT          — .
#   EQUALS       — =
#
# Skipped (never emitted): whitespace, // line comments, /* */ block comments.
#
# Order of discrimination
# -----------------------
#
# The lexer tries patterns in this order, matching the priority rules from
# the mosaic.tokens grammar spec:
#   1. Whitespace / comments (skip)
#   2. HEX_COLOR (#...) — before NUMBER so '#' is not confused
#   3. STRING ("...")
#   4. DIMENSION / NUMBER — DIMENSION wins when followed by a unit suffix
#   5. NAME or KEYWORD — keyword wins when the text is in %KEYWORDS
#   6. Single-character punctuation
#
# Public API
# ----------
#
#   my ($tokens, $error) = CodingAdventures::MosaicLexer->tokenize($source);
#
# On success : returns (arrayref_of_tokens, undef)
# On failure : returns (undef, error_string)
# The last element of the tokens arrayref is always the EOF sentinel.

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# Keyword set
# ============================================================================
#
# When a NAME-shaped sequence of characters exactly matches one of these
# strings, we emit KEYWORD instead of NAME. This mirrors the `keywords:`
# section in mosaic.tokens.

my %KEYWORDS = map { $_ => 1 } qw(
    component slot import from as
    text number bool image color node list
    true false when each
);

# ============================================================================
# Public method: tokenize($source)
# ============================================================================
#
# Walk the source string character-by-character, recognising tokens via
# anchored pattern matching (pos / \G). Track line and column numbers for
# every token so error messages are actionable.

sub tokenize {
    my ($class_or_self, $source) = @_;

    my @tokens;
    my $pos  = 0;
    my $len  = length($source);
    my $line = 1;
    my $col  = 1;

    while ($pos < $len) {
        my $ch = substr($source, $pos, 1);

        # ---- 1. Skip whitespace ----------------------------------------------
        if ($ch =~ /[ \t\r\n]/) {
            if ($ch eq "\n") {
                $line++;
                $col = 1;
            } else {
                $col++;
            }
            $pos++;
            next;
        }

        # ---- 2. Skip // line comments ----------------------------------------
        if (substr($source, $pos, 2) eq '//') {
            while ($pos < $len && substr($source, $pos, 1) ne "\n") {
                $pos++;
            }
            # The newline itself will be processed on the next iteration.
            next;
        }

        # ---- 3. Skip /* */ block comments ------------------------------------
        if (substr($source, $pos, 2) eq '/*') {
            $pos += 2;
            $col += 2;
            while ($pos < $len) {
                if (substr($source, $pos, 2) eq '*/') {
                    $pos += 2;
                    $col += 2;
                    last;
                }
                if (substr($source, $pos, 1) eq "\n") {
                    $line++;
                    $col = 1;
                } else {
                    $col++;
                }
                $pos++;
            }
            next;
        }

        # ---- 4. HEX_COLOR: #[0-9a-fA-F]{3,8} --------------------------------
        #
        # Must be tried before NUMBER because '#' starts a color, not a digit.
        # The regex requires 3–8 hex digits after '#'.
        if ($ch eq '#') {
            my $start_col = $col;
            pos($source) = $pos;
            if ($source =~ /\G(#[0-9a-fA-F]{3,8})/gc) {
                my $val = $1;
                push @tokens, { type => 'HEX_COLOR', value => $val,
                                 line => $line, col => $start_col };
                $col += length($val);
                $pos += length($val);
                next;
            }
            return (undef, "MosaicLexer: unexpected character '#' at line $line col $col");
        }

        # ---- 5. STRING: "..." ------------------------------------------------
        #
        # Consumes characters until the closing '"', handling backslash escapes.
        # Newlines inside a string are an error.
        if ($ch eq '"') {
            my $start_col = $col;
            my $str = '"';
            $pos++;
            $col++;
            while ($pos < $len) {
                my $c = substr($source, $pos, 1);
                if ($c eq '"') {
                    $str .= '"';
                    $pos++;
                    $col++;
                    last;
                }
                if ($c eq '\\' && $pos + 1 < $len) {
                    # Consume escape: backslash + next char
                    $str .= $c . substr($source, $pos + 1, 1);
                    $pos += 2;
                    $col += 2;
                    next;
                }
                if ($c eq "\n") {
                    return (undef, "MosaicLexer: unterminated string at line $line col $start_col");
                }
                $str .= $c;
                $pos++;
                $col++;
            }
            push @tokens, { type => 'STRING', value => $str,
                             line => $line, col => $start_col };
            next;
        }

        # ---- 6. DIMENSION or NUMBER ------------------------------------------
        #
        # DIMENSION wins over NUMBER when a unit suffix ([a-zA-Z%]+) immediately
        # follows the numeric part. This mirrors the mosaic.tokens ordering rule:
        # "ORDER MATTERS: DIMENSION before NUMBER."
        #
        # Numbers may start with '-' (negative), digits, or '.'.
        # A bare '-' that is not followed by a digit is NOT a number — fall
        # through to single-char tokens (which will fail, giving an error).
        if ($ch =~ /[0-9.]/ || ($ch eq '-' && $pos + 1 < $len && substr($source, $pos+1, 1) =~ /[0-9.]/)) {
            my $start_col = $col;
            pos($source) = $pos;
            if ($source =~ /\G(-?[0-9]*\.?[0-9]+)([a-zA-Z%]+)?/gc) {
                my ($num, $unit) = ($1, $2 // '');
                my $val  = $num . $unit;
                my $type = $unit ? 'DIMENSION' : 'NUMBER';
                push @tokens, { type => $type, value => $val,
                                 line => $line, col => $start_col };
                $col += length($val);
                $pos += length($val);
                next;
            }
        }

        # ---- 7. NAME or KEYWORD ---------------------------------------------
        #
        # Identifiers start with a letter or underscore, and may contain
        # letters, digits, underscores, and hyphens (for CSS-style names such
        # as 'corner-radius' or 'a11y-label').
        #
        # A hyphen followed by another letter continues the identifier.
        # A hyphen NOT followed by a letter ends it (e.g., "foo -" is "foo", "-").
        if ($ch =~ /[a-zA-Z_]/) {
            my $start_col = $col;
            pos($source) = $pos;
            if ($source =~ /\G([a-zA-Z_][a-zA-Z0-9_-]*)/gc) {
                my $val  = $1;
                my $type = exists $KEYWORDS{$val} ? 'KEYWORD' : 'NAME';
                push @tokens, { type => $type, value => $val,
                                 line => $line, col => $start_col };
                $col += length($val);
                $pos += length($val);
                next;
            }
        }

        # ---- 8. Single-character punctuation ---------------------------------
        my $start_col = $col;
        my %PUNCT = (
            '{' => 'LBRACE',
            '}' => 'RBRACE',
            '<' => 'LANGLE',
            '>' => 'RANGLE',
            ':' => 'COLON',
            ';' => 'SEMICOLON',
            '@' => 'AT',
            ',' => 'COMMA',
            '.' => 'DOT',
            '=' => 'EQUALS',
        );

        if (exists $PUNCT{$ch}) {
            push @tokens, { type => $PUNCT{$ch}, value => $ch,
                             line => $line, col => $start_col };
            $pos++;
            $col++;
            next;
        }

        # ---- 9. Unknown character --------------------------------------------
        return (undef,
            sprintf("MosaicLexer: unexpected character '%s' at line %d col %d",
                    $ch, $line, $col));
    }

    # Sentinel EOF token — always the final element.
    push @tokens, { type => 'EOF', value => '', line => $line, col => $col };
    return (\@tokens, undef);
}

1;

__END__

=head1 NAME

CodingAdventures::MosaicLexer - Hand-written Mosaic tokenizer

=head1 SYNOPSIS

    use CodingAdventures::MosaicLexer;

    my ($tokens, $error) = CodingAdventures::MosaicLexer->tokenize($source);
    die $error if $error;
    for my $tok (@$tokens) {
        printf "%s  %s\n", $tok->{type}, $tok->{value};
    }

=head1 DESCRIPTION

A hand-written lexer for the Mosaic Component Description Language.
Returns an arrayref of token hashrefs (keys: type, value, line, col).
Whitespace and comments (// and /* */) are silently consumed.
The last token is always C<EOF>.

=head1 METHODS

=head2 tokenize($source)

Tokenize a Mosaic source string.
Returns C<($tokens, undef)> on success or C<(undef, $error_string)> on failure.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
