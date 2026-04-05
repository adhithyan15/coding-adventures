use strict;
use warnings;
use Test2::V0;

use lib '../wasm-leb128/lib';
use lib '../wasm-types/lib';
use lib '../wasm-opcodes/lib';
use lib '../wasm-module-parser/lib';
use lib '../virtual-machine/lib';

use CodingAdventures::WasmValidator qw(validate);

# ============================================================================
# Helper: build a minimal valid module
# ============================================================================

sub _make_module {
    my (%overrides) = @_;
    return {
        types     => $overrides{types}   // [{ params => [0x7F], results => [0x7F] }],
        functions => $overrides{functions} // [0],
        exports   => $overrides{exports}  // [],
        imports   => $overrides{imports}  // [],
        memories  => $overrides{memories} // [],
        code      => $overrides{code}     // [],
        start     => $overrides{start},
    };
}

# ============================================================================
# Valid modules
# ============================================================================

subtest 'validates a minimal module' => sub {
    my $module = _make_module();
    my $result = validate($module);
    ok($result, 'validate returns a result');
    ok($result->{module}, 'result contains original module');
    ok($result->{func_types}, 'result contains func_types');
    is(scalar(@{ $result->{func_types} }), 1, 'one function type resolved');
};

subtest 'validates module with multiple types and functions' => sub {
    my $module = _make_module(
        types     => [
            { params => [0x7F], results => [0x7F] },  # (i32) -> (i32)
            { params => [0x7F, 0x7F], results => [0x7F] },  # (i32, i32) -> (i32)
        ],
        functions => [0, 1, 0],  # three functions using two types
    );
    my $result = validate($module);
    is(scalar(@{ $result->{func_types} }), 3, 'three function types resolved');
};

subtest 'validates module with exports' => sub {
    my $module = _make_module(
        exports => [
            { name => 'add', desc => { kind => 'func', idx => 0 } },
        ],
    );
    my $result = validate($module);
    ok($result, 'module with export validates');
};

subtest 'validates module with memory' => sub {
    my $module = _make_module(
        memories => [{ min => 1, max => 10 }],
    );
    my $result = validate($module);
    ok($result, 'module with memory validates');
};

subtest 'validates module with memory (no max)' => sub {
    my $module = _make_module(
        memories => [{ min => 1, max => undef }],
    );
    my $result = validate($module);
    ok($result, 'module with open-ended memory validates');
};

subtest 'validates module with imported function' => sub {
    my $module = _make_module(
        imports => [
            { module => 'env', name => 'log', desc => { kind => 'func', idx => 0 } },
        ],
        functions => [0],
    );
    my $result = validate($module);
    is(scalar(@{ $result->{func_types} }), 2, 'import + module func = 2 types');
};

# ============================================================================
# Invalid type indices
# ============================================================================

subtest 'rejects invalid type index in function section' => sub {
    my $module = _make_module(
        types     => [{ params => [], results => [] }],
        functions => [99],  # out of range
    );
    my $died = 0;
    my $err;
    eval { validate($module) };
    if ($@) {
        $died = 1;
        $err = $@;
    }
    ok($died, 'dies on invalid type index');
    is($err->kind(), 'invalid_type_index', 'error kind is invalid_type_index');
};

subtest 'rejects invalid type index in import' => sub {
    my $module = _make_module(
        types   => [{ params => [], results => [] }],
        imports => [
            { module => 'env', name => 'f', desc => { kind => 'func', idx => 5 } },
        ],
    );
    my $died = 0;
    eval { validate($module) };
    $died = 1 if $@;
    ok($died, 'dies on invalid import type index');
};

# ============================================================================
# Memory validation
# ============================================================================

subtest 'rejects multiple memories' => sub {
    my $module = _make_module(
        memories => [{ min => 1, max => 10 }, { min => 1, max => 5 }],
    );
    my $died = 0;
    my $err;
    eval { validate($module) };
    if ($@) { $died = 1; $err = $@; }
    ok($died, 'dies on multiple memories');
    is($err->kind(), 'multiple_memories', 'error kind is multiple_memories');
};

subtest 'rejects memory min exceeding max pages' => sub {
    my $module = _make_module(
        memories => [{ min => 70000, max => undef }],
    );
    my $died = 0;
    my $err;
    eval { validate($module) };
    if ($@) { $died = 1; $err = $@; }
    ok($died, 'dies on excessive min pages');
    is($err->kind(), 'memory_limit_exceeded', 'correct error kind');
};

subtest 'rejects memory max exceeding max pages' => sub {
    my $module = _make_module(
        memories => [{ min => 1, max => 70000 }],
    );
    my $died = 0;
    my $err;
    eval { validate($module) };
    if ($@) { $died = 1; $err = $@; }
    ok($died, 'dies on excessive max pages');
    is($err->kind(), 'memory_limit_exceeded', 'correct error kind');
};

subtest 'rejects memory min > max' => sub {
    my $module = _make_module(
        memories => [{ min => 10, max => 5 }],
    );
    my $died = 0;
    my $err;
    eval { validate($module) };
    if ($@) { $died = 1; $err = $@; }
    ok($died, 'dies on min > max');
    is($err->kind(), 'memory_limit_order', 'correct error kind');
};

# ============================================================================
# Export validation
# ============================================================================

subtest 'rejects duplicate export names' => sub {
    my $module = _make_module(
        exports => [
            { name => 'foo', desc => { kind => 'func', idx => 0 } },
            { name => 'foo', desc => { kind => 'func', idx => 0 } },
        ],
    );
    my $died = 0;
    my $err;
    eval { validate($module) };
    if ($@) { $died = 1; $err = $@; }
    ok($died, 'dies on duplicate export names');
    is($err->kind(), 'duplicate_export_name', 'correct error kind');
};

subtest 'allows different export names' => sub {
    my $module = _make_module(
        exports => [
            { name => 'add',    desc => { kind => 'func', idx => 0 } },
            { name => 'square', desc => { kind => 'func', idx => 0 } },
        ],
    );
    my $result = validate($module);
    ok($result, 'different export names are ok');
};

subtest 'rejects export with out-of-range function index' => sub {
    my $module = _make_module(
        exports => [
            { name => 'bad', desc => { kind => 'func', idx => 99 } },
        ],
    );
    my $died = 0;
    my $err;
    eval { validate($module) };
    if ($@) { $died = 1; $err = $@; }
    ok($died, 'dies on out-of-range export index');
    is($err->kind(), 'export_index_out_of_range', 'correct error kind');
};

# ============================================================================
# Start function validation
# ============================================================================

subtest 'validates valid start function' => sub {
    my $module = _make_module(
        types     => [
            { params => [], results => [] },       # type 0: () -> ()
            { params => [0x7F], results => [0x7F] },
        ],
        functions => [0, 1],
        start     => 0,
    );
    my $result = validate($module);
    ok($result, 'valid start function accepted');
};

subtest 'rejects start function with wrong signature' => sub {
    my $module = _make_module(
        types     => [{ params => [0x7F], results => [0x7F] }],
        functions => [0],
        start     => 0,
    );
    my $died = 0;
    my $err;
    eval { validate($module) };
    if ($@) { $died = 1; $err = $@; }
    ok($died, 'dies on start function with params');
    is($err->kind(), 'start_function_bad_type', 'correct error kind');
};

subtest 'rejects start function with out-of-range index' => sub {
    my $module = _make_module(
        start => 99,
    );
    my $died = 0;
    my $err;
    eval { validate($module) };
    if ($@) { $died = 1; $err = $@; }
    ok($died, 'dies on out-of-range start index');
    is($err->kind(), 'invalid_func_index', 'correct error kind');
};

# ============================================================================
# ValidationError structure
# ============================================================================

subtest 'ValidationError has kind and message' => sub {
    my $err = CodingAdventures::WasmValidator::ValidationError->new(
        kind    => 'test_error',
        message => 'test message',
    );
    is($err->kind(), 'test_error', 'kind accessor works');
    is($err->message(), 'test message', 'message accessor works');
};

done_testing;
