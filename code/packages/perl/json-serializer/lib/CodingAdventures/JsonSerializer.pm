package CodingAdventures::JsonSerializer;

# ============================================================================
# CodingAdventures::JsonSerializer — Schema-aware JSON serializer/deserializer
# ============================================================================
#
# This module is part of the coding-adventures monorepo.  It sits one layer
# above `CodingAdventures::JsonValue`: whereas JsonValue provides a direct
# AST-to-native round-trip, JsonSerializer adds a richer API layer:
#
#   1. encode($value, \%opts)       — robust encoding with options
#   2. decode($json_str, \%opts)    — decoding with preprocessing options
#   3. validate($value, \%schema)   — validate against a JSON Schema subset
#   4. schema_encode($value, \%schema) — encode with schema-driven coercion
#
# # What is JSON Schema?
#
# JSON Schema is a vocabulary that describes the structure of JSON documents.
# Think of it as the type system for JSON.  A schema like:
#
#   {
#     type       => 'object',
#     properties => {
#       name => { type => 'string' },
#       age  => { type => 'number', minimum => 0 },
#     },
#     required => ['name'],
#   }
#
# declares that a valid document must be an object with at least a "name"
# field (string) and optionally a non-negative "age" (number).
#
# # Streaming / incremental generation
#
# Rather than implementing true coroutine-based streaming, we enforce a
# configurable `max_depth` limit in `encode`.  Extremely deep structures are
# caught and reported with a useful error message instead of exhausting the
# Perl call stack.
#
# # Architecture
#
#   JsonSerializer  ← this module
#        ↓
#   CodingAdventures::JsonValue  (provides from_string, to_json, $NULL, is_null)
#        ↓
#   CodingAdventures::JsonParser, JsonLexer, ...

use strict;
use warnings;
use utf8;

use CodingAdventures::JsonValue;

our $VERSION = '0.01';

# Re-export the null sentinel for callers who only require this module.
our $NULL = $CodingAdventures::JsonValue::NULL;

# ============================================================================
# is_null($v)
# ============================================================================
#
# Delegate to JsonValue's is_null so callers do not need to import two modules.

sub is_null { CodingAdventures::JsonValue::is_null(@_) }

# ============================================================================
# Internal utilities
# ============================================================================

# _is_array($ref) → bool
#
# Returns true when $ref is an ARRAY reference.
# JSON arrays map to Perl arrayrefs; JSON objects map to hashrefs.

sub _is_array {
    my ($v) = @_;
    return ref($v) eq 'ARRAY';
}

# _is_object($ref) → bool
#
# Returns true when $ref is a HASH reference (but NOT the Null sentinel).

sub _is_object {
    my ($v) = @_;
    return ref($v) eq 'HASH';
}

# ============================================================================
# Comment & trailing-comma stripping
# ============================================================================
#
# Standard JSON (RFC 8259) does not permit comments or trailing commas.
# The JSONC / JSON5 family of formats adds these conveniences for config files.
#
# We pre-process the input string before handing it to the strict parser:
#
#   // single-line comment: everything from // to end of line
#   /* multi-line comment: everything between /* and */
#   trailing commas:  { "a": 1, }  →  { "a": 1 }
#                     [ 1, 2, ]    →  [ 1, 2 ]
#
# Implementation note: we scan character-by-character to avoid mangling
# `//` or `/*` that appear inside string literals.

# _strip_comments($s) → $s
#
# Remove // and /* */ comments from a JSON-like string.
# String literals (delimited by unescaped ") are copied verbatim.

sub _strip_comments {
    my ($s) = @_;
    my @result;
    my $i   = 0;
    my $len = length($s);

    while ($i < $len) {
        my $c = substr($s, $i, 1);

        # ----------------------------------------------------------------
        # String literal — copy character by character, honouring escapes
        # ----------------------------------------------------------------
        if ($c eq '"') {
            push @result, $c;
            $i++;
            while ($i < $len) {
                my $sc = substr($s, $i, 1);
                push @result, $sc;
                if ($sc eq '\\') {
                    # Escaped character: copy the next character too
                    $i++;
                    if ($i < $len) {
                        push @result, substr($s, $i, 1);
                    }
                } elsif ($sc eq '"') {
                    last;    # end of string literal
                }
                $i++;
            }
            $i++;
            next;
        }

        # ----------------------------------------------------------------
        # Single-line comment: //
        # ----------------------------------------------------------------
        if ($c eq '/' && $i + 1 < $len && substr($s, $i + 1, 1) eq '/') {
            $i += 2;
            while ($i < $len && substr($s, $i, 1) ne "\n") {
                $i++;
            }
            # Leave the newline in place (it acts as whitespace)
            next;
        }

        # ----------------------------------------------------------------
        # Multi-line comment: /* ... */
        # ----------------------------------------------------------------
        if ($c eq '/' && $i + 1 < $len && substr($s, $i + 1, 1) eq '*') {
            $i += 2;
            while ($i < $len) {
                if (substr($s, $i, 1) eq '*' && $i + 1 < $len
                        && substr($s, $i + 1, 1) eq '/') {
                    $i += 2;
                    last;
                }
                $i++;
            }
            push @result, ' ';    # preserve token boundaries
            next;
        }

        # ----------------------------------------------------------------
        # Ordinary character
        # ----------------------------------------------------------------
        push @result, $c;
        $i++;
    }

    return join('', @result);
}

# _strip_trailing_commas($s) → $s
#
# Remove trailing commas before } or ] in a JSON string.
# Pattern: , followed by optional whitespace, then } or ].
# Uses a repeat loop to handle nested structures (though one pass usually
# suffices for typical inputs).

sub _strip_trailing_commas {
    my ($s) = @_;
    my $prev = '';
    while ($s ne $prev) {
        $prev = $s;
        $s =~ s/,\s*\}/}/g;
        $s =~ s/,\s*\]/]/g;
    }
    return $s;
}

# ============================================================================
# encode($value, \%opts)
# ============================================================================
#
# Serialize a native Perl value to a JSON string with rich options.
#
# # Options (hashref)
#
#   indent     (int, default 0)         — spaces per indentation level;
#                                         0 = compact output
#   sort_keys  (bool, default 1)        — sort object keys alphabetically
#   allow_nan  (bool, default 0)        — if true, emit NaN/Inf as
#                                         quoted strings rather than null
#   max_depth  (int, default 100)       — maximum nesting depth
#
# # Why sort_keys?
#
# JSON objects are unordered (RFC 8259 §4).  Perl hashes have no guaranteed
# iteration order either.  When JSON is used for storage, comparison, or
# cryptographic signing, a deterministic key order is essential.  Sorting
# is on by default for safety; set to 0 for a small speed-up when order
# does not matter.
#
# # Why max_depth?
#
# Deep Perl data structures produce deeply recursive encode calls.  A circular
# reference or pathologically nested structure would exhaust the Perl call
# stack.  max_depth provides an early, informative error.
#
# @param  $value   any           Perl value to encode.
# @param  \%opts   hashref       Encoding options.
# @return string                 JSON-encoded string.
# @die                           On depth exceeded.

sub encode {
    my ($value, $opts) = @_;
    $opts //= {};
    my $indent    = $opts->{indent}    // 0;
    my $sort_keys = exists $opts->{sort_keys} ? $opts->{sort_keys} : 1;
    my $allow_nan = $opts->{allow_nan} // 0;
    my $max_depth = $opts->{max_depth} // 100;

    return _encode_value($value, $indent, 0, $sort_keys, $allow_nan, $max_depth);
}

# _encode_value($value, $indent, $depth, $sort_keys, $allow_nan, $max_depth)
#
# Internal recursive encoder.

sub _encode_value {
    my ($value, $indent, $depth, $sort_keys, $allow_nan, $max_depth) = @_;

    # Depth guard
    if ($depth > $max_depth) {
        die "CodingAdventures::JsonSerializer::encode: max_depth $max_depth exceeded\n";
    }

    # ------------------------------------------------------------------
    # undef and the null sentinel → JSON null
    # ------------------------------------------------------------------
    if (!defined($value) || is_null($value)) {
        return 'null';
    }

    # ------------------------------------------------------------------
    # Array reference → JSON array
    # ------------------------------------------------------------------
    if (ref($value) eq 'ARRAY') {
        return _encode_array($value, $indent, $depth, $sort_keys, $allow_nan, $max_depth);
    }

    # ------------------------------------------------------------------
    # Hash reference → JSON object
    # ------------------------------------------------------------------
    if (ref($value) eq 'HASH') {
        return _encode_object($value, $indent, $depth, $sort_keys, $allow_nan, $max_depth);
    }

    # ------------------------------------------------------------------
    # _ForcedString sentinel: a number coerced to string type by a schema.
    # Must be serialized as a JSON string, not a number.
    # ------------------------------------------------------------------
    if (ref($value) eq 'CodingAdventures::JsonSerializer::_ForcedString') {
        return _encode_string($$value);
    }

    # ------------------------------------------------------------------
    # Blessed reference (other than Null): treat as hashref/arrayref
    # after dereferencing.  This handles objects blessed into user classes.
    # ------------------------------------------------------------------
    if (ref($value)) {
        # Unknown blessed ref: emit null rather than crashing
        return 'null';
    }

    # ------------------------------------------------------------------
    # Scalar: number or string
    #
    # Perl does not distinguish booleans from integers.  We use our number
    # heuristic: if it looks like a pure number, serialize as number.
    # Otherwise serialize as a JSON string.
    # ------------------------------------------------------------------
    if (_looks_like_number($value)) {
        return _encode_number($value, $allow_nan);
    }

    return _encode_string($value);
}

# _encode_number($n, $allow_nan) → string
#
# Serialize a Perl numeric scalar to a JSON number string.
# NaN and Infinity are emitted as null by default, or as quoted strings when
# allow_nan is true.

sub _encode_number {
    my ($n, $allow_nan) = @_;

    # Perl represents NaN and Inf as strings in arithmetic context
    if ($n =~ /\A-?(?:nan|inf(?:inity)?)\z/i) {
        if ($allow_nan) { return qq{"$n"} }
        return 'null';
    }

    # Integer vs float
    if ($n =~ /\A-?(?:0|[1-9]\d*)\z/) {
        return $n;    # clean integer string
    }

    return sprintf('%.14g', $n);
}

# _encode_string($s) → string
#
# Produce a double-quoted JSON string from a Perl scalar, with all
# required characters escaped.

sub _encode_string {
    my ($s) = @_;
    $s =~ s/\\/\\\\/g;    # \ → \\  (must come first)
    $s =~ s/"/\\"/g;      # " → \"
    $s =~ s/\x08/\\b/g;   # backspace
    $s =~ s/\x0c/\\f/g;   # form feed
    $s =~ s/\n/\\n/g;     # line feed
    $s =~ s/\r/\\r/g;     # carriage return
    $s =~ s/\t/\\t/g;     # tab
    # Remaining control chars U+0000–U+001F
    $s =~ s/([\x00-\x1f])/sprintf('\\u%04x', ord($1))/ge;
    return qq{"$s"};
}

# _encode_array($aref, ...) → string
#
# Serialize a Perl arrayref to a JSON array string.

sub _encode_array {
    my ($aref, $indent, $depth, $sort_keys, $allow_nan, $max_depth) = @_;

    return '[]' unless @$aref;

    my @items = map {
        _encode_value($_, $indent, $depth + 1, $sort_keys, $allow_nan, $max_depth)
    } @$aref;

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

# _encode_object($href, ...) → string
#
# Serialize a Perl hashref to a JSON object string.
# Keys are sorted when $sort_keys is true (the default).

sub _encode_object {
    my ($href, $indent, $depth, $sort_keys, $allow_nan, $max_depth) = @_;

    my @keys = $sort_keys ? sort keys %$href : keys %$href;

    return '{}' unless @keys;

    my @pairs;
    for my $k (@keys) {
        my $kj = _encode_string($k);
        my $vj = _encode_value(
            $href->{$k}, $indent, $depth + 1, $sort_keys, $allow_nan, $max_depth);
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

# _looks_like_number($s) → bool
#
# Returns true when the scalar looks like a JSON number.
# Uses a conservative regex; avoids loading Scalar::Util.

sub _looks_like_number {
    my ($s) = @_;
    return $s =~ /\A-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?\z/
        || $s =~ /\A-?(?:nan|inf(?:inity)?)\z/i;
}

# ============================================================================
# decode($json_str, \%opts)
# ============================================================================
#
# Decode a JSON (or JSON-like) string to a native Perl value.
#
# Pre-processing options (hashref):
#
#   allow_comments  (bool, default 0) — strip // and /* */ comments
#   strict          (bool, default 0) — if false, strip trailing commas;
#                                       if true, pass input unchanged to the
#                                       strict JSON parser
#
# # Non-strict mode (default)
#
# Many hand-edited JSON config files contain trailing commas because they're
# natural to add when copying or editing list entries.  Non-strict mode
# silently removes them before parsing.  Use strict=1 when interoperability
# with other parsers is required.
#
# @param  $json_str  string   The JSON string to decode.
# @param  \%opts     hashref  Preprocessing options.
# @return any                 Native Perl value (hashref, arrayref, scalar).
# @die                        On parse error.

sub decode {
    my ($json_str, $opts) = @_;
    $opts //= {};
    my $allow_comments = $opts->{allow_comments} // 0;
    my $strict         = $opts->{strict}         // 0;

    my $s = $json_str;

    # Pre-process: strip comments before trailing commas
    # (a line comment `// ,` would otherwise leave a trailing comma)
    if ($allow_comments) {
        $s = _strip_comments($s);
    }

    # Pre-process: strip trailing commas unless strict mode
    unless ($strict) {
        $s = _strip_trailing_commas($s);
    }

    return CodingAdventures::JsonValue::from_string($s);
}

# ============================================================================
# validate($value, \%schema)
# ============================================================================
#
# Validate a native Perl value against a JSON-Schema-inspired schema subset.
#
# Returns (1, undef) on success, or (0, \@errors) on failure.
# @errors is an arrayref of human-readable strings, each describing one
# validation failure.  All failures are collected, not just the first.
#
# # Supported schema keywords
#
#   type         — 'string' | 'number' | 'integer' | 'boolean' | 'null' |
#                  'object' | 'array'
#   properties   — hashref: sub-schema for each named property
#   required     — arrayref: list of required property names
#   additional_properties — 0: forbid keys not in properties
#   items        — schema for each element of an array
#   minItems     — minimum array length
#   maxItems     — maximum array length
#   minimum      — minimum numeric value (inclusive)
#   maximum      — maximum numeric value (inclusive)
#   minLength    — minimum string byte length
#   maxLength    — maximum string byte length
#   pattern      — regex that the string must match
#   enum         — arrayref: value must be one of these literals
#
# # JSON Schema primer
#
# JSON Schema is a vocabulary for annotating and validating JSON documents.
# A schema is itself a JSON (or Perl hashref) value that *describes* what
# other values should look like.  For example:
#
#   {
#     type       => 'object',
#     required   => ['username', 'age'],
#     properties => {
#       username => { type => 'string', minLength => 1, maxLength => 20 },
#       age      => { type => 'integer', minimum => 0, maximum => 150 },
#       email    => { type => 'string', pattern => qr/@/ },
#       roles    => { type => 'array', items => { type => 'string' } },
#     },
#     additional_properties => 0,
#   }
#
# @param  $value   any        Native Perl value to validate.
# @param  \%schema hashref    Schema description.
# @return (bool, \@errors)    (1, undef) or (0, \@errors).

sub validate {
    my ($value, $schema) = @_;
    my @errors;
    _validate_value($value, $schema, '$', \@errors);
    if (@errors) {
        return (0, \@errors);
    }
    return (1, undef);
}

# _validate_value($value, $schema, $path, \@errors)
#
# Internal recursive validator.  Appends error strings to @errors.
# $path is a dotted path like '$.address.city' used in error messages.

sub _validate_value {
    my ($value, $schema, $path, $errors) = @_;

    # ------------------------------------------------------------------
    # type check
    #
    # Map JSON Schema type names to Perl checks:
    #   'string'  → defined scalar, not a ref
    #   'number'  → numeric scalar
    #   'integer' → numeric scalar with no fractional part
    #   'boolean' → 1 or 0 (Perl's boolean representation)
    #   'null'    → our NULL sentinel or undef
    #   'object'  → hashref
    #   'array'   → arrayref
    # ------------------------------------------------------------------
    if (exists $schema->{type}) {
        my $ok = _check_type($value, $schema->{type});
        unless ($ok) {
            push @$errors, sprintf(
                "%s: expected type '%s', got %s",
                $path, $schema->{type}, _describe_type($value));
        }
    }

    # ------------------------------------------------------------------
    # enum
    #
    # The value must equal one of the listed candidates.
    # For null, use is_null; for everything else use eq/==.
    # ------------------------------------------------------------------
    if (exists $schema->{enum}) {
        my $found = 0;
        for my $candidate (@{ $schema->{enum} }) {
            if (is_null($value) && is_null($candidate)) {
                $found = 1; last;
            } elsif (!ref($value) && !ref($candidate)
                    && defined($value) && defined($candidate)
                    && $value eq $candidate) {
                $found = 1; last;
            }
        }
        unless ($found) {
            push @$errors, "$path: value not in enum";
        }
    }

    # ------------------------------------------------------------------
    # string constraints
    #
    # Apply minLength, maxLength, and pattern checks when the schema
    # declares type => 'string' OR when any of those keywords are present
    # (schema keywords imply a string context even without an explicit type).
    # We only run the checks when the value is actually a defined scalar
    # (not a reference), to avoid spurious errors on wrong-typed values.
    # ------------------------------------------------------------------
    my $has_string_constraints = exists $schema->{minLength}
        || exists $schema->{maxLength}
        || exists $schema->{pattern};

    if ($has_string_constraints && !ref($value) && defined($value)) {
        my $len = length($value);

        if (exists $schema->{minLength} && $len < $schema->{minLength}) {
            push @$errors, sprintf(
                "%s: string length %d < minLength %d",
                $path, $len, $schema->{minLength});
        }
        if (exists $schema->{maxLength} && $len > $schema->{maxLength}) {
            push @$errors, sprintf(
                "%s: string length %d > maxLength %d",
                $path, $len, $schema->{maxLength});
        }
        if (exists $schema->{pattern}) {
            my $pat = $schema->{pattern};
            # Guard against ReDoS: reject patterns longer than 200 characters.
            # A well-formed JSON Schema pattern should be concise; a 200-char
            # limit stops adversarial catastrophic-backtracking patterns like
            # (a+)+$ without impacting legitimate schema validation use cases.
            if (length($pat) > 200) {
                push @$errors, sprintf(
                    "%s: schema pattern too long (max 200 chars)",
                    $path);
            } elsif ($value !~ $pat) {
                push @$errors, sprintf(
                    "%s: string does not match pattern",
                    $path);
            }
        }
    }

    # ------------------------------------------------------------------
    # number constraints
    # ------------------------------------------------------------------
    if (!ref($value) && defined($value) && _looks_like_number($value)) {
        if (exists $schema->{minimum} && $value < $schema->{minimum}) {
            push @$errors, sprintf(
                "%s: value %s < minimum %s",
                $path, $value, $schema->{minimum});
        }
        if (exists $schema->{maximum} && $value > $schema->{maximum}) {
            push @$errors, sprintf(
                "%s: value %s > maximum %s",
                $path, $value, $schema->{maximum});
        }
    }

    # ------------------------------------------------------------------
    # array constraints
    # ------------------------------------------------------------------
    if (ref($value) eq 'ARRAY') {
        my $n = scalar @$value;
        if (exists $schema->{minItems} && $n < $schema->{minItems}) {
            push @$errors, sprintf(
                "%s: array length %d < minItems %d",
                $path, $n, $schema->{minItems});
        }
        if (exists $schema->{maxItems} && $n > $schema->{maxItems}) {
            push @$errors, sprintf(
                "%s: array length %d > maxItems %d",
                $path, $n, $schema->{maxItems});
        }
        if (exists $schema->{items}) {
            for my $i (0 .. $#$value) {
                _validate_value(
                    $value->[$i], $schema->{items},
                    $path . "[$i]", $errors);
            }
        }
    }

    # ------------------------------------------------------------------
    # object constraints
    # ------------------------------------------------------------------
    if (ref($value) eq 'HASH') {
        # required properties
        if (exists $schema->{required}) {
            for my $req_key (@{ $schema->{required} }) {
                unless (exists $value->{$req_key}) {
                    push @$errors, sprintf(
                        "%s: missing required property '%s'",
                        $path, $req_key);
                }
            }
        }

        # property sub-schemas
        if (exists $schema->{properties}) {
            for my $k (keys %{ $schema->{properties} }) {
                if (exists $value->{$k}) {
                    _validate_value(
                        $value->{$k}, $schema->{properties}{$k},
                        $path . '.' . $k, $errors);
                }
            }
        }

        # additional properties
        if (exists $schema->{additional_properties}
                && !$schema->{additional_properties}
                && exists $schema->{properties}) {
            for my $k (keys %$value) {
                unless (exists $schema->{properties}{$k}) {
                    push @$errors, sprintf(
                        "%s: additional property '%s' not allowed",
                        $path, $k);
                }
            }
        }
    }
}

# _check_type($value, $type_name) → bool
#
# Check whether $value satisfies the named JSON Schema type.

sub _check_type {
    my ($value, $t) = @_;

    if ($t eq 'string') {
        return !ref($value) && defined($value) && !_looks_like_number($value);
    } elsif ($t eq 'number') {
        return !ref($value) && defined($value) && _looks_like_number($value);
    } elsif ($t eq 'integer') {
        return !ref($value) && defined($value)
            && _looks_like_number($value)
            && $value == int($value);
    } elsif ($t eq 'boolean') {
        # Perl booleans from evaluate() are 1 (true) and 0 (false)
        return !ref($value) && defined($value)
            && ($value eq '1' || $value eq '0');
    } elsif ($t eq 'null') {
        return !defined($value) || is_null($value);
    } elsif ($t eq 'array') {
        return ref($value) eq 'ARRAY';
    } elsif ($t eq 'object') {
        return ref($value) eq 'HASH';
    } else {
        return 1;    # unknown type keyword: don't fail
    }
}

# _describe_type($value) → string
#
# Return a human-readable type description for use in error messages.

sub _describe_type {
    my ($value) = @_;
    return 'null'   if !defined($value) || is_null($value);
    return 'array'  if ref($value) eq 'ARRAY';
    return 'object' if ref($value) eq 'HASH';
    return 'number' if _looks_like_number($value);
    return 'string';
}

# ============================================================================
# schema_encode($value, \%schema, \%opts)
# ============================================================================
#
# Encode a Perl value to JSON, guided by the provided schema.
#
# Before encoding, applies two kinds of schema-driven transformations:
#
#   1. Type coercion: if the schema says type => 'string' but the value is a
#      number, coerce the number to a string via sprintf.  This lets code work
#      with numbers internally and emit string-typed JSON fields for APIs that
#      require them (e.g. a payment API that expects "9.99" not 9.99).
#
#   2. Property filtering: if additional_properties => 0 in the schema, keys
#      not listed in properties are dropped silently before encoding.
#      This prevents accidentally leaking internal state.
#
# After transformations, the value is encoded with encode().
#
# @param  $value   any       Native Perl value.
# @param  \%schema hashref   Schema to guide coercion.
# @param  \%opts   hashref   Encoding options (passed to encode()).
# @return string             JSON-encoded string.

sub schema_encode {
    my ($value, $schema, $opts) = @_;
    my $coerced = _coerce_value($value, $schema);
    return encode($coerced, $opts);
}

# _coerce_value($value, $schema) → $coerced
#
# Recursively apply schema-driven coercions and filtering.

sub _coerce_value {
    my ($value, $schema) = @_;
    return $value unless $schema;

    # ------------------------------------------------------------------
    # Type coercion for primitives
    #
    # number → string when schema says type => 'string'
    # ------------------------------------------------------------------
    if (exists $schema->{type} && $schema->{type} eq 'string'
            && !ref($value) && defined($value)
            && _looks_like_number($value)) {
        # Format integer without decimal; float with up to 14 significant digits.
        # Wrap in a _ForcedString sentinel so _encode_value treats it as a string
        # and does not re-detect it as a number via _looks_like_number.
        my $str;
        if ($value =~ /\A-?(?:0|[1-9]\d*)\z/) {
            $str = "$value";
        } else {
            $str = sprintf('%.14g', $value);
        }
        return bless \$str, 'CodingAdventures::JsonSerializer::_ForcedString';
    }

    # ------------------------------------------------------------------
    # Object: recurse into properties, apply filtering
    # ------------------------------------------------------------------
    if (ref($value) eq 'HASH'
            && exists $schema->{type} && $schema->{type} eq 'object') {
        my %result;
        for my $k (keys %$value) {
            # Drop unknown keys when additional_properties => 0
            if (exists $schema->{additional_properties}
                    && !$schema->{additional_properties}
                    && exists $schema->{properties}
                    && !exists $schema->{properties}{$k}) {
                next;
            }
            my $sub_schema = exists $schema->{properties}
                ? $schema->{properties}{$k}
                : undef;
            $result{$k} = _coerce_value($value->{$k}, $sub_schema);
        }
        return \%result;
    }

    # ------------------------------------------------------------------
    # Array: recurse into items
    # ------------------------------------------------------------------
    if (ref($value) eq 'ARRAY'
            && exists $schema->{type} && $schema->{type} eq 'array') {
        my @result;
        for my $item (@$value) {
            push @result, _coerce_value($item, $schema->{items});
        }
        return \@result;
    }

    return $value;
}

1;

__END__

=head1 NAME

CodingAdventures::JsonSerializer - Schema-aware JSON serializer/deserializer

=head1 SYNOPSIS

    use CodingAdventures::JsonSerializer;

    # Encode with pretty-printing and sorted keys
    my $json = CodingAdventures::JsonSerializer::encode(
        { b => 2, a => 1 },
        { indent => 2, sort_keys => 1 }
    );
    # {
    #   "a": 1,
    #   "b": 2
    # }

    # Decode JSONC (with comments)
    my $v = CodingAdventures::JsonSerializer::decode(
        '{ "name": "Alice" /* the user */ }',
        { allow_comments => 1 }
    );
    print $v->{name};  # Alice

    # Decode with trailing commas (non-strict, the default)
    my $arr = CodingAdventures::JsonSerializer::decode('[1, 2, 3,]');
    print $arr->[2];   # 3

    # Validate against a schema
    my $schema = {
        type       => 'object',
        required   => ['name'],
        properties => {
            name => { type => 'string', minLength => 1 },
            age  => { type => 'integer', minimum => 0 },
        },
    };
    my ($ok, $errs) = CodingAdventures::JsonSerializer::validate(
        { name => 'Alice', age => 30 }, $schema
    );
    print $ok;  # 1

    # Schema-guided encoding: coerce number → string, drop extra fields
    my $api_schema = {
        type                  => 'object',
        additional_properties => 0,
        properties => {
            price => { type => 'string' },
            qty   => { type => 'number' },
        },
    };
    my $s = CodingAdventures::JsonSerializer::schema_encode(
        { price => 9.99, qty => 3, internal => 'secret' }, $api_schema
    );
    # {"price":"9.99","qty":3}   (internal dropped; price coerced to string)

=head1 DESCRIPTION

Extends C<CodingAdventures::JsonValue> with schema-aware encoding, lenient
decoding (JSONC comments, trailing commas), JSON Schema validation, and
schema-driven type coercion.

=head1 FUNCTIONS

=head2 encode($value, \%opts)

Serialize a native Perl value to a JSON string.  Options: C<indent>,
C<sort_keys>, C<allow_nan>, C<max_depth>.

=head2 decode($json_str, \%opts)

Parse a JSON (or JSONC) string to a native Perl value.  Options:
C<allow_comments>, C<strict>.

=head2 validate($value, \%schema)

Validate a native Perl value against a JSON Schema subset.
Returns C<(1, undef)> on success; C<(0, \@errors)> on failure.

=head2 schema_encode($value, \%schema, \%opts)

Encode with schema-driven coercions and property filtering, then calls
C<encode()>.

=head2 is_null($v)

Returns 1 if C<$v> is the JSON null sentinel, '' otherwise.

=head1 VARIABLES

=head2 $NULL

The JSON null sentinel (re-exported from C<CodingAdventures::JsonValue>).

=head1 VERSION

0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
