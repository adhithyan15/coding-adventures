import 'package:coding_adventures_wasm_opcodes/wasm_opcodes.dart';
import 'package:test/test.dart';

void main() {
  test('looks up known opcode metadata', () {
    final opcode = opcodeByCode(0x6c);
    expect(opcode?.mnemonic, 'i32.mul');
  });

  test('returns null for unknown opcodes', () {
    expect(opcodeByCode(0xff), isNull);
  });
}
