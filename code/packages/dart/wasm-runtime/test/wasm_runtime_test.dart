import 'dart:typed_data';

import 'package:coding_adventures_wasm_leb128/wasm_leb128.dart';
import 'package:coding_adventures_wasm_runtime/wasm_runtime.dart';
import 'package:test/test.dart';

Uint8List _buildSquareModule() {
  final parts = <int>[
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
  ];

  final typePayload = [0x01, 0x60, 0x01, 0x7f, 0x01, 0x7f];
  parts..add(0x01)..addAll(encodeUnsigned(typePayload.length))..addAll(typePayload);

  final functionPayload = [0x01, 0x00];
  parts..add(0x03)..addAll(encodeUnsigned(functionPayload.length))..addAll(functionPayload);

  final exportName = 'square'.codeUnits;
  final exportPayload = [0x01, ...encodeUnsigned(exportName.length), ...exportName, 0x00, 0x00];
  parts..add(0x07)..addAll(encodeUnsigned(exportPayload.length))..addAll(exportPayload);

  final body = [0x00, 0x20, 0x00, 0x20, 0x00, 0x6c, 0x0b];
  final codePayload = [0x01, ...encodeUnsigned(body.length), ...body];
  parts..add(0x0a)..addAll(encodeUnsigned(codePayload.length))..addAll(codePayload);

  return Uint8List.fromList(parts);
}

void main() {
  test('runs a hand-authored square module', () {
    final runtime = WasmRuntime();
    final wasmBytes = _buildSquareModule();

    expect(runtime.loadAndRun(wasmBytes, exportName: 'square', args: const [5]), [25]);
    expect(runtime.loadAndRun(wasmBytes, exportName: 'square', args: const [-3]), [9]);
  });
}
