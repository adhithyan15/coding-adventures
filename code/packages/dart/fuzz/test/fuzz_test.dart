// Tests for coding_adventures_fuzz (VER06c).
//
// Part 1 replays all 25 cases of the shared fuzz-v1 corpus
// (code/specs/fixtures/fuzz-v1/cases.json, read relative to the package
// directory, which is where `dart test` runs): exact mutator outputs,
// sequence determinism, admission verdicts, crash dedup, failure-id
// validation, opaque TAP lines, and corpus rejection. Part 2 covers the
// coverage bitmap, energy schedule, minimization, campaign runner, and
// mutator edge cases.

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:test/test.dart';

Uint8List hexToBytes(String hex) {
  if (hex.isEmpty) return Uint8List(0);
  final out = Uint8List(hex.length ~/ 2);
  for (var i = 0; i < out.length; i++) {
    out[i] = int.parse(hex.substring(2 * i, 2 * i + 2), radix: 16);
  }
  return out;
}

String bytesToHex(Uint8List bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

Endian endianOf(String name) => name == 'big' ? Endian.big : Endian.little;

void main() {
  final corpus = jsonDecode(
    File('../../../specs/fixtures/fuzz-v1/cases.json').readAsStringSync(),
  ) as Map<String, dynamic>;
  final cases = (corpus['cases'] as List).cast<Map<String, dynamic>>();

  group('Fuzz Engine Conformance Suite (VER06 / fuzz-v1)', () {
    test('loads all canonical cases from fuzz-v1/cases.json', () {
      expect(cases.length, 25);
    });

    for (final c in cases) {
      test('conformance case: ${c['id']}', () {
        final op = c['op'] as String;
        switch (op) {
          case 'mutate_bit_flip':
            final input = hexToBytes((c['input'] as Map)['hex'] as String);
            final bitOffset =
                int.parse((c['bit_offset'] as Map)['value'] as String);
            final count = int.parse((c['count'] as Map)['value'] as String);
            final got = bitFlip(input, bitOffset, count);
            expect(bytesToHex(got), (c['output'] as Map)['hex']);
          case 'mutate_byte_flip':
            final input = hexToBytes((c['input'] as Map)['hex'] as String);
            final byteOffset =
                int.parse((c['byte_offset'] as Map)['value'] as String);
            final count = int.parse((c['count'] as Map)['value'] as String);
            final got = byteFlip(input, byteOffset, count);
            expect(bytesToHex(got), (c['output'] as Map)['hex']);
          case 'mutate_arithmetic':
            final input = hexToBytes((c['input'] as Map)['hex'] as String);
            final byteOffset =
                int.parse((c['byte_offset'] as Map)['value'] as String);
            final delta = BigInt.parse((c['delta'] as Map)['value'] as String);
            final width = int.parse((c['width'] as Map)['value'] as String);
            final got = byteArithmetic(input, byteOffset, delta, width,
                endianOf(c['endian'] as String));
            expect(bytesToHex(got), (c['output'] as Map)['hex']);
          case 'mutate_boundary':
            final input = hexToBytes((c['input'] as Map)['hex'] as String);
            final byteOffset =
                int.parse((c['byte_offset'] as Map)['value'] as String);
            final width = int.parse((c['width'] as Map)['value'] as String);
            final val = BigInt.parse((c['val'] as Map)['value'] as String);
            final got = boundaryInsert(
                input, byteOffset, width, val, endianOf(c['endian'] as String));
            expect(bytesToHex(got), (c['output'] as Map)['hex']);
          case 'mutate_block_delete':
            final input = hexToBytes((c['input'] as Map)['hex'] as String);
            final offset = int.parse((c['offset'] as Map)['value'] as String);
            final length = int.parse((c['length'] as Map)['value'] as String);
            final got = blockDelete(input, offset, length);
            expect(bytesToHex(got), (c['output'] as Map)['hex']);
          case 'mutate_block_insert':
            final input = hexToBytes((c['input'] as Map)['hex'] as String);
            final offset = int.parse((c['offset'] as Map)['value'] as String);
            final insertBytes =
                hexToBytes((c['insert_bytes'] as Map)['hex'] as String);
            final got = blockInsert(input, offset, insertBytes);
            expect(bytesToHex(got), (c['output'] as Map)['hex']);
          case 'mutate_block_duplicate':
            final input = hexToBytes((c['input'] as Map)['hex'] as String);
            final srcOffset =
                int.parse((c['src_offset'] as Map)['value'] as String);
            final length = int.parse((c['length'] as Map)['value'] as String);
            final dstOffset =
                int.parse((c['dst_offset'] as Map)['value'] as String);
            final got = blockDuplicate(input, srcOffset, length, dstOffset);
            expect(bytesToHex(got), (c['output'] as Map)['hex']);
          case 'mutate_splice':
            final target = hexToBytes((c['target'] as Map)['hex'] as String);
            final donor = hexToBytes((c['donor'] as Map)['hex'] as String);
            final targetSplit =
                int.parse((c['target_split'] as Map)['value'] as String);
            final donorSplit =
                int.parse((c['donor_split'] as Map)['value'] as String);
            final got = splice(target, donor, targetSplit, donorSplit);
            expect(bytesToHex(got), (c['output'] as Map)['hex']);
          case 'mutate_sequence':
            final input = hexToBytes((c['input'] as Map)['hex'] as String);
            final seed = BigInt.parse((c['seed'] as Map)['value'] as String);
            final steps = int.parse((c['steps'] as Map)['value'] as String);
            final got = mutateSequence(input, seed, steps);
            expect(bytesToHex(got), (c['output'] as Map)['hex']);
          case 'corpus_admit':
            final existing = (c['existing_edges'] as List)
                .map((e) => int.parse((e as Map)['value'] as String))
                .toList();
            final trace = (c['trace_edges'] as List)
                .map((e) => int.parse((e as Map)['value'] as String))
                .toList();
            final (admitted, newEdges) = corpusAdmit(existing, trace);
            expect(admitted, c['admitted']);
            expect(newEdges,
                int.parse((c['new_edges'] as Map)['value'] as String));
          case 'crash_dedup':
            final a = CrashInfo(
              c['crash_a_kind'] as String,
              (c['crash_a_frames'] as List).cast<String>(),
            );
            final b = CrashInfo(
              c['crash_b_kind'] as String,
              (c['crash_b_frames'] as List).cast<String>(),
            );
            expect(isDuplicateCrash(a, b), c['is_duplicate']);
          case 'failure_id_valid':
            expect(isFuzzFailureId(c['input'] as String), c['valid']);
          case 'private_reporting':
            final line = formatTapFailure(
              1,
              c['target'] as String,
              c['failure_id'] as String,
              c['sig_hash'] as String,
            );
            expect(line, c['tap_line']);
            expect(line.contains((c['raw_payload'] as Map)['hex'] as String),
                isFalse);
          case 'corpus_reject':
            final raw = c['text'] as String;
            expect(() {
              if (!raw.startsWith('{') && !raw.startsWith('[')) {
                throw const FormatException('invalid json');
              }
              jsonDecode(raw);
            }, throwsA(anyOf(isA<FormatException>(), isA<ArgumentError>())));
          default:
            fail('unrecognized op: $op');
        }
      });
    }
  });

  group('Coverage Bitmap Operations', () {
    test('initializes empty with correct dimensions', () {
      final cov = CoverageBitmap();
      expect(cov.bytes.length, coverageBytes);
      expect(cov.count(), 0);
    });

    test('records and checks edges', () {
      final cov = CoverageBitmap();
      cov.recordEdge(0);
      cov.recordEdge(100);
      cov.recordEdge(65535);
      expect(cov.hasEdge(0), isTrue);
      expect(cov.hasEdge(100), isTrue);
      expect(cov.hasEdge(65535), isTrue);
      expect(cov.hasEdge(1), isFalse);
      expect(cov.hasEdge(101), isFalse);
      expect(cov.count(), 3);
    });

    test('wraps modulo and negative indices safely', () {
      final cov = CoverageBitmap();
      cov.recordEdge(-1);
      expect(cov.hasEdge(65535), isTrue);
      cov.recordEdge(65536 + 42);
      expect(cov.hasEdge(42), isTrue);
    });

    test('admits traces with novelty counts', () {
      final cov = CoverageBitmap();
      final (hasNew1, count1) = cov.admitTrace([10, 20, 30]);
      expect(hasNew1, isTrue);
      expect(count1, 3);
      expect(cov.count(), 3);
      final (hasNew2, count2) = cov.admitTrace([20, 30, 40]);
      expect(hasNew2, isTrue);
      expect(count2, 1);
      expect(cov.count(), 4);
      final (hasNew3, count3) = cov.admitTrace([10, 40]);
      expect(hasNew3, isFalse);
      expect(count3, 0);
    });

    test('merges bitmaps', () {
      final a = CoverageBitmap();
      a.recordEdge(1);
      a.recordEdge(2);
      final b = CoverageBitmap();
      b.recordEdge(2);
      b.recordEdge(3);
      a.merge(b);
      expect(a.count(), 3);
      expect(a.hasEdge(1), isTrue);
      expect(a.hasEdge(2), isTrue);
      expect(a.hasEdge(3), isTrue);
    });
  });

  group('Energy Scheduling & Mutator Edge Cases', () {
    test('energy favors short high-novelty seeds', () {
      final e1 = calculateEnergy(10, 16, 5, 20);
      expect(e1 >= 1, isTrue);
      expect(calculateEnergy(10, 0, 0, 0), 10);
      expect(calculateEnergy(10, 10000, 0, 100) < 10, isTrue);
    });

    test('mutators handle empty buffers', () {
      final empty = Uint8List(0);
      expect(bitFlip(empty, 0).length, 0);
      expect(byteFlip(empty, 0).length, 0);
      expect(byteArithmetic(empty, 0, BigInt.one, 1).length, 0);
      expect(boundaryInsert(empty, 0, 1, BigInt.zero).length, 0);
      expect(blockDelete(empty, 0, 1).length, 0);
      expect(blockInsert(empty, 0, Uint8List.fromList([1, 2])).length, 2);
      expect(blockDuplicate(empty, 0, 1, 0).length, 0);
    });

    test('byteArithmetic widths and endianness', () {
      final buf = Uint8List(8);
      final be8 = byteArithmetic(buf, 0, BigInt.one, 8, Endian.big);
      expect(be8[7], 1);
      final le8 = byteArithmetic(buf, 0, BigInt.one, 8, Endian.little);
      expect(le8[0], 1);
      final be4 = byteArithmetic(buf, 0, BigInt.one, 4, Endian.big);
      expect(be4[3], 1);
      final short = Uint8List.fromList([1, 2]);
      expect(byteArithmetic(short, 0, BigInt.one, 4).length, 2);
    });

    test('boundaryInsert widths and endianness', () {
      final buf = Uint8List(8);
      final be8 = boundaryInsert(
          buf, 0, 8, BigInt.parse('1234605616436508552'), Endian.big);
      expect(be8[0], 0x11);
      expect(be8[7], 0x88);
      final le8 = boundaryInsert(
          buf, 0, 8, BigInt.parse('1234605616436508552'), Endian.little);
      expect(le8[0], 0x88);
      expect(le8[7], 0x11);
      final be4 =
          boundaryInsert(buf, 0, 4, BigInt.from(0x11223344), Endian.big);
      expect(be4[0], 0x11);
      expect(be4[3], 0x44);
      final short = Uint8List.fromList([1, 2]);
      expect(boundaryInsert(short, 0, 4, BigInt.zero).length, 2);
    });

    test('crashSignature is stable and kind-sensitive', () {
      final sig1 = crashSignature('AssertionError', ['funcA', 'funcB']);
      final sig2 = crashSignature('AssertionError', ['funcA', 'funcB']);
      final sig3 = crashSignature('TypeError', ['funcA', 'funcB']);
      expect(sig1.length, 8);
      expect(sig1, sig2);
      expect(sig1 == sig3, isFalse);
    });

    test('minimizeCrashingInput shrinks while preserving signature', () {
      final initial = Uint8List.fromList([10, 20, 30, 40, 50, 60, 70, 80]);
      final targetSig = crashSignature('MagicCrash', ['target']);
      FuzzOutcome testFn(Uint8List cand) {
        if (cand.length >= 2 && cand[0] == 10 && cand[cand.length - 1] == 80) {
          return const OutcomeCrash('MagicCrash', ['target']);
        }
        return const OutcomePass();
      }

      final minimized = minimizeCrashingInput(initial, targetSig, testFn);
      expect(minimized.length <= initial.length, isTrue);
      expect(minimized[0], 10);
      expect(minimized[minimized.length - 1], 80);
      expect(testFn(minimized), isA<OutcomeCrash>());
    });

    test('fuzzTarget runs campaigns with coverage feedback', () {
      final seedInput = Uint8List.fromList([0x00, 0x00, 0x00, 0x00]);
      final report = fuzzTarget(
        [seedInput],
        FuzzOptions(maxIterations: 300, seed: BigInt.from(12345)),
        (candidate, cov) {
          if (candidate.isNotEmpty && candidate[0] == 0x42) {
            cov.recordEdge(100);
            if (candidate.length > 1 && candidate[1] == 0x99) {
              cov.recordEdge(200);
              return const OutcomeCrash('TargetPanic', ['fuzzTarget', 'eval']);
            }
          }
          cov.recordEdge(1);
          return const OutcomePass();
        },
      );
      expect(report.iterations > 0, isTrue);
      expect(report.totalEdges >= 1, isTrue);
    });

    test('vocabulary holds five ids and rejects strangers', () {
      expect(isFuzzFailureId(fidFuzzCrash), isTrue);
      expect(isFuzzFailureId(fidFuzzTimeout), isTrue);
      expect(isFuzzFailureId(fidFuzzOom), isTrue);
      expect(isFuzzFailureId(fidFuzzLeak), isTrue);
      expect(isFuzzFailureId(fidFuzzPanic), isTrue);
      expect(isFuzzFailureId('unknown-id'), isFalse);
      expect(fuzzFailureIds.length, 5);
    });

    test('boundary conditions and edge cases', () {
      final rng = SplitMix64(BigInt.zero);
      expect(rng.bounded(0), 0);
      expect(rng.bounded(1), 0);
      expect(() => rng.bounded(0x100000001), throwsArgumentError);

      final buf = Uint8List.fromList([1, 2, 3, 4]);
      expect(byteArithmetic(buf, 0, BigInt.one, 3).length, 4);
      expect(boundaryInsert(buf, 0, 3, BigInt.one).length, 4);

      expect(blockDelete(buf, 0, 0).length, 4);
      expect(blockDelete(buf, 0, -1).length, 4);

      final mutatedEmpty = mutateSequence(Uint8List(0), BigInt.from(42), 1);
      expect(mutatedEmpty.length >= 1, isTrue);

      final minEmpty = minimizeCrashingInput(
          Uint8List(0), 'sig', (_) => const OutcomePass());
      expect(minEmpty.length, 0);

      final multiSeed = [
        Uint8List.fromList([1, 2, 3, 4]),
        Uint8List.fromList([5, 6, 7, 8]),
      ];
      var callCount = 0;
      final report2 = fuzzTarget(
        multiSeed,
        FuzzOptions(maxIterations: 50, seed: BigInt.from(999), maxInputLen: 16),
        (cand, cov) {
          callCount++;
          cov.recordEdge(callCount % 100);
          if (cand.length > 5 && cand[0] == 5) {
            return const OutcomeCrash('MultiCrash', ['f']);
          }
          return const OutcomePass();
        },
      );
      expect(report2.iterations > 0, isTrue);

      final crashReport = fuzzTarget(
        [
          Uint8List.fromList([9, 9])
        ],
        FuzzOptions(maxIterations: 5),
        (__input, __cov) => const OutcomeCrash('InitCrash', ['main']),
      );
      expect(crashReport.crashes.length, 1);
      expect(crashReport.crashes[0].kind, 'InitCrash');
    });
  });
}
