# coding_adventures_block_ram (Dart)

Gate-level SRAM cells, arrays, and configurable RAM modules with
cycle-accurate tick semantics. The Dart lane of the Rust `block-ram`
crate ([spec](../../../specs/F00-block-ram.md)).

```dart
import 'package:coding_adventures_block_ram/block_ram.dart';

final ram = SinglePortRAM(16, 8, ReadMode.readFirst);
ram.tick(0, 3, [1, 0, 1, 0, 1, 0, 1, 0], 1);
final out = ram.tick(1, 3, [1, 0, 1, 0, 1, 0, 1, 0], 1); // rising edge writes
print(ram.dump()[3]); // [1, 0, 1, 0, 1, 0, 1, 0]
```

## API

- `SramCell` models one 6T bit (`read` / `write` / `value`); `SramArray`
  grids cells into rows × cols (`read` / `write` / `shape`).
- `SinglePortRAM` ticks half-cycles (`tick`, `dump`, `depth`, `width`)
  under a `ReadMode` (`readFirst` / `writeFirst` / `noChange`): only
  0→1 clock edges operate; everything else repeats the last output.
- `DualPortRAM` runs two independent ports over shared storage
  (`tick` returns `(dataOutA, dataOutB)`); same-address double writes
  on an edge throw `WriteCollisionError`.
- `ConfigurableBRAM` wraps a true dual-port block with a fixed bit
  budget: `reconfigure` re-partitions depth × width (clearing data),
  `tickA` / `tickB` drive each port with the other idle.

Bit-level signals are plain `int`s that must be 0 or 1; geometry and
signal violations throw `ArgumentError`, never silently corrupt.

## Deliberate divergences from the reference

- Bit validation throws `ArgumentError` always (fail-closed
  strengthening): the reference uses `debug_assert`, so release-mode
  Rust silently stores garbage bits. Check *placement* still mirrors
  the reference exactly — read-path `data_in` is never inspected, so
  garbage with `we=0` reads cleanly on both lanes.
- `ReadMode` is a Dart enum rather than a Rust enum; `dump` returns
  `List<List<int>>` instead of `Vec<Vec<u8>>`; `WriteCollisionError`
  is an `Exception` with value equality instead of a struct.

## Validation

- **Unit** (`test/block_ram_test.dart`): all 14 reference tests plus
  the gaps the reference leaves — `NoChange` mode, collision gating
  on edges only, same-address write+read visibility, port B, stuck
  clocks, reconfigure clearing, and the full `ArgumentError` matrix.
- **Property** (`test/properties_test.dart`): 8 seeded properties over
  the in-lane `pbt` package — write-then-read against a mirror
  memory, read-mode outputs, edges-only mutation, port independence,
  collision exactness, the BRAM bit budget, bit-domain outputs.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over the in-lane `fuzz` package driving an opcode machine across
  every surface; only `ArgumentError` and `WriteCollisionError` escape.
  The campaign caught a real contract hole pre-release (a debug-only
  assert on cell lines), fixed by the fail-closed strengthening above.
- **Coverage**: 100% lines. The single uncovered function record is
  the `ReadMode` enum's implicit constructor, a coverage artifact with
  no callable body.
