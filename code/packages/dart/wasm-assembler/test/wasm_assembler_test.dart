import 'package:coding_adventures_wasm_assembler/wasm_assembler.dart';
import 'package:test/test.dart';

void main() {
  test('assembles the square function body', () {
    final bytes = assemble(const [
      AssemblyInstruction('local.get', [0]),
      AssemblyInstruction('local.get', [0]),
      AssemblyInstruction('i32.mul'),
      AssemblyInstruction('end'),
    ]);

    expect(bytes, [0x20, 0x00, 0x20, 0x00, 0x6c, 0x0b]);
  });
}
