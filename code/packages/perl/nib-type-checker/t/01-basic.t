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

subtest 'infers the type of multi-operand arithmetic (mul_expr cascade)' => sub {
    # Regression: `1 +% 2` parses as an add_expr whose operands are mul_expr
    # nodes (LANG-FULL N1 precedence level). If mul_expr is filtered out of the
    # operand walk the expression infers undef, and an invalid bool initializer
    # slips through unchecked. It must be rejected as a u4-vs-bool mismatch.
    my $result = check_source('fn main() { let x: bool = 1 +% 2; }');

    ok(!$result->{ok}, 'type check failed');
    like(
        $result->{errors}[0]{message},
        qr/type 'u4'/,
        'arithmetic operand inferred as u4, not undef',
    );
};

subtest 'infers the type of a plain two-operand add_expr (shift_expr wrapper)' => sub {
    # Regression: since #11257 (shift expressions), add_expr's operands
    # are shift_expr nodes, not mul_expr nodes directly (add_expr = shift_expr
    # { (PLUS|MINUS|WRAP_ADD|SAT_ADD) shift_expr }). If shift_expr is filtered
    # out of the operand walk in _check_add_expression, a plain `a + b` infers
    # undef and an invalid bool initializer slips through unchecked. It must be
    # rejected as a u4-vs-bool mismatch, just like the `1 +% 2`
    # mul_expr-cascade case above.
    my $result = check_source('fn main() { let x: bool = 1 + 2; }');

    ok(!$result->{ok}, 'type check failed');
    like(
        $result->{errors}[0]{message},
        qr/type 'u4'/,
        'arithmetic operand inferred as u4, not undef',
    );
};

subtest 'infers shift expressions as numeric values' => sub {
    my $invalid = check_source('fn main() { let x: bool = 1 << 2; }');

    ok(!$invalid->{ok}, 'numeric shift cannot initialize a bool');
    like(
        $invalid->{errors}[0]{message},
        qr/type 'u4'/,
        'shift expression retains its u4 type',
    );

    my $valid = check_source('fn main() -> u4 { return 1 << 2; }');
    ok($valid->{ok}, 'numeric shift is accepted for a u4 result');
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
