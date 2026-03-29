use strict;
use warnings;
use Test2::V0;

use CodingAdventures::Brainfuck qw(validate compile_to_opcodes run_opcodes interpret);

# ---------------------------------------------------------------------------
# validate()
# ---------------------------------------------------------------------------

subtest 'validate() — empty program' => sub {
    my ($ok, $err) = validate("");
    ok($ok,           'empty program is valid');
    ok(!defined $err, 'no error');
};

subtest 'validate() — no brackets' => sub {
    my ($ok) = validate("+++---");
    ok($ok, 'no-bracket program is valid');
};

subtest 'validate() — properly nested' => sub {
    ok(validate("[[][]]"), 'nested [] valid');
    ok(validate("[[[]]]"), 'deeply nested valid');
    ok(validate("[]"),     'empty loop valid');
};

subtest 'validate() — lone [' => sub {
    my ($ok, $err) = validate("[");
    ok(!$ok,           'lone [ is invalid');
    ok(defined $err,   'error message returned');
};

subtest 'validate() — lone ]' => sub {
    my ($ok, $err) = validate("]");
    ok(!$ok,         '] alone is invalid');
    ok(defined $err, 'error message');
};

subtest 'validate() — extra ] after pairs' => sub {
    my ($ok) = validate("[]]]");
    ok(!$ok, 'extra ] invalid');
};

subtest 'validate() — ignores non-command chars' => sub {
    my ($ok) = validate("Hello [World]!");
    ok($ok, 'comments allowed inside brackets');
};

# ---------------------------------------------------------------------------
# compile_to_opcodes()
# ---------------------------------------------------------------------------

subtest 'compile_to_opcodes() — opcode mapping' => sub {
    my ($ops, $err) = compile_to_opcodes("><+-.,");
    ok(!defined $err, 'no error');
    is(scalar @$ops, 7, '6 commands + HALT');
    is($ops->[0]{op}, CodingAdventures::Brainfuck::OP_RIGHT(),      'op 0 = RIGHT');
    is($ops->[1]{op}, CodingAdventures::Brainfuck::OP_LEFT(),       'op 1 = LEFT');
    is($ops->[2]{op}, CodingAdventures::Brainfuck::OP_INC(),        'op 2 = INC');
    is($ops->[3]{op}, CodingAdventures::Brainfuck::OP_DEC(),        'op 3 = DEC');
    is($ops->[4]{op}, CodingAdventures::Brainfuck::OP_OUTPUT(),     'op 4 = OUTPUT');
    is($ops->[5]{op}, CodingAdventures::Brainfuck::OP_INPUT(),      'op 5 = INPUT');
    is($ops->[6]{op}, CodingAdventures::Brainfuck::OP_HALT(),       'op 6 = HALT');
};

subtest 'compile_to_opcodes() — ignores comments' => sub {
    my ($ops) = compile_to_opcodes("Hello+World.");
    # Only + and . are commands, plus HALT
    is(scalar @$ops, 3, '3 instructions (2 cmds + HALT)');
};

subtest 'compile_to_opcodes() — jump targets for [+]' => sub {
    # Index:  0  1  2  3(HALT)
    # Op:     [  +  ]  HALT
    my ($ops, $err) = compile_to_opcodes("[+]");
    ok(!defined $err, 'no error');
    # [ at 0: jump to 3 (HALT) if cell==0, i.e., one past ] at index 2
    is($ops->[0]{operand}, 3, '[ jumps to 3 (past ])');
    # ] at 2: jump back to [ at 0
    is($ops->[2]{operand}, 0, '] jumps to [');
};

subtest 'compile_to_opcodes() — nested jump targets [[]]' => sub {
    # Index: 0  1  2  3  4(HALT)
    # Op:    [  [  ]  ]  HALT
    my ($ops) = compile_to_opcodes("[[]]");
    is($ops->[0]{operand}, 4, 'outer [ → 4');
    is($ops->[1]{operand}, 3, 'inner [ → 3');
    is($ops->[2]{operand}, 1, 'inner ] → 1');
    is($ops->[3]{operand}, 0, 'outer ] → 0');
};

subtest 'compile_to_opcodes() — error on unbalanced brackets' => sub {
    my ($ops, $err) = compile_to_opcodes("[");
    ok(!defined $ops, 'ops is undef on error');
    ok(defined $err,  'error message returned');
};

subtest 'compile_to_opcodes() — HALT appended' => sub {
    my ($ops) = compile_to_opcodes("+++");
    is($ops->[-1]{op}, CodingAdventures::Brainfuck::OP_HALT(), 'last op is HALT');
};

# ---------------------------------------------------------------------------
# interpret() — basic operations
# ---------------------------------------------------------------------------

subtest 'interpret() — +++++ outputs char(5)' => sub {
    my ($out, $err) = interpret("+++++.", "");
    ok(!defined $err,       'no error');
    is($out, chr(5),        'output is char(5)');
};

subtest 'interpret() — 72 + then . outputs H' => sub {
    my $prog = "+" x 72 . ".";
    my ($out, $err) = interpret($prog, "");
    ok(!defined $err, 'no error');
    is($out, "H",     'output is H');
};

subtest 'interpret() — multiple outputs' => sub {
    my ($out, $err) = interpret("++.+.", "");
    ok(!defined $err, 'no error');
    is($out, chr(2) . chr(3), 'char(2)char(3)');
};

subtest 'interpret() — empty program' => sub {
    my ($out, $err) = interpret("", "");
    ok(!defined $err, 'no error');
    is($out, "",      'no output');
};

subtest 'interpret() — comments are ignored' => sub {
    my ($out, $err) = interpret("Hello ++. World", "");
    ok(!defined $err,   'no error');
    is($out, chr(2),    'char(2) output');
};

subtest 'interpret() — error for unbalanced [' => sub {
    my ($out, $err) = interpret("[", "");
    ok(!defined $out, 'output is undef');
    ok(defined $err,  'error returned');
};

# ---------------------------------------------------------------------------
# Cell wrapping
# ---------------------------------------------------------------------------

subtest 'cell wrapping — 255 + 1 = 0' => sub {
    my $prog = "+" x 256 . ".";
    my ($out) = interpret($prog, "");
    is($out, chr(0), '255+1 wraps to 0');
};

subtest 'cell wrapping — 0 - 1 = 255' => sub {
    my ($out) = interpret("-.", "");
    is($out, chr(255), '0-1 wraps to 255');
};

# ---------------------------------------------------------------------------
# Loops
# ---------------------------------------------------------------------------

subtest 'loop — [+] skipped when cell is 0' => sub {
    my ($out) = interpret("[+].", "");
    is($out, chr(0), 'loop body skipped when cell is 0');
};

subtest 'loop — +++[-] zeros the cell' => sub {
    my ($out) = interpret("+++[-].", "");
    is($out, chr(0), 'cell decremented to 0');
};

subtest 'loop — copy value to next cell' => sub {
    # +++[->+<]>.  =  move 3 from cell[0] to cell[1]
    my ($out) = interpret("+++[->+<]>.", "");
    is($out, chr(3), 'value copied to cell[1]');
};

# ---------------------------------------------------------------------------
# Input / EOF
# ---------------------------------------------------------------------------

subtest 'input — reads byte from input' => sub {
    my ($out) = interpret(",.", "A");
    is($out, "A", 'reads and echoes A');
};

subtest 'input — ,[.,] echoes string (cat)' => sub {
    my ($out) = interpret(",[.,]", "hello");
    is($out, "hello", 'cat program echoes input');
};

subtest 'input — EOF sets cell to 0' => sub {
    my ($out) = interpret(",.", "");
    is($out, chr(0), 'EOF gives cell = 0');
};

subtest 'input — reads multiple bytes' => sub {
    my ($out) = interpret(",.,.", "AB");
    is($out, "AB", 'reads two bytes in sequence');
};

# ---------------------------------------------------------------------------
# Data pointer
# ---------------------------------------------------------------------------

subtest 'data pointer — > initialises cell to 0' => sub {
    my ($out) = interpret(">.", "");
    is($out, chr(0), 'cell[1] starts at 0');
};

subtest 'data pointer — independent cells' => sub {
    my ($out) = interpret("+.>++.", "");
    is($out, chr(1) . chr(2), 'cell[0]=1, cell[1]=2');
};

# ---------------------------------------------------------------------------
# Hello World "H"
# ---------------------------------------------------------------------------

subtest 'Hello World H via multiplication' => sub {
    # 9 * 8 = 72 = 'H'
    my ($out, $err) = interpret("+++++++++[>++++++++<-]>.", "");
    ok(!defined $err, 'no error');
    is($out, "H", 'output is H');
};

done_testing();
