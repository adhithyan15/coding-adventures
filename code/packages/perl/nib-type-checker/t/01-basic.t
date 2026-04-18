use strict;
use warnings;
use Test2::V0;

use CodingAdventures::NibTypeChecker qw(check_source);

subtest 'accepts a well-typed Nib program' => sub {
    my $result = check_source(<<'NIB');
fn add(a: u4, b: u4) -> u4 {
    return a +% b;
}

fn main() -> u4 {
    return add(3, 4);
}
NIB

    ok($result->{ok}, 'type check succeeded');
    isa_ok($result->{typed_ast}, 'CodingAdventures::NibTypeChecker::TypedAst');
    ok($result->{typed_ast}->root, 'typed AST keeps the parser root');
    is($result->{errors}, [], 'no diagnostics on success');
};

subtest 'reports assignment type mismatches' => sub {
    my $result = check_source(<<'NIB');
fn main() {
    let flag: bool = true;
    flag = 1;
}
NIB

    ok(!$result->{ok}, 'type check failed');
    like(
        $result->{errors}[0]{message},
        qr/Cannot assign expression of type 'u4' to 'flag' of type 'bool'\./,
        'diagnostic explains the mismatch',
    );
};

subtest 'reports parse failures through the protocol result' => sub {
    my $result = check_source('fn main(');

    ok(!$result->{ok}, 'parse failure reported as an error result');
    like(
        $result->{errors}[0]{message},
        qr/parse|unexpected|expected/i,
        'diagnostic carries parser context',
    );
};

done_testing;
