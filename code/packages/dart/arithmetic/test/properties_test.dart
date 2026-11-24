/// Property tests for `coding_adventures_arithmetic`, using the in-lane
/// `pbt` package: every operation is checked against an independent
/// [BigInt] oracle model (values, carries, and signed-overflow ranges),
/// plus commutativity, identities, and Not's blindness to `b`.
/// Fixed seeds throughout.
library;

import 'package:coding_adventures_arithmetic/coding_adventures_arithmetic.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

List<int> _toBits(BigInt value, int width) =>
    [for (var i = 0; i < width; i++) (value >> i).toInt() & 1];

BigInt _fromBits(List<int> bits) {
  var acc = BigInt.zero;
  for (var i = 0; i < bits.length; i++) {
    if (bits[i] == 1) acc |= BigInt.one << i;
  }
  return acc;
}

/// Signed two's-complement reading of [value] at [width] bits.
BigInt _signed(BigInt value, int width) {
  final half = BigInt.one << (width - 1);
  return value >= half ? value - (BigInt.one << width) : value;
}

Gen<int> _width() =>
    Gen.integer(BigInt.one, BigInt.from(12)).map((n) => n.toInt());

Gen<BigInt> _value(int width) =>
    Gen.integer(BigInt.zero, (BigInt.one << width) - BigInt.one);

/// (width, a, b) with values in range.
Gen<(int, BigInt, BigInt)> _addCase() => _width().flatMap(
    (w) => Gen.tuple2(_value(w), _value(w)).map((t) => (w, t.$1, t.$2)));

void _check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

void main() {
  group('properties', () {
    test('half and full adders match integer sums', () {
      final gen = Gen.tuple3(Gen.u8(), Gen.u8(), Gen.u8());
      final verdict = checkProperty<(int, int, int)>(
        BigInt.from(121),
        PbtOptions(numTests: 200, seed: BigInt.from(121)),
        (rng) => gen.sample(rng),
        (input) {
          final a = input.$1 % 2;
          final b = input.$2 % 2;
          final cin = input.$3 % 2;
          final (hs, hc) = halfAdder(a, b);
          final (fs, fc) = fullAdder(a, b, cin);
          return hs + 2 * hc == a + b && fs + 2 * fc == a + b + cin;
        },
      );
      _check('adder-models', verdict);
    });

    test('ripple matches BigInt addition with carry and overflow', () {
      final gen = _addCase();
      final verdict = checkProperty<(int, BigInt, BigInt)>(
        BigInt.from(122),
        PbtOptions(numTests: 300, seed: BigInt.from(122)),
        (rng) => gen.sample(rng),
        (input) {
          final w = input.$1;
          final a = input.$2;
          final b = input.$3;
          final r = rippleCarryAdder(_toBits(a, w), _toBits(b, w));
          final mod = BigInt.one << w;
          final wantSum = (a + b) % mod;
          final wantCarry = (a + b) >= mod ? 1 : 0;
          final lo = -(BigInt.one << (w - 1));
          final hi = (BigInt.one << (w - 1)) - BigInt.one;
          final signed = _signed(a, w) + _signed(b, w);
          final wantOverflow = signed < lo || signed > hi;
          return _fromBits(r.sum) == wantSum &&
              r.carryOut == wantCarry &&
              r.overflow == wantOverflow &&
              r.sum.length == w;
        },
      );
      _check('ripple-model', verdict);
    });

    test('with-carry adds the carry-in', () {
      final gen = _addCase().flatMap((t) => Gen.integer(BigInt.zero, BigInt.one)
          .map((cin) => (t.$1, t.$2, t.$3, cin.toInt())));
      final verdict = checkProperty<(int, BigInt, BigInt, int)>(
        BigInt.from(123),
        PbtOptions(numTests: 300, seed: BigInt.from(123)),
        (rng) => gen.sample(rng),
        (input) {
          final w = input.$1;
          final a = input.$2;
          final b = input.$3;
          final cin = input.$4;
          final r =
              rippleCarryAdderWithCarry(_toBits(a, w), _toBits(b, w), cin);
          final mod = BigInt.one << w;
          return _fromBits(r.sum) == (a + b + BigInt.from(cin)) % mod &&
              r.carryOut == ((a + b + BigInt.from(cin)) >= mod ? 1 : 0) &&
              r.sum.length == w;
        },
      );
      _check('with-carry-model', verdict);
    });

    test('addition commutes and zero is the identity', () {
      final gen = _addCase();
      final verdict = checkProperty<(int, BigInt, BigInt)>(
        BigInt.from(124),
        PbtOptions(numTests: 200, seed: BigInt.from(124)),
        (rng) => gen.sample(rng),
        (input) {
          final w = input.$1;
          final a = _toBits(input.$2, w);
          final b = _toBits(input.$3, w);
          final ab = rippleCarryAdder(a, b);
          final ba = rippleCarryAdder(b, a);
          final id = rippleCarryAdder(a, List<int>.filled(w, 0));
          return ab == ba &&
              _fromBits(id.sum) == _fromBits(a) &&
              id.carryOut == 0 &&
              !id.overflow;
        },
      );
      _check('add-laws', verdict);
    });

    test('alu add/sub match the BigInt model with flags', () {
      final gen = _addCase();
      final verdict = checkProperty<(int, BigInt, BigInt)>(
        BigInt.from(125),
        PbtOptions(numTests: 300, seed: BigInt.from(125)),
        (rng) => gen.sample(rng),
        (input) {
          final w = input.$1;
          final a = input.$2;
          final b = input.$3;
          final mod = BigInt.one << w;
          final add = alu(_toBits(a, w), _toBits(b, w), AluOp.add);
          final sub = alu(_toBits(a, w), _toBits(b, w), AluOp.sub);
          final addVal = _fromBits(add.result);
          final subVal = _fromBits(sub.result);
          final lo = -(BigInt.one << (w - 1));
          final hi = (BigInt.one << (w - 1)) - BigInt.one;
          final addSigned = _signed(a, w) + _signed(b, w);
          final subSigned = _signed(a, w) - _signed(b, w);
          return addVal == (a + b) % mod &&
              add.carry == (a + b >= mod) &&
              add.overflow == (addSigned < lo || addSigned > hi) &&
              add.zero == (addVal == BigInt.zero) &&
              subVal == (a - b) % mod &&
              sub.carry == (a >= b) &&
              sub.overflow == (subSigned < lo || subSigned > hi) &&
              sub.zero == (subVal == BigInt.zero) &&
              add.result.length == w &&
              sub.result.length == w;
        },
      );
      _check('alu-arith-model', verdict);
    });

    test('alu logic ops match BigInt bitwise ops', () {
      final gen = _addCase();
      final verdict = checkProperty<(int, BigInt, BigInt)>(
        BigInt.from(126),
        PbtOptions(numTests: 300, seed: BigInt.from(126)),
        (rng) => gen.sample(rng),
        (input) {
          final w = input.$1;
          final a = input.$2;
          final b = input.$3;
          final mask = (BigInt.one << w) - BigInt.one;
          final and = alu(_toBits(a, w), _toBits(b, w), AluOp.and);
          final or = alu(_toBits(a, w), _toBits(b, w), AluOp.or);
          final xor = alu(_toBits(a, w), _toBits(b, w), AluOp.xor);
          final not = alu(_toBits(a, w), _toBits(b, w), AluOp.not);
          return _fromBits(and.result) == (a & b) &&
              _fromBits(or.result) == (a | b) &&
              _fromBits(xor.result) == (a ^ b) &&
              _fromBits(not.result) == (mask ^ a) &&
              !and.carry &&
              !or.carry &&
              !xor.carry &&
              !not.carry &&
              !and.overflow &&
              !or.overflow &&
              !xor.overflow &&
              !not.overflow &&
              and.zero == ((a & b) == BigInt.zero) &&
              not.result.length == w;
        },
      );
      _check('alu-logic-model', verdict);
    });

    test('not ignores hostile b inputs', () {
      final gen = Gen.tuple2(
          _width().flatMap((w) =>
              Gen.tuple2(Gen.constant(w), _value(w).map((v) => _toBits(v, w)))),
          Gen.list(0, 10, Gen.u8()));
      final verdict = checkProperty<((int, List<int>), List<int>)>(
        BigInt.from(127),
        PbtOptions(numTests: 300, seed: BigInt.from(127)),
        (rng) => gen.sample(rng),
        (input) {
          final a = input.$1.$2;
          // b may hold non-bits and any length: Not never reads it.
          final hostile = alu(a, input.$2, AluOp.not);
          final clean = alu(a, List<int>.filled(a.length, 0), AluOp.not);
          return hostile == clean;
        },
      );
      _check('not-blind', verdict);
    });
  });
}
