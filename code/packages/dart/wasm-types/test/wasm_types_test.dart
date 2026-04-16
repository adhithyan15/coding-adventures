import 'package:coding_adventures_wasm_types/wasm_types.dart';
import 'package:test/test.dart';

void main() {
  test('resolves value types from bytes', () {
    expect(ValueType.fromCode(0x7f), ValueType.i32);
    expect(ValueType.fromCode(0x7c), ValueType.f64);
  });

  test('builds a mutable module container', () {
    final module = WasmModule();
    module.types.add(FuncType(params: [ValueType.i32], results: [ValueType.i32]));
    module.functions.add(0);

    expect(module.types.single.params, [ValueType.i32]);
    expect(module.functions.single, 0);
  });
}
