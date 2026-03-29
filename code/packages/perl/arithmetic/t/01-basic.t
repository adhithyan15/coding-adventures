use strict;
use warnings;
use Test2::V0;

# Load the module using eval so we get a clear error if it fails to compile.
ok(eval { require CodingAdventures::Arithmetic; 1 }, 'CodingAdventures::Arithmetic loads');

# Convenience alias for the top-level package
my $pkg = 'CodingAdventures::Arithmetic';

# ============================================================================
# Half Adder — truth table exhaustive check
# ============================================================================
#
# There are only 4 input combinations for two single bits, so we test every
# one of them. This is called "exhaustive testing" and is only feasible for
# very small input spaces.

subtest 'half_adder — truth table' => sub {
    # 0 + 0 = 0, no carry
    my ($s, $c) = $pkg->can('half_adder')->(0, 0);
    is($s, 0, 'half_adder(0,0) sum=0');
    is($c, 0, 'half_adder(0,0) carry=0');

    # 0 + 1 = 1, no carry
    ($s, $c) = $pkg->can('half_adder')->(0, 1);
    is($s, 1, 'half_adder(0,1) sum=1');
    is($c, 0, 'half_adder(0,1) carry=0');

    # 1 + 0 = 1, no carry (commutative)
    ($s, $c) = $pkg->can('half_adder')->(1, 0);
    is($s, 1, 'half_adder(1,0) sum=1');
    is($c, 0, 'half_adder(1,0) carry=0');

    # 1 + 1 = 2 = 10₂ → sum=0, carry=1
    ($s, $c) = $pkg->can('half_adder')->(1, 1);
    is($s, 0, 'half_adder(1,1) sum=0');
    is($c, 1, 'half_adder(1,1) carry=1');
};

# ============================================================================
# Full Adder — truth table exhaustive check
# ============================================================================
#
# 8 input combinations for three single bits (a, b, carry_in).

subtest 'full_adder — truth table' => sub {
    # Collect results for all 8 combinations
    # Expected: sum = (a+b+cin) % 2, cout = (a+b+cin) >= 2
    for my $a (0, 1) {
        for my $b (0, 1) {
            for my $cin (0, 1) {
                my ($s, $c) = $pkg->can('full_adder')->($a, $b, $cin);
                my $total = $a + $b + $cin;
                my $exp_s = $total % 2;
                my $exp_c = ($total >= 2) ? 1 : 0;
                is($s, $exp_s, "full_adder($a,$b,$cin) sum=$exp_s");
                is($c, $exp_c, "full_adder($a,$b,$cin) carry=$exp_c");
            }
        }
    }
};

# ============================================================================
# Ripple Carry Adder
# ============================================================================

subtest 'ripple_carry_adder — basic additions' => sub {
    # 5 + 3 = 8 (4-bit numbers, LSB-first)
    # 5 = 0101 → [1,0,1,0]
    # 3 = 0011 → [1,1,0,0]
    # 8 = 1000 → [0,0,0,1]
    my ($bits_ref, $cout) = $pkg->can('ripple_carry_adder')->([1,0,1,0], [1,1,0,0], 0);
    is($bits_ref, [0,0,0,1], 'ripple_carry_adder: 5+3=8 bits');
    is($cout, 0, 'ripple_carry_adder: 5+3 no carry');

    # 7 + 8 = 15 (4-bit)
    # 7 = 0111 → [1,1,1,0]
    # 8 = 1000 → [0,0,0,1]
    # 15 = 1111 → [1,1,1,1]
    ($bits_ref, $cout) = $pkg->can('ripple_carry_adder')->([1,1,1,0], [0,0,0,1], 0);
    is($bits_ref, [1,1,1,1], 'ripple_carry_adder: 7+8=15 bits');
    is($cout, 0, 'ripple_carry_adder: 7+8 no carry');

    # 15 + 1 = 16 → overflow for 4-bit (result is 0 with carry=1)
    ($bits_ref, $cout) = $pkg->can('ripple_carry_adder')->([1,1,1,1], [1,0,0,0], 0);
    is($bits_ref, [0,0,0,0], 'ripple_carry_adder: 15+1 overflows to 0');
    is($cout, 1, 'ripple_carry_adder: 15+1 carry=1');

    # Carry-in propagation: 0 + 0 + cin=1 = 1
    ($bits_ref, $cout) = $pkg->can('ripple_carry_adder')->([0,0,0,0], [0,0,0,0], 1);
    is($bits_ref, [1,0,0,0], 'ripple_carry_adder: 0+0+cin=1 gives 1');
    is($cout, 0, 'ripple_carry_adder: 0+0+1 no final carry');
};

# ============================================================================
# ALU construction
# ============================================================================

subtest 'ALU construction' => sub {
    my $alu = CodingAdventures::Arithmetic::ALU->new(4);
    ok($alu, 'ALU 4-bit created');
    is($alu->{bits}, 4, 'ALU has correct bit width');

    ok(eval { CodingAdventures::Arithmetic::ALU->new(1); 1 }, 'ALU width=1 is valid');

    # Bit width < 1 should die
    ok(!eval { CodingAdventures::Arithmetic::ALU->new(0); 1 }, 'ALU width=0 dies');
};

# ============================================================================
# ALU ADD operation
# ============================================================================

subtest 'ALU ADD' => sub {
    my $alu = CodingAdventures::Arithmetic::ALU->new(4);

    # 3 + 4 = 7
    # 3 = [1,1,0,0], 4 = [0,0,1,0], 7 = [1,1,1,0]
    my $r = $alu->execute('add', [1,1,0,0], [0,0,1,0]);
    is($r->{value}, [1,1,1,0], 'ALU ADD: 3+4=7');
    is($r->{zero},  0, 'ADD 3+4 zero=0');
    is($r->{carry}, 0, 'ADD 3+4 carry=0');

    # 0 + 0 = 0 → zero flag
    $r = $alu->execute('add', [0,0,0,0], [0,0,0,0]);
    is($r->{zero}, 1, 'ADD 0+0 zero=1');

    # Unsigned overflow: 8 + 8 = 16 → carry=1, value=0
    # 8 = [0,0,0,1] in 4-bit LSB-first
    $r = $alu->execute('add', [0,0,0,1], [0,0,0,1]);
    is($r->{carry}, 1, 'ADD overflow carry=1');
    is($r->{zero},  1, 'ADD overflow zero=1');
};

# ============================================================================
# ALU SUB operation
# ============================================================================

subtest 'ALU SUB' => sub {
    my $alu = CodingAdventures::Arithmetic::ALU->new(4);

    # 5 - 3 = 2
    # 5=[1,0,1,0], 3=[1,1,0,0], 2=[0,1,0,0]
    my $r = $alu->execute('sub', [1,0,1,0], [1,1,0,0]);
    is($r->{value}, [0,1,0,0], 'ALU SUB: 5-3=2');
    is($r->{zero},  0, 'SUB 5-3 zero=0');

    # n - n = 0
    $r = $alu->execute('sub', [1,0,1,0], [1,0,1,0]);
    is($r->{zero}, 1, 'SUB n-n=0 zero=1');
};

# ============================================================================
# ALU AND / OR / XOR / NOT
# ============================================================================

subtest 'ALU bitwise ops' => sub {
    my $alu = CodingAdventures::Arithmetic::ALU->new(4);

    # AND: 0b1010 & 0b1100 = 0b1000
    my $r = $alu->execute('and', [0,1,0,1], [0,0,1,1]);
    is($r->{value}, [0,0,0,1], 'ALU AND: 1010 & 1100 = 1000');

    # OR: 0b1010 | 0b1100 = 0b1110
    $r = $alu->execute('or', [0,1,0,1], [0,0,1,1]);
    is($r->{value}, [0,1,1,1], 'ALU OR: 1010 | 1100 = 1110');

    # XOR: 0b1010 ^ 0b1100 = 0b0110
    $r = $alu->execute('xor', [0,1,0,1], [0,0,1,1]);
    is($r->{value}, [0,1,1,0], 'ALU XOR: 1010 ^ 1100 = 0110');

    # NOT: !0b1010 = 0b0101
    $r = $alu->execute('not', [0,1,0,1], undef);
    is($r->{value}, [1,0,1,0], 'ALU NOT: !1010 = 0101');
};

# ============================================================================
# ALU SHL / SHR
# ============================================================================

subtest 'ALU shift ops' => sub {
    my $alu = CodingAdventures::Arithmetic::ALU->new(4);

    # SHL: [1,0,1,0] → shift left → [0,1,0,1] (MSB shifts into carry)
    # Bits stored LSB-first: [1,0,1,0] = 0101 in MSB-first = 5
    # After SHL: 0101 << 1 = 1010 (LSB-first: [0,1,0,1])
    my $r = $alu->execute('shl', [1,0,1,0], undef);
    is($r->{value}, [0,1,0,1], 'ALU SHL');

    # SHR: [1,0,1,0] → shift right → [0,1,0,0] (LSB=1 shifts into carry)
    $r = $alu->execute('shr', [1,0,1,0], undef);
    is($r->{value}, [0,1,0,0], 'ALU SHR shifts LSB out');
    is($r->{carry}, 1, 'ALU SHR carry=1 for odd number');
};

# ============================================================================
# ALU negative flag
# ============================================================================

subtest 'ALU negative flag' => sub {
    my $alu = CodingAdventures::Arithmetic::ALU->new(4);

    # NOT of zero = all ones → MSB=1 → negative
    my $r = $alu->execute('not', [0,0,0,0], undef);
    is($r->{negative}, 1, 'NOT(0) → negative=1');

    # AND of all zeros
    $r = $alu->execute('and', [0,0,0,0], [1,1,1,1]);
    is($r->{negative}, 0, 'AND zero → negative=0');
};

done_testing;
