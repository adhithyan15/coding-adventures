import 'dart:typed_data';

import 'package:coding_adventures_wasm_leb128/wasm_leb128.dart';

class AssemblyInstruction {
  const AssemblyInstruction(this.mnemonic, [this.operands = const []]);

  final String mnemonic;
  final List<Object> operands;
}

Uint8List assemble(List<AssemblyInstruction> instructions) {
  final bytes = <int>[];

  for (final instruction in instructions) {
    switch (instruction.mnemonic) {
      case 'end':
        bytes.add(0x0b);
        break;
      case 'return':
        bytes.add(0x0f);
        break;
      case 'call':
        bytes..add(0x10)..addAll(encodeUnsigned(instruction.operands.first as int));
        break;
      case 'drop':
        bytes.add(0x1a);
        break;
      case 'local.get':
        bytes..add(0x20)..addAll(encodeUnsigned(instruction.operands.first as int));
        break;
      case 'local.set':
        bytes..add(0x21)..addAll(encodeUnsigned(instruction.operands.first as int));
        break;
      case 'local.tee':
        bytes..add(0x22)..addAll(encodeUnsigned(instruction.operands.first as int));
        break;
      case 'global.get':
        bytes..add(0x23)..addAll(encodeUnsigned(instruction.operands.first as int));
        break;
      case 'global.set':
        bytes..add(0x24)..addAll(encodeUnsigned(instruction.operands.first as int));
        break;
      case 'i32.const':
        bytes..add(0x41)..addAll(encodeSigned(instruction.operands.first as int));
        break;
      case 'i32.add':
        bytes.add(0x6a);
        break;
      case 'i32.sub':
        bytes.add(0x6b);
        break;
      case 'i32.mul':
        bytes.add(0x6c);
        break;
      default:
        throw ArgumentError('Unsupported assembler mnemonic ${instruction.mnemonic}.');
    }
  }

  return Uint8List.fromList(bytes);
}
