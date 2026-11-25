/// Coverage-guided fuzz engine: the Dart lane of VER06/VER06c.
///
/// A zero-dependency Level 2 engine: deterministic mutator algebra over raw
/// bytes, 65,536-bit coverage-bitmap feedback, power-schedule energy, crash
/// deduplication via normalized FNV-1a signatures, crashing-input
/// minimization, closed failure vocabulary, and privacy-preserving TAP
/// reporting that never prints payload bytes.
///
/// Behavioral port of the TypeScript VER06b engine: same seeds, same streams,
/// same mutations. All 64-bit state is [BigInt], exact on VM and JavaScript.
///
/// ```dart
/// import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
///
/// void main() {
///   final report = fuzzTarget(
///     [Uint8List.fromList([0, 0, 0, 0])],
///     FuzzOptions(maxIterations: 500, seed: BigInt.from(7)),
///     (input, cov) {
///       cov.recordEdge(input.length);
///       if (input.isNotEmpty && input[0] == 0x42) {
///         return Crash('MagicByte', ['target']);
///       }
///       return const Pass();
///     },
///   );
///   print('edges=${report.totalEdges} crashes=${report.crashes.length}');
/// }
/// ```
library fuzz;

import 'dart:convert';
import 'dart:math' as math;
import 'dart:typed_data';

// ─── PRNG & Hashing (SplitMix64 & FNV-1a) ───────────────────────────────────
//
// Same construction as the VER05b lane: SplitMix64 state in BigInt, masked to
// 64 bits after every op. Deliberately self-contained (not imported from the
// pbt package) so this engine keeps zero runtime dependencies.

/// 2^64, the SplitMix64 state modulus.
final BigInt _mask64Plus1 = BigInt.one << 64;

/// 2^64 - 1, for masking state back into 64 bits.
final BigInt _mask64 = _mask64Plus1 - BigInt.one;

/// Deterministic 64-bit PRNG. Same seed, same stream, on every platform.
class SplitMix64 {
  BigInt _state;

  /// Seeds the generator; the seed is reduced modulo 2^64.
  SplitMix64(BigInt seed) : _state = seed & _mask64;

  /// Draws the next unsigned 64-bit value.
  BigInt nextU64() {
    _state = (_state + _goldenGamma()) & _mask64;
    var z = _state;
    z = (((z ^ (z >> 30)) * _scramble1()) & _mask64);
    z = (((z ^ (z >> 27)) * _scramble2()) & _mask64);
    return (z ^ (z >> 31)) & _mask64;
  }

  /// Draws a uniform `int` from `0` (inclusive) to `n` (exclusive).
  ///
  /// Lemire near-divisionless rejection. `n <= 1` returns 0;
  /// `n > 2^32` throws.
  int bounded(int n) {
    final bn = BigInt.from(n);
    if (bn <= BigInt.one) return 0;
    if (bn > BigInt.from(0x100000000)) {
      throw ArgumentError('bounded(n) needs 1 <= n <= 2^32, got $n');
    }
    final threshold = (_mask64Plus1 - bn) % bn;
    while (true) {
      final x = nextU64();
      if (x >= threshold) {
        return (x % bn).toInt();
      }
    }
  }
}

BigInt _goldenGamma() =>
    (BigInt.from(0x9e3779b9) << 32) | BigInt.from(0x7f4a7c15);

BigInt _scramble1() =>
    (BigInt.from(0xbf58476d) << 32) | BigInt.from(0x1ce4e5b9);

BigInt _scramble2() =>
    (BigInt.from(0x94d049bb) << 32) | BigInt.from(0x133111eb);

/// 64-bit FNV-1a hash over [data].
BigInt fnv1a64(Uint8List data) {
  var hash = (BigInt.from(0xcbf29ce4) << 32) | BigInt.from(0x84222325);
  final prime = BigInt.from(0x100000001b3);
  for (final byte in data) {
    hash ^= BigInt.from(byte);
    hash = (hash * prime) & _mask64;
  }
  return hash;
}

// ─── Closed Failure Vocabulary (VER06 §5.1, §6) ─────────────────────────────

/// Target crashed (unexpected exception / abort).
const String fidFuzzCrash = 'fuzz-crash';

/// Target exceeded its step budget.
const String fidFuzzTimeout = 'fuzz-timeout';

/// Target exhausted memory.
const String fidFuzzOom = 'fuzz-oom';

/// Target leaked a resource.
const String fidFuzzLeak = 'fuzz-leak';

/// Harness itself panicked.
const String fidFuzzPanic = 'fuzz-panic';

/// Every registered fuzz failure id.
const Set<String> fuzzFailureIds = {
  fidFuzzCrash,
  fidFuzzTimeout,
  fidFuzzOom,
  fidFuzzLeak,
  fidFuzzPanic,
};

/// Returns true if [id] is in the closed fuzz vocabulary.
bool isFuzzFailureId(String id) => fuzzFailureIds.contains(id);

// ─── Mutator Algebra (VER06 §3) ─────────────────────────────────────────────
//
// Every mutator copies: inputs are never modified, so a corpus seed can be
// reused across iterations. Offsets wrap modulo the buffer size, which keeps
// every mutator total (defined on all inputs, including empty ones).

/// Inverts [count] consecutive bits from `bitOffset mod totalBits`.
///
/// Bits run LSB-first within each byte (`byte ^= 1 << (pos % 8)`). Empty
/// input returns empty.
Uint8List bitFlip(Uint8List data, int bitOffset, [int count = 1]) {
  if (data.isEmpty) return Uint8List(0);
  final buf = Uint8List.fromList(data);
  final totalBits = buf.length * 8;
  final base = bitOffset % totalBits;
  for (var i = 0; i < count; i++) {
    final pos = (base + (i % totalBits)) % totalBits;
    final byteIdx = pos ~/ 8;
    final bitIdx = pos % 8;
    buf[byteIdx] ^= 1 << bitIdx;
  }
  return buf;
}

/// XORs [count] consecutive bytes with `0xFF` from `byteOffset mod len`.
Uint8List byteFlip(Uint8List data, int byteOffset, [int count = 1]) {
  if (data.isEmpty) return Uint8List(0);
  final buf = Uint8List.fromList(data);
  final totalBytes = buf.length;
  final base = byteOffset % totalBytes;
  for (var i = 0; i < count; i++) {
    final pos = (base + (i % totalBytes)) % totalBytes;
    buf[pos] ^= 0xff;
  }
  return buf;
}

int _wrappedOffset(int byteOffset, int length, int width) =>
    byteOffset % (length - width + 1);

/// Adds signed [delta] mod 2^(8·[width]) at the wrapped [byteOffset].
///
/// Widths {1,2,4,8} with explicit [endian]. Buffers shorter than [width]
/// return a copy unchanged; other widths are no-ops.
Uint8List byteArithmetic(
  Uint8List data,
  int byteOffset,
  BigInt delta, [
  int width = 1,
  Endian endian = Endian.little,
]) {
  if (data.length < width) return Uint8List.fromList(data);
  final buf = Uint8List.fromList(data);
  final offset = _wrappedOffset(byteOffset, buf.length, width);
  final view = ByteData.sublistView(buf);

  switch (width) {
    case 1:
      final val = BigInt.from(buf[offset]);
      buf[offset] = ((val + delta) & BigInt.from(0xff)).toInt();
    case 2:
      final val = BigInt.from(view.getUint16(offset, endian));
      final next = ((val + delta) & BigInt.from(0xffff)).toInt();
      view.setUint16(offset, next, endian);
    case 4:
      final val = BigInt.from(view.getUint32(offset, endian));
      final next = ((val + delta) & BigInt.from(0xffffffff)).toInt();
      view.setUint32(offset, next, endian);
    case 8:
      // Assembled byte-wise: ByteData u64 accessors box through int, which
      // is lossy past 2^53 under JavaScript. This stays exact everywhere.
      var cur = BigInt.zero;
      for (var i = 0; i < 8; i++) {
        final b =
            endian == Endian.little ? buf[offset + i] : buf[offset + 7 - i];
        cur |= BigInt.from(b) << (8 * i);
      }
      final next = (cur + delta) & _mask64;
      for (var i = 0; i < 8; i++) {
        final b = ((next >> (8 * i)) & BigInt.from(0xff)).toInt();
        if (endian == Endian.little) {
          buf[offset + i] = b;
        } else {
          buf[offset + 7 - i] = b;
        }
      }
    default:
      break;
  }
  return buf;
}

/// Overwrites [width] bytes at the wrapped [byteOffset] with [val].
///
/// Same short-buffer and width rules as [byteArithmetic].
Uint8List boundaryInsert(
  Uint8List data,
  int byteOffset,
  int width,
  BigInt val, [
  Endian endian = Endian.little,
]) {
  if (data.length < width) return Uint8List.fromList(data);
  final buf = Uint8List.fromList(data);
  final offset = _wrappedOffset(byteOffset, buf.length, width);
  final view = ByteData.sublistView(buf);

  switch (width) {
    case 1:
      buf[offset] = (val & BigInt.from(0xff)).toInt();
    case 2:
      view.setUint16(offset, (val & BigInt.from(0xffff)).toInt(), endian);
    case 4:
      view.setUint32(offset, (val & BigInt.from(0xffffffff)).toInt(), endian);
    case 8:
      // Byte-wise for the same JS-exactness reason as byteArithmetic.
      final v = val & _mask64;
      for (var i = 0; i < 8; i++) {
        final b = ((v >> (8 * i)) & BigInt.from(0xff)).toInt();
        if (endian == Endian.little) {
          buf[offset + i] = b;
        } else {
          buf[offset + 7 - i] = b;
        }
      }
    default:
      break;
  }
  return buf;
}

/// Deletes up to [length] bytes at `offset mod len`.
Uint8List blockDelete(Uint8List data, int offset, int length) {
  if (data.isEmpty || length <= 0) return Uint8List.fromList(data);
  final actualOffset = offset % data.length;
  final actualLength = math.min(length, data.length - actualOffset);
  final out = Uint8List(data.length - actualLength);
  out.setRange(0, actualOffset, data, 0);
  out.setRange(actualOffset, out.length, data, actualOffset + actualLength);
  return out;
}

/// Inserts [insertBytes] at `offset mod (len + 1)`.
Uint8List blockInsert(Uint8List data, int offset, Uint8List insertBytes) {
  if (data.isEmpty) return Uint8List.fromList(insertBytes);
  final actualOffset = offset % (data.length + 1);
  final out = Uint8List(data.length + insertBytes.length);
  out.setRange(0, actualOffset, data, 0);
  out.setRange(actualOffset, actualOffset + insertBytes.length, insertBytes);
  out.setRange(
      actualOffset + insertBytes.length, out.length, data, actualOffset);
  return out;
}

/// Copies up to [length] bytes from `srcOffset` to `dstOffset`.
Uint8List blockDuplicate(
    Uint8List data, int srcOffset, int length, int dstOffset) {
  if (data.isEmpty || length <= 0) return Uint8List.fromList(data);
  final actualSrc = srcOffset % data.length;
  final actualLen = math.min(length, data.length - actualSrc);
  final chunk = data.sublist(actualSrc, actualSrc + actualLen);
  final actualDst = dstOffset % (data.length + 1);

  final out = Uint8List(data.length + actualLen);
  out.setRange(0, actualDst, data, 0);
  out.setRange(actualDst, actualDst + actualLen, chunk);
  out.setRange(actualDst + actualLen, out.length, data, actualDst);
  return out;
}

/// Joins `target[0:tSplit]` with `donor[dSplit:]`, splits clamped to bounds.
Uint8List splice(
    Uint8List target, Uint8List donor, int targetSplit, int donorSplit) {
  final tSplit = targetSplit.clamp(0, target.length);
  final dSplit = donorSplit.clamp(0, donor.length);
  final out = Uint8List(tSplit + (donor.length - dSplit));
  out.setRange(0, tSplit, target, 0);
  out.setRange(tSplit, out.length, donor, dSplit);
  return out;
}

/// Applies [steps] PRNG-chosen mutations from [seed].
///
/// Each step draws `bounded(7)`: bitFlip, byteFlip, small-delta arithmetic,
/// interest-value insert, blockDelete, single-byte blockInsert, or
/// blockDuplicate. Empty buffers refill to `[0]` first. Fully deterministic.
Uint8List mutateSequence(Uint8List data, BigInt seed, int steps) {
  final rng = SplitMix64(seed);
  var current = Uint8List.fromList(data);

  for (var i = 0; i < steps; i++) {
    if (current.isEmpty) {
      current = Uint8List.fromList([0]);
    }
    final opChoice = rng.bounded(7);
    switch (opChoice) {
      case 0:
        final bitOff = rng.bounded(current.length * 8);
        current = bitFlip(current, bitOff, 1);
      case 1:
        final byteOff = rng.bounded(current.length);
        current = byteFlip(current, byteOff, 1);
      case 2:
        const deltas = [-35, -8, -1, 1, 8, 35];
        final delta = deltas[rng.bounded(deltas.length)];
        final byteOff = rng.bounded(current.length);
        current = byteArithmetic(
            current, byteOff, BigInt.from(delta), 1, Endian.little);
      case 3:
        const interest = [0, 1, 127, 128, 255];
        final val = interest[rng.bounded(interest.length)];
        final byteOff = rng.bounded(current.length);
        current = boundaryInsert(
            current, byteOff, 1, BigInt.from(val), Endian.little);
      case 4:
        if (current.length > 1) {
          final maxDel = math.max(1, current.length ~/ 2);
          final delLen = 1 + rng.bounded(maxDel);
          final off = rng.bounded(current.length);
          current = blockDelete(current, off, delLen);
        }
      case 5:
        final insByte = Uint8List.fromList([rng.bounded(256)]);
        final off = rng.bounded(current.length + 1);
        current = blockInsert(current, off, insByte);
      default:
        if (current.length > 1) {
          final maxDup = math.max(1, current.length ~/ 2);
          final dupLen = 1 + rng.bounded(maxDup);
          final srcOff = rng.bounded(current.length);
          final dstOff = rng.bounded(current.length + 1);
          current = blockDuplicate(current, srcOff, dupLen, dstOff);
        }
    }
  }
  return current;
}

// ─── Corpus Admission & Coverage Bitmap (VER06 §4) ───────────────────────────

/// Pure novelty check: does [trace] hold edges absent from [existing]?
///
/// Returns `(admitted, newEdges)` counting unique novel edges.
(bool, int) corpusAdmit(List<int> existingEdges, List<int> traceEdges) {
  final existing = existingEdges.toSet();
  final uniqueNew = <int>{};
  for (final edge in traceEdges) {
    if (!existing.contains(edge)) {
      uniqueNew.add(edge);
    }
  }
  if (uniqueNew.isNotEmpty) {
    return (true, uniqueNew.length);
  }
  return (false, 0);
}

/// Number of distinct edges in the coverage map.
const int coverageMapSize = 65536;

/// Bitmap storage in bytes (65,536 bits).
const int coverageBytes = coverageMapSize ~/ 8;

/// Shared 65,536-bit execution coverage bitmap.
class CoverageBitmap {
  /// Raw bitmap storage, LSB-first within each byte.
  final Uint8List bytes = Uint8List(coverageBytes);

  /// Records [edge] (modulo 65,536) as executed.
  void recordEdge(int edge) {
    final e = edge % coverageMapSize;
    bytes[e ~/ 8] |= 1 << (e % 8);
  }

  /// Returns true if [edge] has been covered.
  bool hasEdge(int edge) {
    final e = edge % coverageMapSize;
    return (bytes[e ~/ 8] & (1 << (e % 8))) != 0;
  }

  /// Admits [trace], returning `(hasNew, countNew)`.
  (bool, int) admitTrace(List<int> trace) {
    var countNew = 0;
    for (final edge in trace) {
      final e = edge % coverageMapSize;
      final byteIdx = e ~/ 8;
      final bitMask = 1 << (e % 8);
      if ((bytes[byteIdx] & bitMask) == 0) {
        bytes[byteIdx] |= bitMask;
        countNew++;
      }
    }
    return (countNew > 0, countNew);
  }

  /// Total distinct covered edges (popcount).
  int count() {
    var total = 0;
    for (var i = 0; i < bytes.length; i++) {
      var b = bytes[i];
      while (b > 0) {
        total += b & 1;
        b >>= 1;
      }
    }
    return total;
  }

  /// ORs [other] into this bitmap.
  void merge(CoverageBitmap other) {
    for (var i = 0; i < bytes.length; i++) {
      bytes[i] |= other.bytes[i];
    }
  }
}

// ─── Crash Deduplication & Privacy-Preserving TAP ────────────────────────────

/// 8-hex-digit crash signature: FNV-1a over `"kind:f1:f2:..."`, lower 32 bits.
String crashSignature(String kind, List<String> frames) {
  final raw = '$kind:${frames.join(':')}';
  final h = fnv1a64(Uint8List.fromList(utf8.encode(raw)));
  final lower32 = (h & BigInt.from(0xffffffff)).toInt();
  return lower32.toRadixString(16).padLeft(8, '0');
}

/// A crash report for deduplication comparison.
class CrashInfo {
  /// Crash kind (exception type / abort reason).
  final String kind;

  /// Normalized stack frames.
  final List<String> frames;
  const CrashInfo(this.kind, this.frames);
}

/// Returns true if both crashes share a normalized signature.
bool isDuplicateCrash(CrashInfo a, CrashInfo b) =>
    crashSignature(a.kind, a.frames) == crashSignature(b.kind, b.frames);

/// Formats an opaque TAP failure line (no payload bytes, ever).
String formatTapFailure(
        int testNum, String targetId, String failureId, String sigHash) =>
    'not ok $testNum - $targetId [$failureId:$sigHash]';

// ─── Energy Scheduling (VER06 §4.3) ─────────────────────────────────────────

/// Power-schedule energy: favors short, high-novelty seeds.
///
/// `max(1, round(base·(1 + new/total) / sqrt(max(1, len))))`.
int calculateEnergy(int baseEnergy, int seedLen, int newEdges, int totalEdges) {
  final base = math.max(1, baseEnergy);
  final edgeBonus = totalEdges > 0 ? 1.0 + newEdges / totalEdges : 1.0;
  final lenFactor = math.sqrt(math.max(1, seedLen));
  final energy = ((base * edgeBonus) / lenFactor).round();
  return math.max(1, energy);
}

// ─── Outcomes, Records, Options ─────────────────────────────────────────────

/// Outcome of a single target invocation.
sealed class FuzzOutcome {
  const FuzzOutcome();
}

/// Target completed without incident.
class OutcomePass extends FuzzOutcome {
  const OutcomePass();
}

/// Target crashed with a kind and normalized frames.
class OutcomeCrash extends FuzzOutcome {
  /// Crash kind.
  final String kind;

  /// Normalized stack frames.
  final List<String> frames;
  const OutcomeCrash(this.kind, this.frames);
}

/// Target exceeded its step budget.
class OutcomeTimeout extends FuzzOutcome {
  const OutcomeTimeout();
}

/// Target exhausted memory.
class OutcomeOom extends FuzzOutcome {
  const OutcomeOom();
}

/// A deduplicated crash with its minimal reproducer.
class CrashRecord {
  /// Normalized 8-hex signature.
  final String signature;

  /// Crash kind.
  final String kind;

  /// Normalized frames.
  final List<String> frames;

  /// Shortest input known to reproduce this signature.
  Uint8List minimalInput;

  /// Times this signature was observed.
  int hits;
  CrashRecord({
    required this.signature,
    required this.kind,
    required this.frames,
    required this.minimalInput,
    required this.hits,
  });
}

/// Options for a bounded fuzz campaign.
class FuzzOptions {
  /// Maximum target executions (default 1000).
  final int maxIterations;

  /// Campaign seed (default 42).
  final BigInt seed;

  /// Candidate truncation length (default 4096).
  final int maxInputLen;

  /// Base energy per seed (default 10).
  final int baseEnergy;
  FuzzOptions({
    this.maxIterations = 1000,
    BigInt? seed,
    this.maxInputLen = 4096,
    this.baseEnergy = 10,
  }) : seed = seed ?? BigInt.from(42);
}

/// Summary returned after a bounded fuzzing run.
class FuzzReport {
  /// Target executions performed.
  final int iterations;

  /// Seeds in the final corpus.
  final int admittedSeeds;

  /// Distinct edges covered.
  final int totalEdges;

  /// Deduplicated crashes.
  final List<CrashRecord> crashes;
  const FuzzReport({
    required this.iterations,
    required this.admittedSeeds,
    required this.totalEdges,
    required this.crashes,
  });
}

/// A seed held in the active corpus.
class CorpusSeed {
  /// Seed bytes.
  final Uint8List data;

  /// Novel edges this seed contributed on admission.
  final int newEdges;

  /// Mutation generations from triage.
  final int generation;
  CorpusSeed({
    required this.data,
    required this.newEdges,
    required this.generation,
  });
}

// ─── Crash Minimization & Engine Runner (VER06 §5.3, §7) ─────────────────────

/// Minimizes [input] while preserving its exact crash [targetSig].
///
/// Pass 1 deletes blocks in halving chunk sizes; pass 2 zeroes individual
/// non-essential bytes. Every candidate re-executes [testFn].
Uint8List minimizeCrashingInput(
  Uint8List input,
  String targetSig,
  FuzzOutcome Function(Uint8List candidate) testFn,
) {
  var current = Uint8List.fromList(input);
  if (current.isEmpty) return current;

  // Pass 1: recursive block deletion with halving chunk sizes.
  var chunkSize = current.length ~/ 2;
  while (chunkSize > 0) {
    var offset = 0;
    while (offset < current.length) {
      final actualLen = math.min(chunkSize, current.length - offset);
      final candidate = Uint8List(current.length - actualLen);
      candidate.setRange(0, offset, current, 0);
      candidate.setRange(offset, candidate.length, current, offset + actualLen);

      final outcome = testFn(candidate);
      if (outcome is OutcomeCrash &&
          crashSignature(outcome.kind, outcome.frames) == targetSig) {
        current = candidate;
        continue;
      }
      offset += chunkSize;
    }
    chunkSize ~/= 2;
  }

  // Pass 2: byte zeroing / normalization.
  for (var i = 0; i < current.length; i++) {
    if (current[i] == 0) continue;
    final originalByte = current[i];
    current[i] = 0;
    final outcome = testFn(current);
    var matched = false;
    if (outcome is OutcomeCrash &&
        crashSignature(outcome.kind, outcome.frames) == targetSig) {
      matched = true;
    }
    if (!matched) {
      current[i] = originalByte;
    }
  }

  return current;
}

List<int> _traceOf(CoverageBitmap stepCov) {
  final trace = <int>[];
  for (var edge = 0; edge < coverageMapSize; edge++) {
    if (stepCov.hasEdge(edge)) trace.add(edge);
  }
  return trace;
}

/// Runs a coverage-guided campaign over [initialCorpus].
///
/// Triage admits passing seeds with novel coverage (first seed always
/// admitted) and records triage crashes; the loop then spends per-seed
/// energy on mutated candidates with occasional splicing, admitting novel
/// coverage and deduplicating crashes by signature.
FuzzReport fuzzTarget(
  List<Uint8List> initialCorpus,
  FuzzOptions options,
  FuzzOutcome Function(Uint8List input, CoverageBitmap cov) target,
) {
  final maxIterations = options.maxIterations;
  final maxInputLen = options.maxInputLen;
  final baseEnergy = options.baseEnergy;

  final rng = SplitMix64(options.seed);
  final cumulativeCoverage = CoverageBitmap();
  final corpus = <CorpusSeed>[];
  final crashes = <CrashRecord>[];

  // 1. Initial corpus triage.
  final seeds = initialCorpus.isEmpty ? [Uint8List(1)] : initialCorpus;

  for (final s in seeds) {
    final stepCov = CoverageBitmap();
    final outcome = target(s, stepCov);
    if (outcome is OutcomePass) {
      final (hasNew, newCount) =
          cumulativeCoverage.admitTrace(_traceOf(stepCov));
      if (hasNew || corpus.isEmpty) {
        corpus.add(CorpusSeed(
          data: Uint8List.fromList(s),
          newEdges: newCount,
          generation: 0,
        ));
      }
    } else if (outcome is OutcomeCrash) {
      final sig = crashSignature(outcome.kind, outcome.frames);
      crashes.add(CrashRecord(
        signature: sig,
        kind: outcome.kind,
        frames: outcome.frames,
        minimalInput: Uint8List.fromList(s),
        hits: 1,
      ));
    }
  }

  // 2. Fuzzing loop.
  var iterations = 0;
  while (iterations < maxIterations && corpus.isNotEmpty) {
    final seedIdx = rng.bounded(corpus.length);
    final chosenSeed = corpus[seedIdx];
    final totalCov = cumulativeCoverage.count();
    final energy = calculateEnergy(
        baseEnergy, chosenSeed.data.length, chosenSeed.newEdges, totalCov);

    for (var e = 0; e < energy; e++) {
      if (iterations >= maxIterations) break;
      iterations++;

      final mutationSteps = 1 + rng.bounded(4);
      final subSeedVal = rng.nextU64();
      var candidate =
          mutateSequence(chosenSeed.data, subSeedVal, mutationSteps);

      // Splice with another seed if the corpus holds 2+ and the draw says so.
      if (corpus.length > 1 && rng.bounded(4) == 0) {
        final donorIdx = rng.bounded(corpus.length);
        final donor = corpus[donorIdx].data;
        final tSplit = rng.bounded(candidate.length + 1);
        final dSplit = rng.bounded(donor.length + 1);
        candidate = splice(candidate, donor, tSplit, dSplit);
      }

      if (candidate.length > maxInputLen) {
        candidate = candidate.sublist(0, maxInputLen);
      }

      final stepCov = CoverageBitmap();
      final outcome = target(candidate, stepCov);

      if (outcome is OutcomePass) {
        final (hasNew, newCount) =
            cumulativeCoverage.admitTrace(_traceOf(stepCov));
        if (hasNew) {
          corpus.add(CorpusSeed(
            data: candidate,
            newEdges: newCount,
            generation: chosenSeed.generation + 1,
          ));
        }
      } else if (outcome is OutcomeCrash) {
        final sig = crashSignature(outcome.kind, outcome.frames);
        CrashRecord? existing;
        for (final c in crashes) {
          if (c.signature == sig) {
            existing = c;
            break;
          }
        }
        if (existing != null) {
          existing.hits++;
          if (candidate.length < existing.minimalInput.length) {
            existing.minimalInput = candidate;
          }
        } else {
          crashes.add(CrashRecord(
            signature: sig,
            kind: outcome.kind,
            frames: outcome.frames,
            minimalInput: candidate,
            hits: 1,
          ));
        }
      }
    }
  }

  return FuzzReport(
    iterations: iterations,
    admittedSeeds: corpus.length,
    totalEdges: cumulativeCoverage.count(),
    crashes: crashes,
  );
}
