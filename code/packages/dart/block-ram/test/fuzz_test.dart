/// Hostile-input fuzz campaign for `coding_adventures_block_ram`,
/// using the in-lane `fuzz` package: raw bytes drive an opcode machine
/// over every public surface — single and dual ticks with hostile
/// addresses, wrong-length words, garbage (non-bit) data, BRAM
/// reconfigures with ragged widths, and direct array/cell access. Only
/// [ArgumentError] and [WriteCollisionError] may escape; everything
/// else is a crash. Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:typed_data';

import 'package:coding_adventures_block_ram/block_ram.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:test/test.dart';

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final ram = SinglePortRAM(8, 8, ReadMode.readFirst);
  final dual = DualPortRAM(8, 8, ReadMode.readFirst, ReadMode.readFirst);
  final bram = ConfigurableBRAM(64, 8);
  final arr = SramArray(8, 8);
  final cell = SramCell();
  var i = 0;

  List<int> word(int seed, int width) =>
      List.generate(width, (j) => (seed >> j) & 1);

  while (i < input.length) {
    final op = input[i] % 7;
    cov.recordEdge(op * 16 + (input[i] % 16));
    i++;
    if (i + 3 >= input.length) break;
    final a = input[i], b = input[i + 1], c = input[i + 2];
    final d = input[i + 3];
    i += 4;
    try {
      switch (op) {
        case 0:
          // Single tick: hostile address half the time (raw byte can
          // exceed depth 8), garbage data bits always possible.
          final address = c.isEven ? c : c & 7;
          final garbage = List.generate(8, (j) => (d * (j + 1)) - 300);
          ram.tick(a & 1, address, b.isEven ? garbage : word(b, 8), d & 1);
        case 1:
          // Dual tick: hostile addresses and colliding writes welcome.
          final addrA = a.isEven ? a : a & 7;
          final addrB = b.isEven ? b : b & 7;
          dual.tick(c & 1, addrA, word(c, 8), c & 1, addrB, word(d, 8), d & 1);
        case 2:
          // BRAM port tick: hostile address half the time.
          final address = b.isEven ? b : b & 7;
          if (a.isEven) {
            bram.tickA(c & 1, address, word(c, bram.width()), d & 1);
          } else {
            bram.tickB(c & 1, address, word(c, bram.width()), d & 1);
          }
        case 3:
          // Reconfigure: ragged widths throw; survivors keep the budget.
          final width = a % 9;
          try {
            bram.reconfigure(width);
          } on ArgumentError {
            cov.recordEdge(200);
            break;
          }
          if (bram.totalBits() != 64 || bram.depth() * bram.width() != 64) {
            return const OutcomeCrash('model-divergence', ['bram-budget']);
          }
        case 4:
          // Direct array access: hostile rows and ragged data.
          final row = a.isEven ? a : a & 7;
          final data = b.isEven ? word(b, 8) : word(b, b % 9);
          if (c.isEven) {
            arr.write(row, data);
          } else {
            arr.read(row);
          }
        case 5:
          // Cell with hostile select/data lines: non-bits throw
          // ArgumentError (fail-closed strengthening over the
          // reference's debug-only assert), caught by the target.
          cell.write(a, b);
          cell.read(c);
          cell.value();
        case 6:
          // Pure observers: shapes must stay consistent.
          if (ram.dump().length != 8 ||
              arr.shape() != (8, 8) ||
              ram.depth() != 8 ||
              bram.totalBits() != 64) {
            return const OutcomeCrash('model-divergence', ['shape']);
          }
      }
    } on ArgumentError {
      cov.recordEdge(210 + op);
    } on WriteCollisionError {
      cov.recordEdge(220);
    }
  }
  return const OutcomePass();
}

void main() {
  test('fuzz opcode machine never escapes its vocabulary', () {
    final seeds = [
      Uint8List(0),
      Uint8List.fromList([0]),
      Uint8List.fromList(List.generate(64, (i) => i)),
      Uint8List.fromList(List.filled(64, 255)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(2109)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
