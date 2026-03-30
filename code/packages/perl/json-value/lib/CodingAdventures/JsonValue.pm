package CodingAdventures::JsonValue;

# ============================================================================
# CodingAdventures::JsonValue — JSON AST evaluator and serializer
# ============================================================================
#
# This module is part of the coding-adventures monorepo.  It sits one layer
# above `CodingAdventures::JsonParser`: whereas the parser produces an
# Abstract Syntax Tree (AST) that faithfully mirrors the JSON grammar,
# JsonValue *evaluates* that AST into native Perl data structures.
#
# # The evaluation model
#
# JSON has six value types.  After evaluation, each maps to a Perl equivalent:
#
#   JSON type   │  Perl type
#   ────────────┼──────────────────────────────────────────────────────
#   object      │  hashref  ({ key => value, ... })
#   array       │  arrayref ([ val, val, ... ])
#   string      │  scalar string
#   number      │  scalar number (integer or float)
#   boolean     │  scalar: 1 (true) or '' (false, empty string)
#   null        │  $CodingAdventures::JsonValue::NULL  (blessed sentinel)
#
# # The null problem
#
# Perl's `undef` is often used as "no value", but it is ambiguous: an
# `undef` hash value could mean "key is absent" or "key maps to JSON null".
# We use a *sentinel*: a blessed reference to an empty hashref with class
# `CodingAdventures::JsonValue::Null`.  `is_null($v)` tests for this class.
#
# # Serialization
#
# `to_json($value, $indent)` walks native Perl values and produces a JSON
# string.  Arrayrefs become JSON arrays; hashrefs become JSON objects.
# The optional `$indent` argument enables pretty-printing.
#
# # Architecture
#
#   JsonValue  ← this module
#        ↓
#   CodingAdventures::JsonParser  (provides parse() → ASTNode)
#        ↓
#   CodingAdventures::JsonLexer, JsonParser::ASTNode

use strict;
use warnings;
use utf8;
use Encode qw(encode_utf8);

use CodingAdventures::JsonParser;

our $VERSION = '0.01';

# ============================================================================
# Null sentinel
# ============================================================================
#
# `$NULL` is a blessed reference to an empty hashref.  The class name
# `CodingAdventures::JsonValue::Null` acts as a type tag.
#
# We bless into a separate package so that the class is self-documenting
# and so that `ref($v)` returns the exact class name.
#
# Usage:
#   my $v = CodingAdventures::JsonValue::from_string("null");
#   is_null($v);   # true
#   $v == $NULL;   # also works (same reference)

our $NULL = bless {}, 'CodingAdventures::JsonValue::Null';

# ============================================================================
# is_null($v)
# ============================================================================
#
# Returns true (1) when $v is the JSON null sentinel, false ('') otherwise.
#
# We check the ref class rather than identity so that callers who create
# their own Null instances (unusual but possible) also pass the test.
#
# @param  $v   any scalar or reference
# @return 1 or ''

sub is_null {
    my ($v) = @_;
    return ref($v) eq 'CodingAdventures::JsonValue::Null' ? 1 : '';
}

# ============================================================================
# Internal: string unescaping
# ============================================================================
#
# JSON strings are enclosed in double quotes and may contain escape sequences:
#
#   \"  →  "    (double quote)
#   \\  →  \    (backslash)
#   \/  →  /    (forward slash)
#   \n  →  LF   (U+000A)
#   \t  →  HT   (U+0009)
#   \r  →  CR   (U+000D)
#   \f  →  FF   (U+000C)
#   \b  →  BS   (U+0008)
#   \uXXXX → Unicode code point, encoded as UTF-8 bytes
#
# We strip the surrounding quotes, then apply substitutions in a single pass.

# _unescape_string($raw) → $str
#
# $raw is the raw token value including surrounding quotes, e.g. '"hello\nworld"'.
#
# Steps:
#   1. Strip leading and trailing '"'.
#   2. Replace all recognised escape sequences.
#
# For \uXXXX we convert the four-hex-digit code point to UTF-8.  Perl's
# `chr()` returns a character in Perl's internal representation.  `Encode::encode_utf8`
# converts it to a byte string with the correct UTF-8 encoding.

sub _unescape_string {
    my ($raw) = @_;

    # Strip surrounding double quotes.
    # The JSON lexer guarantees the token starts and ends with '"'.
    my $s = substr($raw, 1, length($raw) - 2);

    # Replace \uXXXX sequences first, before the generic single-char escapes,
    # to avoid double-processing.
    $s =~ s/\\u([0-9a-fA-F]{4})/encode_utf8(chr(hex($1)))/ge;

    # Replace remaining single-character escape sequences.
    $s =~ s|\\(["\\/ntrfb])|_unescape_char($1)|ge;

    return $s;
}

# _unescape_char($c) → $decoded
#
# Map a single character after a backslash to its decoded equivalent.
# Called by the regex in _unescape_string.

sub _unescape_char {
    my ($c) = @_;
    my %map = (
        '"'  => '"',
        '\\' => '\\',
        '/'  => '/',
        'n'  => "\n",
        't'  => "\t",
        'r'  => "\r",
        'f'  => "\f",
        'b'  => "\b",
    );
    return exists $map{$c} ? $map{$c} : "\\$c";
}

# ============================================================================
# evaluate($ast_node) → native Perl value
# ============================================================================
#
# Recursively walk an ASTNode (from CodingAdventures::JsonParser::parse)
# and convert it to a native Perl value.
#
# # Walking the AST
#
# The AST structure mirrors the JSON grammar:
#
#   value  = object | array | STRING | NUMBER | TRUE | FALSE | NULL
#   object = LBRACE [ pair { COMMA pair } ] RBRACE
#   pair   = STRING COLON value
#   array  = LBRACKET [ value { COMMA value } ] RBRACKET
#
# Each ASTNode (CodingAdventures::JsonParser::ASTNode) provides:
#   $node->rule_name  — grammar rule name or "token" for leaves
#   $node->children   — arrayref of child ASTNodes
#   $node->is_leaf    — 1 for token-wrapping leaves, 0 otherwise
#   $node->token      — hashref { type, value, line, col }; leaves only
#
# # Dispatch strategy
#
# We dispatch on `rule_name` and iterate children, skipping punctuation
# tokens (LBRACE, RBRACE, LBRACKET, RBRACKET, COMMA, COLON).

sub evaluate {
    my ($node) = @_;
    my $rule = $node->rule_name;

    # ------------------------------------------------------------------
    # "value" node — wrapper around the actual matched alternative.
    # Exactly one semantically meaningful child (rule node or leaf).
    # ------------------------------------------------------------------
    if ($rule eq 'value') {
        for my $child (@{ $node->children }) {
            # Skip anything that isn't a proper ASTNode
            next unless ref($child) && $child->can('rule_name');
            return evaluate($child);
        }
        die "CodingAdventures::JsonValue::evaluate: empty value node";
    }

    # ------------------------------------------------------------------
    # "object" node — LBRACE [ pair { COMMA pair } ] RBRACE
    # We collect only "pair" children; punctuation is ignored.
    # ------------------------------------------------------------------
    elsif ($rule eq 'object') {
        my %result;
        for my $child (@{ $node->children }) {
            next unless ref($child) && $child->can('rule_name');
            if ($child->rule_name eq 'pair') {
                my ($k, $v) = _evaluate_pair($child);
                $result{$k} = $v;
            }
        }
        return \%result;
    }

    # ------------------------------------------------------------------
    # "pair" node — STRING COLON value
    # Normally called via _evaluate_pair (which returns two values).
    # Direct evaluate() of a pair returns an arrayref [key, value].
    # ------------------------------------------------------------------
    elsif ($rule eq 'pair') {
        my ($k, $v) = _evaluate_pair($node);
        return [$k, $v];
    }

    # ------------------------------------------------------------------
    # "array" node — LBRACKET [ value { COMMA value } ] RBRACKET
    # Collect only "value" children; skip punctuation.
    # ------------------------------------------------------------------
    elsif ($rule eq 'array') {
        my @result;
        for my $child (@{ $node->children }) {
            next unless ref($child) && $child->can('rule_name');
            if ($child->rule_name eq 'value') {
                push @result, evaluate($child);
            }
        }
        return \@result;
    }

    # ------------------------------------------------------------------
    # "token" node — leaf wrapping a single lexer token.
    # Dispatch on token type.
    # ------------------------------------------------------------------
    elsif ($rule eq 'token') {
        my $tok   = $node->token;
        my $ttype = $tok->{type};
        my $tval  = $tok->{value};

        if ($ttype eq 'STRING') {
            return _unescape_string($tval);

        } elsif ($ttype eq 'NUMBER') {
            # `0+` coerces the string to a numeric value.
            # Perl auto-detects whether it is integer or float.
            return 0 + $tval;

        } elsif ($ttype eq 'TRUE') {
            # JSON true → Perl true (1)
            return 1;

        } elsif ($ttype eq 'FALSE') {
            # JSON false → Perl false (empty string, which is false in boolean
            # context but is a defined scalar, not undef)
            return 0;

        } elsif ($ttype eq 'NULL') {
            return $NULL;

        } else {
            # Punctuation tokens should never reach here during normal
            # evaluation — the parent handlers skip them.
            die "CodingAdventures::JsonValue::evaluate: unexpected token type: $ttype";
        }
    }

    else {
        die "CodingAdventures::JsonValue::evaluate: unknown rule_name: $rule";
    }
}

# _evaluate_pair($pair_node) → ($key, $value)
#
# Extract the string key and evaluated value from a "pair" ASTNode.
# Returning a two-element list avoids building an intermediate arrayref.
#
# Children of a pair node: STRING leaf, COLON leaf, value node.
# We find the first STRING leaf (the key) and the value node.

sub _evaluate_pair {
    my ($pair_node) = @_;

    my ($key_node, $value_node);

    for my $child (@{ $pair_node->children }) {
        next unless ref($child) && $child->can('rule_name');

        if ($child->rule_name eq 'token') {
            my $tok = $child->token;
            # The STRING token is the key; COLON is punctuation to skip.
            if ($tok->{type} eq 'STRING' && !defined $key_node) {
                $key_node = $child;
            }

        } elsif ($child->rule_name eq 'value') {
            $value_node = $child;
        }
    }

    die "CodingAdventures::JsonValue::_evaluate_pair: no STRING key found"
        unless defined $key_node;
    die "CodingAdventures::JsonValue::_evaluate_pair: no value node found"
        unless defined $value_node;

    my $key = _unescape_string($key_node->token->{value});
    my $val = evaluate($value_node);
    return ($key, $val);
}

# ============================================================================
# from_string($json_str) → native Perl value
# ============================================================================
#
# Convenience function combining CodingAdventures::JsonParser->parse and
# evaluate() in a single call.
#
# This is the primary entry point for most callers who do not need to
# inspect the intermediate AST.
#
# @param  $json_str  string  A JSON-encoded string.
# @return any                Native Perl value (hashref, arrayref, scalar).
# @die                       On any lexer, parser, or evaluator error.
#
# Example:
#
#   use CodingAdventures::JsonValue;
#   my $t = CodingAdventures::JsonValue::from_string('{"name":"Alice"}');
#   print $t->{name};  # Alice

sub from_string {
    my ($json_str) = @_;
    my $ast = CodingAdventures::JsonParser->parse($json_str);
    return evaluate($ast);
}

# ============================================================================
# to_json($value, $indent) → string
# ============================================================================
#
# Serialize a native Perl value to a JSON string.
#
# # Type mapping (reverse of evaluate)
#
#   Perl type / value                     │  JSON output
#   ──────────────────────────────────────┼─────────────────────────
#   undef                                 │  "null"
#   $NULL (Null sentinel)                 │  "null"
#   1 (numeric true)                      │  "true"  ← see note below
#   0 (numeric false)                     │  "false" ← see note below
#   string                                │  double-quoted, with escapes
#   integer number                        │  decimal integer
#   float number                          │  decimal float
#   arrayref                              │  JSON array [...]
#   hashref                               │  JSON object {...}
#
# # Boolean note
#
# Perl does not have a distinct boolean type.  We serialise:
#   - The integer 1 → "true"
#   - The integer 0 → "false"
#   - Other numeric values → their numeric representation
#
# This matches the round-trip behaviour of evaluate(), which produces 1 for
# JSON `true` and 0 for JSON `false`.
#
# # Pretty-printing
#
# When $indent is a positive integer, each nested level is indented by
# $indent additional spaces.  $indent = 0 (or undef) gives compact output.
#
# @param  $value   any     The Perl value to serialize.
# @param  $indent  int     Spaces per indentation level (default 0).
# @param  $_depth  int     (internal) Current depth; do not pass.
# @return string           JSON-encoded string.

sub to_json {
    my ($value, $indent, $_depth) = @_;
    $indent //= 0;
    $_depth //= 0;

    # ------------------------------------------------------------------
    # undef and the null sentinel → JSON null
    # ------------------------------------------------------------------
    if (!defined($value) || is_null($value)) {
        return 'null';
    }

    # ------------------------------------------------------------------
    # Blessed reference: check for Null sentinel first; everything else
    # we treat as an object (hashref) or array (arrayref) after stripping
    # the blessing.  (Unusual; included for robustness.)
    # ------------------------------------------------------------------

    # ------------------------------------------------------------------
    # Reference types: arrayref → array, hashref → object
    # ------------------------------------------------------------------
    if (ref($value) eq 'ARRAY') {
        return _array_to_json($value, $indent, $_depth);
    }
    if (ref($value) eq 'HASH') {
        return _object_to_json($value, $indent, $_depth);
    }

    # ------------------------------------------------------------------
    # Scalar: number or string or boolean-like
    #
    # Perl does not distinguish numbers from strings at the type level,
    # but we can use a heuristic:
    #   - If the scalar "looks numeric" (Scalar::Util::looks_like_number),
    #     check whether it is 0 or 1 (the boolean round-trip values) or a
    #     general number.
    #   - Otherwise treat it as a JSON string.
    #
    # We avoid depending on Scalar::Util by using a simple regex.
    # ------------------------------------------------------------------
    if (_looks_like_number($value)) {
        # Special boolean cases produced by evaluate()
        if ($value == 1 && $value eq '1') {
            # Could be true (1) or the number 1 — they are indistinguishable
            # without a typed boolean.  We output "1" as a number, which
            # round-trips correctly through JSON since evaluate() maps
            # JSON `true` to Perl 1.
            # To preserve true/false, callers should use JSON::PP::true etc.
            # For our monorepo round-trip tests we verify 1 == true via
            # evaluate().
        }
        return _number_to_json($value);
    }

    # Treat as a JSON string
    return _string_to_json($value);
}

# _looks_like_number($s) → bool
#
# Returns true when the scalar looks like a JSON number.
# Handles integers, floats, scientific notation, negative numbers.
# Uses a conservative regex rather than `Scalar::Util::looks_like_number`
# to avoid a dependency.

sub _looks_like_number {
    my ($s) = @_;
    return $s =~ /\A-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?\z/;
}

# _number_to_json($n) → string
#
# Serialize a Perl number to a JSON number string.
# Integers are emitted without a decimal point; floats keep their
# representation.

sub _number_to_json {
    my ($n) = @_;
    # Check for integer: matches only if no decimal point or exponent
    if ($n =~ /\A-?(?:0|[1-9]\d*)\z/) {
        return $n;    # already a clean integer string
    }
    # Use sprintf to get a consistent floating-point representation.
    # %.14g gives up to 14 significant digits, matching Lua's behaviour.
    return sprintf('%.14g', $n);
}

# _string_to_json($s) → string
#
# Produce a double-quoted JSON string from a Perl scalar string,
# with all required characters escaped.
#
# Escape rules:
#   "  → \"
#   \  → \\   (must be first to avoid double-escaping)
#   BS → \b
#   FF → \f
#   LF → \n
#   CR → \r
#   HT → \t
#   U+0000–U+001F (other control chars) → \uXXXX

sub _string_to_json {
    my ($s) = @_;
    $s =~ s/\\/\\\\/g;    # \ → \\  (must be first!)
    $s =~ s/"/\\"/g;      # " → \"
    $s =~ s/\x08/\\b/g;   # backspace
    $s =~ s/\x0c/\\f/g;   # form feed
    $s =~ s/\n/\\n/g;     # line feed
    $s =~ s/\r/\\r/g;     # carriage return
    $s =~ s/\t/\\t/g;     # tab
    # Escape remaining control characters U+0000–U+001F
    $s =~ s/([\x00-\x1f])/sprintf('\\u%04x', ord($1))/ge;
    return qq{"$s"};
}

# _array_to_json($aref, $indent, $depth) → string
#
# Serialize a Perl arrayref to a JSON array string.

sub _array_to_json {
    my ($aref, $indent, $depth) = @_;

    return '[]' unless @$aref;

    my @items = map { to_json($_, $indent, $depth + 1) } @$aref;

    if ($indent > 0) {
        my $inner_pad = ' ' x ($indent * ($depth + 1));
        my $outer_pad = ' ' x ($indent * $depth);
        return "[\n"
            . $inner_pad
            . join(",\n" . $inner_pad, @items)
            . "\n" . $outer_pad . "]";
    } else {
        return '[' . join(',', @items) . ']';
    }
}

# _object_to_json($href, $indent, $depth) → string
#
# Serialize a Perl hashref to a JSON object string.
# Keys are sorted alphabetically for deterministic output.

sub _object_to_json {
    my ($href, $indent, $depth) = @_;

    my @keys = sort keys %$href;

    return '{}' unless @keys;

    my @pairs;
    for my $k (@keys) {
        my $kj = _string_to_json($k);
        my $vj = to_json($href->{$k}, $indent, $depth + 1);
        if ($indent > 0) {
            push @pairs, "$kj: $vj";
        } else {
            push @pairs, "$kj:$vj";
        }
    }

    if ($indent > 0) {
        my $inner_pad = ' ' x ($indent * ($depth + 1));
        my $outer_pad = ' ' x ($indent * $depth);
        return "{\n"
            . $inner_pad
            . join(",\n" . $inner_pad, @pairs)
            . "\n" . $outer_pad . "}";
    } else {
        return '{' . join(',', @pairs) . '}';
    }
}

1;

__END__

=head1 NAME

CodingAdventures::JsonValue - JSON AST evaluator and serializer

=head1 SYNOPSIS

    use CodingAdventures::JsonValue;

    # One-step parse + evaluate
    my $t = CodingAdventures::JsonValue::from_string('{"name":"Alice","age":30}');
    print $t->{name};   # Alice
    print $t->{age};    # 30

    # JSON null
    my $v = CodingAdventures::JsonValue::from_string('null');
    print CodingAdventures::JsonValue::is_null($v);  # 1

    # Serialize to compact JSON
    print CodingAdventures::JsonValue::to_json({x => 1, y => 2});
    # → {"x":1,"y":2}

    # Serialize to pretty JSON (2-space indent)
    print CodingAdventures::JsonValue::to_json({x => 1, y => 2}, 2);
    # → {
    #     "x": 1,
    #     "y": 2
    #   }

=head1 DESCRIPTION

Evaluates the Abstract Syntax Tree produced by C<CodingAdventures::JsonParser>
into native Perl data structures (hashrefs, arrayrefs, scalars).  Also
serializes native Perl values back to JSON strings.

=head1 FUNCTIONS

=head2 from_string($json_str)

Parse a JSON string and return the evaluated native Perl value.  Dies on error.

=head2 evaluate($ast_node)

Walk an ASTNode from C<CodingAdventures::JsonParser> and return the native
Perl value.

=head2 to_json($value, $indent)

Serialize a native Perl value to a JSON string.  C<$indent> (optional,
default 0) enables pretty-printing with that many spaces per level.

=head2 is_null($v)

Returns 1 if C<$v> is the JSON null sentinel, '' otherwise.

=head1 EXPORTS

Nothing is exported by default.  All functions are called as
C<CodingAdventures::JsonValue::function_name(...)>.

=head1 VARIABLES

=head2 $NULL

The JSON null sentinel.  A blessed reference of class
C<CodingAdventures::JsonValue::Null>.

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
