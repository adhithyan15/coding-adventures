import 'dart:io';

import 'package:coding_adventures_wasm_runtime/wasm_runtime.dart';
import 'package:test/test.dart';

void main() {
  test('runs the Rust square fixture', () {
    final fixture = File('test/fixtures/square_nostd.wasm');
    expect(fixture.existsSync(), isTrue);

    final runtime = WasmRuntime();
    final bytes = fixture.readAsBytesSync();

    expect(runtime.loadAndRun(bytes, exportName: 'square', args: const [0]), [0]);
    expect(runtime.loadAndRun(bytes, exportName: 'square', args: const [5]), [25]);
    expect(runtime.loadAndRun(bytes, exportName: 'square', args: const [-3]), [9]);
    expect(runtime.loadAndRun(bytes, exportName: 'square', args: const [2147483647]), [1]);
  });
}
