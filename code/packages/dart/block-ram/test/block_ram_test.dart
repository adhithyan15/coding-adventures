/// Unit tests for `coding_adventures_block_ram`: the 14 reference
/// tests plus the extras the reference leaves uncovered — the full
/// `ArgumentError` matrix, `NoChange` mode, collision gating on edges,
/// same-address cross-port visibility, port B, reconfigure clearing,
/// and value semantics.
library;

import 'package:coding_adventures_block_ram/block_ram.dart';
import 'package:test/test.dart';

/// Drives a full 0→1 clock cycle, returning the rising-edge output.
List<int> cycle(SinglePortRAM ram, int address, List<int> data, int we) {
  ram.tick(0, address, data, we);
  return ram.tick(1, address, data, we);
}

/// Drives a full cycle on both dual ports.
(List<int>, List<int>) cycleDual(DualPortRAM ram, int addressA, List<int> dataA,
    int weA, int addressB, List<int> dataB, int weB) {
  ram.tick(0, addressA, dataA, weA, addressB, dataB, weB);
  return ram.tick(1, addressA, dataA, weA, addressB, dataB, weB);
}

void main() {
  group('SramCell', () {
    test('initial value is zero', () {
      expect(SramCell().value(), 0);
    });

    test('write and read when selected', () {
      final cell = SramCell();
      cell.write(1, 1);
      expect(cell.read(1), 1);
      expect(cell.value(), 1);
    });

    test('unselected access is inert and silent', () {
      final cell = SramCell();
      cell.write(1, 1);
      expect(cell.read(0), isNull);
      cell.write(0, 0);
      expect(cell.value(), 1);
    });

    test('non-bit lines throw ArgumentError, always', () {
      final cell = SramCell();
      expect(() => cell.write(2, 1), throwsArgumentError);
      expect(() => cell.write(1, 7), throwsArgumentError);
      expect(() => cell.write(-1, 0), throwsArgumentError);
      expect(() => cell.read(5), throwsArgumentError);
      // The rejections left no trace: the cell still holds reset state.
      expect(cell.value(), 0);
    });

    test('cells compare by value', () {
      final a = SramCell();
      final b = SramCell();
      expect(a, b);
      expect(a.hashCode, b.hashCode);
      expect(a.toString(), 'SramCell(0)');
      b.write(1, 1);
      expect(a, isNot(b));
      expect(a, isNot('not a cell'));
    });
  });

  group('SramArray', () {
    test('read and write rows', () {
      final arr = SramArray(4, 8);
      arr.write(0, [1, 0, 1, 0, 0, 1, 0, 1]);
      expect(arr.read(0), [1, 0, 1, 0, 0, 1, 0, 1]);
      expect(arr.read(1), [0, 0, 0, 0, 0, 0, 0, 0]);
    });

    test('shape reports dimensions', () {
      expect(SramArray(2, 4).shape(), (2, 4));
    });

    test('rows are independent', () {
      final arr = SramArray(2, 2);
      arr.write(0, [1, 1]);
      arr.write(1, [1, 0]);
      expect(arr.read(0), [1, 1]);
      expect(arr.read(1), [1, 0]);
    });

    test('zero dimensions throw ArgumentError', () {
      expect(() => SramArray(0, 4), throwsArgumentError);
      expect(() => SramArray(4, 0), throwsArgumentError);
    });

    test('out-of-range rows throw ArgumentError', () {
      final arr = SramArray(2, 2);
      expect(() => arr.read(2), throwsArgumentError);
      expect(() => arr.read(-1), throwsArgumentError);
      expect(() => arr.write(2, [0, 0]), throwsArgumentError);
    });

    test('wrong-length data throws ArgumentError', () {
      final arr = SramArray(2, 2);
      expect(() => arr.write(0, [0]), throwsArgumentError);
      expect(() => arr.write(0, [0, 0, 0]), throwsArgumentError);
    });

    test('garbage data bits throw ArgumentError', () {
      final arr = SramArray(2, 2);
      expect(() => arr.write(0, [0, 5]), throwsArgumentError);
      expect(() => arr.write(0, [-1, 0]), throwsArgumentError);
      expect(arr.read(0), [0, 0]);
    });
  });

  group('SinglePortRAM', () {
    test('write and read back', () {
      final ram = SinglePortRAM(4, 8, ReadMode.readFirst);
      cycle(ram, 0, [1, 0, 1, 0, 1, 0, 1, 0], 1);
      expect(
          cycle(ram, 0, [0, 0, 0, 0, 0, 0, 0, 0], 0), [1, 0, 1, 0, 1, 0, 1, 0]);
    });

    test('read-first returns the old value', () {
      final ram = SinglePortRAM(4, 4, ReadMode.readFirst);
      cycle(ram, 0, [1, 0, 1, 0], 1);
      expect(cycle(ram, 0, [0, 1, 0, 1], 1), [1, 0, 1, 0]);
    });

    test('write-first returns the new value', () {
      final ram = SinglePortRAM(4, 4, ReadMode.writeFirst);
      cycle(ram, 0, [1, 0, 1, 0], 1);
      expect(cycle(ram, 0, [0, 1, 0, 1], 1), [0, 1, 0, 1]);
    });

    test('no-change holds the output during writes', () {
      final ram = SinglePortRAM(4, 4, ReadMode.noChange);
      cycle(ram, 0, [1, 0, 1, 0], 1);
      // The write lands, but the output still shows pre-write zeros.
      expect(cycle(ram, 0, [0, 1, 0, 1], 1), [0, 0, 0, 0]);
      // A read picks up the written value.
      expect(cycle(ram, 0, [0, 0, 0, 0], 0), [0, 1, 0, 1]);
    });

    test('non-edges repeat the output without operating', () {
      final ram = SinglePortRAM(4, 4, ReadMode.readFirst);
      // Prime the clock history with one clean read edge.
      expect(ram.tick(0, 0, [0, 0, 0, 0], 0), [0, 0, 0, 0]);
      expect(ram.tick(1, 0, [0, 0, 0, 0], 0), [0, 0, 0, 0]);
      // Clock stuck at 1: never an edge, writes never land.
      expect(ram.tick(1, 0, [1, 1, 1, 1], 1), [0, 0, 0, 0]);
      expect(ram.tick(1, 0, [1, 1, 1, 1], 1), [0, 0, 0, 0]);
      expect(cycle(ram, 0, [0, 0, 0, 0], 0), [0, 0, 0, 0]);
    });

    test('falling edge is not an edge', () {
      final ram = SinglePortRAM(4, 4, ReadMode.readFirst);
      ram.tick(0, 0, [1, 1, 1, 1], 1);
      ram.tick(1, 0, [1, 1, 1, 1], 1);
      // Falling edge: output repeats, no write of the new data.
      expect(ram.tick(0, 0, [0, 0, 0, 0], 1), [0, 0, 0, 0]);
      expect(cycle(ram, 0, [0, 0, 0, 0], 0), [1, 1, 1, 1]);
    });

    test('dimensions and dump', () {
      final ram = SinglePortRAM(2, 2, ReadMode.readFirst);
      expect(ram.depth(), 2);
      expect(ram.width(), 2);
      cycle(ram, 1, [1, 0], 1);
      expect(ram.dump(), [
        [0, 0],
        [1, 0]
      ]);
    });

    test('bad construction throws ArgumentError', () {
      expect(
          () => SinglePortRAM(0, 4, ReadMode.readFirst), throwsArgumentError);
      expect(
          () => SinglePortRAM(4, 0, ReadMode.readFirst), throwsArgumentError);
    });

    test('bad ticks throw ArgumentError', () {
      final ram = SinglePortRAM(2, 2, ReadMode.readFirst);
      expect(() => ram.tick(0, 2, [0, 0], 0), throwsArgumentError);
      expect(() => ram.tick(0, 0, [0], 0), throwsArgumentError);
    });

    test('non-bit clock and write-enable throw ArgumentError', () {
      final ram = SinglePortRAM(2, 2, ReadMode.readFirst);
      expect(() => ram.tick(2, 0, [0, 0], 0), throwsArgumentError);
      expect(() => ram.tick(0, 0, [0, 0], 3), throwsArgumentError);
    });

    test('write-path garbage rejected, read-path garbage ignored', () {
      final ram = SinglePortRAM(2, 2, ReadMode.readFirst);
      // Write path validates contents (via the array), like the
      // reference: garbage with we=1 throws.
      expect(() => cycle(ram, 0, [0, 9], 1), throwsArgumentError);
      // Read path never inspects data_in (reference parity): garbage
      // with we=0 reads cleanly and the row stays zeros.
      expect(cycle(ram, 0, [7, 7], 0), [0, 0]);
    });
  });

  group('DualPortRAM', () {
    test('independent read and write across ports', () {
      final ram = DualPortRAM(4, 4, ReadMode.readFirst, ReadMode.readFirst);
      cycleDual(ram, 0, [1, 1, 0, 0], 1, 1, [0, 0, 0, 0], 0);
      final (_, outB) = cycleDual(ram, 0, [0, 0, 0, 0], 0, 0, [0, 0, 0, 0], 0);
      expect(outB, [1, 1, 0, 0]);
    });

    test('same-address double write collides', () {
      final ram = DualPortRAM(4, 4, ReadMode.readFirst, ReadMode.readFirst);
      expect(() => cycleDual(ram, 0, [1, 1, 1, 1], 1, 0, [0, 0, 0, 0], 1),
          throwsA(const WriteCollisionError(address: 0)));
    });

    test('collision needs a rising edge', () {
      final ram = DualPortRAM(4, 4, ReadMode.readFirst, ReadMode.readFirst);
      // Both writing the same address off-edge: no error, lasts repeat.
      final (outA, outB) = ram.tick(0, 0, [1, 1, 1, 1], 1, 0, [0, 0, 0, 0], 1);
      expect(outA, [0, 0, 0, 0]);
      expect(outB, [0, 0, 0, 0]);
    });

    test('write plus read on one address is not a collision', () {
      final ram = DualPortRAM(4, 4, ReadMode.readFirst, ReadMode.readFirst);
      // Port A writes while B reads the same address: ports process in
      // order (A first), so B observes A's write.
      final (outA, outB) =
          cycleDual(ram, 0, [1, 0, 1, 0], 1, 0, [0, 0, 0, 0], 0);
      expect(outA, [0, 0, 0, 0]); // read-first: old zeros
      expect(outB, [1, 0, 1, 0]); // sees A's write
    });

    test('ports honor their own read modes', () {
      final ram = DualPortRAM(4, 4, ReadMode.writeFirst, ReadMode.noChange);
      final (outA, outB) =
          cycleDual(ram, 0, [1, 1, 0, 0], 1, 1, [0, 0, 1, 1], 1);
      expect(outA, [1, 1, 0, 0]); // write-first: new
      expect(outB, [0, 0, 0, 0]); // no-change: held
    });

    test('dimensions', () {
      final ram = DualPortRAM(8, 2, ReadMode.readFirst, ReadMode.readFirst);
      expect(ram.depth(), 8);
      expect(ram.width(), 2);
    });

    test('bad construction throws ArgumentError', () {
      expect(() => DualPortRAM(0, 4, ReadMode.readFirst, ReadMode.readFirst),
          throwsArgumentError);
      expect(() => DualPortRAM(4, 0, ReadMode.readFirst, ReadMode.readFirst),
          throwsArgumentError);
    });

    test('bad ticks throw ArgumentError', () {
      final ram = DualPortRAM(2, 2, ReadMode.readFirst, ReadMode.readFirst);
      expect(
          () => ram.tick(0, 2, [0, 0], 0, 0, [0, 0], 0), throwsArgumentError);
      expect(
          () => ram.tick(0, 0, [0, 0], 0, 2, [0, 0], 0), throwsArgumentError);
      expect(() => ram.tick(0, 0, [0], 0, 0, [0, 0], 0), throwsArgumentError);
      expect(() => ram.tick(0, 0, [0, 0], 0, 0, [0], 0), throwsArgumentError);
    });

    test('collision errors compare by value', () {
      const error = WriteCollisionError(address: 3);
      expect(error, const WriteCollisionError(address: 3));
      expect(error.hashCode, const WriteCollisionError(address: 3).hashCode);
      expect(
          error.toString(), 'Write collision: both ports writing to address 3');
      expect(error, isA<Exception>());
      expect(error, isNot(const WriteCollisionError(address: 4)));
      expect(error, isNot('not an error'));
    });
  });

  group('ConfigurableBRAM', () {
    test('initial configuration divides evenly', () {
      final bram = ConfigurableBRAM(1024, 8);
      expect(bram.depth(), 128);
      expect(bram.width(), 8);
      expect(bram.totalBits(), 1024);
    });

    test('reconfigure changes the aspect ratio', () {
      final bram = ConfigurableBRAM(1024, 8);
      bram.reconfigure(16);
      expect(bram.depth(), 64);
      expect(bram.width(), 16);
    });

    test('write and read through port A', () {
      final bram = ConfigurableBRAM(64, 4);
      bram.tickA(0, 0, [1, 0, 1, 0], 1);
      bram.tickA(1, 0, [1, 0, 1, 0], 1);
      bram.tickA(0, 0, [0, 0, 0, 0], 0);
      expect(bram.tickA(1, 0, [0, 0, 0, 0], 0), [1, 0, 1, 0]);
    });

    test('port B shares storage with port A', () {
      final bram = ConfigurableBRAM(64, 4);
      bram.tickA(0, 2, [1, 1, 0, 0], 1);
      bram.tickA(1, 2, [1, 1, 0, 0], 1);
      bram.tickB(0, 2, [0, 0, 0, 0], 0);
      expect(bram.tickB(1, 2, [0, 0, 0, 0], 0), [1, 1, 0, 0]);
    });

    test('reconfigure clears stored data', () {
      final bram = ConfigurableBRAM(64, 4);
      bram.tickA(0, 0, [1, 1, 1, 1], 1);
      bram.tickA(1, 0, [1, 1, 1, 1], 1);
      bram.reconfigure(8);
      expect(bram.depth(), 8);
      bram.tickA(0, 0, [0, 0, 0, 0, 0, 0, 0, 0], 0);
      expect(bram.tickA(1, 0, [0, 0, 0, 0, 0, 0, 0, 0], 0),
          [0, 0, 0, 0, 0, 0, 0, 0]);
    });

    test('ragged widths throw ArgumentError', () {
      expect(() => ConfigurableBRAM(1024, 3), throwsArgumentError);
      expect(() => ConfigurableBRAM(0, 8), throwsArgumentError);
      expect(() => ConfigurableBRAM(1024, 0), throwsArgumentError);
      final bram = ConfigurableBRAM(1024, 8);
      expect(() => bram.reconfigure(3), throwsArgumentError);
      expect(() => bram.reconfigure(0), throwsArgumentError);
    });

    test('bad ticks throw ArgumentError', () {
      final bram = ConfigurableBRAM(64, 4);
      expect(() => bram.tickA(0, 16, [0, 0, 0, 0], 0), throwsArgumentError);
      expect(() => bram.tickB(0, 0, [0], 0), throwsArgumentError);
    });
  });
}
