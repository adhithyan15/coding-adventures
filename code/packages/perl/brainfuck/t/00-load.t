use strict;
use warnings;
use Test2::V0;

ok(eval { require CodingAdventures::Brainfuck; 1 }, 'CodingAdventures::Brainfuck loads');

# Verify exported functions exist
my @fns = qw(validate compile_to_opcodes run_opcodes interpret);
for my $fn (@fns) {
    ok(CodingAdventures::Brainfuck->can($fn), "can $fn");
}

# Verify opcode constants
ok(defined CodingAdventures::Brainfuck::OP_RIGHT(),      'OP_RIGHT defined');
ok(defined CodingAdventures::Brainfuck::OP_HALT(),       'OP_HALT defined');

ok(defined $CodingAdventures::Brainfuck::VERSION, 'has VERSION');

done_testing();
