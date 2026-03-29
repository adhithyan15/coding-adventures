use strict;
use warnings;
use Test2::V0;
use FindBin;
use lib "$FindBin::Bin/../../lexer/lib";
use lib "$FindBin::Bin/../../parser/lib";

ok(eval { require CodingAdventures::Lexer;            1 }, 'Lexer loads');
ok(eval { require CodingAdventures::Parser;           1 }, 'Parser loads');
ok(eval { require CodingAdventures::BytecodeCompiler; 1 }, 'BytecodeCompiler loads');

CodingAdventures::BytecodeCompiler->import(qw(disassemble));

# Helper: lex + parse + compile
sub _compile {
    my ($src) = @_;
    my @tokens   = CodingAdventures::Lexer->new($src)->tokenize();
    my $parser   = CodingAdventures::Parser->new(\@tokens);
    my $ast      = $parser->parse();
    my $compiler = CodingAdventures::BytecodeCompiler->new();
    return $compiler->compile($ast);
}

# Opcode constants (mirror those in the module — defined as my vars so strict is happy)
my $OP_PUSH  =  0;
my $OP_POP   =  1;
my $OP_ADD   =  2;
my $OP_SUB   =  3;
my $OP_MUL   =  4;
my $OP_DIV   =  5;
my $OP_AND   =  6;
my $OP_OR    =  7;
my $OP_NOT   =  8;
my $OP_JMP   =  9;
my $OP_JZ    = 10;
my $OP_JNZ   = 11;
my $OP_HALT  = 12;
my $OP_LOAD  = 13;
my $OP_STORE = 14;
my $OP_DUP   = 15;
my $OP_SWAP  = 16;

# ============================================================================
# Construction
# ============================================================================

subtest 'new creates compiler' => sub {
    my $c = CodingAdventures::BytecodeCompiler->new();
    ok(defined $c, 'compiler created');
    ok($c->isa('CodingAdventures::BytecodeCompiler'), 'isa BytecodeCompiler');
};

# ============================================================================
# Opcode constants from module
# ============================================================================

subtest 'opcode constants' => sub {
    is(CodingAdventures::BytecodeCompiler->OP_PUSH,  0,  'PUSH=0');
    is(CodingAdventures::BytecodeCompiler->OP_ADD,   2,  'ADD=2');
    is(CodingAdventures::BytecodeCompiler->OP_HALT,  12, 'HALT=12');
    is(CodingAdventures::BytecodeCompiler->OP_LOAD,  13, 'LOAD=13');
    is(CodingAdventures::BytecodeCompiler->OP_STORE, 14, 'STORE=14');
};

# ============================================================================
# Number literal compilation
# ============================================================================

subtest 'compile number literal' => sub {
    my $bc = _compile('42');
    # Expected: PUSH 42, POP (expr stmt), HALT
    is($bc->[0], $OP_PUSH, 'first is PUSH');
    is($bc->[1], 42,       'operand is 42');
};

subtest 'compile returns arrayref' => sub {
    my $bc = _compile('1');
    ok(ref($bc) eq 'ARRAY', 'compile returns arrayref');
    ok(scalar @$bc > 0, 'non-empty bytecode');
};

subtest 'bytecode ends with HALT' => sub {
    my $bc = _compile('99');
    is($bc->[-1], $OP_HALT, 'last instruction is HALT');
};

# ============================================================================
# Binary operations
# ============================================================================

subtest 'compile addition' => sub {
    my $bc = _compile('1 + 2');
    # PUSH 1, PUSH 2, ADD, POP (discarded), HALT
    is($bc->[0], $OP_PUSH, 'PUSH');
    is($bc->[1], 1,        'value 1');
    is($bc->[2], $OP_PUSH, 'PUSH');
    is($bc->[3], 2,        'value 2');
    is($bc->[4], $OP_ADD,  'ADD');
};

subtest 'compile subtraction' => sub {
    my $bc = _compile('5 - 3');
    ok((grep { $_ == $OP_SUB } @$bc), 'has SUB opcode');
};

subtest 'compile multiplication' => sub {
    my $bc = _compile('3 * 4');
    ok((grep { $_ == $OP_MUL } @$bc), 'has MUL opcode');
};

subtest 'compile division' => sub {
    my $bc = _compile('8 / 2');
    ok((grep { $_ == $OP_DIV } @$bc), 'has DIV opcode');
};

# ============================================================================
# Let binding
# ============================================================================

subtest 'compile let binding' => sub {
    my $bc = _compile('let x = 42');
    # PUSH 42, STORE "x", HALT
    ok((grep { $_ == $OP_STORE } @$bc), 'has STORE opcode');
    # Find STORE and check next element is "x"
    for my $i (0 .. $#$bc - 1) {
        if ($bc->[$i] == $OP_STORE) {
            is($bc->[$i + 1], 'x', 'stores into x');
            last;
        }
    }
};

subtest 'compile load variable' => sub {
    my @tokens = CodingAdventures::Lexer->new('x')->tokenize();
    my $parser = CodingAdventures::Parser->new(\@tokens);
    my $ast    = $parser->parse();
    my $c      = CodingAdventures::BytecodeCompiler->new();
    my $bc     = $c->compile($ast);
    ok((grep { $_ == $OP_LOAD } @$bc), 'identifier compiles to LOAD');
};

# ============================================================================
# If expression
# ============================================================================

subtest 'compile if expression' => sub {
    my $bc = _compile('if x then 1 else 2');
    ok((grep { $_ == $OP_JZ } @$bc),  'if has JZ');
    ok((grep { $_ == $OP_JMP } @$bc), 'if has JMP');
};

# ============================================================================
# Unary operators
# ============================================================================

subtest 'compile unary not' => sub {
    my $bc = _compile('!x');
    ok((grep { $_ == $OP_NOT } @$bc), 'unary ! compiles to NOT');
};

subtest 'compile unary minus' => sub {
    my $bc = _compile('-5');
    ok((grep { $_ == $OP_MUL } @$bc), 'unary - uses MUL with -1');
};

# ============================================================================
# disassemble
# ============================================================================

subtest 'disassemble output' => sub {
    my $bc  = _compile('1 + 2');
    my $dis = disassemble($bc);
    ok(defined $dis,         'disassemble returns string');
    like($dis, qr/PUSH/,     'contains PUSH');
    like($dis, qr/ADD/,      'contains ADD');
    like($dis, qr/HALT/,     'contains HALT');
    like($dis, qr/^\d{4}:/m, 'has address prefix');
};

subtest 'disassemble let binding' => sub {
    my $bc  = _compile('let x = 99');
    my $dis = disassemble($bc);
    like($dis, qr/PUSH\s+99/, 'has PUSH 99');
    like($dis, qr/STORE\s+x/, 'has STORE x');
};

# ============================================================================
# Multiple statements
# ============================================================================

subtest 'compile multiple statements' => sub {
    my $bc = _compile("let a = 1\nlet b = 2");
    # Should have two STORE operations
    my @stores = grep { $_ == $OP_STORE } @$bc;
    is(scalar @stores, 2, 'two STORE opcodes');
};

done_testing;
