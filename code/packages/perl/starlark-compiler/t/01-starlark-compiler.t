use strict;
use warnings;
use v5.28;
use Test2::V0;

use CodingAdventures::StarlarkCompiler qw(:all);

# Independent verification table of all 61 opcodes: literal constants defined outside the provider
my @independent_opcodes = (
    [ 0x01, 'LoadConst', 'stack' ],
    [ 0x02, 'Pop', 'stack' ],
    [ 0x03, 'Dup', 'stack' ],
    [ 0x04, 'LoadNone', 'stack' ],
    [ 0x05, 'LoadTrue', 'stack' ],
    [ 0x06, 'LoadFalse', 'stack' ],
    [ 0x10, 'StoreName', 'variable' ],
    [ 0x11, 'LoadName', 'variable' ],
    [ 0x12, 'StoreLocal', 'variable' ],
    [ 0x13, 'LoadLocal', 'variable' ],
    [ 0x14, 'StoreClosure', 'variable' ],
    [ 0x15, 'LoadClosure', 'variable' ],
    [ 0x20, 'Add', 'arithmetic' ],
    [ 0x21, 'Sub', 'arithmetic' ],
    [ 0x22, 'Mul', 'arithmetic' ],
    [ 0x23, 'Div', 'arithmetic' ],
    [ 0x24, 'FloorDiv', 'arithmetic' ],
    [ 0x25, 'Mod', 'arithmetic' ],
    [ 0x26, 'Power', 'arithmetic' ],
    [ 0x27, 'Negate', 'arithmetic' ],
    [ 0x28, 'BitAnd', 'arithmetic' ],
    [ 0x29, 'BitOr', 'arithmetic' ],
    [ 0x2A, 'BitXor', 'arithmetic' ],
    [ 0x2B, 'BitNot', 'arithmetic' ],
    [ 0x2C, 'LShift', 'arithmetic' ],
    [ 0x2D, 'RShift', 'arithmetic' ],
    [ 0x30, 'CmpEq', 'comparison' ],
    [ 0x31, 'CmpNe', 'comparison' ],
    [ 0x32, 'CmpLt', 'comparison' ],
    [ 0x33, 'CmpGt', 'comparison' ],
    [ 0x34, 'CmpLe', 'comparison' ],
    [ 0x35, 'CmpGe', 'comparison' ],
    [ 0x36, 'CmpIn', 'comparison' ],
    [ 0x37, 'CmpNotIn', 'comparison' ],
    [ 0x38, 'Not', 'comparison' ],
    [ 0x40, 'Jump', 'control_flow' ],
    [ 0x41, 'JumpIfFalse', 'control_flow' ],
    [ 0x42, 'JumpIfTrue', 'control_flow' ],
    [ 0x43, 'JumpIfFalseOrPop', 'control_flow' ],
    [ 0x44, 'JumpIfTrueOrPop', 'control_flow' ],
    [ 0x50, 'MakeFunction', 'function' ],
    [ 0x51, 'CallFunction', 'function' ],
    [ 0x52, 'CallFunctionKw', 'function' ],
    [ 0x53, 'Return', 'function' ],
    [ 0x60, 'BuildList', 'collection' ],
    [ 0x61, 'BuildDict', 'collection' ],
    [ 0x62, 'BuildTuple', 'collection' ],
    [ 0x63, 'ListAppend', 'collection' ],
    [ 0x64, 'DictSet', 'collection' ],
    [ 0x70, 'LoadSubscript', 'subscript_attribute' ],
    [ 0x71, 'StoreSubscript', 'subscript_attribute' ],
    [ 0x72, 'LoadAttr', 'subscript_attribute' ],
    [ 0x73, 'StoreAttr', 'subscript_attribute' ],
    [ 0x74, 'LoadSlice', 'subscript_attribute' ],
    [ 0x80, 'GetIter', 'iteration' ],
    [ 0x81, 'ForIter', 'iteration' ],
    [ 0x82, 'UnpackSequence', 'iteration' ],
    [ 0x90, 'LoadModule', 'module' ],
    [ 0x91, 'ImportFrom', 'module' ],
    [ 0xA0, 'Print', 'io' ],
    [ 0xFF, 'Halt', 'vm_control' ],
);

my @independent_binary_operators = (
    [ '+', 'Add' ],
    [ '-', 'Sub' ],
    [ '*', 'Mul' ],
    [ '/', 'Div' ],
    [ '//', 'FloorDiv' ],
    [ '%', 'Mod' ],
    [ '**', 'Power' ],
    [ '&', 'BitAnd' ],
    [ '|', 'BitOr' ],
    [ '^', 'BitXor' ],
    [ '<<', 'LShift' ],
    [ '>>', 'RShift' ],
);

my @independent_compare_operators = (
    [ '==', 'CmpEq' ],
    [ '!=', 'CmpNe' ],
    [ '<', 'CmpLt' ],
    [ '>', 'CmpGt' ],
    [ '<=', 'CmpLe' ],
    [ '>=', 'CmpGe' ],
    [ 'in', 'CmpIn' ],
    [ 'not in', 'CmpNotIn' ],
);

my @independent_augassign_operators = (
    [ '+=', 'Add' ],
    [ '-=', 'Sub' ],
    [ '*=', 'Mul' ],
    [ '/=', 'Div' ],
    [ '//=', 'FloorDiv' ],
    [ '%=', 'Mod' ],
    [ '**=', 'Power' ],
    [ '&=', 'BitAnd' ],
    [ '|=', 'BitOr' ],
    [ '^=', 'BitXor' ],
    [ '<<=', 'LShift' ],
    [ '>>=', 'RShift' ],
);

my @independent_unary_operators = (
    [ '-', 'Negate' ],
    [ '~', 'BitNot' ],
);

subtest 'Canonical 61 Opcodes & 12 Categories' => sub {
    my $ops = all_ops();
    is(scalar(@$ops), 61, 'all_ops returns exactly 61 opcodes');
    is(scalar(@independent_opcodes), 61, 'independent table defines 61 opcodes');

    for my $spec (@independent_opcodes) {
        my ($byte_val, $symbol, $cat_id) = @$spec;

        my $op = op_from_byte($byte_val);
        ok(defined $op, "op_from_byte($byte_val) defined");
        is($op->{byte_value}, $byte_val, "byte_value matches $byte_val");
        is($op->{symbol}, $symbol, "symbol matches $symbol");
        is($op->{category}{id}, $cat_id, "category matches $cat_id");

        is(op_byte($op), $byte_val, "op_byte returns $byte_val");
        is(op_symbol($op), $symbol, "op_symbol returns $symbol");
        is(op_category($op)->{id}, $cat_id, "op_category returns $cat_id");

        my $from_sym = op_from_symbol($symbol);
        ok(defined $from_sym, "op_from_symbol($symbol) defined");
        is($from_sym->{byte_value}, $byte_val, "byte_value from symbol matches $byte_val");

        my $cat_from_byte = op_category_from_byte($byte_val);
        ok(defined $cat_from_byte, "op_category_from_byte($byte_val) defined");
        is($cat_from_byte->{id}, $cat_id, "category from byte matches $cat_id");
    }
};

subtest '256 Decode Matrix & 195 Holes' => sub {
    my $assigned_count = 0;
    my $hole_count = 0;

    for my $b (0 .. 255) {
        my $op = op_from_byte($b);
        if (defined $op) {
            $assigned_count++;
            is($op->{byte_value}, $b, "byte matches $b");
        } else {
            $hole_count++;
            is(op_category_from_byte($b), undef, "category is undef for hole byte $b");
        }
    }

    is($assigned_count, 61, '61 assigned bytes');
    is($hole_count, 195, '195 holes');
    is($assigned_count + $hole_count, 256, 'total 256 byte positions');

    # Out of bounds and non-numeric inputs
    is(op_from_byte(-1), undef, 'op_from_byte(-1) undef');
    is(op_from_byte(256), undef, 'op_from_byte(256) undef');
    is(op_from_byte(1000), undef, 'op_from_byte(1000) undef');
    is(op_from_byte('abc'), undef, 'op_from_byte non-numeric undef');
    is(op_from_byte("1\n"), undef, 'op_from_byte with trailing newline undef');
    is(op_from_byte({}), undef, 'op_from_byte hashref undef');
    is(op_from_byte([]), undef, 'op_from_byte arrayref undef');
    is(op_from_byte(undef), undef, 'op_from_byte undef input');

    is(op_category_from_byte(-1), undef, 'op_category_from_byte(-1) undef');
    is(op_category_from_byte(256), undef, 'op_category_from_byte(256) undef');
    is(op_category_from_byte('xyz'), undef, 'op_category_from_byte non-numeric undef');
    is(op_category_from_byte(undef), undef, 'op_category_from_byte undef');

    is(op_from_symbol('NonExistent'), undef, 'op_from_symbol unknown undef');
    is(op_from_symbol(''), undef, 'op_from_symbol empty undef');
    is(op_from_symbol(undef), undef, 'op_from_symbol undef input');

    is(op_byte(undef), undef, 'op_byte(undef) undef');
    is(op_symbol(undef), undef, 'op_symbol(undef) undef');
    is(op_category(undef), undef, 'op_category(undef) undef');
};

subtest '34 Operator Mappings & Fresh-Map Ownership' => sub {
    # 12 binary operators
    my $b_map = binary_op_map();
    is(scalar(keys %$b_map), 12, '12 binary operators in map');
    for my $spec (@independent_binary_operators) {
        my ($sym, $exp_sym) = @$spec;
        ok(defined $b_map->{$sym}, "binary operator $sym mapped");
        is($b_map->{$sym}{symbol}, $exp_sym, "binary operator $sym maps to $exp_sym");
        is(binary_opcode($sym)->{symbol}, $exp_sym, "binary_opcode($sym) matches $exp_sym");
    }

    # 8 comparison operators
    my $c_map = compare_op_map();
    is(scalar(keys %$c_map), 8, '8 comparison operators in map');
    for my $spec (@independent_compare_operators) {
        my ($sym, $exp_sym) = @$spec;
        ok(defined $c_map->{$sym}, "compare operator $sym mapped");
        is($c_map->{$sym}{symbol}, $exp_sym, "compare operator $sym maps to $exp_sym");
        is(compare_opcode($sym)->{symbol}, $exp_sym, "compare_opcode($sym) matches $exp_sym");
    }

    # 12 augmented assign operators
    my $a_map = augmented_assign_map();
    is(scalar(keys %$a_map), 12, '12 augmented assign operators in map');
    for my $spec (@independent_augassign_operators) {
        my ($sym, $exp_sym) = @$spec;
        ok(defined $a_map->{$sym}, "augassign operator $sym mapped");
        is($a_map->{$sym}{symbol}, $exp_sym, "augassign operator $sym maps to $exp_sym");
        is(augmented_assign_opcode($sym)->{symbol}, $exp_sym, "augmented_assign_opcode($sym) matches $exp_sym");
    }

    # 2 unary operators
    my $u_map = unary_op_map();
    is(scalar(keys %$u_map), 2, '2 unary operators in map');
    for my $spec (@independent_unary_operators) {
        my ($sym, $exp_sym) = @$spec;
        ok(defined $u_map->{$sym}, "unary operator $sym mapped");
        is($u_map->{$sym}{symbol}, $exp_sym, "unary operator $sym maps to $exp_sym");
        is(unary_opcode($sym)->{symbol}, $exp_sym, "unary_opcode($sym) matches $exp_sym");
    }

    # Unknown operators return undef
    is(binary_opcode('unknown'), undef, 'binary_opcode unknown undef');
    is(binary_opcode(undef), undef, 'binary_opcode undef input');
    is(compare_opcode('unknown'), undef, 'compare_opcode unknown undef');
    is(compare_opcode(undef), undef, 'compare_opcode undef input');
    is(augmented_assign_opcode('unknown'), undef, 'augmented_assign_opcode unknown undef');
    is(augmented_assign_opcode(undef), undef, 'augmented_assign_opcode undef input');
    is(unary_opcode('unknown'), undef, 'unary_opcode unknown undef');
    is(unary_opcode(undef), undef, 'unary_opcode undef input');

    # Fresh map ownership
    my $m1 = binary_op_map();
    my $m2 = binary_op_map();
    isnt("$m1", "$m2", 'binary_op_map returns fresh hashref instances');
    delete $m1->{'+'};
    $m1->{'__pollute__'} = 1;
    ok(!exists $m1->{'+'}, 'mutated m1');
    ok(exists $m2->{'+'}, 'm2 unaffected by mutation of m1');
    ok(!exists $m2->{'__pollute__'}, 'm2 unaffected by pollution');
    ok(defined binary_opcode('+'), 'binary_opcode still returns Add');

    my $l1 = all_ops();
    my $l2 = all_ops();
    isnt("$l1", "$l2", 'all_ops returns fresh arrayref instances');
    shift @$l1;
    is(scalar(@$l1), 60, 'mutated l1 length 60');
    is(scalar(@$l2), 61, 'l2 unaffected length 61');
    is(scalar(@{all_ops()}), 61, 'all_ops unaffected length 61');

    my $cm1 = compare_op_map();
    my $cm2 = compare_op_map();
    isnt("$cm1", "$cm2", 'compare_op_map returns fresh instance');

    my $am1 = augmented_assign_map();
    my $am2 = augmented_assign_map();
    isnt("$am1", "$am2", 'augmented_assign_map returns fresh instance');

    my $um1 = unary_op_map();
    my $um2 = unary_op_map();
    isnt("$um1", "$um2", 'unary_op_map returns fresh instance');
};

subtest 'CamelCase Aliases' => sub {
    is(\&allOps, \&all_ops, 'allOps alias');
    is(\&opFromByte, \&op_from_byte, 'opFromByte alias');
    is(\&opFromSymbol, \&op_from_symbol, 'opFromSymbol alias');
    is(\&opByte, \&op_byte, 'opByte alias');
    is(\&opSymbol, \&op_symbol, 'opSymbol alias');
    is(\&opCategory, \&op_category, 'opCategory alias');
    is(\&opCategoryFromByte, \&op_category_from_byte, 'opCategoryFromByte alias');
    is(\&binaryOpMap, \&binary_op_map, 'binaryOpMap alias');
    is(\&compareOpMap, \&compare_op_map, 'compareOpMap alias');
    is(\&augmentedAssignMap, \&augmented_assign_map, 'augmentedAssignMap alias');
    is(\&unaryOpMap, \&unary_op_map, 'unaryOpMap alias');
    is(\&binaryOpcode, \&binary_opcode, 'binaryOpcode alias');
    is(\&compareOpcode, \&compare_opcode, 'compareOpcode alias');
    is(\&augmentedAssignOpcode, \&augmented_assign_opcode, 'augmentedAssignOpcode alias');
    is(\&unaryOpcode, \&unary_opcode, 'unaryOpcode alias');

    is(opFromByte(0x01)->{symbol}, 'LoadConst', 'opFromByte works');
    is(opFromSymbol('Halt')->{byte_value}, 0xFF, 'opFromSymbol works');
    is(opByte(opFromSymbol('Pop')), 0x02, 'opByte works');
    is(opSymbol(opFromByte(0x02)), 'Pop', 'opSymbol works');
    is(opCategory(opFromByte(0x01))->{id}, 'stack', 'opCategory works');
    is(opCategoryFromByte(0x01)->{id}, 'stack', 'opCategoryFromByte works');
};

subtest 'Standing Rule #20: Centralized Diagnostics & Payload Blindness' => sub {
    my $e1 = invalid_byte();
    is($e1->error_id, ERROR_ID_INVALID_BYTE, 'invalid_byte error_id');
    is($e1->message, MSG_INVALID_BYTE, 'invalid_byte message');
    is("$e1", MSG_INVALID_BYTE, 'string overloading');
    ok(is_starlark_compiler_error($e1), 'is_starlark_compiler_error true');

    my $e2 = unknown_opcode();
    is($e2->error_id, ERROR_ID_UNKNOWN_OPCODE, 'unknown_opcode error_id');
    is($e2->message, MSG_UNKNOWN_OPCODE, 'unknown_opcode message');
    ok(is_starlark_compiler_error($e2), 'is_starlark_compiler_error true');

    my $e3 = unknown_operator();
    is($e3->error_id, ERROR_ID_UNKNOWN_OPERATOR, 'unknown_operator error_id');
    is($e3->message, MSG_UNKNOWN_OPERATOR, 'unknown_operator message');
    ok(is_starlark_compiler_error($e3), 'is_starlark_compiler_error true');

    my $e4 = corpus_byte_limit_exceeded();
    is($e4->error_id, ERROR_ID_INVALID_CORPUS, 'corpus_byte_limit_exceeded error_id');
    is($e4->message, MSG_CORPUS_BYTE_LIMIT_EXCEEDED, 'corpus_byte_limit_exceeded message');
    ok(is_starlark_compiler_error($e4), 'is_starlark_compiler_error true');

    my $e5 = corpus_malformed_transport();
    is($e5->error_id, ERROR_ID_INVALID_CORPUS, 'corpus_malformed_transport error_id');
    is($e5->message, MSG_CORPUS_MALFORMED_TRANSPORT, 'corpus_malformed_transport message');
    ok(is_starlark_compiler_error($e5), 'is_starlark_compiler_error true');

    my $e6 = corpus_schema_violation();
    is($e6->error_id, ERROR_ID_INVALID_CORPUS, 'corpus_schema_violation error_id');
    is($e6->message, MSG_CORPUS_SCHEMA_VIOLATION, 'corpus_schema_violation message');
    ok(is_starlark_compiler_error($e6), 'is_starlark_compiler_error true');

    ok(!is_starlark_compiler_error(undef), 'undef is not error');
    ok(!is_starlark_compiler_error('string error'), 'string is not error');
    ok(!is_starlark_compiler_error({}), 'hash is not error');

    my @all_errs = ($e1, $e2, $e3, $e4, $e5, $e6);
    for my $err (@all_errs) {
        ok(index($err->message, '%s') == -1, 'no %s formatting placeholder');
        ok(index($err->message, '%d') == -1, 'no %d formatting placeholder');
        ok(index($err->message, '<script>') == -1, 'no <script> injection');
    }

    is(REDACTED, '<redacted>', 'REDACTED constant');
    is(INVALID, '<invalid>', 'INVALID constant');
};

subtest '5-Class Security Resilience Showcase' => sub {
    # Class 1: Bounded Memory & Input Envelope Protection
    is(MAX_CORPUS_BYTES, 131072, 'MAX_CORPUS_BYTES 131072');
    is(MAX_JSON_NESTING_DEPTH, 8, 'MAX_JSON_NESTING_DEPTH 8');
    is(EXPECTED_HOLE_COUNT, 195, 'EXPECTED_HOLE_COUNT 195');
    is(EXPECTED_DECODE_COUNT, 256, 'EXPECTED_DECODE_COUNT 256');

    # Class 2: Lexical, Character & Encoding Integrity
    is(MIN_SURROGATE_CODE_POINT, 0xD800, 'MIN_SURROGATE_CODE_POINT');
    is(MAX_SURROGATE_CODE_POINT, 0xDFFF, 'MAX_SURROGATE_CODE_POINT');

    # Class 3: Deterministic Fresh-State Isolation
    my $m1 = binary_op_map();
    my $m2 = binary_op_map();
    isnt("$m1", "$m2", 'Fresh state isolation');

    # Class 4: Closed Diagnostic Catalog & Payload Blindness
    my $err = invalid_byte();
    ok(defined $err->message, 'Error has static message');
    ok(index($err->message, '<script>') == -1, 'No reflected script tags');

    # Class 5: Adversarial Transport & Mutation Resilience
    is(STP01_SCHEMA_VERSION, 1, 'STP01_SCHEMA_VERSION 1');
    is(STP01_PROFILE, 'standalone-provider-v1', 'STP01_PROFILE standalone-provider-v1');
    is(MIN_BYTE_VALUE, 0, 'MIN_BYTE_VALUE 0');
    is(MAX_BYTE_VALUE, 255, 'MAX_BYTE_VALUE 255');
};

subtest '200-Case Deterministic PRNG Property Test' => sub {
    srand(0x5741524C); # deterministic seed

    for my $i (1 .. 200) {
        my $query_byte = int(rand(300)) - 20; # range -20 .. 279
        my $op = op_from_byte($query_byte);
        my $cat = op_category_from_byte($query_byte);

        if ($query_byte < 0 || $query_byte > 255) {
            is($op, undef, "out-of-bounds byte $query_byte returns undef op");
            is($cat, undef, "out-of-bounds byte $query_byte returns undef cat");
        } elsif (defined $op) {
            is($op->{byte_value}, $query_byte, "byte_value matches $query_byte");
            is($op->{category}, $cat, "category matches op_category_from_byte");
            is(op_byte($op), $query_byte, "op_byte matches $query_byte");
            is($op->{symbol}, op_symbol($op), "symbol matches op_symbol");
            is($op->{category}, op_category($op), "category matches op_category");
            is($op, op_from_symbol($op->{symbol}), "roundtrip op_from_symbol");
        } else {
            is($cat, undef, "hole byte $query_byte returns undef category");
        }
    }
};

done_testing;
