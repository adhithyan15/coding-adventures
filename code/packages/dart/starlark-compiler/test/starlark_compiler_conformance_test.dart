import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:coding_adventures_starlark_compiler/starlark_compiler.dart';
import 'package:test/test.dart';

// Independent verification tables: literal constants specified outside the provider.
const List<(int, String, String)> _independentOpcodes = [
  (0x01, 'LoadConst', 'stack'),
  (0x02, 'Pop', 'stack'),
  (0x03, 'Dup', 'stack'),
  (0x04, 'LoadNone', 'stack'),
  (0x05, 'LoadTrue', 'stack'),
  (0x06, 'LoadFalse', 'stack'),
  (0x10, 'StoreName', 'variable'),
  (0x11, 'LoadName', 'variable'),
  (0x12, 'StoreLocal', 'variable'),
  (0x13, 'LoadLocal', 'variable'),
  (0x14, 'StoreClosure', 'variable'),
  (0x15, 'LoadClosure', 'variable'),
  (0x20, 'Add', 'arithmetic'),
  (0x21, 'Sub', 'arithmetic'),
  (0x22, 'Mul', 'arithmetic'),
  (0x23, 'Div', 'arithmetic'),
  (0x24, 'FloorDiv', 'arithmetic'),
  (0x25, 'Mod', 'arithmetic'),
  (0x26, 'Power', 'arithmetic'),
  (0x27, 'Negate', 'arithmetic'),
  (0x28, 'BitAnd', 'arithmetic'),
  (0x29, 'BitOr', 'arithmetic'),
  (0x2A, 'BitXor', 'arithmetic'),
  (0x2B, 'BitNot', 'arithmetic'),
  (0x2C, 'LShift', 'arithmetic'),
  (0x2D, 'RShift', 'arithmetic'),
  (0x30, 'CmpEq', 'comparison'),
  (0x31, 'CmpNe', 'comparison'),
  (0x32, 'CmpLt', 'comparison'),
  (0x33, 'CmpGt', 'comparison'),
  (0x34, 'CmpLe', 'comparison'),
  (0x35, 'CmpGe', 'comparison'),
  (0x36, 'CmpIn', 'comparison'),
  (0x37, 'CmpNotIn', 'comparison'),
  (0x38, 'Not', 'comparison'),
  (0x40, 'Jump', 'control_flow'),
  (0x41, 'JumpIfFalse', 'control_flow'),
  (0x42, 'JumpIfTrue', 'control_flow'),
  (0x43, 'JumpIfFalseOrPop', 'control_flow'),
  (0x44, 'JumpIfTrueOrPop', 'control_flow'),
  (0x50, 'MakeFunction', 'function'),
  (0x51, 'CallFunction', 'function'),
  (0x52, 'CallFunctionKw', 'function'),
  (0x53, 'Return', 'function'),
  (0x60, 'BuildList', 'collection'),
  (0x61, 'BuildDict', 'collection'),
  (0x62, 'BuildTuple', 'collection'),
  (0x63, 'ListAppend', 'collection'),
  (0x64, 'DictSet', 'collection'),
  (0x70, 'LoadSubscript', 'subscript_attribute'),
  (0x71, 'StoreSubscript', 'subscript_attribute'),
  (0x72, 'LoadAttr', 'subscript_attribute'),
  (0x73, 'StoreAttr', 'subscript_attribute'),
  (0x74, 'LoadSlice', 'subscript_attribute'),
  (0x80, 'GetIter', 'iteration'),
  (0x81, 'ForIter', 'iteration'),
  (0x82, 'UnpackSequence', 'iteration'),
  (0x90, 'LoadModule', 'module'),
  (0x91, 'ImportFrom', 'module'),
  (0xA0, 'Print', 'io'),
  (0xFF, 'Halt', 'vm_control'),
];

const Map<String, List<(String, String)>> _independentOperators = {
  'binary': [
    ('+', 'Add'),
    ('-', 'Sub'),
    ('*', 'Mul'),
    ('/', 'Div'),
    ('//', 'FloorDiv'),
    ('%', 'Mod'),
    ('**', 'Power'),
    ('&', 'BitAnd'),
    ('|', 'BitOr'),
    ('^', 'BitXor'),
    ('<<', 'LShift'),
    ('>>', 'RShift'),
  ],
  'compare': [
    ('==', 'CmpEq'),
    ('!=', 'CmpNe'),
    ('<', 'CmpLt'),
    ('>', 'CmpGt'),
    ('<=', 'CmpLe'),
    ('>=', 'CmpGe'),
    ('in', 'CmpIn'),
    ('not in', 'CmpNotIn'),
  ],
  'augmented_assign': [
    ('+=', 'Add'),
    ('-=', 'Sub'),
    ('*=', 'Mul'),
    ('/=', 'Div'),
    ('//=', 'FloorDiv'),
    ('%=', 'Mod'),
    ('**=', 'Power'),
    ('&=', 'BitAnd'),
    ('|=', 'BitOr'),
    ('^=', 'BitXor'),
    ('<<=', 'LShift'),
    ('>>=', 'RShift'),
  ],
  'unary': [
    ('-', 'Negate'),
    ('~', 'BitNot'),
  ],
};

const List<(String, String)> _independentUnknowns = [
  ('binary', '@'),
  ('binary', '++'),
  ('binary', '＋'),
  ('compare', 'IN'),
  ('compare', 'not  in'),
  ('augmented_assign', ':='),
  ('unary', '+'),
  ('unary', 'not'),
];

const List<String> _groupOrder = [
  'binary',
  'compare',
  'augmented_assign',
  'unary',
];

class _StpRow {
  _StpRow({
    required this.byteNumber,
    this.symbol,
    this.category,
    this.absent = false,
  });

  final int byteNumber;
  final String? symbol;
  final String? category;
  final bool absent;
}

class _StpOperator {
  _StpOperator(this.operatorSymbol, this.opcodeSymbol);
  final String operatorSymbol;
  final String opcodeSymbol;
}

class _StpUnknown {
  _StpUnknown(this.group, this.operatorSymbol);
  final String group;
  final String operatorSymbol;
}

class _StpCorpus {
  final List<_StpRow> opcodes = [];
  final List<_StpRow> decode = [];
  final Map<String, List<_StpOperator>> operators = {
    for (final g in _groupOrder) g: <_StpOperator>[],
  };
  final List<_StpUnknown> unknowns = [];
}

class _RawNumber {
  _RawNumber(this.raw, this.value);
  final String raw;
  final num value;
}

/// Strict, bounded JSON parser detecting duplicate keys, depth limit, and array limits.
class _StrictJsonParser {
  _StrictJsonParser(this.source);
  final String source;
  int _index = 0;

  dynamic parse({int maxDepth = maxJsonNestingDepth}) {
    _skipWhitespace();
    if (_index >= source.length) {
      throw const InvalidCorpusException.malformedTransport();
    }
    final value = _parseValue(1, maxDepth);
    _skipWhitespace();
    if (_index < source.length) {
      throw const InvalidCorpusException.malformedTransport();
    }
    return value;
  }

  void _skipWhitespace() {
    while (_index < source.length) {
      final ch = source.codeUnitAt(_index);
      if (ch == 0x20 || ch == 0x09 || ch == 0x0A || ch == 0x0D) {
        _index++;
      } else {
        break;
      }
    }
  }

  dynamic _parseValue(int depth, int maxDepth) {
    if (depth > maxDepth) {
      throw const InvalidCorpusException.schemaViolation();
    }
    _skipWhitespace();
    if (_index >= source.length) {
      throw const InvalidCorpusException.malformedTransport();
    }
    final ch = source.codeUnitAt(_index);
    if (ch == 0x7B) {
      return _parseObject(depth, maxDepth);
    } else if (ch == 0x5B) {
      return _parseArray(depth, maxDepth);
    } else if (ch == 0x22) {
      return _parseString();
    } else if (ch == 0x74) {
      return _parseLiteral('true', true);
    } else if (ch == 0x66) {
      return _parseLiteral('false', false);
    } else if (ch == 0x6E) {
      return _parseLiteral('null', null);
    } else if (ch == 0x2D || (ch >= 0x30 && ch <= 0x39)) {
      return _parseNumber();
    } else {
      throw const InvalidCorpusException.malformedTransport();
    }
  }

  Map<String, dynamic> _parseObject(int depth, int maxDepth) {
    _index++; // Skip '{'
    final map = <String, dynamic>{};
    _skipWhitespace();
    if (_index < source.length && source.codeUnitAt(_index) == 0x7D) {
      _index++; // Skip '}'
      return map;
    }
    while (true) {
      _skipWhitespace();
      if (_index >= source.length || source.codeUnitAt(_index) != 0x22) {
        throw const InvalidCorpusException.malformedTransport();
      }
      final key = _parseString();
      if (map.containsKey(key)) {
        throw const InvalidCorpusException.schemaViolation();
      }
      _skipWhitespace();
      if (_index >= source.length || source.codeUnitAt(_index) != 0x3A) {
        throw const InvalidCorpusException.malformedTransport();
      }
      _index++; // Skip ':'
      final value = _parseValue(depth + 1, maxDepth);
      map[key] = value;
      _skipWhitespace();
      if (_index >= source.length) {
        throw const InvalidCorpusException.malformedTransport();
      }
      final next = source.codeUnitAt(_index);
      if (next == 0x2C) {
        _index++; // Skip ','
      } else if (next == 0x7D) {
        _index++; // Skip '}'
        break;
      } else {
        throw const InvalidCorpusException.malformedTransport();
      }
    }
    return map;
  }

  List<dynamic> _parseArray(int depth, int maxDepth) {
    _index++; // Skip '['
    final list = <dynamic>[];
    _skipWhitespace();
    if (_index < source.length && source.codeUnitAt(_index) == 0x5D) {
      _index++; // Skip ']'
      return list;
    }
    while (true) {
      if (list.length >= 512) {
        throw const InvalidCorpusException.schemaViolation();
      }
      final value = _parseValue(depth + 1, maxDepth);
      list.add(value);
      _skipWhitespace();
      if (_index >= source.length) {
        throw const InvalidCorpusException.malformedTransport();
      }
      final next = source.codeUnitAt(_index);
      if (next == 0x2C) {
        _index++; // Skip ','
      } else if (next == 0x5D) {
        _index++; // Skip ']'
        break;
      } else {
        throw const InvalidCorpusException.malformedTransport();
      }
    }
    return list;
  }

  String _parseString() {
    _index++; // Skip opening '"'
    final sb = StringBuffer();
    while (_index < source.length) {
      final ch = source.codeUnitAt(_index);
      if (ch == 0x22) {
        _index++; // Closing '"'
        return sb.toString();
      }
      if (ch == 0x5C) {
        _index++;
        if (_index >= source.length) {
          throw const InvalidCorpusException.malformedTransport();
        }
        final esc = source.codeUnitAt(_index);
        _index++;
        switch (esc) {
          case 0x22:
            sb.write('"');
          case 0x5C:
            sb.write('\\');
          case 0x2F:
            sb.write('/');
          case 0x62:
            sb.write('\b');
          case 0x66:
            sb.write('\f');
          case 0x6E:
            sb.write('\n');
          case 0x72:
            sb.write('\r');
          case 0x74:
            sb.write('\t');
          case 0x75:
            if (_index + 4 > source.length) {
              throw const InvalidCorpusException.malformedTransport();
            }
            final hex = source.substring(_index, _index + 4);
            _index += 4;
            final code = int.tryParse(hex, radix: 16);
            if (code == null) {
              throw const InvalidCorpusException.malformedTransport();
            }
            if (code >= minHighSurrogate && code <= maxHighSurrogate) {
              if (_index + 6 > source.length ||
                  source.codeUnitAt(_index) != 0x5C ||
                  source.codeUnitAt(_index + 1) != 0x75) {
                throw const InvalidCorpusException.malformedTransport();
              }
              final lowHex = source.substring(_index + 2, _index + 6);
              _index += 6;
              final lowCode = int.tryParse(lowHex, radix: 16);
              if (lowCode == null ||
                  lowCode < minLowSurrogate ||
                  lowCode > maxLowSurrogate) {
                throw const InvalidCorpusException.malformedTransport();
              }
              final scalar = 0x10000 +
                  ((code - minHighSurrogate) << 10) +
                  (lowCode - minLowSurrogate);
              sb.writeCharCode(scalar);
            } else if (code >= minLowSurrogate && code <= maxLowSurrogate) {
              throw const InvalidCorpusException.malformedTransport();
            } else {
              sb.writeCharCode(code);
            }
          default:
            throw const InvalidCorpusException.malformedTransport();
        }
      } else {
        sb.writeCharCode(ch);
        _index++;
      }
    }
    throw const InvalidCorpusException.malformedTransport();
  }

  dynamic _parseLiteral(String expected, dynamic value) {
    if (_index + expected.length > source.length ||
        source.substring(_index, _index + expected.length) != expected) {
      throw const InvalidCorpusException.malformedTransport();
    }
    _index += expected.length;
    return value;
  }

  _RawNumber _parseNumber() {
    final start = _index;
    if (source.codeUnitAt(_index) == 0x2D) {
      _index++;
    }
    while (_index < source.length) {
      final ch = source.codeUnitAt(_index);
      if ((ch >= 0x30 && ch <= 0x39) ||
          ch == 0x2E ||
          ch == 0x65 ||
          ch == 0x45 ||
          ch == 0x2B ||
          ch == 0x2D) {
        _index++;
      } else {
        break;
      }
    }
    final raw = source.substring(start, _index);
    final val = num.tryParse(raw);
    if (val == null) {
      throw const InvalidCorpusException.malformedTransport();
    }
    return _RawNumber(raw, val);
  }
}

/// Inspects raw bytes for transport integrity: size ceiling, UTF-8 validity, BOM, and surrogate escapes.
String _stpScalars(Uint8List bytes) {
  if (bytes.length > maxCorpusBytes) {
    throw const InvalidCorpusException.byteLimitExceeded();
  }

  // Reject UTF-8 BOM
  if (bytes.length >= 3 &&
      bytes[0] == 0xEF &&
      bytes[1] == 0xBB &&
      bytes[2] == 0xBF) {
    throw const InvalidCorpusException.malformedTransport();
  }

  // Validate strict UTF-8
  final String text;
  try {
    text = utf8.decode(bytes);
  } catch (_) {
    throw const InvalidCorpusException.malformedTransport();
  }

  // Inspect raw byte escapes for lone surrogates in JSON string literals
  var inString = false;
  for (var i = 0; i < bytes.length; i++) {
    final b = bytes[i];
    if (b == 0x22) {
      inString = !inString;
      continue;
    }
    if (!inString || b != 0x5C) {
      continue;
    }
    i++;
    if (i >= bytes.length) {
      throw const InvalidCorpusException.malformedTransport();
    }
    if (bytes[i] != 0x75) {
      continue;
    }
    if (i + 4 >= bytes.length) {
      throw const InvalidCorpusException.malformedTransport();
    }
    final hexStr = String.fromCharCodes(bytes.sublist(i + 1, i + 5));
    final code = int.tryParse(hexStr, radix: 16);
    if (code == null) {
      throw const InvalidCorpusException.malformedTransport();
    }
    i += 4;
    if (code >= minLowSurrogate && code <= maxLowSurrogate) {
      throw const InvalidCorpusException.malformedTransport();
    }
    if (code >= minHighSurrogate && code <= maxHighSurrogate) {
      if (i + 6 >= bytes.length ||
          bytes[i + 1] != 0x5C ||
          bytes[i + 2] != 0x75) {
        throw const InvalidCorpusException.malformedTransport();
      }
      final lowHex = String.fromCharCodes(bytes.sublist(i + 3, i + 7));
      final lowCode = int.tryParse(lowHex, radix: 16);
      if (lowCode == null ||
          lowCode < minLowSurrogate ||
          lowCode > maxLowSurrogate) {
        throw const InvalidCorpusException.malformedTransport();
      }
      i += 6;
    }
  }

  return text;
}

Map<String, dynamic> _expectObject(dynamic v, List<String> requiredKeys) {
  if (v is! Map<String, dynamic>) {
    throw const InvalidCorpusException.schemaViolation();
  }
  if (v.length != requiredKeys.length) {
    throw const InvalidCorpusException.schemaViolation();
  }
  for (final key in requiredKeys) {
    if (!v.containsKey(key)) {
      throw const InvalidCorpusException.schemaViolation();
    }
  }
  return v;
}

List<dynamic> _expectArray(dynamic v, int expectedCount) {
  if (v is! List<dynamic> || v.length != expectedCount) {
    throw const InvalidCorpusException.schemaViolation();
  }
  return v;
}

String _expectText(dynamic v, int maxLimit) {
  if (v is! String || v.runes.length > maxLimit) {
    throw const InvalidCorpusException.schemaViolation();
  }
  return v;
}

int _expectInteger(dynamic v, int max) {
  if (v is! _RawNumber) {
    throw const InvalidCorpusException.schemaViolation();
  }
  if (v.raw.contains('.') || v.raw.contains('e') || v.raw.contains('E')) {
    throw const InvalidCorpusException.schemaViolation();
  }
  final val = v.value;
  if (val is! int || val < minByteValue || val > max) {
    throw const InvalidCorpusException.schemaViolation();
  }
  return val;
}

/// Decodes and validates the STP01 corpus document against specification invariants.
_StpCorpus _stpDecode(Uint8List bytes) {
  final text = _stpScalars(bytes);
  final dynamic rootRaw = _StrictJsonParser(text).parse();
  final root = _expectObject(rootRaw, [
    'schema_version',
    'profile',
    'opcodes',
    'decode',
    'operators',
    'unknown_lookups',
  ]);

  if (_expectInteger(root['schema_version'], 1) != stp01SchemaVersion ||
      _expectText(root['profile'], 64) != stp01Profile) {
    throw const InvalidCorpusException.schemaViolation();
  }

  final corpus = _StpCorpus();

  // Validate 61 opcodes table
  final opcodesRaw = _expectArray(root['opcodes'], expectedOpcodeCount);
  var previousByte = -1;
  final seenSymbols = <String>{};
  for (final rowRaw in opcodesRaw) {
    final rowMap = _expectObject(rowRaw, ['byte', 'symbol', 'category']);
    final byteVal = _expectInteger(rowMap['byte'], maxByteValue);
    if (byteVal <= previousByte) {
      throw const InvalidCorpusException.schemaViolation();
    }
    previousByte = byteVal;

    final symbol = _expectText(rowMap['symbol'], 64);
    final category = _expectText(rowMap['category'], 64);
    if (seenSymbols.contains(symbol)) {
      throw const InvalidCorpusException.schemaViolation();
    }
    seenSymbols.add(symbol);
    corpus.opcodes.add(_StpRow(
      byteNumber: byteVal,
      symbol: symbol,
      category: category,
    ));
  }

  // Validate 256 decode table
  final decodeRaw = _expectArray(root['decode'], expectedDecodeCount);
  for (var i = 0; i < expectedDecodeCount; i++) {
    final rowMap = _expectObject(decodeRaw[i], ['byte', 'symbol', 'category']);
    final byteVal = _expectInteger(rowMap['byte'], maxByteValue);
    if (byteVal != i) {
      throw const InvalidCorpusException.schemaViolation();
    }

    if (rowMap['symbol'] == null && rowMap['category'] == null) {
      corpus.decode.add(_StpRow(byteNumber: byteVal, absent: true));
    } else {
      final symbol = _expectText(rowMap['symbol'], 64);
      final category = _expectText(rowMap['category'], 64);
      corpus.decode.add(_StpRow(
        byteNumber: byteVal,
        symbol: symbol,
        category: category,
      ));
    }
  }

  // Validate operator mappings
  final operatorsGroup = _expectObject(root['operators'], _groupOrder);
  for (final group in _groupOrder) {
    final expectedOps = _independentOperators[group]!;
    final opsRaw = _expectArray(operatorsGroup[group], expectedOps.length);
    for (var i = 0; i < expectedOps.length; i++) {
      final opMap = _expectObject(opsRaw[i], ['operator', 'symbol']);
      final opSym = _expectText(opMap['operator'], 32);
      final codeSym = _expectText(opMap['symbol'], 64);
      if (opSym != expectedOps[i].$1) {
        throw const InvalidCorpusException.schemaViolation();
      }
      corpus.operators[group]!.add(_StpOperator(opSym, codeSym));
    }
  }

  // Validate unknown lookups
  final unknownsRaw =
      _expectArray(root['unknown_lookups'], expectedUnknownLookupCount);
  for (var i = 0; i < expectedUnknownLookupCount; i++) {
    final unkMap =
        _expectObject(unknownsRaw[i], ['group', 'operator', 'symbol']);
    if (unkMap['symbol'] != null) {
      throw const InvalidCorpusException.schemaViolation();
    }
    final group = _expectText(unkMap['group'], 64);
    final opSym = _expectText(unkMap['operator'], 32);
    if (group != _independentUnknowns[i].$1 ||
        opSym != _independentUnknowns[i].$2) {
      throw const InvalidCorpusException.schemaViolation();
    }
    corpus.unknowns.add(_StpUnknown(group, opSym));
  }

  return corpus;
}

/// Compares decoded corpus against both the independent verification table
/// and the live provider implementation, returning the 5 canonical counts [61, 256, 34, 8, 195].
List<int> _stpCompare(_StpCorpus corpus) {
  final counts = [0, 0, 0, 0, 0];

  // 1. Cross-check opcodes against independent table and live provider
  expect(corpus.opcodes.length, equals(expectedOpcodeCount));
  final liveOps = allOps();
  expect(liveOps.length, equals(expectedOpcodeCount));

  for (var i = 0; i < expectedOpcodeCount; i++) {
    final row = corpus.opcodes[i];
    final ind = _independentOpcodes[i];
    final live = liveOps[i];

    expect(row.byteNumber, equals(ind.$1));
    expect(row.symbol, equals(ind.$2));
    expect(row.category, equals(ind.$3));

    expect(live.byteValue, equals(ind.$1));
    expect(live.symbol, equals(ind.$2));
    expect(live.category.id, equals(ind.$3));

    final fromByte = opFromByte(row.byteNumber);
    expect(fromByte, equals(live));

    counts[0]++;
  }

  // 2. Cross-check 256 decodes
  expect(corpus.decode.length, equals(expectedDecodeCount));
  for (var i = 0; i < expectedDecodeCount; i++) {
    final row = corpus.decode[i];
    final fromByte = opFromByte(row.byteNumber);
    final catFromByte = opCategoryFromByte(row.byteNumber);

    if (row.absent) {
      expect(fromByte, isNull);
      expect(catFromByte, isNull);
      counts[4]++; // Hole count
    } else {
      expect(fromByte, isNotNull);
      expect(fromByte!.byteValue, equals(row.byteNumber));
      expect(fromByte.symbol, equals(row.symbol));
      expect(fromByte.category.id, equals(row.category));
      expect(catFromByte, equals(fromByte.category));
    }
    counts[1]++;
  }

  // 3. Cross-check operators
  final opMaps = <String, Map<String, Op>>{
    'binary': binaryOpMap(),
    'compare': compareOpMap(),
    'augmented_assign': augmentedAssignMap(),
    'unary': unaryOpMap(),
  };

  final opLookups = <String, Op? Function(String)>{
    'binary': binaryOpcode,
    'compare': compareOpcode,
    'augmented_assign': augmentedAssignOpcode,
    'unary': unaryOpcode,
  };

  for (final group in _groupOrder) {
    final rows = corpus.operators[group]!;
    final expectedList = _independentOperators[group]!;
    final map = opMaps[group]!;
    final lookup = opLookups[group]!;

    expect(rows.length, equals(expectedList.length));
    expect(map.length, equals(expectedList.length));

    for (var i = 0; i < rows.length; i++) {
      final row = rows[i];
      final exp = expectedList[i];

      expect(row.operatorSymbol, equals(exp.$1));
      expect(row.opcodeSymbol, equals(exp.$2));

      final opFromMap = map[row.operatorSymbol];
      final opFromLookup = lookup(row.operatorSymbol);

      expect(opFromMap, isNotNull);
      expect(opFromLookup, isNotNull);
      expect(opFromMap, equals(opFromLookup));
      expect(opFromMap!.symbol, equals(exp.$2));

      counts[2]++;
    }
  }

  // 4. Cross-check unknowns
  expect(corpus.unknowns.length, equals(expectedUnknownLookupCount));
  for (final unk in corpus.unknowns) {
    final lookup = opLookups[unk.group]!;
    expect(lookup(unk.operatorSymbol), isNull);
    counts[3]++;
  }

  return counts;
}

void main() {
  late Uint8List fixtureBytes;

  setUpAll(() {
    // Locate cases.json relative to repository root or test directory
    final candidates = [
      '../../../specs/fixtures/starlark-opcode-provider-v1/cases.json',
      '../../specs/fixtures/starlark-opcode-provider-v1/cases.json',
      'code/specs/fixtures/starlark-opcode-provider-v1/cases.json',
    ];
    File? foundFile;
    for (final path in candidates) {
      final f = File(path);
      if (f.existsSync()) {
        foundFile = f;
        break;
      }
    }
    if (foundFile == null) {
      fail('STP01 fixture cases.json not found in candidate paths');
    }

    final raw = foundFile.readAsBytesSync();
    if (raw.length > maxCorpusBytes) {
      throw const InvalidCorpusException.byteLimitExceeded();
    }
    fixtureBytes = Uint8List.fromList(raw);
  });

  group('STP01 Conformance Reader & Oracle Comparison', () {
    test('shared corpus matches exactly [61, 256, 34, 8, 195]', () {
      final corpus = _stpDecode(fixtureBytes);
      final counts = _stpCompare(corpus);
      expect(counts, equals([61, 256, 34, 8, 195]));
      expect(counts[0], equals(expectedOpcodeCount));
      expect(counts[1], equals(expectedDecodeCount));
      expect(counts[2], equals(expectedOperatorCount));
      expect(counts[3], equals(expectedUnknownLookupCount));
      expect(counts[4], equals(expectedHoleCount));
    });

    test('replays fresh map ownership against mutate-and-verify loops', () {
      final corpus = _stpDecode(fixtureBytes);

      // Verify mutating map instances during comparison does not affect live state
      final bMap = binaryOpMap();
      bMap.clear();
      final cMap = compareOpMap();
      cMap.clear();
      final aMap = augmentedAssignMap();
      aMap.clear();
      final uMap = unaryOpMap();
      uMap.clear();

      final counts = _stpCompare(corpus);
      expect(counts, equals([61, 256, 34, 8, 195]));
    });
  });

  group('STP01 Transport & Scalar Rejection', () {
    test('rejects oversize corpus > 131,072 bytes', () {
      final big = Uint8List(maxCorpusBytes + 1);
      big.fillRange(0, big.length, 0x20);
      expect(
        () => _stpDecode(big),
        throwsA(isA<InvalidCorpusException>()),
      );
    });

    test('rejects UTF-8 BOM', () {
      final withBom = Uint8List.fromList([0xEF, 0xBB, 0xBF, ...fixtureBytes]);
      expect(
        () => _stpDecode(withBom),
        throwsA(isA<InvalidCorpusException>()),
      );
    });

    test('rejects invalid UTF-8 bytes', () {
      final badUtf8 = Uint8List.fromList(fixtureBytes);
      badUtf8[10] = 0xFF;
      expect(
        () => _stpDecode(badUtf8),
        throwsA(isA<InvalidCorpusException>()),
      );
    });

    test('rejects lone high surrogate in JSON string', () {
      final s = utf8.decode(fixtureBytes).replaceFirst('LoadConst', r'\ud800');
      expect(
        () => _stpDecode(Uint8List.fromList(utf8.encode(s))),
        throwsA(isA<InvalidCorpusException>()),
      );
    });

    test('rejects lone low surrogate in JSON string', () {
      final s = utf8.decode(fixtureBytes).replaceFirst('LoadConst', r'\udc00');
      expect(
        () => _stpDecode(Uint8List.fromList(utf8.encode(s))),
        throwsA(isA<InvalidCorpusException>()),
      );
    });

    test('rejects reversed surrogate pair in JSON string', () {
      final s =
          utf8.decode(fixtureBytes).replaceFirst('LoadConst', r'\udc00\ud800');
      expect(
        () => _stpDecode(Uint8List.fromList(utf8.encode(s))),
        throwsA(isA<InvalidCorpusException>()),
      );
    });

    test('rejects nonadjacent surrogate pair in JSON string', () {
      final s =
          utf8.decode(fixtureBytes).replaceFirst('LoadConst', r'\ud800x\udc00');
      expect(
        () => _stpDecode(Uint8List.fromList(utf8.encode(s))),
        throwsA(isA<InvalidCorpusException>()),
      );
    });

    test('rejects trailing content after valid JSON object', () {
      final s = '${utf8.decode(fixtureBytes)} {}';
      expect(
        () => _stpDecode(Uint8List.fromList(utf8.encode(s))),
        throwsA(isA<InvalidCorpusException>()),
      );
    });

    test('rejects duplicate keys in root object', () {
      final s = utf8.decode(fixtureBytes).replaceFirst(
            '"schema_version": 1',
            '"schema_version": 1, "schema_version": 1',
          );
      expect(
        () => _stpDecode(Uint8List.fromList(utf8.encode(s))),
        throwsA(isA<InvalidCorpusException>()),
      );
    });

    test('rejects escaped duplicate key in root object', () {
      final s = utf8.decode(fixtureBytes).replaceFirst(
            '"profile":',
            r'"pro\u0066ile": "standalone-provider-v1", "profile":',
          );
      expect(
        () => _stpDecode(Uint8List.fromList(utf8.encode(s))),
        throwsA(isA<InvalidCorpusException>()),
      );
    });

    test('rejects duplicate keys in row object', () {
      final s = utf8.decode(fixtureBytes).replaceFirst(
            '"byte": 1',
            '"byte": 1, "byte": 1',
          );
      expect(
        () => _stpDecode(Uint8List.fromList(utf8.encode(s))),
        throwsA(isA<InvalidCorpusException>()),
      );
    });

    test('rejects non-integer byte values', () {
      for (final badVal in [
        'true',
        '1.0',
        '1e0',
        '-1',
        '256',
        '32768',
        'NaN'
      ]) {
        final s = utf8.decode(fixtureBytes).replaceFirst(
              '"byte": 1',
              '"byte": $badVal',
            );
        expect(
          () => _stpDecode(Uint8List.fromList(utf8.encode(s))),
          throwsA(isA<InvalidCorpusException>()),
          reason: 'Failed to reject byte value: $badVal',
        );
      }
    });

    test('rejects missing or null required root keys', () {
      final keys = [
        'schema_version',
        'profile',
        'opcodes',
        'decode',
        'operators',
        'unknown_lookups',
      ];
      final text = utf8.decode(fixtureBytes);
      for (final key in keys) {
        // Test missing key
        final parsed = jsonDecode(text) as Map<String, dynamic>;
        parsed.remove(key);
        final encodedMissing =
            Uint8List.fromList(utf8.encode(jsonEncode(parsed)));
        expect(
          () => _stpDecode(encodedMissing),
          throwsA(isA<InvalidCorpusException>()),
          reason: 'Accepted missing root key: $key',
        );

        // Test null key
        final parsedNull = jsonDecode(text) as Map<String, dynamic>;
        parsedNull[key] = null;
        final encodedNull =
            Uint8List.fromList(utf8.encode(jsonEncode(parsedNull)));
        expect(
          () => _stpDecode(encodedNull),
          throwsA(isA<InvalidCorpusException>()),
          reason: 'Accepted null root key: $key',
        );
      }
    });

    test('rejects extra root keys', () {
      final text = utf8.decode(fixtureBytes);
      final parsed = jsonDecode(text) as Map<String, dynamic>;
      parsed['extension'] = 0;
      final encoded = Uint8List.fromList(utf8.encode(jsonEncode(parsed)));
      expect(
        () => _stpDecode(encoded),
        throwsA(isA<InvalidCorpusException>()),
      );
    });

    test('rejects schema version mismatch or unknown profile', () {
      final text = utf8.decode(fixtureBytes);
      final parsedVer = jsonDecode(text) as Map<String, dynamic>;
      parsedVer['schema_version'] = 2;
      expect(
        () =>
            _stpDecode(Uint8List.fromList(utf8.encode(jsonEncode(parsedVer)))),
        throwsA(isA<InvalidCorpusException>()),
      );

      final parsedProf = jsonDecode(text) as Map<String, dynamic>;
      parsedProf['profile'] = 'future';
      expect(
        () =>
            _stpDecode(Uint8List.fromList(utf8.encode(jsonEncode(parsedProf)))),
        throwsA(isA<InvalidCorpusException>()),
      );
    });
  });

  group('STP01 Semantic Corruption & Drift Detection', () {
    test('detects tampered opcode symbol', () {
      final text = utf8.decode(fixtureBytes);
      final parsed = jsonDecode(text) as Map<String, dynamic>;
      (parsed['opcodes'] as List<dynamic>)[0]['symbol'] = 'Bogus';
      expect(
        () => _stpCompare(
            _stpDecode(Uint8List.fromList(utf8.encode(jsonEncode(parsed))))),
        throwsA(anything),
      );
    });

    test('detects tampered category', () {
      final text = utf8.decode(fixtureBytes);
      final parsed = jsonDecode(text) as Map<String, dynamic>;
      (parsed['opcodes'] as List<dynamic>)[0]['category'] = 'module';
      expect(
        () => _stpCompare(
            _stpDecode(Uint8List.fromList(utf8.encode(jsonEncode(parsed))))),
        throwsA(anything),
      );
    });

    test('detects invented hole', () {
      final text = utf8.decode(fixtureBytes);
      final parsed = jsonDecode(text) as Map<String, dynamic>;
      final row0 =
          (parsed['decode'] as List<dynamic>)[0] as Map<String, dynamic>;
      row0['symbol'] = 'InventedHole';
      row0['category'] = 'vm_control';
      expect(
        () => _stpCompare(
            _stpDecode(Uint8List.fromList(utf8.encode(jsonEncode(parsed))))),
        throwsA(anything),
      );
    });

    test('detects tampered operator mapping', () {
      final text = utf8.decode(fixtureBytes);
      final parsed = jsonDecode(text) as Map<String, dynamic>;
      final binary = (parsed['operators'] as Map<String, dynamic>)['binary']
          as List<dynamic>;
      (binary[0] as Map<String, dynamic>)['symbol'] = 'Sub';
      expect(
        () => _stpCompare(
            _stpDecode(Uint8List.fromList(utf8.encode(jsonEncode(parsed))))),
        throwsA(anything),
      );
    });
  });

  group('10,000-Iteration Adversarial Mutation Fuzz Loop', () {
    test(
        'resiliently handles 10,000 mutated corpus variants without unhandled crash',
        () {
      final rng = Random(42);
      final base = Uint8List.fromList(fixtureBytes);

      for (var iter = 0; iter < 10000; iter++) {
        final mutated = Uint8List.fromList(base);
        final mutationType = rng.nextInt(5);

        switch (mutationType) {
          case 0:
            // Single byte bit-flip
            final pos = rng.nextInt(mutated.length);
            mutated[pos] ^= (1 << rng.nextInt(8));
          case 1:
            // Byte insertion
            final pos = rng.nextInt(mutated.length);
            final inserted = Uint8List(mutated.length + 1);
            inserted.setRange(0, pos, mutated);
            inserted[pos] = rng.nextInt(256);
            inserted.setRange(pos + 1, inserted.length, mutated.sublist(pos));
          case 2:
            // Byte deletion
            final pos = rng.nextInt(mutated.length - 1);
            final deleted = Uint8List(mutated.length - 1);
            deleted.setRange(0, pos, mutated);
            deleted.setRange(pos, deleted.length, mutated.sublist(pos + 1));
          case 3:
            // Random chunk replacement
            final pos = rng.nextInt(mutated.length - 10);
            final len = rng.nextInt(10) + 1;
            for (var k = 0; k < len; k++) {
              mutated[pos + k] = rng.nextInt(256);
            }
          case 4:
            // Truncation
            final truncLen = rng.nextInt(mutated.length);
            final truncated = Uint8List.sublistView(mutated, 0, truncLen);
            try {
              final c = _stpDecode(truncated);
              _stpCompare(c);
            } catch (e) {
              expect(
                e,
                anyOf(
                  isA<InvalidCorpusException>(),
                  isA<FormatException>(),
                  isA<TestFailure>(),
                ),
              );
            }
            continue;
        }

        try {
          final corpus = _stpDecode(mutated);
          _stpCompare(corpus);
        } catch (e) {
          expect(
            e,
            anyOf(
              isA<InvalidCorpusException>(),
              isA<FormatException>(),
              isA<TestFailure>(),
            ),
          );
        }
      }
    });
  });
}
