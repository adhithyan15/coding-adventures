import 'dart:convert';

import 'package:coding_adventures_json_lexer/json_lexer.dart';
import 'package:coding_adventures_lexer/lexer.dart';
import 'package:coding_adventures_rank_statistics/coding_adventures_rank_statistics.dart';

final BigInt _minI64 = -(BigInt.one << 63);
final BigInt _maxI64 = (BigInt.one << 63) - BigInt.one;
const int _maxPairs = 10000;
const int _maxJsonBytes = 4 * 1024 * 1024;

BigInt signed64(Object? value, String field) {
  if (value is! BigInt || value < _minI64 || value > _maxI64) {
    throw FormatException('$field must be a signed 64-bit integer');
  }
  return value;
}

class Config {
  Config(
      {this.pairs = 40,
      this.confidencePercent = 95,
      this.minimumFactor = 1.05,
      BigInt? seed,
      BigInt? targetNs,
      BigInt? maxBatch})
      : seed = seed ?? BigInt.zero,
        targetNs = targetNs ?? BigInt.from(100000),
        maxBatch = maxBatch ?? BigInt.from(65536);
  final int pairs;
  final int confidencePercent;
  final double minimumFactor;
  final BigInt seed;
  final BigInt targetNs;
  final BigInt maxBatch;
  void validate() {
    if (pairs < 1 || pairs > _maxPairs)
      throw ArgumentError('pairs must be 1..10000');
    if (confidencePercent < 1 || confidencePercent > 99)
      throw ArgumentError('confidence must be 1..99');
    if (!minimumFactor.isFinite || minimumFactor <= 1)
      throw ArgumentError('factor must be finite and > 1');
    signed64(seed, 'seed');
    signed64(targetNs, 'target_ns');
    signed64(maxBatch, 'max_batch');
    if (targetNs <= BigInt.zero || maxBatch <= BigInt.zero)
      throw ArgumentError('target and max batch must be positive');
  }
}

class Pair {
  const Pair(this.baselineNs, this.candidateNs, this.order);
  final BigInt baselineNs;
  final BigInt candidateNs;
  final String order;
  Map<String, Object> toMap() =>
      {'baseline_ns': baselineNs, 'candidate_ns': candidateNs, 'order': order};
}

class Analysis {
  const Analysis(this.ratio, this.interval, this.verdict);
  final double ratio;
  final List<double>? interval;
  final String verdict;
  Map<String, Object?> toMap() =>
      {'ratio': ratio, 'interval': interval, 'verdict': verdict};
}

class Report {
  const Report(this.config, this.batchSize, this.pairs, this.analysis,
      [this.metadata = const {}]);
  final Config config;
  final BigInt batchSize;
  final List<Pair> pairs;
  final Analysis analysis;
  final Map<String, Object?> metadata;
  Map<String, Object?> toMap() => {
        ...metadata,
        'config': {
          'pairs': config.pairs,
          'confidence_percent': config.confidencePercent,
          'minimum_factor': config.minimumFactor,
          'seed': config.seed,
          'target_ns': config.targetNs,
          'max_batch': config.maxBatch,
        },
        'batch_size': batchSize,
        'pairs': pairs.map((p) => p.toMap()).toList(),
        'analysis': analysis.toMap()
      };
  String toJson() => _writeJson(toMap());
  static Report fromJson(String text) => _decodeReport(text);
  static Report fromBytes(List<int> bytes) {
    if (bytes.length > _maxJsonBytes) {
      throw FormatException('input exceeds 4 MiB');
    }
    return _decodeReport(utf8.decode(bytes));
  }
}

Analysis analyze(List<Pair> pairs, int confidencePercent, double factor) {
  if (pairs.length > _maxPairs) {
    throw ArgumentError('at most 10000 pairs are supported');
  }
  if (pairs.isEmpty ||
      confidencePercent < 1 ||
      confidencePercent > 99 ||
      !factor.isFinite ||
      factor <= 1) {
    throw ArgumentError('invalid analysis inputs');
  }
  var baselineFirst = 0;
  final ratios = <double>[];
  for (final pair in pairs) {
    if (pair.baselineNs <= BigInt.zero ||
        pair.candidateNs <= BigInt.zero ||
        !{'baseline-first', 'candidate-first'}.contains(pair.order))
      throw ArgumentError('invalid pair');
    if (pair.order == 'baseline-first') baselineFirst++;
    ratios.add(pair.candidateNs.toDouble() / pair.baselineNs.toDouble());
  }
  if (baselineFirst != (pairs.length + 1) ~/ 2)
    throw ArgumentError('unbalanced order');
  final ratio = median(ratios);
  final ranks = medianInterval(ratios, confidencePercent);
  if (ranks == null) return Analysis(ratio, null, 'inconclusive');
  final lo = ranks.low, hi = ranks.high, fast = 1 / factor;
  final verdict = hi < fast
      ? 'faster'
      : lo > factor
          ? 'slower'
          : lo >= fast && hi <= factor
              ? 'equivalent'
              : 'inconclusive';
  return Analysis(ratio, [lo, hi], verdict);
}

typedef Batch = BigInt Function(BigInt operations);
typedef Clock = BigInt Function();

Report collect(
    {required Config config,
    required BigInt expectedChecksum,
    required Batch baseline,
    required Batch candidate,
    required Clock now,
    Map<String, Object?> metadata = const {}}) {
  config.validate();
  signed64(expectedChecksum, 'expected checksum');
  void verify(Batch fn, BigInt size) {
    final expected = signed64(expectedChecksum * size, 'checksum product');
    if (fn(size) != expected) throw StateError('checksum mismatch');
  }

  verify(baseline, BigInt.one);
  verify(candidate, BigInt.one);
  var last = BigInt.from(-1);
  BigInt read() {
    final tick = signed64(now(), 'clock tick');
    if (tick < BigInt.zero || (last >= BigInt.zero && tick < last))
      throw StateError('clock regressed');
    last = tick;
    return tick;
  }

  BigInt measure(Batch fn, BigInt size, {required bool calibration}) {
    final start = read();
    final sum = fn(size);
    final stop = read();
    if (sum != signed64(expectedChecksum * size, 'checksum product'))
      throw StateError('checksum mismatch');
    final elapsed = stop - start;
    if (!calibration && elapsed <= BigInt.zero)
      throw StateError('measured duration must be positive');
    return elapsed;
  }

  var size = BigInt.one;
  while (true) {
    final baseElapsed = measure(baseline, size, calibration: true);
    final candidateElapsed = measure(candidate, size, calibration: true);
    if (baseElapsed >= config.targetNs && candidateElapsed >= config.targetNs)
      break;
    if (size >= config.maxBatch)
      throw StateError('calibration limit exhausted');
    size = size * BigInt.two > config.maxBatch
        ? config.maxBatch
        : size * BigInt.two;
  }
  final order = List<bool>.generate(config.pairs, (index) => index.isEven);
  // Xorshift has an absorbing zero state. Keep the public seed and report
  // unchanged, but map its default zero seed to a documented nonzero state.
  var randomState = config.seed.toUnsigned(64);
  if (randomState == BigInt.zero) {
    randomState = BigInt.parse('9E3779B97F4A7C15', radix: 16);
  }
  final mask64 = (BigInt.one << 64) - BigInt.one;
  for (var i = order.length - 1; i > 0; i--) {
    randomState ^= (randomState << 13) & mask64;
    randomState ^= randomState >> 7;
    randomState ^= (randomState << 17) & mask64;
    final j = (randomState % BigInt.from(i + 1)).toInt();
    final v = order[i];
    order[i] = order[j];
    order[j] = v;
  }
  final pairs = <Pair>[];
  for (final baselineFirst in order) {
    final a = baselineFirst
        ? measure(baseline, size, calibration: false)
        : measure(candidate, size, calibration: false);
    final b = baselineFirst
        ? measure(candidate, size, calibration: false)
        : measure(baseline, size, calibration: false);
    pairs.add(baselineFirst
        ? Pair(a, b, 'baseline-first')
        : Pair(b, a, 'candidate-first'));
  }
  return Report(config, size, pairs,
      analyze(pairs, config.confidencePercent, config.minimumFactor), metadata);
}

List<BigInt> makeValues() =>
    List<BigInt>.generate(4096, (i) => BigInt.from((17 * i + 3) % 251));
BigInt indexedBatch(List<BigInt> values, BigInt operations) =>
    _work(values, operations, true);
BigInt iteratorBatch(List<BigInt> values, BigInt operations) =>
    _work(values, operations, false);
BigInt _work(List<BigInt> values, BigInt operations, bool indexed) {
  if (values.isEmpty) throw ArgumentError('values must not be empty');
  if (operations < BigInt.zero)
    throw ArgumentError('operations must be nonnegative');
  if (operations > _maxI64)
    throw ArgumentError('operations must fit signed 64-bit');
  BigInt sum = BigInt.zero;
  for (var op = BigInt.zero; op < operations; op += BigInt.one) {
    BigInt one = BigInt.zero;
    if (indexed) {
      for (var i = 0; i < values.length; i++) {
        one += values[i];
        if (one < _minI64 || one > _maxI64) {
          throw RangeError('per-operation checksum overflow');
        }
      }
    } else {
      for (final value in values) {
        one += value;
        if (one < _minI64 || one > _maxI64) {
          throw RangeError('per-operation checksum overflow');
        }
      }
    }
    if (one < _minI64 || one > _maxI64)
      throw RangeError('per-operation checksum overflow');
    sum += one;
    if (sum < _minI64 || sum > _maxI64)
      throw RangeError('batch checksum overflow');
  }
  return sum;
}

String _writeJson(Object? value) {
  if (value == null || value is bool || value is num) return jsonEncode(value);
  if (value is BigInt) return value.toString();
  if (value is String) return jsonEncode(value);
  if (value is List) return '[${value.map(_writeJson).join(',')}]';
  if (value is Map<String, Object?>)
    return '{${value.entries.map((e) => '${jsonEncode(e.key)}:${_writeJson(e.value)}').join(',')}}';
  throw ArgumentError('unsupported JSON value ${value.runtimeType}');
}

String _decodeJsonStringToken(String raw) {
  var index = 0;
  int escapedCodeUnit(int start) {
    if (start + 6 > raw.length ||
        raw.codeUnitAt(start) != 0x5c ||
        raw.codeUnitAt(start + 1) != 0x75) {
      throw const FormatException('malformed Unicode escape');
    }
    return int.parse(raw.substring(start + 2, start + 6), radix: 16);
  }

  while (index < raw.length) {
    final codeUnit = raw.codeUnitAt(index);
    if (codeUnit == 0x5c) {
      if (index + 1 < raw.length && raw.codeUnitAt(index + 1) == 0x75) {
        final escaped = escapedCodeUnit(index);
        if (escaped >= 0xd800 && escaped <= 0xdbff) {
          if (index + 12 > raw.length || raw.codeUnitAt(index + 6) != 0x5c) {
            throw const FormatException('unpaired high surrogate escape');
          }
          final low = escapedCodeUnit(index + 6);
          if (low < 0xdc00 || low > 0xdfff) {
            throw const FormatException('unpaired high surrogate escape');
          }
          index += 12;
        } else if (escaped >= 0xdc00 && escaped <= 0xdfff) {
          throw const FormatException('unpaired low surrogate escape');
        } else {
          index += 6;
        }
      } else {
        index += 2;
      }
    } else if (codeUnit >= 0xd800 && codeUnit <= 0xdbff) {
      if (index + 1 >= raw.length) {
        throw const FormatException('unpaired high surrogate');
      }
      final low = raw.codeUnitAt(index + 1);
      if (low < 0xdc00 || low > 0xdfff) {
        throw const FormatException('unpaired high surrogate');
      }
      index += 2;
    } else if (codeUnit >= 0xdc00 && codeUnit <= 0xdfff) {
      throw const FormatException('unpaired low surrogate');
    } else {
      index++;
    }
  }
  return jsonDecode('"$raw"') as String;
}

// Lexer tokens preserve the original numeric spelling; no large integer is
// ever decoded through a JavaScript Number.
dynamic _decodeReport(String source) {
  if (utf8.encode(source).length > _maxJsonBytes)
    throw FormatException('input exceeds 4 MiB');
  final tokens = tokenizeJson(source);
  var index = 0, nodes = 0, depth = 0;
  Token peek() => tokens[index];
  Token take(String type) {
    final token = peek();
    if (token.type != type)
      throw FormatException('expected $type at ${token.line}:${token.column}');
    index++;
    return token;
  }

  dynamic value() {
    nodes++;
    if (nodes > 65536) throw FormatException('too many JSON values');
    final token = peek();
    if (token.type == 'LBRACE') {
      if (++depth > 128) throw FormatException('too deeply nested');
      index++;
      final result = <String, dynamic>{};
      if (peek().type != 'RBRACE') {
        while (true) {
          final keyToken = take('STRING');
          final key = _decodeJsonStringToken(keyToken.value);
          if (result.containsKey(key))
            throw FormatException('duplicate object key');
          take('COLON');
          result[key] = value();
          if (peek().type != 'COMMA') break;
          index++;
        }
      }
      take('RBRACE');
      depth--;
      return result;
    }
    if (token.type == 'LBRACKET') {
      if (++depth > 128) throw FormatException('too deeply nested');
      index++;
      final result = <dynamic>[];
      if (peek().type != 'RBRACKET') {
        while (true) {
          result.add(value());
          if (peek().type != 'COMMA') break;
          index++;
        }
      }
      take('RBRACKET');
      depth--;
      return result;
    }
    index++;
    if (token.type == 'STRING') return _decodeJsonStringToken(token.value);
    if (token.type == 'TRUE') return true;
    if (token.type == 'FALSE') return false;
    if (token.type == 'NULL') return null;
    if (token.type == 'NUMBER') {
      if (!RegExp(r'^-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?$')
          .hasMatch(token.value)) throw FormatException('invalid JSON number');
      if (!token.value.contains(RegExp(r'[.eE]'))) {
        if (token.value == '-0')
          throw FormatException('negative zero is forbidden');
        return BigInt.parse(token.value);
      }
      final parsed = double.parse(token.value);
      if (!parsed.isFinite) throw FormatException('nonfinite number');
      return parsed;
    }
    throw FormatException('unexpected JSON token ${token.type}');
  }

  final decoded = value();
  if (peek().type != 'EOF') throw FormatException('trailing JSON tokens');
  if (decoded is! Map<String, dynamic>)
    throw FormatException('report root must be an object');
  return _reportFromMap(decoded);
}

Report _reportFromMap(Map<String, dynamic> map) {
  const topAllowed = {
    'config',
    'batch_size',
    'pairs',
    'analysis',
    'workload',
    'invocation',
    'revision',
    'runtime',
    'os',
    'arch',
    'profile'
  };
  if (map.keys.any((key) => !topAllowed.contains(key)))
    throw FormatException('unknown report field');
  final configMap = map['config'];
  final pairValues = map['pairs'];
  final analysisMap = map['analysis'];
  if (configMap is! Map<String, dynamic> ||
      pairValues is! List ||
      analysisMap is! Map<String, dynamic>)
    throw FormatException('report fields have wrong types');
  const configKeys = {
    'pairs',
    'confidence_percent',
    'minimum_factor',
    'seed',
    'target_ns',
    'max_batch'
  };
  if (configMap.length != configKeys.length ||
      configMap.keys.any((key) => !configKeys.contains(key)))
    throw FormatException('invalid config fields');
  int smallInt(Object? value, String field, int min, int max) {
    if (value is! BigInt ||
        value < BigInt.from(min) ||
        value > BigInt.from(max)) throw FormatException('$field out of range');
    return value.toInt();
  }

  final factorValue = configMap['minimum_factor'];
  final factor = factorValue is BigInt ? factorValue.toDouble() : factorValue;
  if (factor is! double || !factor.isFinite)
    throw FormatException('minimum_factor must be finite number');
  final config = Config(
      pairs: smallInt(configMap['pairs'], 'pairs', 1, 10000),
      confidencePercent: smallInt(
          configMap['confidence_percent'], 'confidence_percent', 1, 99),
      minimumFactor: factor,
      seed: signed64(configMap['seed'], 'seed'),
      targetNs: signed64(configMap['target_ns'], 'target_ns'),
      maxBatch: signed64(configMap['max_batch'], 'max_batch'));
  config.validate();
  if (pairValues.length != config.pairs || pairValues.isEmpty)
    throw FormatException('pair count mismatch');
  final pairs = <Pair>[];
  for (final value in pairValues) {
    if (value is! Map<String, dynamic> ||
        value.length != 3 ||
        value.keys
            .toSet()
            .difference({'baseline_ns', 'candidate_ns', 'order'}).isNotEmpty)
      throw FormatException('invalid pair');
    final order = value['order'];
    if (order is! String) throw FormatException('invalid order');
    pairs.add(Pair(signed64(value['baseline_ns'], 'baseline_ns'),
        signed64(value['candidate_ns'], 'candidate_ns'), order));
  }
  final batch = signed64(map['batch_size'], 'batch_size');
  if (batch <= BigInt.zero || batch > config.maxBatch)
    throw FormatException('invalid batch_size');
  final recomputed =
      analyze(pairs, config.confidencePercent, config.minimumFactor);
  const analysisKeys = {'ratio', 'interval', 'verdict'};
  if (analysisMap.length != 3 ||
      analysisMap.keys.any((key) => !analysisKeys.contains(key)))
    throw FormatException('invalid analysis fields');
  final ratioValue = analysisMap['ratio'];
  final verdict = analysisMap['verdict'];
  final intervalValue = analysisMap['interval'];
  double jsonNumber(Object? value) {
    if (value is double) return value;
    if (value is BigInt) return value.toDouble();
    throw FormatException('expected JSON number');
  }

  final ratio = jsonNumber(ratioValue);
  if (!ratio.isFinite ||
      verdict is! String ||
      ratio != recomputed.ratio ||
      verdict != recomputed.verdict)
    throw FormatException('analysis does not match observations');
  List<double>? interval;
  if (intervalValue != null) {
    if (intervalValue is! List || intervalValue.length != 2)
      throw FormatException('invalid interval');
    interval = intervalValue.map(jsonNumber).toList();
    if (interval.any((v) => !v.isFinite))
      throw FormatException('invalid interval');
  }
  if (!_sameInterval(interval, recomputed.interval))
    throw FormatException('interval does not match observations');
  const metadataKeys = [
    'workload',
    'invocation',
    'revision',
    'runtime',
    'os',
    'arch'
  ];
  final presentMetadata = metadataKeys.where(map.containsKey).length;
  final hasProfile = map.containsKey('profile');
  if (presentMetadata == 0) {
    if (hasProfile) throw FormatException('profile requires complete metadata');
    return Report(config, batch, List.unmodifiable(pairs), recomputed);
  }
  if (presentMetadata != metadataKeys.length)
    throw FormatException('standard metadata must be complete or absent');

  final metadata = <String, Object?>{};
  for (final key in ['workload', 'revision', 'runtime', 'os', 'arch']) {
    final v = map[key];
    if (v is! String || v.isEmpty || v.contains(String.fromCharCode(0)))
      throw FormatException('invalid metadata $key');
    if (key == 'workload' && !_validWorkloadIdentifier(v))
      throw FormatException('workload must be nonblank and contain no NUL');
    metadata[key] = v;
  }
  final invocation = map['invocation'];
  if (invocation is! List ||
      invocation.any((e) => e is! String || e.contains(String.fromCharCode(0))))
    throw FormatException('invalid invocation');
  metadata['invocation'] = invocation;
  if (hasProfile) {
    final v = map['profile'];
    if (v is! String || v.isEmpty || v.contains(String.fromCharCode(0)))
      throw FormatException('invalid profile');
    metadata['profile'] = v;
  }
  return Report(config, batch, List.unmodifiable(pairs), recomputed, metadata);
}

bool _validWorkloadIdentifier(String value) {
  if (value.isEmpty || value.contains(String.fromCharCode(0))) return false;
  return !value.runes
      .every((rune) => rune == 0x20 || (rune >= 0x09 && rune <= 0x0d));
}

bool _sameInterval(List<double>? a, List<double>? b) =>
    a == null ? b == null : b != null && a[0] == b[0] && a[1] == b[1];
