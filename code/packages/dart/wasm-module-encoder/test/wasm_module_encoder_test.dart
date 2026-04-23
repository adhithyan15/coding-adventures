import 'dart:typed_data';

import 'package:coding_adventures_wasm_module_encoder/wasm_module_encoder.dart';
import 'package:coding_adventures_wasm_module_parser/wasm_module_parser.dart';
import 'package:coding_adventures_wasm_types/wasm_types.dart';
import 'package:test/test.dart';

void main() {
  test('encodes and parses a small square module', () {
    final module = WasmModule()
      ..types.add(FuncType(params: [ValueType.i32], results: [ValueType.i32]))
      ..functions.add(0)
      ..exports.add(const ExportEntry(name: 'square', kind: ExternalKind.function, index: 0))
      ..code.add(
        FunctionBody(
          locals: const [],
          code: Uint8List.fromList([0x20, 0x00, 0x20, 0x00, 0x6c, 0x0b]),
        ),
      );

    final encoded = encodeModule(module);
    final reparsed = WasmModuleParser().parse(encoded);

    expect(reparsed.types.length, 1);
    expect(reparsed.exports.single.name, 'square');
  });
}
