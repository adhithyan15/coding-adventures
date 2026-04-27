use strict;
use warnings;
use Test2::V0;

use CodingAdventures::CompilerIr::IrOp qw(op_name);
use CodingAdventures::NibIrCompiler qw(compile compile_source release_config);
use CodingAdventures::NibTypeChecker qw(check_source);

sub _op_names {
    my ($program) = @_;
    return [ map { op_name($_->{opcode}) } @{ $program->{instructions} } ];
}

subtest 'lowers a typed program into entry-point IR' => sub {
    my $typed = check_source('fn main() -> u4 { return 7; }');
    ok($typed->{ok}, 'source type checks');

    my $compiled = compile($typed->{typed_ast}, release_config());
    is($compiled->{program}{entry_label}, '_start', 'IR entry point is _start');

    my $ops = _op_names($compiled->{program});
    ok(grep($_ eq 'LABEL', @$ops), 'labels are emitted');
    ok(grep($_ eq 'CALL', @$ops), 'entry point calls main');
    ok(grep($_ eq 'HALT', @$ops), 'entry point halts');
};

subtest 'loop-heavy programs emit explicit control-flow opcodes' => sub {
    my $compiled = compile_source(<<'NIB', release_config());
fn count_to(n: u4) -> u4 {
    let acc: u4 = 0;
    for i: u4 in 0..n {
        acc = acc +% 1;
    }
    return acc;
}
NIB

    my $ops = _op_names($compiled->{program});
    ok(grep($_ eq 'BRANCH_Z', @$ops), 'loop lowering emits a zero-branch');
    ok(grep($_ eq 'JUMP', @$ops), 'loop lowering emits a back edge');
    ok(grep($_ eq 'CMP_LT', @$ops), 'loop lowering compares iterator and limit');
};

done_testing;
