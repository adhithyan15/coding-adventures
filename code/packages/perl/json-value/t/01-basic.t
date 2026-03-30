use strict;
use warnings;
use Test2::V0;

use CodingAdventures::JsonValue;

# ============================================================================
# Helper aliases (shorter to write in tests)
# ============================================================================

my $NULL = $CodingAdventures::JsonValue::NULL;

sub is_null  { CodingAdventures::JsonValue::is_null(@_)    }
sub evaluate { CodingAdventures::JsonValue::evaluate(@_)   }
sub from_str { CodingAdventures::JsonValue::from_string(@_) }
sub to_json  { CodingAdventures::JsonValue::to_json(@_)    }

# ============================================================================
# Null sentinel
# ============================================================================

subtest 'null sentinel identity' => sub {
    ok( defined $NULL, '$NULL is defined' );
    ok( ref($NULL) eq 'CodingAdventures::JsonValue::Null',
        'ref is Null class' );
};

subtest 'is_null with null sentinel' => sub {
    ok( is_null($NULL), 'is_null($NULL) is true' );
};

subtest 'is_null with undef' => sub {
    ok( !is_null(undef), 'is_null(undef) is false' );
};

subtest 'is_null with ordinary values' => sub {
    ok( !is_null(0),       'is_null(0) is false' );
    ok( !is_null('null'),  'is_null("null") is false' );
    ok( !is_null({}),      'is_null({}) is false' );
    ok( !is_null([]),      'is_null([]) is false' );
};

# ============================================================================
# evaluate / from_string: scalar values
# ============================================================================

subtest 'evaluate string' => sub {
    my $v = from_str('"hello"');
    is( $v, 'hello', 'string evaluated to hello' );
};

subtest 'evaluate empty string' => sub {
    my $v = from_str('""');
    is( $v, '', 'empty string' );
};

subtest 'evaluate integer' => sub {
    my $v = from_str('42');
    is( $v, 42, 'integer 42' );
};

subtest 'evaluate negative integer' => sub {
    my $v = from_str('-7');
    is( $v, -7, 'negative integer -7' );
};

subtest 'evaluate float' => sub {
    my $v = from_str('3.14');
    ok( abs($v - 3.14) < 1e-10, 'float 3.14' );
};

subtest 'evaluate true' => sub {
    my $v = from_str('true');
    ok( $v, 'true is truthy' );
    is( $v, 1, 'true evaluates to 1' );
};

subtest 'evaluate false' => sub {
    my $v = from_str('false');
    ok( !$v, 'false is falsy' );
    is( $v, 0, 'false evaluates to 0' );
};

subtest 'evaluate null' => sub {
    my $v = from_str('null');
    ok( is_null($v), 'null evaluates to sentinel' );
};

# ============================================================================
# evaluate / from_string: string escape sequences
# ============================================================================

subtest 'unescape double quote' => sub {
    my $v = from_str('"say \\"hi\\""');
    is( $v, 'say "hi"', 'unescaped double quote' );
};

subtest 'unescape backslash' => sub {
    my $v = from_str('"a\\\\b"');
    is( $v, 'a\\b', 'unescaped backslash' );
};

subtest 'unescape forward slash' => sub {
    my $v = from_str('"a\\/b"');
    is( $v, 'a/b', 'unescaped forward slash' );
};

subtest 'unescape newline' => sub {
    my $v = from_str('"line1\\nline2"');
    is( $v, "line1\nline2", 'unescaped newline' );
};

subtest 'unescape tab' => sub {
    my $v = from_str('"a\\tb"');
    is( $v, "a\tb", 'unescaped tab' );
};

subtest 'unescape carriage return' => sub {
    my $v = from_str('"a\\rb"');
    is( $v, "a\rb", 'unescaped CR' );
};

subtest 'unescape form feed' => sub {
    my $v = from_str('"a\\fb"');
    is( $v, "a\x0cb", 'unescaped form feed' );
};

subtest 'unescape backspace' => sub {
    my $v = from_str('"a\\bb"');
    is( $v, "a\x08b", 'unescaped backspace' );
};

subtest 'unescape \\u0041 (A)' => sub {
    my $v = from_str('"\\u0041"');
    is( $v, 'A', '\\u0041 → A' );
};

subtest 'unescape \\u00e9 (é in UTF-8)' => sub {
    my $v = from_str('"\\u00e9"');
    # U+00E9 = é, UTF-8 bytes 0xC3 0xA9
    is( $v, "\xc3\xa9", '\\u00e9 → é UTF-8' );
};

subtest 'unescape \\u4e2d (中 in UTF-8)' => sub {
    my $v = from_str('"\\u4e2d"');
    # U+4E2D = 中, UTF-8: 0xE4 0xB8 0xAD
    is( $v, "\xe4\xb8\xad", '\\u4e2d → 中 UTF-8' );
};

# ============================================================================
# evaluate / from_string: objects
# ============================================================================

subtest 'evaluate empty object' => sub {
    my $v = from_str('{}');
    ok( ref($v) eq 'HASH', 'empty object is hashref' );
    is( scalar keys %$v, 0, 'no keys' );
};

subtest 'evaluate simple key-value object' => sub {
    my $v = from_str('{"key": "value"}');
    is( $v->{key}, 'value', 'key → value' );
};

subtest 'evaluate object with integer' => sub {
    my $v = from_str('{"n": 42}');
    is( $v->{n}, 42, 'n → 42' );
};

subtest 'evaluate object with boolean values' => sub {
    my $v = from_str('{"a": true, "b": false}');
    ok( $v->{a}, 'a is truthy' );
    ok( !$v->{b}, 'b is falsy' );
};

subtest 'evaluate object with null value' => sub {
    my $v = from_str('{"x": null}');
    ok( is_null($v->{x}), 'x is null sentinel' );
};

subtest 'evaluate object with multiple pairs' => sub {
    my $v = from_str('{"a": 1, "b": 2, "c": 3}');
    is( $v->{a}, 1, 'a → 1' );
    is( $v->{b}, 2, 'b → 2' );
    is( $v->{c}, 3, 'c → 3' );
};

subtest 'evaluate nested object' => sub {
    my $v = from_str('{"outer": {"inner": 99}}');
    ok( ref($v->{outer}) eq 'HASH', 'outer is hashref' );
    is( $v->{outer}{inner}, 99, 'inner → 99' );
};

# ============================================================================
# evaluate / from_string: arrays
# ============================================================================

subtest 'evaluate empty array' => sub {
    my $v = from_str('[]');
    ok( ref($v) eq 'ARRAY', 'empty array is arrayref' );
    is( scalar @$v, 0, 'no elements' );
};

subtest 'evaluate array of numbers' => sub {
    my $v = from_str('[1, 2, 3]');
    is( $v->[0], 1, 'v[0] = 1' );
    is( $v->[1], 2, 'v[1] = 2' );
    is( $v->[2], 3, 'v[2] = 3' );
};

subtest 'evaluate array of strings' => sub {
    my $v = from_str('["a", "b", "c"]');
    is( $v->[0], 'a', 'v[0] = a' );
    is( $v->[2], 'c', 'v[2] = c' );
};

subtest 'evaluate mixed array' => sub {
    my $v = from_str('[1, "two", true, false, null]');
    is( $v->[0], 1,      'v[0] = 1' );
    is( $v->[1], 'two',  'v[1] = two' );
    ok( $v->[2],         'v[2] truthy (true)' );
    ok( !$v->[3],        'v[3] falsy (false)' );
    ok( is_null($v->[4]), 'v[4] is null' );
};

subtest 'evaluate nested array' => sub {
    my $v = from_str('[[1, 2], [3, 4]]');
    is( $v->[0][0], 1, '[0][0] = 1' );
    is( $v->[1][1], 4, '[1][1] = 4' );
};

subtest 'evaluate array of objects' => sub {
    my $v = from_str('[{"id": 1}, {"id": 2}]');
    is( $v->[0]{id}, 1, 'v[0]{id} = 1' );
    is( $v->[1]{id}, 2, 'v[1]{id} = 2' );
};

# ============================================================================
# evaluate / from_string: complex mixed structure
# ============================================================================

subtest 'realistic JSON document' => sub {
    my $src = <<'END_JSON';
{
  "name": "Alice",
  "age": 30,
  "active": true,
  "score": -1.5,
  "tags": ["perl", "json"],
  "address": {
    "city": "Metropolis",
    "zip": null
  }
}
END_JSON
    my $v = from_str($src);
    is( $v->{name},            'Alice',       'name' );
    is( $v->{age},             30,            'age' );
    ok( $v->{active},                         'active truthy' );
    ok( abs($v->{score} - (-1.5)) < 1e-10,    'score' );
    is( $v->{tags}[0],         'perl',        'tags[0]' );
    is( $v->{tags}[1],         'json',        'tags[1]' );
    is( $v->{address}{city},   'Metropolis',  'city' );
    ok( is_null($v->{address}{zip}),          'zip is null' );
};

# ============================================================================
# to_json: scalar values
# ============================================================================

subtest 'to_json undef → null' => sub {
    is( to_json(undef), 'null', 'undef → null' );
};

subtest 'to_json $NULL → null' => sub {
    is( to_json($NULL), 'null', 'null sentinel → null' );
};

subtest 'to_json true (1)' => sub {
    # Perl 1 is serialized as "1" (a number) rather than JSON "true",
    # because Perl has no boolean type.  After a round-trip, from_str("1")
    # gives the number 1, not the JSON boolean true.  This is the expected
    # behavior for the monorepo: evaluate(true) = 1 in Perl.
    # Note: "true" and "false" would require a typed boolean wrapper.
    my $s = to_json(1);
    ok( $s eq '1' || $s eq 'true', 'to_json(1) is 1 or true' );
};

subtest 'to_json integer' => sub {
    is( to_json(42),  '42',  'integer 42' );
    is( to_json(-7),  '-7',  'negative -7' );
    is( to_json(0),   '0',   'zero' );
};

subtest 'to_json float' => sub {
    my $s = to_json(3.14);
    ok( $s =~ /3\.14/, 'float 3.14 contains decimal' );
};

subtest 'to_json string' => sub {
    is( to_json('hello'), '"hello"', 'plain string' );
    is( to_json(''),      '""',      'empty string' );
};

# ============================================================================
# to_json: string escaping
# ============================================================================

subtest 'to_json escape double quote' => sub {
    is( to_json('say "hi"'), '"say \\"hi\\""', 'double quote escaped' );
};

subtest 'to_json escape backslash' => sub {
    is( to_json('a\\b'), '"a\\\\b"', 'backslash escaped' );
};

subtest 'to_json escape newline' => sub {
    is( to_json("a\nb"), '"a\\nb"', 'newline escaped' );
};

subtest 'to_json escape tab' => sub {
    is( to_json("a\tb"), '"a\\tb"', 'tab escaped' );
};

subtest 'to_json escape CR' => sub {
    is( to_json("a\rb"), '"a\\rb"', 'CR escaped' );
};

subtest 'to_json escape control chars' => sub {
    my $s = to_json("\x01");
    like( $s, qr/\\u0001/, 'SOH escaped as \\u0001' );
};

# ============================================================================
# to_json: arrays
# ============================================================================

subtest 'to_json empty arrayref → []' => sub {
    is( to_json([]), '[]', 'empty array' );
};

subtest 'to_json array of numbers' => sub {
    is( to_json([1, 2, 3]), '[1,2,3]', 'array of numbers' );
};

subtest 'to_json array of strings' => sub {
    is( to_json(['a', 'b']), '["a","b"]', 'array of strings' );
};

subtest 'to_json array with null sentinel' => sub {
    is( to_json([1, $NULL, 3]), '[1,null,3]', 'array with null' );
};

subtest 'to_json nested array' => sub {
    is( to_json([[1,2],[3,4]]), '[[1,2],[3,4]]', 'nested array' );
};

subtest 'to_json pretty array' => sub {
    my $s = to_json([1, 2, 3], 2);
    like( $s, qr/\n/, 'pretty array has newlines' );
    like( $s, qr/  1/, 'pretty array has indented 1' );
};

# ============================================================================
# to_json: objects
# ============================================================================

subtest 'to_json empty hashref → {}' => sub {
    is( to_json({}), '{}', 'empty object' );
};

subtest 'to_json simple object' => sub {
    is( to_json({key => 'value'}), '{"key":"value"}', 'simple object' );
};

subtest 'to_json object keys sorted' => sub {
    is( to_json({b => 2, a => 1, c => 3}), '{"a":1,"b":2,"c":3}',
        'keys sorted alphabetically' );
};

subtest 'to_json nested object' => sub {
    is( to_json({a => {b => 1}}), '{"a":{"b":1}}', 'nested object' );
};

subtest 'to_json pretty object' => sub {
    my $s = to_json({key => 'value'}, 2);
    like( $s, qr/\n/, 'pretty object has newlines' );
};

# ============================================================================
# to_json: mixed nested
# ============================================================================

subtest 'object containing array' => sub {
    is( to_json({tags => ['perl', 'json']}),
        '{"tags":["perl","json"]}', 'object with array' );
};

subtest 'array of objects' => sub {
    is( to_json([{id => 1}, {id => 2}]),
        '[{"id":1},{"id":2}]', 'array of objects' );
};

# ============================================================================
# Round-trip: from_string → to_json → from_string
# ============================================================================

sub round_trip {
    my ($json_str) = @_;
    my $v1 = from_str($json_str);
    my $s  = to_json($v1);
    my $v2 = from_str($s);
    return ($v1, $v2);
}

subtest 'round-trip integer' => sub {
    my ($v1, $v2) = round_trip('42');
    is( $v2, $v1, 'integer round-trip' );
};

subtest 'round-trip string' => sub {
    my ($v1, $v2) = round_trip('"hello world"');
    is( $v2, $v1, 'string round-trip' );
};

subtest 'round-trip null' => sub {
    my ($v1, $v2) = round_trip('null');
    ok( is_null($v1), 'v1 is null' );
    ok( is_null($v2), 'v2 is null' );
};

subtest 'round-trip array of numbers' => sub {
    my ($v1, $v2) = round_trip('[1,2,3]');
    is( $v2->[0], $v1->[0], 'v[0] matches' );
    is( $v2->[2], $v1->[2], 'v[2] matches' );
};

subtest 'round-trip simple object' => sub {
    my ($v1, $v2) = round_trip('{"x":1,"y":2}');
    is( $v2->{x}, $v1->{x}, 'x matches' );
    is( $v2->{y}, $v1->{y}, 'y matches' );
};

subtest 'round-trip string with escape sequences' => sub {
    my ($v1, $v2) = round_trip('"hello\\nworld"');
    is( $v2, $v1, 'escaped string round-trip' );
    ok( $v1 =~ /\n/, 'contains newline after decode' );
};

subtest 'round-trip complex nested structure' => sub {
    my $src = '{"name":"Bob","scores":[10,20,30],"active":1}';
    my ($v1, $v2) = round_trip($src);
    is( $v2->{name},      $v1->{name},      'name matches' );
    is( $v2->{scores}[1], $v1->{scores}[1], 'scores[1] matches' );
    is( $v2->{active},    $v1->{active},    'active matches' );
};

# ============================================================================
# Pretty-print produces valid JSON
# ============================================================================

subtest 'pretty-print produces parseable JSON' => sub {
    my $data   = { name => 'Alice', age => 30, tags => ['perl', 'json'] };
    my $pretty = to_json($data, 2);
    my $v      = from_str($pretty);
    is( $v->{name},    'Alice', 'name from pretty' );
    is( $v->{age},     30,      'age from pretty' );
    is( $v->{tags}[0], 'perl',  'tags[0] from pretty' );
};

done_testing;
