use strict;
use warnings;
use v5.28;
use Test2::V0;
use JSON::PP;
use File::Basename qw(dirname);
use File::Spec;

use CodingAdventures::StarlarkCompiler qw(:all);

sub find_cases_json {
    my @candidates = (
        File::Spec->catfile(dirname(__FILE__), '..', '..', '..', '..', 'specs', 'fixtures', 'starlark-opcode-provider-v1', 'cases.json'),
        File::Spec->catfile(dirname(__FILE__), '..', '..', '..', 'specs', 'fixtures', 'starlark-opcode-provider-v1', 'cases.json'),
        File::Spec->catfile(dirname(__FILE__), '..', '..', 'specs', 'fixtures', 'starlark-opcode-provider-v1', 'cases.json'),
        File::Spec->catfile('code', 'specs', 'fixtures', 'starlark-opcode-provider-v1', 'cases.json'),
    );

    for my $path (@candidates) {
        if (-f $path) {
            my $size = -s $path;
            if ($size > MAX_CORPUS_BYTES) {
                die corpus_byte_limit_exceeded();
            }
            open my $fh, '<:raw', $path or die corpus_malformed_transport();
            my $content = do { local $/; <$fh> };
            close $fh;
            return ($content, $path);
        }
    }
    die "STP01 fixture cases.json not found in candidate paths";
}

subtest 'STP01 Conformance Fixture Reader: cases.json' => sub {
    my ($raw_text, $path) = find_cases_json();
    ok(defined $raw_text, "Found and loaded STP01 fixture from $path");
    ok(length($raw_text) <= MAX_CORPUS_BYTES, 'Corpus byte length <= MAX_CORPUS_BYTES');

    # Enforce UTF-8 BOM rejection
    ok($raw_text !~ /^\xEF\xBB\xBF/, 'Fixture does not contain forbidden UTF-8 BOM');

    my $data = eval { decode_json($raw_text) };
    if ($@ || !ref($data)) {
        die corpus_malformed_transport();
    }

    is($data->{schema_version}, STP01_SCHEMA_VERSION, 'schema_version == 1');
    is($data->{profile}, STP01_PROFILE, 'profile == standalone-provider-v1');

    my $opcodes = $data->{opcodes};
    is(scalar(@$opcodes), EXPECTED_OPCODE_COUNT, 'fixture defines 61 opcodes');

    for my $item (@$opcodes) {
        my $op = op_from_byte($item->{byte});
        ok(defined $op, "op_from_byte($item->{byte}) defined");
        is($op->{symbol}, $item->{symbol}, "symbol matches $item->{symbol}");
        is($op->{category}{id}, $item->{category}, "category matches $item->{category}");
    }

    my $decode = $data->{decode};
    is(scalar(@$decode), EXPECTED_DECODE_COUNT, 'decode array has 256 items');

    my $assigned_seen = 0;
    my $holes_seen = 0;

    for my $item (@$decode) {
        my $op = op_from_byte($item->{byte});
        if (!defined $item->{symbol}) {
            $holes_seen++;
            is($op, undef, "hole at byte $item->{byte} returns undef");
        } else {
            $assigned_seen++;
            ok(defined $op, "assigned byte $item->{byte} returns defined op");
            is($op->{symbol}, $item->{symbol}, "symbol matches $item->{symbol}");
            is($op->{category}{id}, $item->{category}, "category matches $item->{category}");
        }
    }

    is($assigned_seen, EXPECTED_OPCODE_COUNT, '61 assigned bytes in decode array');
    is($holes_seen, EXPECTED_HOLE_COUNT, '195 holes in decode array');

    # Verify operators section from fixture
    my $operators = $data->{operators};
    ok(defined $operators, 'operators section present in fixture');

    for my $item (@{$operators->{binary} || []}) {
        my $op = binary_opcode($item->{operator});
        ok(defined $op, "binary operator $item->{operator} defined");
        is($op->{symbol}, $item->{symbol}, "binary operator $item->{operator} maps to $item->{symbol}");
    }

    for my $item (@{$operators->{compare} || []}) {
        my $op = compare_opcode($item->{operator});
        ok(defined $op, "compare operator $item->{operator} defined");
        is($op->{symbol}, $item->{symbol}, "compare operator $item->{operator} maps to $item->{symbol}");
    }

    for my $item (@{$operators->{augmented_assign} || []}) {
        my $op = augmented_assign_opcode($item->{operator});
        ok(defined $op, "augassign operator $item->{operator} defined");
        is($op->{symbol}, $item->{symbol}, "augassign operator $item->{operator} maps to $item->{symbol}");
    }

    for my $item (@{$operators->{unary} || []}) {
        my $op = unary_opcode($item->{operator});
        ok(defined $op, "unary operator $item->{operator} defined");
        is($op->{symbol}, $item->{symbol}, "unary operator $item->{operator} maps to $item->{symbol}");
    }

    my $unknown_lookups = $data->{unknown_lookups};
    is(scalar(@$unknown_lookups), EXPECTED_UNKNOWN_LOOKUP_COUNT, '8 unknown lookups');

    for my $item (@$unknown_lookups) {
        my $group = $item->{group};
        my $op_str = $item->{operator};
        if ($group eq 'binary') {
            is(binary_opcode($op_str), undef, "unknown binary operator $op_str returns undef");
        } elsif ($group eq 'compare') {
            is(compare_opcode($op_str), undef, "unknown compare operator $op_str returns undef");
        } elsif ($group eq 'augmented_assign') {
            is(augmented_assign_opcode($op_str), undef, "unknown augassign operator $op_str returns undef");
        } elsif ($group eq 'unary') {
            is(unary_opcode($op_str), undef, "unknown unary operator $op_str returns undef");
        }
    }
};

subtest 'Adversarial Transport & Rejection Invariants' => sub {
    # Test oversized corpus detection
    my $oversized = " " x (MAX_CORPUS_BYTES + 1);
    my $err_limit = corpus_byte_limit_exceeded();
    is($err_limit->error_id, ERROR_ID_INVALID_CORPUS, 'byte limit exceeded error id');
    is($err_limit->message, MSG_CORPUS_BYTE_LIMIT_EXCEEDED, 'byte limit exceeded message');

    # Test malformed transport
    my $malformed_err = corpus_malformed_transport();
    is($malformed_err->error_id, ERROR_ID_INVALID_CORPUS, 'malformed transport error id');
    is($malformed_err->message, MSG_CORPUS_MALFORMED_TRANSPORT, 'malformed transport message');

    # Test schema violation
    my $schema_err = corpus_schema_violation();
    is($schema_err->error_id, ERROR_ID_INVALID_CORPUS, 'schema violation error id');
    is($schema_err->message, MSG_CORPUS_SCHEMA_VIOLATION, 'schema violation message');
};

subtest '10,000-Iteration Adversarial Mutation Fuzz Loop' => sub {
    srand(0x5741524C);
    my $handled = 0;

    my @test_symbols = (
        'LoadConst', 'Halt', 'Add', 'Sub', 'Jump', 'Print',
        'InvalidOpcode', '', ' ', '123', "\0", "LoadConst\0",
        'foo', 'BAR', undef,
    );

    my @test_ops = (
        '+', '-', '*', '/', '//', '%', '**', '&', '|', '^', '<<', '>>',
        '==', '!=', '<', '>', '<=', '>=', 'in', 'not in',
        '+=', '-=', '*=', '/=', '//=', '%=', '**=', '&=', '|=', '^=', '<<=', '>>=',
        '~', '@', '++', ':=', 'invalid', '', undef,
    );

    for my $i (1 .. 10000) {
        my $mode = int(rand(4)) + 1;
        if ($mode == 1) {
            my $b = int(rand(350)) - 50;
            my $op = op_from_byte($b);
            $handled++;
        } elsif ($mode == 2) {
            my $sym = $test_symbols[int(rand(scalar @test_symbols))];
            my $op = op_from_symbol($sym);
            $handled++;
        } elsif ($mode == 3) {
            my $op_sym = $test_ops[int(rand(scalar @test_ops))];
            my $b_op = binary_opcode($op_sym);
            my $c_op = compare_opcode($op_sym);
            my $a_op = augmented_assign_opcode($op_sym);
            my $u_op = unary_opcode($op_sym);
            $handled++;
        } else {
            my $m = binary_op_map();
            delete $m->{'+'};
            $handled++;
        }
    }

    is($handled, 10000, 'Handled 10,000 adversarial fuzz mutations successfully');
};

done_testing;
