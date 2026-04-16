use strict;
use warnings;
use Test2::V0;
use FindBin qw($Bin);
use Cwd qw(realpath);
BEGIN {
    # $Bin = .../code/packages/perl/brainfuck-ir-compiler/t
    # Climb 2 levels → .../code/packages/perl
    my $perl_dir = realpath("$Bin/../..");
    for my $pkg (qw(
        brainfuck-ir-compiler
        compiler-ir
        compiler-source-map
        brainfuck
        grammar-tools
        lexer
        virtual-machine
    )) {
        my $lib = "$perl_dir/$pkg/lib";
        push @INC, $lib if -d $lib;
    }
}

# ============================================================================
# Tests for CodingAdventures::BrainfuckIrCompiler
# ============================================================================
#
# These tests mirror the Go compiler test suite to verify cross-language
# parity.
#
# Test sections:
#   1. BuildConfig — debug_config and release_config presets
#   2. Empty program — prologue + HALT
#   3. Single commands — +, -, >, <, ., ,
#   4. Loop compilation — labels, BRANCH_Z, JUMP
#   5. Debug mode — bounds checking
#   6. Source map — SourceToAst and AstToIr segments
#   7. IR text output — printer integration
#   8. Roundtrip — compile → print → parse → print
#   9. Complex programs
#  10. Custom tape size
#  11. Instruction ID uniqueness
#  12. Error cases
#
# ============================================================================

use CodingAdventures::BrainfuckIrCompiler qw(compile);
use CodingAdventures::BrainfuckIrCompiler::BuildConfig;
use CodingAdventures::Brainfuck::Parser;
use CodingAdventures::CompilerIr qw(print_ir parse_ir);
use CodingAdventures::CompilerIr::IrOp;

# ── Helper: opcode constants ──────────────────────────────────────────────
my $IrOp = 'CodingAdventures::CompilerIr::IrOp';
my $Cfg  = 'CodingAdventures::BrainfuckIrCompiler::BuildConfig';

# compile_source($source, $config) — tokenize, parse, and compile.
sub compile_source {
    my ($source, $config) = @_;
    my $ast = CodingAdventures::Brainfuck::Parser->parse($source);
    return compile($ast, 'test.bf', $config);
}

# must_compile($source, $config) — compile or die with test failure.
sub must_compile {
    my ($source, $config) = @_;
    my $result = eval { compile_source($source, $config) };
    if ($@) {
        fail("compile failed for '$source': $@");
        return undef;
    }
    return $result;
}

# count_opcode($program, $opcode) — count instructions with the given opcode.
sub count_opcode {
    my ($program, $opcode) = @_;
    my $count = 0;
    for my $instr (@{ $program->{instructions} }) {
        $count++ if $instr->{opcode} == $opcode;
    }
    return $count;
}

# has_label($program, $name) — check if a LABEL instruction with $name exists.
sub has_label {
    my ($program, $name) = @_;
    for my $instr (@{ $program->{instructions} }) {
        if ($instr->{opcode} == $IrOp->LABEL
            && @{ $instr->{operands} }
            && $instr->{operands}[0]->{name} eq $name)
        {
            return 1;
        }
    }
    return 0;
}

# ============================================================================
# Section 1: BuildConfig
# ============================================================================

subtest 'BuildConfig — debug_config' => sub {
    my $cfg = $Cfg->debug_config;
    ok($cfg->{insert_bounds_checks}, 'debug: bounds checks ON');
    ok($cfg->{insert_debug_locs},    'debug: debug locs ON');
    ok($cfg->{mask_byte_arithmetic}, 'debug: byte masking ON');
    is($cfg->{tape_size}, 30000,     'debug: tape_size = 30000');
};

subtest 'BuildConfig — release_config' => sub {
    my $cfg = $Cfg->release_config;
    ok(!$cfg->{insert_bounds_checks}, 'release: bounds checks OFF');
    ok(!$cfg->{insert_debug_locs},    'release: debug locs OFF');
    ok($cfg->{mask_byte_arithmetic},  'release: byte masking ON');
    is($cfg->{tape_size}, 30000,      'release: tape_size = 30000');
};

subtest 'BuildConfig — custom' => sub {
    my $cfg = $Cfg->new(
        insert_bounds_checks => 0,
        mask_byte_arithmetic => 0,
        tape_size            => 1000,
    );
    ok(!$cfg->{mask_byte_arithmetic}, 'custom: masking OFF');
    is($cfg->{tape_size}, 1000,       'custom: tape_size = 1000');
};

# ============================================================================
# Section 2: Empty program
# ============================================================================

subtest 'empty program — has _start label' => sub {
    my $result = must_compile('', $Cfg->release_config);
    ok(has_label($result->{program}, '_start'), '_start label present');
};

subtest 'empty program — has HALT' => sub {
    my $result = must_compile('', $Cfg->release_config);
    is(count_opcode($result->{program}, $IrOp->HALT), 1, 'exactly 1 HALT');
};

subtest 'empty program — version and entry' => sub {
    my $result = must_compile('', $Cfg->release_config);
    is($result->{program}{version},     1,       'version is 1');
    is($result->{program}{entry_label}, '_start', 'entry is _start');
};

subtest 'empty program — tape data declaration' => sub {
    my $result = must_compile('', $Cfg->release_config);
    is(scalar @{ $result->{program}{data} }, 1, '1 data declaration');
    is($result->{program}{data}[0]{label},   'tape',  'label is tape');
    is($result->{program}{data}[0]{size},    30000,   'size is 30000');
    is($result->{program}{data}[0]{init},    0,       'init is 0');
};

# ============================================================================
# Section 3: Single commands
# ============================================================================

subtest 'INC (+) — LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE' => sub {
    my $result = must_compile('+', $Cfg->release_config);
    ok(count_opcode($result->{program}, $IrOp->LOAD_BYTE)  >= 1, 'has LOAD_BYTE');
    ok(count_opcode($result->{program}, $IrOp->STORE_BYTE) >= 1, 'has STORE_BYTE');
    ok(count_opcode($result->{program}, $IrOp->AND_IMM)    >= 1, 'has AND_IMM');
};

subtest 'INC (+) without masking — no AND_IMM' => sub {
    my $cfg = $Cfg->release_config;
    $cfg->{mask_byte_arithmetic} = 0;
    my $result = must_compile('+', $cfg);
    is(count_opcode($result->{program}, $IrOp->AND_IMM), 0, 'no AND_IMM when masking OFF');
};

subtest 'DEC (-) — ADD_IMM with -1' => sub {
    my $result = must_compile('-', $Cfg->release_config);
    my $found = 0;
    for my $instr (@{ $result->{program}{instructions} }) {
        if ($instr->{opcode} == $IrOp->ADD_IMM
            && @{ $instr->{operands} } >= 3
            && $instr->{operands}[2]->{value} == -1)
        {
            $found = 1;
            last;
        }
    }
    ok($found, 'ADD_IMM with -1 found for DEC');
};

subtest 'RIGHT (>) — ADD_IMM v1, v1, 1' => sub {
    my $result = must_compile('>', $Cfg->release_config);
    my $found = 0;
    for my $instr (@{ $result->{program}{instructions} }) {
        if ($instr->{opcode} == $IrOp->ADD_IMM
            && @{ $instr->{operands} } >= 3
            && $instr->{operands}[0]->{index} == 1   # v1
            && $instr->{operands}[2]->{value} == 1)
        {
            $found = 1;
            last;
        }
    }
    ok($found, 'ADD_IMM v1, v1, 1 for RIGHT');
};

subtest 'LEFT (<) — ADD_IMM v1, v1, -1' => sub {
    my $result = must_compile('<', $Cfg->release_config);
    my $found = 0;
    for my $instr (@{ $result->{program}{instructions} }) {
        if ($instr->{opcode} == $IrOp->ADD_IMM
            && @{ $instr->{operands} } >= 3
            && $instr->{operands}[0]->{index} == 1   # v1
            && $instr->{operands}[2]->{value} == -1)
        {
            $found = 1;
            last;
        }
    }
    ok($found, 'ADD_IMM v1, v1, -1 for LEFT');
};

subtest 'OUTPUT (.) — SYSCALL 1' => sub {
    my $result = must_compile('.', $Cfg->release_config);
    my $found_copy = 0;
    my $found = 0;
    for my $instr (@{ $result->{program}{instructions} }) {
        if ($instr->{opcode} == $IrOp->ADD_IMM
            && @{ $instr->{operands} } == 3
            && $instr->{operands}[0]->{index} == 4
            && $instr->{operands}[1]->{index} == 2
            && $instr->{operands}[2]->{value} == 0)
        {
            $found_copy = 1;
        }
        if ($instr->{opcode} == $IrOp->SYSCALL
            && @{ $instr->{operands} }
            && $instr->{operands}[0]->{value} == 1)
        {
            $found = 1;
            last;
        }
    }
    ok($found_copy, 'ADD_IMM copy into syscall arg register for OUTPUT');
    ok($found, 'SYSCALL 1 (write) for OUTPUT');
};

subtest 'INPUT (,) — SYSCALL 2' => sub {
    my $result = must_compile(',', $Cfg->release_config);
    my $found = 0;
    for my $instr (@{ $result->{program}{instructions} }) {
        if ($instr->{opcode} == $IrOp->SYSCALL
            && @{ $instr->{operands} }
            && $instr->{operands}[0]->{value} == 2)
        {
            $found = 1;
            last;
        }
    }
    ok($found, 'SYSCALL 2 (read) for INPUT');
};

# ============================================================================
# Section 4: Loop compilation
# ============================================================================

subtest 'simple loop [-] — labels and control flow' => sub {
    my $result = must_compile('[-]', $Cfg->release_config);
    ok(has_label($result->{program}, 'loop_0_start'), 'loop_0_start label');
    ok(has_label($result->{program}, 'loop_0_end'),   'loop_0_end label');
    ok(count_opcode($result->{program}, $IrOp->BRANCH_Z) >= 1, 'BRANCH_Z for loop entry');
    ok(count_opcode($result->{program}, $IrOp->JUMP)     >= 1, 'JUMP for back-edge');
};

subtest 'nested loops [>[+<-]] — two sets of labels' => sub {
    my $result = must_compile('[>[+<-]]', $Cfg->release_config);
    ok(has_label($result->{program}, 'loop_0_start'), 'loop_0_start label');
    ok(has_label($result->{program}, 'loop_1_start'), 'loop_1_start label');
};

subtest 'empty loop [] — still has labels' => sub {
    my $result = must_compile('[]', $Cfg->release_config);
    ok(has_label($result->{program}, 'loop_0_start'), 'loop_0_start');
    ok(has_label($result->{program}, 'loop_0_end'),   'loop_0_end');
};

# ============================================================================
# Section 5: Debug mode — bounds checking
# ============================================================================

subtest 'debug mode (>) — CMP_GT + BRANCH_NZ + __trap_oob' => sub {
    my $result = must_compile('>', $Cfg->debug_config);
    ok(count_opcode($result->{program}, $IrOp->CMP_GT)    >= 1, 'CMP_GT for right bounds check');
    ok(count_opcode($result->{program}, $IrOp->BRANCH_NZ) >= 1, 'BRANCH_NZ for trap');
    ok(has_label($result->{program}, '__trap_oob'),             '__trap_oob label');
};

subtest 'debug mode (<) — CMP_LT' => sub {
    my $result = must_compile('<', $Cfg->debug_config);
    ok(count_opcode($result->{program}, $IrOp->CMP_LT) >= 1, 'CMP_LT for left bounds check');
};

subtest 'release mode — no bounds check instructions' => sub {
    my $result = must_compile('><', $Cfg->release_config);
    is(count_opcode($result->{program}, $IrOp->CMP_GT), 0, 'no CMP_GT in release');
    is(count_opcode($result->{program}, $IrOp->CMP_LT), 0, 'no CMP_LT in release');
    ok(!has_label($result->{program}, '__trap_oob'),       'no __trap_oob in release');
};

# ============================================================================
# Section 6: Source map
# ============================================================================

subtest 'source map — SourceToAst entries for +.' => sub {
    my $result = must_compile('+.', $Cfg->release_config);
    is(scalar @{ $result->{source_map}{source_to_ast}{entries} },
        2, '2 SourceToAst entries for +.');
};

subtest 'source map — column positions' => sub {
    my $result = must_compile('+.', $Cfg->release_config);
    my @entries = @{ $result->{source_map}{source_to_ast}{entries} };
    is($entries[0]{pos}{column}, 1, '+ is at column 1');
    is($entries[1]{pos}{column}, 2, '. is at column 2');
};

subtest 'source map — filename' => sub {
    my $result = must_compile('+', $Cfg->release_config);
    for my $entry (@{ $result->{source_map}{source_to_ast}{entries} }) {
        is($entry->{pos}{file}, 'test.bf', 'file is test.bf');
    }
};

subtest 'source map — AstToIr for + produces 4 IR IDs' => sub {
    my $result = must_compile('+', $Cfg->release_config);
    is(scalar @{ $result->{source_map}{ast_to_ir}{entries} }, 1,
        '1 AstToIr entry for +');
    my $entry = $result->{source_map}{ast_to_ir}{entries}[0];
    # + produces: LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE = 4 IDs
    is(scalar @{ $entry->{ir_ids} }, 4, '4 IR IDs for + command');
};

subtest 'source map — loop has entries' => sub {
    my $result = must_compile('[-]', $Cfg->release_config);
    ok(scalar @{ $result->{source_map}{source_to_ast}{entries} } >= 2,
        'at least 2 SourceToAst entries for [-]');
};

# ============================================================================
# Section 7: IR text output
# ============================================================================

subtest 'printed IR contains required directives' => sub {
    my $result = must_compile('+.', $Cfg->release_config);
    my $text   = print_ir($result->{program});
    ok($text =~ /\.version 1/,       '.version 1');
    ok($text =~ /\.data tape 30000 0/, '.data tape 30000 0');
    ok($text =~ /\.entry _start/,    '.entry _start');
    ok($text =~ /LOAD_BYTE/,         'LOAD_BYTE present');
    ok($text =~ /HALT/,              'HALT present');
};

# ============================================================================
# Section 8: Roundtrip
# ============================================================================

subtest 'roundtrip — compile → print → parse → print' => sub {
    my $result = must_compile('++[-].', $Cfg->release_config);
    my $text1  = print_ir($result->{program});
    my $parsed = eval { parse_ir($text1) };
    ok(!$@, 'roundtrip parse succeeded') or diag("parse error: $@\nIR:\n$text1");
    my $text2  = print_ir($parsed);
    is($text2, $text1, 'roundtrip: print(parse(print(prog))) == print(prog)');
};

subtest 'roundtrip — instruction count preserved' => sub {
    my $result = must_compile('++[-].', $Cfg->release_config);
    my $text   = print_ir($result->{program});
    my $parsed = parse_ir($text);
    is(
        scalar @{ $parsed->{instructions} },
        scalar @{ $result->{program}{instructions} },
        'instruction count preserved'
    );
};

# ============================================================================
# Section 9: Complex programs
# ============================================================================

subtest 'Hello World fragment — ++++++++[>+++++++++<-]>.' => sub {
    my $source = '++++++++[>+++++++++<-]>.';
    my $result = must_compile($source, $Cfg->release_config);
    ok(has_label($result->{program}, 'loop_0_start'), 'loop_0_start present');

    # Should have SYSCALL 1 (write)
    my $found = 0;
    for my $instr (@{ $result->{program}{instructions} }) {
        if ($instr->{opcode} == $IrOp->SYSCALL
            && @{ $instr->{operands} }
            && $instr->{operands}[0]->{value} == 1)
        {
            $found = 1;
            last;
        }
    }
    ok($found, 'SYSCALL 1 (write) present');
};

subtest 'cat program ,[.,] — read + write syscalls' => sub {
    my $result = must_compile(',[.,]', $Cfg->release_config);
    my ($found_read, $found_write) = (0, 0);
    for my $instr (@{ $result->{program}{instructions} }) {
        if ($instr->{opcode} == $IrOp->SYSCALL && @{ $instr->{operands} }) {
            my $num = $instr->{operands}[0]->{value};
            $found_read  = 1 if $num == 2;
            $found_write = 1 if $num == 1;
        }
    }
    ok($found_read,  'SYSCALL 2 (read)');
    ok($found_write, 'SYSCALL 1 (write)');
};

# ============================================================================
# Section 10: Custom tape size
# ============================================================================

subtest 'custom tape size 1000' => sub {
    my $cfg = $Cfg->release_config;
    $cfg->{tape_size} = 1000;
    my $result = must_compile('', $cfg);
    is($result->{program}{data}[0]{size}, 1000, 'tape size is 1000');
};

subtest 'debug with custom tape — v5 = tape_size - 1' => sub {
    my $cfg = $Cfg->debug_config;
    $cfg->{tape_size} = 500;
    my $result = must_compile('>', $cfg);

    # LOAD_IMM v5, 499 should appear in prologue
    my $found = 0;
    for my $instr (@{ $result->{program}{instructions} }) {
        if ($instr->{opcode} == $IrOp->LOAD_IMM
            && @{ $instr->{operands} } >= 2
            && $instr->{operands}[0]->{index} == 5  # v5
            && $instr->{operands}[1]->{value} == 499)
        {
            $found = 1;
            last;
        }
    }
    ok($found, 'LOAD_IMM v5, 499 (tape_size-1) in prologue');
};

# ============================================================================
# Section 11: Instruction ID uniqueness
# ============================================================================

subtest 'instruction IDs are unique' => sub {
    my $result = must_compile('++[>+<-].', $Cfg->release_config);
    my %seen;
    for my $instr (@{ $result->{program}{instructions} }) {
        next if $instr->{id} == -1;  # labels get -1
        if ($seen{$instr->{id}}) {
            fail("duplicate instruction ID: $instr->{id}");
        }
        $seen{$instr->{id}} = 1;
    }
    pass('all instruction IDs are unique');
    ok(scalar keys %seen > 0, 'at least some instructions have IDs');
};

# ============================================================================
# Section 12: Error cases
# ============================================================================

subtest 'error — non-program root node' => sub {
    my $fake_ast = { type => 'command', token => {}, children => [] };
    my $cfg      = $Cfg->release_config;
    ok(dies { compile($fake_ast, 'test.bf', $cfg) },
        'dies for non-program root node');
};

subtest 'error — zero tape size' => sub {
    my $ast = CodingAdventures::Brainfuck::Parser->parse('');
    my $cfg = $Cfg->release_config;
    $cfg->{tape_size} = 0;
    ok(dies { compile($ast, 'test.bf', $cfg) },
        'dies for tape_size = 0');
};

subtest 'error — negative tape size' => sub {
    my $ast = CodingAdventures::Brainfuck::Parser->parse('');
    my $cfg = $Cfg->release_config;
    $cfg->{tape_size} = -1;
    ok(dies { compile($ast, 'test.bf', $cfg) },
        'dies for tape_size = -1');
};

done_testing();
