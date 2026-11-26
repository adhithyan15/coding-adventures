/// Property tests for `coding_adventures_block_ram`, using the
/// in-lane `pbt` package: every claim below is checked over hundreds of
/// seeded pseudo-random cases and reproduces exactly.
///
/// The model strategy is a *mirror memory*: a plain
/// `List<List<int>>` that shadows the RAM through each generated tick
/// sequence. Whenever the reference semantics are deterministic (write
/// / read / read-mode outputs), the mirror predicts them bit for bit.
library;

import 'dart:typed_data';

import 'package:coding_adventures_block_ram/block_ram.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

Gen<List<int>> wordGen(int width) => Gen.list(width, width, smallInt(0, 1));

bool wordsEqual(List<int> a, List<int> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

/// Drives one full 0→1 cycle and returns the rising-edge output.
List<int> cycle(SinglePortRAM ram, int address, List<int> data, int we) {
  ram.tick(0, address, data, we);
  return ram.tick(1, address, data, we);
}

void main() {
  group('properties', () {
    test('single-port write-then-read round-trips', () {
      check(
        'write-read',
        checkProperty<(int, int, List<(int, List<int>)>)>(
          BigInt.from(2101),
          PbtOptions(numTests: 300, seed: BigInt.from(2101)),
          (rng) {
            final depth = smallInt(1, 8).sample(rng);
            final width = smallInt(1, 8).sample(rng);
            final writes = Gen.list(
              1,
              12,
              Gen.tuple2(smallInt(0, depth - 1), wordGen(width)),
            ).sample(rng);
            return (depth, width, writes);
          },
          (input) {
            final (depth, width, writes) = input;
            final ram = SinglePortRAM(depth, width, ReadMode.readFirst);
            final mirror = List.generate(depth, (_) => List.filled(width, 0));
            for (final (address, data) in writes) {
              cycle(ram, address, data, 1);
              mirror[address] = List.of(data);
            }
            for (var address = 0; address < depth; address++) {
              final out = cycle(ram, address, List.filled(width, 0), 0);
              if (!wordsEqual(out, mirror[address])) return false;
            }
            return true;
          },
        ),
      );
    });

    test('read modes predict colliding write outputs', () {
      check(
        'read-modes',
        checkProperty<(ReadMode, List<int>, List<int>)>(
          BigInt.from(2102),
          PbtOptions(numTests: 300, seed: BigInt.from(2102)),
          (rng) {
            final mode = Gen.oneOf(ReadMode.values.map(Gen.constant).toList())
                .sample(rng);
            final first = wordGen(4).sample(rng);
            final second = wordGen(4).sample(rng);
            return (mode, first, second);
          },
          (input) {
            final (mode, first, second) = input;
            final ram = SinglePortRAM(2, 4, mode);
            cycle(ram, 0, first, 1);
            final out = cycle(ram, 0, second, 1);
            final expected = switch (mode) {
              ReadMode.readFirst => first,
              ReadMode.writeFirst => second,
              ReadMode.noChange => [0, 0, 0, 0],
            };
            if (!wordsEqual(out, expected)) return false;
            // Every mode stores the write regardless of what it shows.
            final back = cycle(ram, 0, [0, 0, 0, 0], 0);
            return wordsEqual(back, second);
          },
        ),
      );
    });

    test('only rising edges mutate state', () {
      check(
        'edges-only',
        checkProperty<List<(int, int, List<int>, int)>>(
          BigInt.from(2103),
          PbtOptions(numTests: 300, seed: BigInt.from(2103)),
          (rng) => Gen.list(
            1,
            20,
            Gen.tuple2(
              Gen.tuple2(smallInt(0, 1), smallInt(0, 3)),
              Gen.tuple2(wordGen(3), smallInt(0, 1)),
            ).map((t) => (t.$1.$1, t.$1.$2, t.$2.$1, t.$2.$2)),
          ).sample(rng),
          (ticks) {
            final ram = SinglePortRAM(4, 3, ReadMode.readFirst);
            final mirror = List.generate(4, (_) => List.filled(3, 0));
            var prevClock = 0;
            for (final (clock, address, data, we) in ticks) {
              ram.tick(clock, address, data, we);
              if (clock == 1 && prevClock == 0 && we == 1) {
                mirror[address] = List.of(data);
              }
              prevClock = clock;
            }
            final dump = ram.dump();
            for (var row = 0; row < 4; row++) {
              if (!wordsEqual(dump[row], mirror[row])) return false;
            }
            return true;
          },
        ),
      );
    });

    test('dual ports stay independent on disjoint addresses', () {
      check(
        'port-independence',
        checkProperty<(List<int>, List<int>)>(
          BigInt.from(2104),
          PbtOptions(numTests: 300, seed: BigInt.from(2104)),
          (rng) {
            final a = wordGen(4).sample(rng);
            final b = wordGen(4).sample(rng);
            return (a, b);
          },
          (input) {
            final (dataA, dataB) = input;
            final ram =
                DualPortRAM(4, 4, ReadMode.readFirst, ReadMode.readFirst);
            ram.tick(0, 0, dataA, 1, 1, dataB, 1);
            ram.tick(1, 0, dataA, 1, 1, dataB, 1);
            ram.tick(0, 0, [0, 0, 0, 0], 0, 1, [0, 0, 0, 0], 0);
            final (outA, outB) =
                ram.tick(1, 0, [0, 0, 0, 0], 0, 1, [0, 0, 0, 0], 0);
            return wordsEqual(outA, dataA) && wordsEqual(outB, dataB);
          },
        ),
      );
    });

    test('collisions fire exactly on same-address edge writes', () {
      check(
        'collision-exactness',
        checkProperty<(int, int, int, int, int)>(
          BigInt.from(2105),
          PbtOptions(numTests: 300, seed: BigInt.from(2105)),
          (rng) {
            final clock = smallInt(0, 1).sample(rng);
            final addressA = smallInt(0, 3).sample(rng);
            final addressB = smallInt(0, 3).sample(rng);
            final weA = smallInt(0, 1).sample(rng);
            final weB = smallInt(0, 1).sample(rng);
            return (clock, addressA, addressB, weA, weB);
          },
          (input) {
            final (clock, addressA, addressB, weA, weB) = input;
            final ram =
                DualPortRAM(4, 4, ReadMode.readFirst, ReadMode.readFirst);
            // Fresh RAM: prevClock is 0, so clock==1 means rising edge.
            final expectCollision =
                clock == 1 && weA == 1 && weB == 1 && addressA == addressB;
            final zeros = [0, 0, 0, 0];
            try {
              ram.tick(clock, addressA, zeros, weA, addressB, zeros, weB);
            } on WriteCollisionError catch (e) {
              return expectCollision && e.address == addressA;
            }
            return !expectCollision;
          },
        ),
      );
    });

    test('BRAM keeps its bit budget across reconfigures', () {
      check(
        'bram-budget',
        checkProperty<(int, List<int>)>(
          BigInt.from(2106),
          PbtOptions(numTests: 300, seed: BigInt.from(2106)),
          (rng) {
            // Total bits drawn from a highly composite menu so legal
            // widths are plentiful.
            final total =
                Gen.oneOf([64, 128, 256, 512, 1024].map(Gen.constant).toList())
                    .sample(rng);
            final divisors = <int>[];
            for (var w = 1; w <= total; w++) {
              if (total % w == 0) divisors.add(w);
            }
            final widths =
                Gen.list(1, 4, Gen.oneOf(divisors.map(Gen.constant).toList()))
                    .sample(rng);
            return (total, widths);
          },
          (input) {
            final (total, widths) = input;
            final bram = ConfigurableBRAM(total, widths.first);
            for (final width in widths) {
              bram.reconfigure(width);
              if (bram.totalBits() != total) return false;
              if (bram.depth() * bram.width() != total) return false;
              // Round-trip row 0 at the new aspect ratio.
              final data = List.generate(width, (i) => i % 2);
              bram.tickA(0, 0, data, 1);
              bram.tickA(1, 0, data, 1);
              bram.tickA(0, 0, List.filled(width, 0), 0);
              final out = bram.tickA(1, 0, List.filled(width, 0), 0);
              if (!wordsEqual(out, data)) return false;
            }
            return true;
          },
        ),
      );
    });

    test('outputs stay in the bit domain', () {
      check(
        'bit-domain',
        checkProperty<Uint8List>(
          BigInt.from(2107),
          PbtOptions(numTests: 300, seed: BigInt.from(2107)),
          (rng) => Gen.bytes(0, 64).sample(rng),
          (input) {
            bool isBitWord(List<int> word) =>
                word.every((b) => b == 0 || b == 1);
            final ram = SinglePortRAM(4, 4, ReadMode.readFirst);
            final dual =
                DualPortRAM(4, 4, ReadMode.readFirst, ReadMode.readFirst);
            var i = 0;
            while (i + 2 < input.length) {
              final clock = input[i] & 1;
              final address = input[i + 1] & 3;
              final half = input[i + 2];
              final word = [
                half & 1,
                (half >> 1) & 1,
                (half >> 2) & 1,
                (half >> 3) & 1
              ];
              final we = (half >> 4) & 1;
              if (!isBitWord(ram.tick(clock, address, word, we))) {
                return false;
              }
              try {
                final (outA, outB) =
                    dual.tick(clock, address, word, we, address, word, we);
                if (!isBitWord(outA) || !isBitWord(outB)) return false;
              } on WriteCollisionError {
                // Documented vocabulary; the single port above still ran.
              }
              i += 3;
            }
            return true;
          },
        ),
      );
    });

    test('invalid geometry throws ArgumentError', () {
      check(
        'error-matrix',
        checkProperty<(int, int)>(
          BigInt.from(2108),
          PbtOptions(numTests: 200, seed: BigInt.from(2108)),
          (rng) {
            final depth = smallInt(-2, 4).sample(rng);
            final width = smallInt(-2, 4).sample(rng);
            return (depth, width);
          },
          (input) {
            final (depth, width) = input;
            final valid = depth > 0 && width > 0;
            var threw = false;
            try {
              SinglePortRAM(depth, width, ReadMode.readFirst);
            } on ArgumentError {
              threw = true;
            }
            if (threw == valid) return false;
            threw = false;
            try {
              SramArray(depth, width);
            } on ArgumentError {
              threw = true;
            }
            return threw != valid;
          },
        ),
      );
    });
  });
}
