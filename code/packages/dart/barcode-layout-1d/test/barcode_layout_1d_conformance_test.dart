import 'dart:collection';
import 'dart:convert';
import 'dart:io';

import 'package:coding_adventures_barcode_layout_1d/barcode_layout_1d.dart';
import 'package:coding_adventures_paint_instructions/paint_instructions.dart';
import 'package:crypto/crypto.dart';
import 'package:test/test.dart';

const _corpusSha =
    'be95aa0381041ef3bd729b36bb4292f7a20692e139b53adca97af4e157cb7388';
final _corpus = File(
  '../../../../code/specs/fixtures/barcode-layout-1d-v1/cases.json',
);

Map<String, dynamic> _load(List<int> bytes) {
  if (bytes.length > 131072) throw const FormatException('fixture-size-limit');
  final text = utf8.decode(bytes, allowMalformed: false);
  var depth = 0;
  var quoted = false;
  var escaped = false;
  var tokenStart = -1;
  final objectKeys = <Set<String>?>[];
  final expectingKey = <bool>[];
  for (var index = 0; index < text.length; index++) {
    final rune = text.codeUnitAt(index);
    if (quoted) {
      if (escaped) {
        escaped = false;
      } else if (rune == 92) {
        escaped = true;
      } else if (rune == 34) {
        quoted = false;
        if (objectKeys.isNotEmpty &&
            objectKeys.last != null &&
            expectingKey.last) {
          final key =
              jsonDecode(text.substring(tokenStart, index + 1)) as String;
          if (!objectKeys.last!.add(key)) {
            throw const FormatException('fixture-duplicate-key');
          }
          expectingKey[expectingKey.length - 1] = false;
        }
      }
    } else if (rune == 34) {
      quoted = true;
      tokenStart = index;
    } else if (rune == 91 || rune == 123) {
      if (++depth > 8) throw const FormatException('fixture-depth-limit');
      objectKeys.add(rune == 123 ? <String>{} : null);
      expectingKey.add(rune == 123);
    } else if (rune == 93 || rune == 125) {
      depth--;
      if (objectKeys.isNotEmpty) {
        objectKeys.removeLast();
        expectingKey.removeLast();
      }
    } else if (rune == 44 && objectKeys.isNotEmpty && objectKeys.last != null) {
      expectingKey[expectingKey.length - 1] = true;
    }
  }
  final value = jsonDecode(text);
  if (value is! Map<String, dynamic> || value['cases'] is! List) {
    throw const FormatException('fixture-schema-invalid');
  }
  _validateScalars(value);
  return value;
}

void _validateScalars(Object? value) {
  if (value is String) {
    final units = value.codeUnits;
    for (var index = 0; index < units.length; index++) {
      final unit = units[index];
      if (unit >= 0xd800 && unit <= 0xdbff) {
        if (++index >= units.length ||
            units[index] < 0xdc00 ||
            units[index] > 0xdfff) {
          throw const FormatException('fixture-invalid-scalar');
        }
      } else if (unit >= 0xdc00 && unit <= 0xdfff) {
        throw const FormatException('fixture-invalid-scalar');
      }
    }
  } else if (value is List) {
    for (final item in value) {
      _validateScalars(item);
    }
  } else if (value is Map) {
    for (final entry in value.entries) {
      _validateScalars(entry.key);
      _validateScalars(entry.value);
    }
  }
}

String _pattern(Map<String, dynamic> input) {
  if (input.containsKey('pattern')) return input['pattern'] as String;
  final repeat = input['repeat'] as Map<String, dynamic>;
  final count = repeat['count'] as int;
  final token = repeat['token'] as String;
  final suffix = repeat['suffix'] as String? ?? '';
  if (count < 0 ||
      count > 65569 ||
      token.runes.length > 2 ||
      token.isEmpty ||
      suffix.runes.length > 1) {
    throw const FormatException('fixture-schema-invalid');
  }
  return List.filled(count, token).join() + suffix;
}

List<Barcode1DRun> _runs(Map<String, dynamic> input) {
  if (input['runs'] case final List rows) {
    return rows.map((row) {
      final item = row as Map<String, dynamic>;
      return Barcode1DRun(
        item['color'] as String,
        item['modules'] as int,
        item['sourceLabel'] as String,
        item['sourceIndex'] as int,
        item['role'] as String,
      );
    }).toList();
  }
  final repeat = input['repeatRuns'] as Map<String, dynamic>;
  final count = repeat['count'] as int;
  if (count < 0 || count > 40980)
    throw const FormatException('fixture-schema-invalid');
  final first = repeat['firstColor'] as String;
  return List.generate(
    count,
    (index) => Barcode1DRun(
      index.isEven ? first : (first == 'bar' ? 'space' : 'bar'),
      repeat['modules'] as int,
      repeat['sourceLabel'] as String,
      repeat['sourceIndex'] as int,
      repeat['role'] as String,
    ),
  );
}

List<Barcode1DSymbol>? _symbols(Map<String, dynamic> input) {
  if (input['symbols'] case final List rows) {
    return rows.map((row) {
      final item = row as Map<String, dynamic>;
      return Barcode1DSymbol(
        item['label'] as String,
        item['modules'] as int,
        item['sourceIndex'] as int,
        item['role'] as String,
      );
    }).toList();
  }
  if (input['repeatSymbols'] == null) return null;
  final repeat = input['repeatSymbols'] as Map<String, dynamic>;
  final count = repeat['count'] as int;
  if (count < 0 || count > 40980)
    throw const FormatException('fixture-schema-invalid');
  return List.generate(
    count,
    (index) => Barcode1DSymbol(
      repeat['label'] as String,
      repeat['modules'] as int,
      index,
      repeat['role'] as String,
    ),
  );
}

Object _execute(Map<String, dynamic> row) {
  final input = row['input'] as Map<String, dynamic>;
  switch (row['operation']) {
    case 'expand-binary':
      return BarcodeLayout1DV1.expandBinary(
        _pattern(input),
        input['sourceLabel'] as String,
        input['sourceIndex'] as int,
        input['role'] as String,
      );
    case 'expand-width':
      return BarcodeLayout1DV1.expandWidth(
        _pattern(input),
        input['narrowMarker'] as String? ?? 'N',
        input['wideMarker'] as String? ?? 'W',
        input['narrowModules'] as int? ?? 1,
        input['wideModules'] as int? ?? 3,
        input['startingColor'] as String? ?? 'bar',
        input['sourceLabel'] as String,
        input['sourceIndex'] as int,
        input['role'] as String,
      );
    case 'compute-layout':
      return BarcodeLayout1DV1.computeLayout(
        _runs(input),
        input['quietZoneModules'] as int,
        _symbols(input),
      );
    case 'project-scene':
      final render = input['renderConfig'] as Map<String, dynamic>? ?? {};
      return BarcodeLayout1DV1.projectScene(
        _runs(input),
        input['quietZoneModules'] as int,
        Barcode1DSceneOptions(
          moduleWidth: render['moduleWidth'] as int? ?? 4,
          barHeight: render['barHeight'] as int? ?? 120,
          foreground: render['foreground'] as String? ?? '#000000',
          background: render['background'] as String? ?? '#ffffff',
          label: input['label'] as String? ?? '1D barcode',
          metadata: Map<String, String>.from(input['metadata'] as Map? ?? {}),
          humanReadableText: input['humanReadableText'] as String?,
          includeHumanReadableText:
              render['includeHumanReadableText'] as bool? ?? false,
          symbols: _symbols(input),
        ),
      );
    default:
      throw StateError('unknown neutral operation');
  }
}

Map<String, Object> _runMap(Barcode1DRun run) => SplayTreeMap.of({
  'color': run.color,
  'modules': run.modules,
  'role': run.role,
  'sourceIndex': run.sourceIndex,
  'sourceLabel': run.sourceLabel,
});

Object _project(Object actual) {
  if (actual is List<Barcode1DRun>) return actual.map(_runMap).toList();
  if (actual is Barcode1DLayout) {
    return {
      'leftQuietZoneModules': actual.leftQuietZoneModules,
      'rightQuietZoneModules': actual.rightQuietZoneModules,
      'contentModules': actual.contentModules,
      'totalModules': actual.totalModules,
      'symbolLayouts': actual.symbolLayouts
          .map(
            (symbol) => {
              'label': symbol.label,
              'startModule': symbol.startModule,
              'endModule': symbol.endModule,
              'sourceIndex': symbol.sourceIndex,
              'role': symbol.role,
            },
          )
          .toList(),
    };
  }
  final scene = actual as PaintScene;
  return {
    'width': scene.width,
    'height': scene.height,
    'background': scene.background,
    'rectangles': scene.instructions.map((instruction) {
      final rect = instruction as PaintRect;
      return {
        'x': rect.x,
        'y': rect.y,
        'width': rect.width,
        'height': rect.height,
        'fill': rect.fill,
        'metadata': rect.metadata,
      };
    }).toList(),
    'metadata': scene.metadata,
  };
}

void main() {
  final bytes = _corpus.readAsBytesSync();
  test('neutral corpus identity and size', () {
    expect(sha256.convert(bytes).toString(), _corpusSha);
    expect((_load(bytes)['cases'] as List).length, 56);
  });
  final cases = _load(bytes)['cases'] as List;
  for (final raw in cases) {
    final row = raw as Map<String, dynamic>;
    test(row['id'] as String, () {
      final expected = row['expected'] as Map<String, dynamic>;
      if (expected.containsKey('error')) {
        expect(
          () => _execute(row),
          throwsA(
            isA<Barcode1DV1Error>().having(
              (error) => error.errorId,
              'errorId',
              expected['error'],
            ),
          ),
        );
        return;
      }
      final actual = _project(_execute(row));
      if (expected['runDigest'] case final Map<String, dynamic> digest) {
        final runs = actual as List;
        expect(runs.length, digest['runCount']);
        expect(
          runs.fold<int>(
            0,
            (sum, run) =>
                sum + ((run as Map<String, Object>)['modules'] as int),
          ),
          digest['contentModules'],
        );
        expect(runs.first, digest['firstRun']);
        expect(runs.last, digest['lastRun']);
        expect(
          sha256.convert(utf8.encode(jsonEncode(runs))).toString(),
          digest['runsSha256'],
        );
      } else {
        final key = expected.containsKey('runs')
            ? 'runs'
            : expected.containsKey('layout')
            ? 'layout'
            : 'scene';
        expect(actual, expected[key]);
      }
    });
  }
  test('hostile corpus inputs fail before dispatch', () {
    expect(() => _load(List.filled(131073, 0)), throwsFormatException);
    expect(
      () => _load(utf8.encode('[[[[[[[[[0]]]]]]]]]')),
      throwsFormatException,
    );
    expect(() => _load([255]), throwsFormatException);
    expect(
      () => _load(utf8.encode('{"cases":[],"cases":[]}')),
      throwsFormatException,
    );
    expect(
      () => _load(utf8.encode('{"cases":[],"bad":"\\uD800"}')),
      throwsFormatException,
    );
    expect(
      () => _pattern({
        'repeat': {'token': '1', 'count': 65570},
      }),
      throwsFormatException,
    );
  });
}
