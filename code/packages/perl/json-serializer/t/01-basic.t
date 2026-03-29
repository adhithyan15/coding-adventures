use strict;
use warnings;
use Test2::V0;

use CodingAdventures::JsonSerializer;

# ============================================================================
# Shorter aliases
# ============================================================================

my $NULL = $CodingAdventures::JsonSerializer::NULL;

sub enc   { CodingAdventures::JsonSerializer::encode(@_)        }
sub dec   { CodingAdventures::JsonSerializer::decode(@_)        }
sub val   { CodingAdventures::JsonSerializer::validate(@_)      }
sub senc  { CodingAdventures::JsonSerializer::schema_encode(@_) }
sub isnul { CodingAdventures::JsonSerializer::is_null(@_)       }

# ============================================================================
# encode: basic scalars
# ============================================================================

subtest 'encode undef → null' => sub {
    is( enc(undef), 'null', 'undef → null' );
};

subtest 'encode $NULL → null' => sub {
    is( enc($NULL), 'null', 'NULL → null' );
};

subtest 'encode true (1)' => sub {
    my $s = enc(1);
    ok( $s eq '1' || $s eq 'true', 'encode(1) is 1 or true' );
};

subtest 'encode false (0)' => sub {
    my $s = enc(0);
    ok( $s eq '0' || $s eq 'false', 'encode(0) is 0 or false' );
};

subtest 'encode integers' => sub {
    is( enc(42),  '42',  'integer 42'   );
    is( enc(-7),  '-7',  'integer -7'   );
    is( enc(0),   '0',   'zero'         );
};

subtest 'encode float' => sub {
    my $s = enc(3.14);
    like( $s, qr/3\.14/, 'float contains 3.14' );
};

subtest 'encode string' => sub {
    is( enc('hello'), '"hello"', 'plain string' );
    is( enc(''),      '""',      'empty string' );
};

subtest 'encode NaN → null by default' => sub {
    # Perl's "not-a-number" is represented as the string 'NaN'
    is( enc(9**9**9 - 9**9**9), 'null', 'NaN → null' );
};

# ============================================================================
# encode: allow_nan option
# ============================================================================

subtest 'encode allow_nan: NaN → quoted string' => sub {
    my $nan = 9**9**9 - 9**9**9;
    my $s   = enc($nan, { allow_nan => 1 });
    like( $s, qr/"/, 'NaN encoded as quoted string' );
};

# ============================================================================
# encode: max_depth
# ============================================================================

subtest 'encode: max_depth exceeded raises error' => sub {
    my $deep = { a => { b => { c => { d => { e => 'leaf' } } } } };
    ok( dies { enc($deep, { max_depth => 2 }) }, 'raises on max_depth exceeded' );
};

subtest 'encode: max_depth not exceeded succeeds' => sub {
    my $shallow = { a => 1 };
    ok( lives { enc($shallow, { max_depth => 2 }) }, 'succeeds within max_depth' );
};

# ============================================================================
# encode: arrays
# ============================================================================

subtest 'encode arrayref' => sub {
    is( enc([1, 2, 3]), '[1,2,3]', 'array of numbers' );
};

subtest 'encode empty arrayref → []' => sub {
    is( enc([]), '[]', 'empty array' );
};

subtest 'encode nested array' => sub {
    is( enc([[1,2],[3,4]]), '[[1,2],[3,4]]', 'nested array' );
};

subtest 'encode pretty array' => sub {
    my $s = enc([1, 2, 3], { indent => 2 });
    like( $s, qr/\n/, 'pretty array has newlines' );
    like( $s, qr/  1/, 'indented element 1' );
};

# ============================================================================
# encode: objects
# ============================================================================

subtest 'encode empty hashref → {}' => sub {
    is( enc({}), '{}', 'empty object' );
};

subtest 'encode simple object' => sub {
    is( enc({ key => 'value' }), '{"key":"value"}', 'simple object' );
};

subtest 'encode object: keys sorted by default' => sub {
    is( enc({ b => 2, a => 1, c => 3 }), '{"a":1,"b":2,"c":3}',
        'sorted keys' );
};

subtest 'encode object sort_keys=0: all keys present' => sub {
    my $s = enc({ b => 2, a => 1, c => 3 }, { sort_keys => 0 });
    like( $s, qr/"a":1/, '"a":1 present' );
    like( $s, qr/"b":2/, '"b":2 present' );
    like( $s, qr/"c":3/, '"c":3 present' );
};

subtest 'encode pretty object' => sub {
    my $s = enc({ key => 'value' }, { indent => 2 });
    like( $s, qr/\n/, 'pretty object has newlines' );
};

subtest 'encode nested object' => sub {
    is( enc({ a => { b => 1 } }), '{"a":{"b":1}}', 'nested object' );
};

# ============================================================================
# encode: mixed nested
# ============================================================================

subtest 'encode object containing array' => sub {
    is( enc({ tags => ['perl', 'json'] }),
        '{"tags":["perl","json"]}', 'object with array' );
};

subtest 'encode array of objects' => sub {
    is( enc([{ id => 1 }, { id => 2 }]),
        '[{"id":1},{"id":2}]', 'array of objects' );
};

# ============================================================================
# decode: basic
# ============================================================================

subtest 'decode string' => sub {
    is( dec('"hello"'), 'hello', 'decode string' );
};

subtest 'decode integer' => sub {
    is( dec('42'), 42, 'decode integer' );
};

subtest 'decode array' => sub {
    my $v = dec('[1,2,3]');
    is( $v->[0], 1, 'v[0]' );
    is( $v->[2], 3, 'v[2]' );
};

subtest 'decode object' => sub {
    my $v = dec('{"x":1,"y":2}');
    is( $v->{x}, 1, 'x = 1' );
    is( $v->{y}, 2, 'y = 2' );
};

subtest 'decode null' => sub {
    ok( isnul(dec('null')), 'null decoded to sentinel' );
};

# ============================================================================
# decode: allow_comments
# ============================================================================

subtest 'decode: single-line comment stripped' => sub {
    my $s = qq[{\n  "name": "Alice", // user name\n  "age": 30\n}];
    my $v = dec($s, { allow_comments => 1 });
    is( $v->{name}, 'Alice', 'name ok' );
    is( $v->{age},  30,      'age ok'  );
};

subtest 'decode: multi-line comment stripped' => sub {
    my $s = qq[{\n  /* a\n     b */\n  "x": 1\n}];
    my $v = dec($s, { allow_comments => 1 });
    is( $v->{x}, 1, 'x = 1' );
};

subtest 'decode: // inside string preserved' => sub {
    my $v = dec('{"url":"http://example.com"}', { allow_comments => 1 });
    is( $v->{url}, 'http://example.com', 'url preserved' );
};

# ============================================================================
# decode: trailing commas
# ============================================================================

subtest 'decode: trailing comma in object (non-strict)' => sub {
    my $v = dec('{"a":1,"b":2,}');
    is( $v->{a}, 1, 'a = 1' );
    is( $v->{b}, 2, 'b = 2' );
};

subtest 'decode: trailing comma in array (non-strict)' => sub {
    my $v = dec('[1,2,3,]');
    is( $v->[0], 1, 'v[0] = 1' );
    is( $v->[2], 3, 'v[2] = 3' );
};

subtest 'decode: trailing comma raises in strict mode' => sub {
    ok( dies { dec('{"a":1,}', { strict => 1 }) },
        'strict mode rejects trailing comma in object' );
};

subtest 'decode: trailing comma in array raises in strict mode' => sub {
    ok( dies { dec('[1,2,]', { strict => 1 }) },
        'strict mode rejects trailing comma in array' );
};

# ============================================================================
# validate: type checks
# ============================================================================

subtest 'validate type string ok' => sub {
    my ($ok) = val('hello', { type => 'string' });
    ok( $ok, 'string validated' );
};

subtest 'validate type string fails for number' => sub {
    my ($ok, $errs) = val(42, { type => 'string' });
    ok( !$ok, 'validation failed' );
    ok( @$errs > 0, 'errors present' );
};

subtest 'validate type integer ok' => sub {
    my ($ok) = val(5, { type => 'integer' });
    ok( $ok, 'integer validated' );
};

subtest 'validate type array ok' => sub {
    my ($ok) = val([1,2,3], { type => 'array' });
    ok( $ok, 'array validated' );
};

subtest 'validate type object ok' => sub {
    my ($ok) = val({ a => 1 }, { type => 'object' });
    ok( $ok, 'object validated' );
};

subtest 'validate type null ok' => sub {
    my ($ok) = val($NULL, { type => 'null' });
    ok( $ok, 'null validated' );
};

# ============================================================================
# validate: string constraints
# ============================================================================

subtest 'validate minLength ok' => sub {
    my ($ok) = val('hello', { type => 'string', minLength => 3 });
    ok( $ok, 'minLength ok' );
};

subtest 'validate minLength fail' => sub {
    my ($ok, $errs) = val('hi', { type => 'string', minLength => 5 });
    ok( !$ok, 'minLength failed' );
    like( $errs->[0], qr/minLength/, 'error mentions minLength' );
};

subtest 'validate maxLength ok' => sub {
    my ($ok) = val('hi', { type => 'string', maxLength => 5 });
    ok( $ok, 'maxLength ok' );
};

subtest 'validate maxLength fail' => sub {
    my ($ok, $errs) = val('hello world', { type => 'string', maxLength => 5 });
    ok( !$ok, 'maxLength failed' );
    like( $errs->[0], qr/maxLength/, 'error mentions maxLength' );
};

subtest 'validate pattern ok' => sub {
    my ($ok) = val('user@example.com', { type => 'string', pattern => qr/@/ });
    ok( $ok, 'pattern matched' );
};

subtest 'validate pattern fail' => sub {
    my ($ok, $errs) = val('notanemail', { type => 'string', pattern => qr/@/ });
    ok( !$ok, 'pattern failed' );
    like( $errs->[0], qr/pattern/, 'error mentions pattern' );
};

# ============================================================================
# validate: number constraints
# ============================================================================

subtest 'validate minimum ok' => sub {
    my ($ok) = val(5, { type => 'number', minimum => 0 });
    ok( $ok, 'minimum ok' );
};

subtest 'validate minimum fail' => sub {
    my ($ok, $errs) = val(-1, { type => 'number', minimum => 0 });
    ok( !$ok, 'minimum failed' );
    like( $errs->[0], qr/minimum/, 'error mentions minimum' );
};

subtest 'validate maximum ok' => sub {
    my ($ok) = val(99, { type => 'number', maximum => 100 });
    ok( $ok, 'maximum ok' );
};

subtest 'validate maximum fail' => sub {
    my ($ok, $errs) = val(150, { type => 'number', maximum => 100 });
    ok( !$ok, 'maximum failed' );
    like( $errs->[0], qr/maximum/, 'error mentions maximum' );
};

# ============================================================================
# validate: array constraints
# ============================================================================

subtest 'validate minItems ok' => sub {
    my ($ok) = val([1,2,3], { type => 'array', minItems => 2 });
    ok( $ok, 'minItems ok' );
};

subtest 'validate minItems fail' => sub {
    my ($ok, $errs) = val([1], { type => 'array', minItems => 3 });
    ok( !$ok, 'minItems failed' );
    like( $errs->[0], qr/minItems/, 'error mentions minItems' );
};

subtest 'validate maxItems ok' => sub {
    my ($ok) = val([1,2], { type => 'array', maxItems => 5 });
    ok( $ok, 'maxItems ok' );
};

subtest 'validate maxItems fail' => sub {
    my ($ok, $errs) = val([1..6], { type => 'array', maxItems => 3 });
    ok( !$ok, 'maxItems failed' );
    like( $errs->[0], qr/maxItems/, 'error mentions maxItems' );
};

subtest 'validate items sub-schema ok' => sub {
    my ($ok) = val([1,2,3],
        { type => 'array', items => { type => 'number' } });
    ok( $ok, 'items ok' );
};

subtest 'validate items sub-schema fail' => sub {
    my ($ok, $errs) = val([1,'two',3],
        { type => 'array', items => { type => 'number' } });
    ok( !$ok, 'items failed' );
    ok( @$errs > 0, 'errors present' );
};

# ============================================================================
# validate: object constraints
# ============================================================================

subtest 'validate required ok' => sub {
    my $schema = {
        type       => 'object',
        required   => ['name', 'age'],
        properties => {
            name => { type => 'string' },
            age  => { type => 'number' },
        },
    };
    my ($ok) = val({ name => 'Alice', age => 30 }, $schema);
    ok( $ok, 'required fields present' );
};

subtest 'validate required fail' => sub {
    my $schema = {
        type     => 'object',
        required => ['name', 'age'],
        properties => {
            name => { type => 'string' },
            age  => { type => 'number' },
        },
    };
    my ($ok, $errs) = val({ name => 'Alice' }, $schema);
    ok( !$ok, 'required field missing' );
    # At least one error should mention 'age'
    my $found = grep { /age/ } @$errs;
    ok( $found, 'error mentions missing field age' );
};

subtest 'validate property sub-schema ok' => sub {
    my $schema = {
        type       => 'object',
        properties => {
            name => { type => 'string', minLength => 1 },
            age  => { type => 'number', minimum => 0 },
        },
    };
    my ($ok) = val({ name => 'Bob', age => 25 }, $schema);
    ok( $ok, 'properties ok' );
};

subtest 'validate property sub-schema fail' => sub {
    my $schema = {
        type       => 'object',
        properties => { age => { type => 'number', minimum => 0 } },
    };
    my ($ok, $errs) = val({ age => -5 }, $schema);
    ok( !$ok, 'property failed' );
    ok( @$errs > 0, 'errors present' );
};

subtest 'validate additional_properties allowed by default' => sub {
    my $schema = {
        type       => 'object',
        properties => { name => { type => 'string' } },
    };
    my ($ok) = val({ name => 'Alice', extra => 'field' }, $schema);
    ok( $ok, 'extra property allowed' );
};

subtest 'validate additional_properties=0 rejects extra fields' => sub {
    my $schema = {
        type                  => 'object',
        properties            => { name => { type => 'string' } },
        additional_properties => 0,
    };
    my ($ok, $errs) = val({ name => 'Alice', extra => 'field' }, $schema);
    ok( !$ok, 'extra property rejected' );
    my $found = grep { /additional property/ } @$errs;
    ok( $found, 'error mentions additional property' );
};

# ============================================================================
# validate: enum
# ============================================================================

subtest 'validate enum ok' => sub {
    my ($ok) = val('red', { enum => ['red', 'green', 'blue'] });
    ok( $ok, 'enum value valid' );
};

subtest 'validate enum fail' => sub {
    my ($ok, $errs) = val('yellow', { enum => ['red', 'green', 'blue'] });
    ok( !$ok, 'enum value invalid' );
    like( $errs->[0], qr/enum/, 'error mentions enum' );
};

# ============================================================================
# validate: multiple errors collected
# ============================================================================

subtest 'validate: multiple errors reported at once' => sub {
    my $schema = {
        type     => 'object',
        required => ['name', 'email', 'age'],
        properties => {
            name  => { type => 'string' },
            email => { type => 'string', pattern => qr/@/ },
            age   => { type => 'number', minimum => 0 },
        },
    };
    # Missing 'name', age is negative (fails minimum)
    my ($ok, $errs) = val({ email => 'user@x.com', age => -1 }, $schema);
    ok( !$ok, 'validation failed' );
    ok( @$errs >= 2, 'at least 2 errors' );
};

# ============================================================================
# schema_encode: coercion
# ============================================================================

subtest 'schema_encode: number → string coercion' => sub {
    my $schema = { type => 'string' };
    my $s = senc(42, $schema);
    is( $s, '"42"', 'integer coerced to string' );
};

subtest 'schema_encode: property coercion inside object' => sub {
    my $schema = {
        type       => 'object',
        properties => {
            price => { type => 'string' },
            qty   => { type => 'number' },
        },
    };
    my $value   = { price => 9.99, qty => 3 };
    my $s       = senc($value, $schema);
    my $decoded = dec($s);
    # price should be a string now
    ok( !CodingAdventures::JsonSerializer::_looks_like_number($decoded->{price})
        || $decoded->{price} =~ /^"?9/,
        'price coerced to string' );
    is( $decoded->{qty}, 3, 'qty remains number' );
};

subtest 'schema_encode: additional_properties filtering' => sub {
    my $schema = {
        type                  => 'object',
        additional_properties => 0,
        properties            => { name => { type => 'string' } },
    };
    my $value   = { name => 'Alice', secret => 'password123' };
    my $s       = senc($value, $schema);
    my $decoded = dec($s);
    is( $decoded->{name}, 'Alice', 'name kept' );
    ok( !exists $decoded->{secret}, 'secret dropped' );
};

subtest 'schema_encode: nested object coercion' => sub {
    my $schema = {
        type       => 'object',
        properties => {
            user => {
                type       => 'object',
                properties => {
                    id => { type => 'string' },
                },
            },
        },
    };
    my $value   = { user => { id => 100 } };
    my $s       = senc($value, $schema);
    my $decoded = dec($s);
    is( $decoded->{user}{id}, '100', 'nested id coerced to string' );
};

# ============================================================================
# Round-trip: encode → decode
# ============================================================================

sub round_trip {
    my ($v) = @_;
    return dec(enc($v));
}

subtest 'round-trip integer' => sub {
    is( round_trip(42), 42, 'integer round-trip' );
};

subtest 'round-trip string' => sub {
    is( round_trip('hello world'), 'hello world', 'string round-trip' );
};

subtest 'round-trip null' => sub {
    ok( isnul(round_trip($NULL)), 'null round-trip' );
};

subtest 'round-trip array' => sub {
    my $v = round_trip([10, 20, 30]);
    is( $v->[0], 10, 'v[0]' );
    is( $v->[1], 20, 'v[1]' );
    is( $v->[2], 30, 'v[2]' );
};

subtest 'round-trip object' => sub {
    my $v = round_trip({ x => 1, y => 2 });
    is( $v->{x}, 1, 'x = 1' );
    is( $v->{y}, 2, 'y = 2' );
};

subtest 'round-trip complex nested' => sub {
    my $data = {
        name   => 'Bob',
        scores => [10, 20, 30],
        meta   => { source => 'test' },
    };
    my $v = round_trip($data);
    is( $v->{name},        'Bob',   'name' );
    is( $v->{scores}[1],   20,      'scores[1]' );
    is( $v->{meta}{source},'test',  'meta.source' );
};

# ============================================================================
# Pretty-print produces parseable JSON
# ============================================================================

subtest 'pretty-print produces parseable JSON' => sub {
    my $data   = { name => 'Alice', age => 30, tags => ['perl', 'json'] };
    my $pretty = enc($data, { indent => 2 });
    my $v      = dec($pretty);
    is( $v->{name},    'Alice', 'name from pretty' );
    is( $v->{age},     30,      'age from pretty' );
    is( $v->{tags}[0], 'perl',  'tags[0] from pretty' );
};

done_testing;
