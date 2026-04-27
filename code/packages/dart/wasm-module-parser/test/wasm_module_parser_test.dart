import 'dart:typed_data';

import 'package:coding_adventures_wasm_leb128/wasm_leb128.dart';
import 'package:coding_adventures_wasm_module_parser/wasm_module_parser.dart';
import 'package:test/test.dart';

const _header = <int>[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
const _i32 = 0x7f;
const _i64 = 0x7e;
const _f32 = 0x7d;
const _f64 = 0x7c;
const _funcref = 0x70;

Uint8List _wasm(List<List<int>> sections) {
  final bytes = <int>[..._header];
  for (final section in sections) {
    bytes.addAll(section);
  }
  return Uint8List.fromList(bytes);
}

List<int> _section(int id, List<int> payload) {
  return <int>[id, ...encodeUnsigned(payload.length), ...payload];
}

List<int> _string(String value) {
  final codeUnits = value.codeUnits;
  return <int>[...encodeUnsigned(codeUnits.length), ...codeUnits];
}

void main() {
  final parser = WasmModuleParser();

  test('parses a minimal module', () {
    final module = parser.parse(_header);
    expect(module.types, isEmpty);
    expect(module.imports, isEmpty);
    expect(module.functions, isEmpty);
    expect(module.start, isNull);
  });

  test('parses type, function, export, and code sections together', () {
    final module = parser.parse(
      _wasm([
        _section(1, [1, 0x60, 2, _i32, _i32, 1, _i32]),
        _section(3, [1, 0]),
        _section(7, [1, ..._string('main'), 0x00, 0x00]),
        _section(10, [1, 7, 0, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b]),
      ]),
    );

    expect(module.types.single.params.map((value) => value.code), [_i32, _i32]);
    expect(module.types.single.results.map((value) => value.code), [_i32]);
    expect(module.functions, [0]);
    expect(module.exports.single.name, 'main');
    expect(module.code.single.code, [0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b]);
  });

  test('parses imports for functions, memory, tables, and globals', () {
    final module = parser.parse(
      _wasm([
        _section(1, [1, 0x60, 0, 0]),
        _section(2, [
          4,
          ..._string('env'),
          ..._string('print'),
          0x00,
          0x00,
          ..._string('env'),
          ..._string('memory'),
          0x02,
          0x00,
          0x01,
          ..._string('env'),
          ..._string('table'),
          0x01,
          _funcref,
          0x00,
          0x01,
          ..._string('env'),
          ..._string('flag'),
          0x03,
          _i32,
          0x01,
        ]),
      ]),
    );

    expect(module.imports.length, 4);
    expect(module.imports[0].kind.code, 0x00);
    expect(module.imports[1].kind.code, 0x02);
    expect(module.imports[2].kind.code, 0x01);
    expect(module.imports[3].kind.code, 0x03);
  });

  test('parses memory, table, global, data, element, start, and custom sections', () {
    final module = parser.parse(
      _wasm([
        _section(0, [..._string('debug'), 0xde, 0xad]),
        _section(4, [1, _funcref, 0x00, 0x01]),
        _section(5, [1, 0x01, 0x01, 0x02]),
        _section(6, [1, _i32, 0x00, 0x41, 0x00, 0x0b]),
        _section(8, [0x02]),
        _section(9, [1, 0x00, 0x41, 0x00, 0x0b, 1, 0x02]),
        _section(11, [1, 0x00, 0x41, 0x00, 0x0b, 1, 0x42]),
      ]),
    );

    expect(module.customs.single.name, 'debug');
    expect(module.tables.single.elementType, _funcref);
    expect(module.memories.single.limits.max, 2);
    expect(module.globals.single.initExpr, [0x41, 0x00, 0x0b]);
    expect(module.start, 2);
    expect(module.elements.single.functionIndices, [2]);
    expect(module.data.single.data, [0x42]);
  });

  test('parses all four value types in a signature', () {
    final module = parser.parse(
      _wasm([
        _section(1, [1, 0x60, 4, _i32, _i64, _f32, _f64, 4, _f64, _f32, _i64, _i32]),
      ]),
    );

    expect(module.types.single.params.map((value) => value.code), [_i32, _i64, _f32, _f64]);
    expect(module.types.single.results.map((value) => value.code), [_f64, _f32, _i64, _i32]);
  });

  test('rejects invalid headers', () {
    expect(() => parser.parse([0x00, 0x00]), throwsA(isA<WasmParseException>()));
    expect(
      () => parser.parse([0x00, 0x61, 0x73, 0x6e, 0x01, 0x00, 0x00, 0x00]),
      throwsA(isA<WasmParseException>()),
    );
  });

  test('rejects sections that extend past the end of the file', () {
    final broken = Uint8List.fromList([..._header, 0x01, 100, 0x01]);
    expect(() => parser.parse(broken), throwsA(isA<WasmParseException>()));
  });

  test('rejects sections that appear out of order', () {
    final bytes = _wasm([
      _section(1, [1, 0x60, 0, 0]),
      _section(7, [1, ..._string('fn'), 0x00, 0x00]),
      _section(3, [1, 0]),
    ]);

    expect(() => parser.parse(bytes), throwsA(isA<WasmParseException>()));
  });

  test('rejects invalid import and export kinds', () {
    final badImport = _wasm([
      _section(1, [1, 0x60, 0, 0]),
      _section(2, [1, ..._string('env'), ..._string('x'), 0x99, 0x00]),
    ]);
    final badExport = _wasm([
      _section(1, [1, 0x60, 0, 0]),
      _section(3, [1, 0]),
      _section(7, [1, ..._string('fn'), 0x99, 0x00]),
      _section(10, [1, 2, 0, 0x0b]),
    ]);

    expect(() => parser.parse(badImport), throwsA(isA<WasmParseException>()));
    expect(() => parser.parse(badExport), throwsA(isA<WasmParseException>()));
  });

  test('rejects invalid table element types and invalid init expressions', () {
    final badTable = _wasm([
      _section(4, [1, 0x6f, 0x00, 0x01]),
    ]);
    final badGlobal = _wasm([
      _section(6, [1, _i32, 0x00, 0x41, 0x2a]),
    ]);

    expect(() => parser.parse(badTable), throwsA(isA<WasmParseException>()));
    expect(() => parser.parse(badGlobal), throwsA(isA<WasmParseException>()));
  });
}
