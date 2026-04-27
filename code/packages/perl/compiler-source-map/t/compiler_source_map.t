use strict;
use warnings;
use Test2::V0;

# ============================================================================
# Tests for CodingAdventures::CompilerSourceMap
# ============================================================================
#
# Test sections:
#   1. SourcePosition — span of characters in source
#   2. SourceToAst — Segment 1 container
#   3. AstToIr — Segment 2 container
#   4. IrToIr — Segment 3 container
#   5. IrToMachineCode — Segment 4 container
#   6. SourceMapChain — full pipeline sidecar + composite queries
#
# ============================================================================

use CodingAdventures::CompilerSourceMap;
use CodingAdventures::CompilerSourceMap::SourcePosition;
use CodingAdventures::CompilerSourceMap::SourceToAst;
use CodingAdventures::CompilerSourceMap::AstToIr;
use CodingAdventures::CompilerSourceMap::IrToIr;
use CodingAdventures::CompilerSourceMap::IrToMachineCode;
use CodingAdventures::CompilerSourceMap::SourceMapChain;

my $Pos     = 'CodingAdventures::CompilerSourceMap::SourcePosition';
my $S2A     = 'CodingAdventures::CompilerSourceMap::SourceToAst';
my $A2I     = 'CodingAdventures::CompilerSourceMap::AstToIr';
my $I2I     = 'CodingAdventures::CompilerSourceMap::IrToIr';
my $I2MC    = 'CodingAdventures::CompilerSourceMap::IrToMachineCode';
my $Chain   = 'CodingAdventures::CompilerSourceMap::SourceMapChain';

# ============================================================================
# Section 1: SourcePosition
# ============================================================================

subtest 'SourcePosition — construction' => sub {
    my $pos = $Pos->new(file => 'hello.bf', line => 1, column => 3, length => 1);
    is($pos->{file},   'hello.bf', 'file stored');
    is($pos->{line},   1,          'line stored');
    is($pos->{column}, 3,          'column stored');
    is($pos->{length}, 1,          'length stored');
};

subtest 'SourcePosition — to_string' => sub {
    my $pos = $Pos->new(file => 'hello.bf', line => 1, column => 3, length => 1);
    is($pos->to_string, 'hello.bf:1:3 (len=1)', 'to_string format');
};

subtest 'SourcePosition — defaults' => sub {
    my $pos = $Pos->new;
    is($pos->{file},   '', 'default file is empty string');
    is($pos->{line},   0,  'default line is 0');
    is($pos->{column}, 0,  'default column is 0');
    is($pos->{length}, 0,  'default length is 0');
};

# ============================================================================
# Section 2: SourceToAst
# ============================================================================

subtest 'SourceToAst — empty' => sub {
    my $s2a = $S2A->new;
    is(scalar @{ $s2a->{entries} }, 0, 'empty entries');
};

subtest 'SourceToAst — add and lookup' => sub {
    my $s2a = $S2A->new;
    my $pos = $Pos->new(file => 'test.bf', line => 1, column => 1, length => 1);
    $s2a->add($pos, 42);

    is(scalar @{ $s2a->{entries} }, 1, 'one entry');

    my $found = $s2a->lookup_by_node_id(42);
    ok(defined $found, 'lookup_by_node_id found something');
    is($found->{file},   'test.bf', 'file matches');
    is($found->{line},   1,         'line matches');
    is($found->{column}, 1,         'column matches');
};

subtest 'SourceToAst — lookup_by_node_id not found' => sub {
    my $s2a = $S2A->new;
    my $found = $s2a->lookup_by_node_id(999);
    ok(!defined $found, 'returns undef when not found');
};

subtest 'SourceToAst — multiple entries' => sub {
    my $s2a = $S2A->new;
    $s2a->add($Pos->new(file => 'a.bf', line => 1, column => 1, length => 1), 1);
    $s2a->add($Pos->new(file => 'a.bf', line => 1, column => 2, length => 1), 2);
    $s2a->add($Pos->new(file => 'a.bf', line => 1, column => 3, length => 1), 3);

    is(scalar @{ $s2a->{entries} }, 3, '3 entries');

    my $pos2 = $s2a->lookup_by_node_id(2);
    is($pos2->{column}, 2, 'node 2 maps to column 2');

    my $pos3 = $s2a->lookup_by_node_id(3);
    is($pos3->{column}, 3, 'node 3 maps to column 3');
};

# ============================================================================
# Section 3: AstToIr
# ============================================================================

subtest 'AstToIr — empty' => sub {
    my $a2i = $A2I->new;
    is(scalar @{ $a2i->{entries} }, 0, 'empty entries');
};

subtest 'AstToIr — add and lookup_by_ast_node_id' => sub {
    my $a2i = $A2I->new;
    $a2i->add(42, [7, 8, 9, 10]);

    my $ir_ids = $a2i->lookup_by_ast_node_id(42);
    ok(defined $ir_ids, 'found entries');
    is(scalar @$ir_ids, 4,  '4 IR IDs');
    is($ir_ids->[0],    7,  'first ID is 7');
    is($ir_ids->[3],    10, 'last ID is 10');
};

subtest 'AstToIr — lookup_by_ir_id' => sub {
    my $a2i = $A2I->new;
    $a2i->add(42, [7, 8, 9, 10]);

    is($a2i->lookup_by_ir_id(7),  42, 'IR ID 7 → node 42');
    is($a2i->lookup_by_ir_id(10), 42, 'IR ID 10 → node 42');
    is($a2i->lookup_by_ir_id(99), -1, 'IR ID 99 → -1 (not found)');
};

subtest 'AstToIr — multiple nodes' => sub {
    my $a2i = $A2I->new;
    $a2i->add(1, [0, 1, 2]);      # node 1 → IR 0,1,2
    $a2i->add(2, [3, 4, 5, 6]);   # node 2 → IR 3,4,5,6

    is($a2i->lookup_by_ir_id(0), 1, 'IR 0 → node 1');
    is($a2i->lookup_by_ir_id(3), 2, 'IR 3 → node 2');
    is($a2i->lookup_by_ir_id(6), 2, 'IR 6 → node 2');
};

# ============================================================================
# Section 4: IrToIr
# ============================================================================

subtest 'IrToIr — construction' => sub {
    my $pass = $I2I->new('contraction');
    is($pass->{pass_name},            'contraction', 'pass_name stored');
    is(scalar @{ $pass->{entries} },   0,            'empty entries');
    is(scalar keys %{ $pass->{deleted} }, 0,         'empty deleted set');
};

subtest 'IrToIr — add_mapping' => sub {
    my $pass = $I2I->new('identity');
    $pass->add_mapping(7, [7]);
    $pass->add_mapping(8, [8]);

    my $ids7 = $pass->lookup_by_original_id(7);
    is($ids7->[0], 7, 'identity: 7 → [7]');
};

subtest 'IrToIr — add_deletion' => sub {
    my $pass = $I2I->new('dead_store');
    $pass->add_deletion(42);

    ok($pass->{deleted}{42}, 'ID 42 is in deleted set');
    my $result = $pass->lookup_by_original_id(42);
    ok(!defined $result, 'deleted ID returns undef');
};

subtest 'IrToIr — lookup_by_new_id' => sub {
    my $pass = $I2I->new('contraction');
    $pass->add_mapping(7, [100]);
    $pass->add_mapping(8, [100]);
    $pass->add_mapping(9, [100]);

    # All three originals map to 100
    is($pass->lookup_by_new_id(100), 7,  'first match for new ID 100 is 7');
    is($pass->lookup_by_new_id(999), -1, 'not found returns -1');
};

subtest 'IrToIr — preservation' => sub {
    my $pass = $I2I->new('identity');
    $pass->add_mapping(5, [5]);

    my $ids = $pass->lookup_by_original_id(5);
    is($ids->[0], 5, 'preserved: 5 → [5]');
    is($pass->lookup_by_new_id(5), 5, 'reverse: 5 → 5');
};

# ============================================================================
# Section 5: IrToMachineCode
# ============================================================================

subtest 'IrToMachineCode — empty' => sub {
    my $i2mc = $I2MC->new;
    is(scalar @{ $i2mc->{entries} }, 0, 'empty entries');
};

subtest 'IrToMachineCode — add and lookup_by_ir_id' => sub {
    my $i2mc = $I2MC->new;
    $i2mc->add(3, 0x14, 8);

    my ($off, $len) = $i2mc->lookup_by_ir_id(3);
    is($off, 0x14, 'offset is 0x14');
    is($len, 8,    'length is 8');
};

subtest 'IrToMachineCode — lookup_by_ir_id not found' => sub {
    my $i2mc = $I2MC->new;
    my ($off, $len) = $i2mc->lookup_by_ir_id(999);
    is($off, -1, 'not found returns offset -1');
    is($len, 0,  'not found returns length 0');
};

subtest 'IrToMachineCode — lookup_by_mc_offset' => sub {
    my $i2mc = $I2MC->new;
    $i2mc->add(3, 0x14, 8);   # IR 3 → bytes 0x14..0x1B

    is($i2mc->lookup_by_mc_offset(0x14), 3, 'offset 0x14 → IR 3');
    is($i2mc->lookup_by_mc_offset(0x17), 3, 'offset 0x17 → IR 3 (inside)');
    is($i2mc->lookup_by_mc_offset(0x1B), 3, 'offset 0x1B → IR 3 (last byte)');
    is($i2mc->lookup_by_mc_offset(0x1C), -1,'offset 0x1C → -1 (past end)');
    is($i2mc->lookup_by_mc_offset(0x13), -1,'offset 0x13 → -1 (before start)');
};

subtest 'IrToMachineCode — multiple entries' => sub {
    my $i2mc = $I2MC->new;
    $i2mc->add(0, 0, 4);    # IR 0 → bytes 0..3
    $i2mc->add(1, 4, 4);    # IR 1 → bytes 4..7
    $i2mc->add(2, 8, 8);    # IR 2 → bytes 8..15

    is($i2mc->lookup_by_mc_offset(2),  0, 'byte 2 → IR 0');
    is($i2mc->lookup_by_mc_offset(5),  1, 'byte 5 → IR 1');
    is($i2mc->lookup_by_mc_offset(12), 2, 'byte 12 → IR 2');
};

# ============================================================================
# Section 6: SourceMapChain
# ============================================================================

subtest 'SourceMapChain — construction' => sub {
    my $chain = $Chain->new_chain;
    ok(defined $chain->{source_to_ast},    'source_to_ast initialized');
    ok(defined $chain->{ast_to_ir},        'ast_to_ir initialized');
    is(ref $chain->{ir_to_ir},     'ARRAY', 'ir_to_ir is arrayref');
    ok(!defined $chain->{ir_to_machine_code}, 'ir_to_machine_code is undef');
};

subtest 'SourceMapChain — add_optimizer_pass' => sub {
    my $chain = $Chain->new_chain;
    my $pass  = $I2I->new('identity');
    $chain->add_optimizer_pass($pass);
    is(scalar @{ $chain->{ir_to_ir} }, 1, 'one pass recorded');
    is($chain->{ir_to_ir}[0]{pass_name}, 'identity', 'pass name stored');
};

subtest 'SourceMapChain — source_to_mc returns undef when chain incomplete' => sub {
    my $chain = $Chain->new_chain;
    my $pos   = $Pos->new(file => 'x.bf', line => 1, column => 1, length => 1);
    my $result = $chain->source_to_mc($pos);
    ok(!defined $result, 'returns undef when ir_to_machine_code is nil');
};

subtest 'SourceMapChain — mc_to_source returns undef when chain incomplete' => sub {
    my $chain  = $Chain->new_chain;
    my $result = $chain->mc_to_source(0);
    ok(!defined $result, 'returns undef when ir_to_machine_code is nil');
};

subtest 'SourceMapChain — full forward chain (source_to_mc)' => sub {
    my $chain = $Chain->new_chain;

    # Set up segment 1: source position (1,1) → AST node 0
    my $pos = $Pos->new(file => 'test.bf', line => 1, column => 1, length => 1);
    $chain->{source_to_ast}->add($pos, 0);

    # Set up segment 2: AST node 0 → IR IDs [0, 1, 2, 3]
    $chain->{ast_to_ir}->add(0, [0, 1, 2, 3]);

    # No optimiser passes (skip segment 3)

    # Set up segment 4: IR IDs → machine code
    my $i2mc = $I2MC->new;
    $i2mc->add(0, 0,  4);
    $i2mc->add(1, 4,  4);
    $i2mc->add(2, 8,  4);
    $i2mc->add(3, 12, 4);
    $chain->{ir_to_machine_code} = $i2mc;

    my $results = $chain->source_to_mc($pos);
    ok(defined $results,          'got results');
    is(scalar @$results, 4,       '4 machine code entries');
    is($results->[0]{ir_id},      0,  'first ir_id is 0');
    is($results->[0]{mc_offset},  0,  'first mc_offset is 0');
    is($results->[3]{mc_offset},  12, 'last mc_offset is 12');
};

subtest 'SourceMapChain — full reverse chain (mc_to_source)' => sub {
    my $chain = $Chain->new_chain;

    # Segment 1: source position (1,5) → AST node 7
    my $pos = $Pos->new(file => 'hello.bf', line => 1, column => 5, length => 1);
    $chain->{source_to_ast}->add($pos, 7);

    # Segment 2: AST node 7 → IR IDs [10, 11]
    $chain->{ast_to_ir}->add(7, [10, 11]);

    # Segment 4: IR ID 10 → bytes 40..47
    my $i2mc = $I2MC->new;
    $i2mc->add(10, 40, 8);
    $i2mc->add(11, 48, 4);
    $chain->{ir_to_machine_code} = $i2mc;

    # Reverse: MC offset 44 (inside IR 10) → source
    my $found_pos = $chain->mc_to_source(44);
    ok(defined $found_pos,             'mc_to_source found a position');
    is($found_pos->{file},   'hello.bf', 'file matches');
    is($found_pos->{line},    1,          'line matches');
    is($found_pos->{column},  5,          'column matches');
};

subtest 'SourceMapChain — chain with optimiser pass' => sub {
    my $chain = $Chain->new_chain;

    # Segment 1
    my $pos = $Pos->new(file => 'opt.bf', line => 1, column => 1, length => 1);
    $chain->{source_to_ast}->add($pos, 0);

    # Segment 2: AST node 0 → IR IDs [5, 6]
    $chain->{ast_to_ir}->add(0, [5, 6]);

    # Segment 3: optimiser renames IR 5→50 and 6→51
    my $pass = $I2I->new('renumber');
    $pass->add_mapping(5, [50]);
    $pass->add_mapping(6, [51]);
    $chain->add_optimizer_pass($pass);

    # Segment 4: new IR IDs 50 and 51 → machine code
    my $i2mc = $I2MC->new;
    $i2mc->add(50, 0, 4);
    $i2mc->add(51, 4, 4);
    $chain->{ir_to_machine_code} = $i2mc;

    # Forward: source → MC
    my $results = $chain->source_to_mc($pos);
    ok(defined $results, 'forward chain works through optimiser pass');
    is(scalar @$results, 2, '2 machine code entries after pass');

    # Reverse: MC offset 2 → source
    my $found = $chain->mc_to_source(2);
    ok(defined $found,         'reverse chain works through optimiser pass');
    is($found->{column}, 1,    'traced back to column 1');
};

subtest 'SourceMapChain — source_to_mc returns undef when position not found' => sub {
    my $chain = $Chain->new_chain;
    $chain->{ir_to_machine_code} = $I2MC->new;

    my $pos = $Pos->new(file => 'missing.bf', line => 99, column => 99, length => 1);
    my $result = $chain->source_to_mc($pos);
    ok(!defined $result, 'returns undef for unknown source position');
};

subtest 'SourceMapChain — mc_to_source returns undef when offset not found' => sub {
    my $chain = $Chain->new_chain;
    $chain->{ir_to_machine_code} = $I2MC->new;  # empty

    my $result = $chain->mc_to_source(0x1000);
    ok(!defined $result, 'returns undef for unknown machine code offset');
};

done_testing();
