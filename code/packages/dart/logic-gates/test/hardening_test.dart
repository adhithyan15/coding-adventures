import 'dart:collection';
import 'dart:convert';

import 'package:coding_adventures_logic_gates/coding_adventures_logic_gates.dart';
import 'package:test/test.dart';

/// A list that lies: reads of `[]` count up, and after [honest] reads every
/// further read returns [lie] instead of the real element.
class LyingList extends ListBase<int> {
  LyingList(this._data, {this.honest = 1 << 30, this.lie = 7});
  final List<int> _data;
  final int honest;
  final int lie;
  int reads = 0;
  final Map<int, int> perIndex = {};

  @override
  int get length => _data.length;
  @override
  set length(int v) => throw UnsupportedError('fixed');

  @override
  int operator [](int i) {
    reads++;
    perIndex[i] = (perIndex[i] ?? 0) + 1;
    return reads > honest ? lie : _data[i];
  }

  @override
  void operator []=(int i, int v) => throw UnsupportedError('fixed');
}

/// Claims a huge length; any element read is a failure.
class HugeList extends ListBase<int> {
  HugeList(this.claimed);
  final int claimed;
  int reads = 0;
  @override
  int get length => claimed;
  @override
  set length(int v) => throw UnsupportedError('fixed');
  @override
  int operator [](int i) {
    reads++;
    return 0;
  }

  @override
  void operator []=(int i, int v) => throw UnsupportedError('fixed');
}

/// Length changes between reads.
class ShrinkingList extends ListBase<int> {
  int lengthReads = 0;
  @override
  int get length => ++lengthReads == 1 ? 4 : 2;
  @override
  set length(int v) => throw UnsupportedError('fixed');
  @override
  int operator [](int i) => i.isEven ? 1 : 0;
  @override
  void operator []=(int i, int v) => throw UnsupportedError('fixed');
}

/// Spies on toString / == / hashCode.
class Spy {
  static int calls = 0;
  @override
  String toString() {
    calls++;
    return 'spy';
  }

  @override
  bool operator ==(Object other) {
    calls++;
    return false;
  }

  @override
  int get hashCode {
    calls++;
    return 0;
  }
}

void main() {
  group('single-read snapshots', () {
    test('muxN never returns a non-bit from a lying list', () {
      for (var honest = 0; honest < 12; honest++) {
        final list = LyingList([0, 1, 0, 1], honest: honest);
        Object? result;
        try {
          result = muxN(list, [1, 1]);
        } on ArgumentError {
          result = null;
        }
        expect(result, anyOf(isNull, 0, 1), reason: 'honest=$honest');
      }
    });

    test('muxN with a lying list that turns bad rejects or stays a bit', () {
      final list = LyingList([0, 1, 0, 1], honest: 4);
      expect(muxN(list, [1, 1]), 1);
      expect(list.reads, 4);
      expect(list.perIndex.values.every((n) => n == 1), isTrue);
    });

    test('lying after the first read is caught as InvalidBitError', () {
      final list = LyingList([0, 1, 0, 1], honest: 1);
      expect(() => muxN(list, [0, 0]), throwsA(isA<InvalidBitError>()));
    });

    test('every list entry point reads each index at most once', () {
      final lists = <String, void Function(LyingList)>{
        'andN': andN,
        'orN': orN,
        'xorN': xorN,
        'encoder': (l) => encoder(l),
        'priorityEncoder': (l) => priorityEncoder(l),
        'decoder': (l) => decoder(l),
        'muxN': (l) => muxN(l, [0, 1]),
        'mux4sel': (l) => mux4(0, 1, 0, 1, l),
        'demux': (l) => demux(1, l),
        'register': (l) => register(l, 1),
      };
      lists.forEach((name, call) {
        final l = LyingList([0, 1, 0, 0]);
        try {
          call(l);
        } on ArgumentError {
          // shape errors are fine; reads are what we count
        }
        expect(l.perIndex.values.every((n) => n <= 1), isTrue, reason: name);
      });
    });

    test('length is read once even if it changes', () {
      final l = ShrinkingList();
      expect(xorN(l), 0);
      expect(l.lengthReads, 1);
    });

    test('caller mutation after the call cannot affect a returned value', () {
      final list = [0, 1, 0, 1];
      final result = muxN(list, [1, 0]);
      list[1] = 0;
      expect(result, 1);
    });

    test('mux8 rejects wrong-size lists without reading elements', () {
      final h = HugeList(1099511627776);
      expect(() => mux8(h, [0, 0, 0]), throwsArgumentError);
      expect(h.reads, 0);
    });
  });

  group('caps before work', () {
    test('huge or lazy lists are rejected without element reads', () {
      final h = HugeList(1099511627776);
      for (final call in <void Function()>[
        () => andN(h),
        () => orN(h),
        () => xorN(h),
        () => muxN(h, [0]),
        () => encoder(h),
        () => priorityEncoder(h),
        () => decoder(h),
        () => demux(1, h),
        () => register(h, 1),
        () => mux4(0, 0, 0, 0, h),
      ]) {
        expect(call, throwsArgumentError);
      }
      expect(h.reads, 0);
    });

    test('negative claimed length is an ArgumentError', () {
      expect(() => xorN(HugeList(-1)), throwsArgumentError);
    });

    test('N-ary and mux boundary at 65536 inputs', () {
      final ok = List<Bit>.filled(65536, 1);
      expect(andN(ok), 1);
      expect(orN(ok), 1);
      expect(xorN(ok), 0);
      final tooMany = List<Bit>.filled(65537, 1);
      expect(() => andN(tooMany), throwsArgumentError);
      expect(() => orN(tooMany), throwsArgumentError);
      expect(() => xorN(tooMany), throwsArgumentError);
      expect(() => encoder(tooMany), throwsArgumentError);
      expect(() => priorityEncoder(tooMany), throwsArgumentError);
      expect(muxN(List<Bit>.filled(65536, 1), List<Bit>.filled(16, 0)), 1);
      expect(
        () => muxN(List<Bit>.filled(131072, 1), List<Bit>.filled(17, 0)),
        throwsArgumentError,
      );
    });

    test('decoder at 16/17/63/64 select bits', () {
      expect(decoder(List<Bit>.filled(16, 0)).length, 65536);
      for (final n in [17, 63, 64, 65]) {
        expect(
          () => decoder(List<Bit>.filled(n, 0)),
          throwsA(isA<ArgumentError>()
              .having((e) => e is RangeError, 'range', isFalse)),
          reason: '$n',
        );
      }
    });

    test('demux select cap and outputCount cap', () {
      expect(demux(1, List<Bit>.filled(16, 0)).length, 65536);
      for (final n in [17, 63, 64]) {
        expect(
          () => demux(1, List<Bit>.filled(n, 0)),
          throwsArgumentError,
        );
      }
      expect(() => demux(1, [0], 4503599627370496), throwsArgumentError);
    });

    test('register / shiftRegister / counter widths', () {
      expect(register(List<Bit>.filled(65536, 0), 1).$1.length, 65536);
      expect(
        () => register(List<Bit>.filled(65537, 0), 1),
        throwsArgumentError,
      );
      expect(
        () => shiftRegister(0, 1, width: 65537),
        throwsArgumentError,
      );
      expect(
          () => shiftRegister(0, 1, width: 1099511627776), throwsArgumentError);
      expect(counter(1, width: 64).$1.length, 64);
      expect(() => counter(1, width: 65), throwsArgumentError);
      expect(() => counter(1, width: 1099511627776), throwsArgumentError);
      expect(
        () => CounterState(value: HugeList(1099511627776), ffState: const []),
        throwsArgumentError,
      );
      expect(
        () => register([0, 1], 1,
            state: List.filled(70000, const FlipFlopState())),
        throwsArgumentError,
      );
    });
  });

  group('no caller code in errors', () {
    test('spy objects smuggled via dynamic lists are never stringified', () {
      Spy.calls = 0;
      final spyList = <Object?>[Spy(), 1];
      expect(
        () => andN(spyList.cast<int>().toList(growable: false)),
        anyOf(throwsA(isA<InvalidBitError>()), throwsA(isA<TypeError>())),
      );
      expect(Spy.calls, 0);
    });

    test('InvalidBitError sanitises non-int values', () {
      Spy.calls = 0;
      final e = InvalidBitError(Spy(), 'a');
      expect(e.toString(), contains('non-integer value'));
      expect(e.invalidValue, 'non-integer value');
      expect(Spy.calls, 0);
      expect(() => validateBit(Spy(), 'x'), throwsA(isA<InvalidBitError>()));
      expect(() => validateBit(1.5, 'x'), throwsA(isA<InvalidBitError>()));
      expect(() => validateBit('1', 'x'), throwsA(isA<InvalidBitError>()));
      expect(() => validateBit(null, 'x'), throwsA(isA<InvalidBitError>()));
      expect(Spy.calls, 0);
    });

    test('shape errors never embed the caller list', () {
      try {
        encoder([1, 1]);
        fail('expected');
      } on ArgumentError catch (e) {
        expect(e.toString(), isNot(contains('[1, 1]')));
      }
    });

    test('error names are built from index, not from caller data', () {
      try {
        andN([1, 0, 3]);
        fail('expected');
      } on InvalidBitError catch (e) {
        expect(e.name, 'inputs[2]');
        expect(e.invalidValue, 3);
      }
    });
  });

  group(
      'web canonical outputs (-0.0 / 1.0)',
      skip: identical(1, 1.0) ? false : 'web-only',
      webCanonicalTests);

  group('additional hostile shapes', () {
    test('xorN / andN / orN with a one-element hostile list', () {
      expect(() => xorN(LyingList([1], honest: 0)),
          throwsA(isA<InvalidBitError>()));
      expect(xorN(LyingList([1])), 1);
      expect(() => andN(LyingList([1], honest: 0)), throwsArgumentError);
    });

    test('a list whose length grows cannot loop forever', () {
      final g = GrowingList();
      expect(xorN(g), 0);
      expect(g.lengthReads, 1);
      expect(() => andN(GrowingList()), returnsNormally);
    });

    test('decoder / demux reject 45 and 62 select bits before allocating', () {
      for (final n in [45, 62]) {
        expect(() => decoder(List<Bit>.filled(n, 0)), throwsArgumentError);
        expect(() => demux(1, List<Bit>.filled(n, 0)), throwsArgumentError);
      }
    });

    test('InvalidBitError is an Error, not an Exception', () {
      final e = InvalidBitError(2, 'a');
      expect(e, isA<Error>());
      expect(e, isNot(isA<Exception>()));
      expect(e, isA<ArgumentError>());
    });

    test('shape errors retain no caller reference', () {
      try {
        andN([1]);
        fail('expected');
      } on ArgumentError catch (e) {
        expect(e.invalidValue, isA<int>());
      }
    });

    test('List<dynamic> from jsonDecode fails at the call boundary', () {
      final Object? decoded = jsonDecode('[1, 0]');
      expect(() => Function.apply(andN, [decoded]), throwsA(isA<TypeError>()));
      final cast = (decoded! as List<dynamic>).cast<int>().toList();
      expect(andN(cast), 0);
      final bad = (jsonDecode('[1, "x"]') as List<dynamic>).cast<int>();
      expect(() => andN(bad.toList()), throwsA(isA<TypeError>()));
    });
  });
}

bool _isNegZero(Object? x) => x is num && x == 0 && (1 / x).isNegative;

void webCanonicalTests() {
  // On dart2js `-0.0` and `1.0` pass as bits; outputs must be fresh canonical
  // ints (never the caller's -0.0). VM cannot even express these as `int`.
  // Lazily typed as int: only evaluated inside web-only tests.
  int nzf() => (-0.0 as dynamic) as int;
  int onef() => (1.0 as dynamic) as int;
  test('srLatch returns canonical 0 for -0.0 state', () {
    final nz = nzf(), one = onef();
    final r = srLatch(0, 0, q: nz, qBar: one);
    expect(_isNegZero(r.$1), isFalse);
    expect(r, (0, 1));
  });
  test('latches, registers, counters, demux, triState are canonical', () {
    final nz = nzf(), one = onef();
    final d = dLatch(nz, 0, q: nz, qBar: one);
    expect(_isNegZero(d.$1), isFalse);
    final reg = register([nz, one], 0);
    expect(reg.$1.any(_isNegZero), isFalse);
    final out = Function.apply(demux, [
      nz,
      [0]
    ]) as List<Bit>;
    expect(out.any(_isNegZero), isFalse);
    expect(_isNegZero(triState(nz, one)), isFalse);
    expect(
        _isNegZero(Function.apply(andN, [
          [nz, one]
        ])),
        isFalse);
    final c = Function.apply(counter, [
      0
    ], {
      #state: CounterState(value: [
        nz,
        nz
      ], ffState: const [
        FlipFlopState(),
        FlipFlopState(),
      ]),
      #width: 2,
    }) as (List<Bit>, CounterState);
    expect(c.$2.value.any(_isNegZero), isFalse);
  });
}

/// Reports a growing length on every read after the first.
class GrowingList extends ListBase<int> {
  int lengthReads = 0;
  @override
  int get length => 4 + lengthReads++;
  @override
  set length(int v) => throw UnsupportedError('fixed');
  @override
  int operator [](int i) => i.isEven ? 1 : 0;
  @override
  void operator []=(int i, int v) => throw UnsupportedError('fixed');
}
