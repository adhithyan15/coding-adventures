import 'dart:math' as math;
import 'dart:typed_data';

import 'package:coding_adventures_wasm_leb128/wasm_leb128.dart';
import 'package:coding_adventures_wasm_opcodes/wasm_opcodes.dart';
import 'package:coding_adventures_wasm_types/wasm_types.dart';

const int _mask32 = 0xffffffff;
const int _sign32 = 0x80000000;
const int _mod32 = 0x100000000;
const int _mask64 = 0xffffffffffffffff;
const int _sign64 = 0x8000000000000000;

class TrapError implements Exception {
  TrapError(this.message);
  final String message;
  @override
  String toString() => 'TrapError: $message';
}

class WasmValue {
  const WasmValue(this.type, this.value);
  final ValueType type;
  final Object value;
}

WasmValue i32(int value) => WasmValue(ValueType.i32, _wrapI32(value));
WasmValue i64(int value) => WasmValue(ValueType.i64, _wrapI64(value));
WasmValue f32(double value) => WasmValue(ValueType.f32, _toF32(value));
WasmValue f64(double value) => WasmValue(ValueType.f64, value);

WasmValue defaultValue(ValueType type) {
  return switch (type) {
    ValueType.i32 => i32(0),
    ValueType.i64 => i64(0),
    ValueType.f32 => f32(0),
    ValueType.f64 => f64(0),
  };
}

int asI32(WasmValue value) {
  if (value.type != ValueType.i32) throw TrapError('Type mismatch: expected i32, got ${value.type.name}');
  return value.value as int;
}

int asI64(WasmValue value) {
  if (value.type != ValueType.i64) throw TrapError('Type mismatch: expected i64, got ${value.type.name}');
  return value.value as int;
}

double asF32(WasmValue value) {
  if (value.type != ValueType.f32) throw TrapError('Type mismatch: expected f32, got ${value.type.name}');
  return value.value as double;
}

double asF64(WasmValue value) {
  if (value.type != ValueType.f64) throw TrapError('Type mismatch: expected f64, got ${value.type.name}');
  return value.value as double;
}

int _wrapI32(int value) {
  final masked = value & _mask32;
  return masked >= _sign32 ? masked - _mod32 : masked;
}

int _wrapI64(int value) {
  final masked = value & _mask64;
  return masked >= _sign64 ? masked - (_mask64 + 1) : masked;
}

int _u32(int value) => value & _mask32;
int _u64(int value) => value & _mask64;

double _toF32(double value) {
  final data = ByteData(4)..setFloat32(0, value, Endian.little);
  return data.getFloat32(0, Endian.little);
}

abstract class HostFunction {
  FuncType get type;
  List<WasmValue> call(List<WasmValue> args);
}

class ResolvedGlobalImport {
  const ResolvedGlobalImport({required this.type, required this.value});
  final GlobalType type;
  final WasmValue value;
}

class HostInterface {
  const HostInterface();
  HostFunction? resolveFunction(String moduleName, String name) => null;
  ResolvedGlobalImport? resolveGlobal(String moduleName, String name) => null;
  LinearMemory? resolveMemory(String moduleName, String name) => null;
  Table? resolveTable(String moduleName, String name) => null;
}

class LinearMemory {
  LinearMemory(int initialPages, [this.maxPages])
      : _bytes = Uint8List(initialPages * pageSize),
        _view = ByteData(initialPages * pageSize);

  static const int pageSize = 65536;
  final int? maxPages;
  Uint8List _bytes;
  ByteData _view;

  void _refreshView() => _view = ByteData.sublistView(_bytes);

  void _boundsCheck(int offset, int width) {
    if (offset < 0 || offset + width > _bytes.length) {
      throw TrapError('Out of bounds memory access: offset=$offset, size=$width, memory size=${_bytes.length}');
    }
  }

  int loadI32(int offset) { _boundsCheck(offset, 4); return _view.getInt32(offset, Endian.little); }
  int loadI64(int offset) { _boundsCheck(offset, 8); return _view.getInt64(offset, Endian.little); }
  double loadF32(int offset) { _boundsCheck(offset, 4); return _view.getFloat32(offset, Endian.little); }
  double loadF64(int offset) { _boundsCheck(offset, 8); return _view.getFloat64(offset, Endian.little); }
  int loadI32_8s(int offset) { _boundsCheck(offset, 1); return _view.getInt8(offset); }
  int loadI32_8u(int offset) { _boundsCheck(offset, 1); return _view.getUint8(offset); }
  int loadI32_16s(int offset) { _boundsCheck(offset, 2); return _view.getInt16(offset, Endian.little); }
  int loadI32_16u(int offset) { _boundsCheck(offset, 2); return _view.getUint16(offset, Endian.little); }
  int loadI64_8s(int offset) { _boundsCheck(offset, 1); return _view.getInt8(offset); }
  int loadI64_8u(int offset) { _boundsCheck(offset, 1); return _view.getUint8(offset); }
  int loadI64_16s(int offset) { _boundsCheck(offset, 2); return _view.getInt16(offset, Endian.little); }
  int loadI64_16u(int offset) { _boundsCheck(offset, 2); return _view.getUint16(offset, Endian.little); }
  int loadI64_32s(int offset) { _boundsCheck(offset, 4); return _view.getInt32(offset, Endian.little); }
  int loadI64_32u(int offset) { _boundsCheck(offset, 4); return _view.getUint32(offset, Endian.little); }
  void storeI32(int offset, int value) { _boundsCheck(offset, 4); _view.setInt32(offset, value, Endian.little); }
  void storeI64(int offset, int value) { _boundsCheck(offset, 8); _view.setInt64(offset, value, Endian.little); }
  void storeF32(int offset, double value) { _boundsCheck(offset, 4); _view.setFloat32(offset, value, Endian.little); }
  void storeF64(int offset, double value) { _boundsCheck(offset, 8); _view.setFloat64(offset, value, Endian.little); }
  void storeI32_8(int offset, int value) { _boundsCheck(offset, 1); _view.setInt8(offset, value); }
  void storeI32_16(int offset, int value) { _boundsCheck(offset, 2); _view.setInt16(offset, value, Endian.little); }
  void storeI64_8(int offset, int value) { _boundsCheck(offset, 1); _view.setInt8(offset, value); }
  void storeI64_16(int offset, int value) { _boundsCheck(offset, 2); _view.setInt16(offset, value, Endian.little); }
  void storeI64_32(int offset, int value) { _boundsCheck(offset, 4); _view.setInt32(offset, value, Endian.little); }

  int grow(int deltaPages) {
    final oldPages = size();
    final newPages = oldPages + deltaPages;
    if (maxPages != null && newPages > maxPages!) return -1;
    if (newPages > 65536) return -1;
    final next = Uint8List(newPages * pageSize);
    next.setAll(0, _bytes);
    _bytes = next;
    _refreshView();
    return oldPages;
  }

  int size() => _bytes.length ~/ pageSize;
  int byteLength() => _bytes.length;
  void writeBytes(int offset, List<int> data) { _boundsCheck(offset, data.length); _bytes.setRange(offset, offset + data.length, data); }
  Uint8List readBytes(int offset, int length) { _boundsCheck(offset, length); return Uint8List.fromList(_bytes.sublist(offset, offset + length)); }
}

class Table {
  Table(int initialSize, [this.maxSize]) : _elements = List<int?>.filled(initialSize, null);
  final int? maxSize;
  final List<int?> _elements;

  int? get(int index) {
    if (index < 0 || index >= _elements.length) throw TrapError('Out of bounds table access: index=$index, table size=${_elements.length}');
    return _elements[index];
  }

  void set(int index, int? funcIndex) {
    if (index < 0 || index >= _elements.length) throw TrapError('Out of bounds table access: index=$index, table size=${_elements.length}');
    _elements[index] = funcIndex;
  }

  int size() => _elements.length;
  int grow(int delta) {
    final oldSize = _elements.length;
    final newSize = oldSize + delta;
    if (maxSize != null && newSize > maxSize!) return -1;
    for (var i = 0; i < delta; i += 1) { _elements.add(null); }
    return oldSize;
  }
}

class Label {
  const Label({required this.arity, required this.targetPc, required this.stackHeight, required this.isLoop});
  final int arity;
  final int targetPc;
  final int stackHeight;
  final bool isLoop;
}

class ControlTarget {
  const ControlTarget({required this.endPc, required this.elsePc});
  final int endPc;
  final int? elsePc;
}

class SavedFrame {
  const SavedFrame({required this.locals, required this.labelStack, required this.stackHeight, required this.controlFlowMap, required this.returnPc, required this.returnArity});
  final List<WasmValue> locals;
  final List<Label> labelStack;
  final int stackHeight;
  final Map<int, ControlTarget> controlFlowMap;
  final int returnPc;
  final int returnArity;
}

class WasmExecutionContext {
  const WasmExecutionContext({required this.memory, required this.tables, required this.globals, required this.globalTypes, required this.funcTypes, required this.funcBodies, required this.hostFunctions});
  final LinearMemory? memory;
  final List<Table> tables;
  final List<WasmValue> globals;
  final List<GlobalType> globalTypes;
  final List<FuncType> funcTypes;
  final List<FunctionBody?> funcBodies;
  final List<HostFunction?> hostFunctions;
}

class MemArg {
  const MemArg({required this.align, required this.offset});
  final int align;
  final int offset;
}

class BranchTableImmediate {
  const BranchTableImmediate({required this.labels, required this.defaultLabel});
  final List<int> labels;
  final int defaultLabel;
}

class CallIndirectImmediate {
  const CallIndirectImmediate({required this.typeIndex, required this.tableIndex});
  final int typeIndex;
  final int tableIndex;
}

class DecodedInstruction {
  const DecodedInstruction({required this.opcode, required this.info, required this.operand, required this.offset, required this.size});
  final int opcode;
  final OpcodeInfo info;
  final Object? operand;
  final int offset;
  final int size;
}

class VmInstruction {
  const VmInstruction({required this.opcode, required this.operand});
  final int opcode;
  final Object? operand;
}

List<DecodedInstruction> decodeFunctionBody(FunctionBody body) {
  final code = body.code;
  final result = <DecodedInstruction>[];
  var offset = 0;
  while (offset < code.length) {
    final start = offset;
    final opcode = code[offset++];
    final info = getOpcode(opcode);
    if (info == null) throw TrapError('Unknown WASM opcode 0x${opcode.toRadixString(16)}');
    Object? operand;
    if (info.immediates.isNotEmpty) {
      final decoded = _decodeImmediates(code, offset, info.immediates);
      operand = decoded.$1;
      offset += decoded.$2;
    }
    result.add(DecodedInstruction(opcode: opcode, info: info, operand: operand, offset: start, size: offset - start));
  }
  return result;
}

(Object?, int) _decodeImmediates(Uint8List code, int offset, List<String> immediates) {
  if (immediates.length == 1) return _decodeImmediate(code, offset, immediates.first);
  if (immediates.length == 2 && immediates[0] == 'typeidx' && immediates[1] == 'tableidx') {
    final first = _decodeImmediate(code, offset, immediates.first);
    final second = _decodeImmediate(code, offset + first.$2, immediates.last);
    return (CallIndirectImmediate(typeIndex: first.$1 as int, tableIndex: second.$1 as int), first.$2 + second.$2);
  }
  throw TrapError('Unsupported immediate sequence: ${immediates.join(', ')}');
}

(Object?, int) _decodeImmediate(Uint8List code, int offset, String type) {
  switch (type) {
    case 'i32':
      final decoded = decodeSigned(code, offset: offset, maxBytes: 5, bitWidth: 32);
      return (decoded.value, decoded.bytesRead);
    case 'i64':
      final decoded = decodeSigned(code, offset: offset, maxBytes: 10, bitWidth: 64);
      return (decoded.value, decoded.bytesRead);
    case 'f32':
      return (ByteData.sublistView(code, offset, offset + 4).getFloat32(0, Endian.little), 4);
    case 'f64':
      return (ByteData.sublistView(code, offset, offset + 8).getFloat64(0, Endian.little), 8);
    case 'labelidx':
    case 'funcidx':
    case 'typeidx':
    case 'localidx':
    case 'globalidx':
    case 'tableidx':
    case 'memidx':
      final decoded = decodeUnsigned(code, offset: offset, maxBytes: 5);
      return (decoded.value, decoded.bytesRead);
    case 'blocktype':
      final byte = code[offset];
      if (byte == emptyBlockType || byte == ValueType.i32.code || byte == ValueType.i64.code || byte == ValueType.f32.code || byte == ValueType.f64.code) {
        return (byte == emptyBlockType ? null : byte, 1);
      }
      final decoded = decodeSigned(code, offset: offset, maxBytes: 5, bitWidth: 32);
      return (decoded.value, decoded.bytesRead);
    case 'memarg':
      final align = decodeUnsigned(code, offset: offset, maxBytes: 5);
      final memOffset = decodeUnsigned(code, offset: offset + align.bytesRead, maxBytes: 5);
      return (MemArg(align: align.value, offset: memOffset.value), align.bytesRead + memOffset.bytesRead);
    case 'vec_labelidx':
      final count = decodeUnsigned(code, offset: offset, maxBytes: 5);
      final labels = <int>[];
      var pos = offset + count.bytesRead;
      for (var i = 0; i < count.value; i += 1) {
        final label = decodeUnsigned(code, offset: pos, maxBytes: 5);
        labels.add(label.value);
        pos += label.bytesRead;
      }
      final defaultLabel = decodeUnsigned(code, offset: pos, maxBytes: 5);
      pos += defaultLabel.bytesRead;
      return (BranchTableImmediate(labels: List<int>.unmodifiable(labels), defaultLabel: defaultLabel.value), pos - offset);
  }
  throw TrapError('Unsupported immediate type: $type');
}

Map<int, ControlTarget> buildControlFlowMap(List<DecodedInstruction> instructions) {
  final map = <int, ControlTarget>{};
  final stack = <({int index, int? elsePc})>[];
  for (var i = 0; i < instructions.length; i += 1) {
    switch (instructions[i].opcode) {
      case 0x02:
      case 0x03:
      case 0x04:
        stack.add((index: i, elsePc: null));
        break;
      case 0x05:
        if (stack.isNotEmpty) {
          final top = stack.removeLast();
          stack.add((index: top.index, elsePc: i));
        }
        break;
      case 0x0b:
        if (stack.isNotEmpty) {
          final opener = stack.removeLast();
          map[opener.index] = ControlTarget(endPc: i, elsePc: opener.elsePc);
        }
        break;
    }
  }
  return Map<int, ControlTarget>.unmodifiable(map);
}

List<VmInstruction> toVmInstructions(List<DecodedInstruction> decoded) {
  return List<VmInstruction>.unmodifiable(decoded.map((value) => VmInstruction(opcode: value.opcode, operand: value.operand)));
}

WasmValue evaluateConstExpr(Uint8List expr, {List<WasmValue> globals = const []}) {
  var offset = 0;
  WasmValue? result;
  final view = ByteData.sublistView(expr);
  while (offset < expr.length) {
    final opcode = expr[offset++];
    switch (opcode) {
      case 0x41:
        final decoded = decodeSigned(expr, offset: offset, maxBytes: 5, bitWidth: 32);
        offset += decoded.bytesRead;
        result = i32(decoded.value);
        break;
      case 0x42:
        final decoded = decodeSigned(expr, offset: offset, maxBytes: 10, bitWidth: 64);
        offset += decoded.bytesRead;
        result = i64(decoded.value);
        break;
      case 0x43:
        result = f32(view.getFloat32(offset, Endian.little));
        offset += 4;
        break;
      case 0x44:
        result = f64(view.getFloat64(offset, Endian.little));
        offset += 8;
        break;
      case 0x23:
        final decoded = decodeUnsigned(expr, offset: offset, maxBytes: 5);
        offset += decoded.bytesRead;
        if (decoded.value < 0 || decoded.value >= globals.length) throw TrapError('global.get: index ${decoded.value} out of bounds');
        result = globals[decoded.value];
        break;
      case 0x0b:
        if (result == null) throw TrapError('Constant expression produced no value');
        return result;
      default:
        throw TrapError('Illegal opcode 0x${opcode.toRadixString(16).padLeft(2, '0')} in constant expression');
    }
  }
  throw TrapError('Constant expression missing end opcode (0x0B)');
}
class WasmExecutionEngine {
  WasmExecutionEngine(this.context);

  final WasmExecutionContext context;
  final Map<int, List<DecodedInstruction>> _decodedCache = <int, List<DecodedInstruction>>{};
  final Map<int, Map<int, ControlTarget>> _controlFlowCache = <int, Map<int, ControlTarget>>{};

  List<WasmValue> callFunction(int funcIndex, List<WasmValue> args) {
    if (funcIndex < 0 || funcIndex >= context.funcTypes.length) throw TrapError('undefined function index $funcIndex');
    final funcType = context.funcTypes[funcIndex];
    if (args.length != funcType.params.length) throw TrapError('function $funcIndex expects ${funcType.params.length} arguments, got ${args.length}');
    final host = context.hostFunctions[funcIndex];
    if (host != null) return host.call(args);

    final stack = <WasmValue>[];
    final frames = <_Frame>[_createFrame(funcIndex, args)];
    List<WasmValue>? finalResults;

    while (frames.isNotEmpty) {
      final frame = frames.last;
      if (frame.pc < 0 || frame.pc >= frame.instructions.length) throw TrapError('Program counter out of bounds for function ${frame.funcIndex}');
      final instruction = frame.instructions[frame.pc];
      switch (instruction.opcode) {
        case 0x00:
          throw TrapError('unreachable instruction executed');
        case 0x01:
          frame.pc += 1;
          break;
        case 0x02:
          _enterBlock(frame, instruction, isLoop: false, stack: stack);
          break;
        case 0x03:
          _enterBlock(frame, instruction, isLoop: true, stack: stack);
          break;
        case 0x04:
          _executeIf(frame, instruction, stack);
          break;
        case 0x05:
          frame.pc = frame.labels.last.targetPc;
          break;
        case 0x0b:
          if (frame.labels.isNotEmpty) {
            frame.labels.removeLast();
            frame.pc += 1;
          } else {
            finalResults = _returnFromFrame(frames, stack, frame.funcType);
          }
          break;
        case 0x0c:
          _executeBranch(frame, stack, instruction.operand as int);
          break;
        case 0x0d:
          final condition = asI32(stack.removeLast());
          if (condition != 0) {
            _executeBranch(frame, stack, instruction.operand as int);
          } else {
            frame.pc += 1;
          }
          break;
        case 0x0e:
          final immediate = instruction.operand as BranchTableImmediate;
          final index = asI32(stack.removeLast());
          final target = index >= 0 && index < immediate.labels.length ? immediate.labels[index] : immediate.defaultLabel;
          _executeBranch(frame, stack, target);
          break;
        case 0x0f:
          finalResults = _returnFromFrame(frames, stack, frame.funcType);
          break;
        case 0x10:
          frame.pc += 1;
          _invokeFunction(instruction.operand as int, frames, stack);
          break;
        case 0x11:
          final immediate = instruction.operand as CallIndirectImmediate;
          final tableIndex = asI32(stack.removeLast());
          final target = context.tables[immediate.tableIndex].get(tableIndex);
          if (target == null) throw TrapError('uninitialized table element');
          _ensureSameFuncType(context.funcTypes[target], context.funcTypes[immediate.typeIndex]);
          frame.pc += 1;
          _invokeFunction(target, frames, stack);
          break;
        case 0x1a:
          stack.removeLast();
          frame.pc += 1;
          break;
        case 0x1b:
          final condition = asI32(stack.removeLast());
          final right = stack.removeLast();
          final left = stack.removeLast();
          stack.add(condition != 0 ? left : right);
          frame.pc += 1;
          break;
        case 0x20:
          stack.add(frame.locals[instruction.operand as int]);
          frame.pc += 1;
          break;
        case 0x21:
          frame.locals[instruction.operand as int] = stack.removeLast();
          frame.pc += 1;
          break;
        case 0x22:
          frame.locals[instruction.operand as int] = stack.last;
          frame.pc += 1;
          break;
        case 0x23:
          stack.add(context.globals[instruction.operand as int]);
          frame.pc += 1;
          break;
        case 0x24:
          final index = instruction.operand as int;
          if (!context.globalTypes[index].mutable) throw TrapError('Cannot mutate an immutable global');
          context.globals[index] = stack.removeLast();
          frame.pc += 1;
          break;
        default:
          if (_executeMemory(instruction, frame, stack)) break;
          if (_executeNumeric(instruction, frame, stack)) break;
          if (_executeConversion(instruction, frame, stack)) break;
          throw TrapError('Unsupported instruction ${instruction.info.name}');
      }
    }

    return finalResults ?? const <WasmValue>[];
  }

  _Frame _createFrame(int funcIndex, List<WasmValue> args) {
    final body = context.funcBodies[funcIndex];
    if (body == null) throw TrapError('no body for function $funcIndex');
    final instructions = _decodedCache.putIfAbsent(funcIndex, () => decodeFunctionBody(body));
    final control = _controlFlowCache.putIfAbsent(funcIndex, () => buildControlFlowMap(instructions));
    return _Frame(funcIndex: funcIndex, funcType: context.funcTypes[funcIndex], locals: <WasmValue>[...args, ...body.locals.map(defaultValue)], instructions: instructions, controlFlowMap: control);
  }

  void _invokeFunction(int funcIndex, List<_Frame> frames, List<WasmValue> stack) {
    final funcType = context.funcTypes[funcIndex];
    final args = <WasmValue>[];
    for (var i = 0; i < funcType.params.length; i += 1) {
      args.insert(0, stack.removeLast());
    }
    final host = context.hostFunctions[funcIndex];
    if (host != null) {
      stack.addAll(host.call(args));
      return;
    }
    frames.add(_createFrame(funcIndex, args));
  }

  List<WasmValue>? _returnFromFrame(List<_Frame> frames, List<WasmValue> stack, FuncType funcType) {
    final results = <WasmValue>[];
    for (var i = 0; i < funcType.results.length; i += 1) {
      results.insert(0, stack.removeLast());
    }
    frames.removeLast();
    if (frames.isNotEmpty) {
      stack.addAll(results);
      return null;
    }
    return results;
  }

  void _enterBlock(_Frame frame, DecodedInstruction instruction, {required bool isLoop, required List<WasmValue> stack}) {
    final blockType = instruction.operand as int?;
    final arity = isLoop ? _blockParamArity(blockType) : _blockArity(blockType);
    final target = frame.controlFlowMap[frame.pc];
    frame.labels.add(Label(arity: arity, targetPc: isLoop ? frame.pc : (target?.endPc ?? frame.pc + 1), stackHeight: stack.length, isLoop: isLoop));
    frame.pc += 1;
  }

  void _executeIf(_Frame frame, DecodedInstruction instruction, List<WasmValue> stack) {
    final condition = asI32(stack.removeLast());
    final target = frame.controlFlowMap[frame.pc];
    frame.labels.add(Label(arity: _blockArity(instruction.operand as int?), targetPc: target?.endPc ?? frame.pc + 1, stackHeight: stack.length, isLoop: false));
    frame.pc = condition != 0 ? frame.pc + 1 : (target?.elsePc != null ? target!.elsePc! + 1 : target?.endPc ?? frame.pc + 1);
  }

  void _executeBranch(_Frame frame, List<WasmValue> stack, int labelIndex) {
    final targetIndex = frame.labels.length - 1 - labelIndex;
    if (targetIndex < 0) throw TrapError('branch target $labelIndex out of range');
    final label = frame.labels[targetIndex];
    final carried = <WasmValue>[];
    for (var i = 0; i < label.arity; i += 1) {
      carried.insert(0, stack.removeLast());
    }
    while (stack.length > label.stackHeight) {
      stack.removeLast();
    }
    stack.addAll(carried);
    frame.labels.length = targetIndex;
    frame.pc = label.targetPc;
  }

  int _blockArity(int? blockType) {
    if (blockType == null) return 0;
    if (blockType == ValueType.i32.code || blockType == ValueType.i64.code || blockType == ValueType.f32.code || blockType == ValueType.f64.code) return 1;
    if (blockType >= 0 && blockType < context.funcTypes.length) return context.funcTypes[blockType].results.length;
    return 0;
  }

  int _blockParamArity(int? blockType) {
    if (blockType == null) return 0;
    if (blockType == ValueType.i32.code || blockType == ValueType.i64.code || blockType == ValueType.f32.code || blockType == ValueType.f64.code) return 0;
    if (blockType >= 0 && blockType < context.funcTypes.length) return context.funcTypes[blockType].params.length;
    return 0;
  }
  bool _executeMemory(DecodedInstruction instruction, _Frame frame, List<WasmValue> stack) {
    if (instruction.info.category != 'memory') return false;
    final memory = context.memory;
    if (memory == null) throw TrapError('no linear memory');
    if (instruction.info.name == 'memory.size') { stack.add(i32(memory.size())); frame.pc += 1; return true; }
    if (instruction.info.name == 'memory.grow') { stack.add(i32(memory.grow(asI32(stack.removeLast())))); frame.pc += 1; return true; }
    final memarg = instruction.operand as MemArg;
    final name = instruction.info.name;
    if (name.contains('.store')) {
      final value = stack.removeLast();
      final base = asI32(stack.removeLast());
      final addr = _u32(base) + memarg.offset;
      switch (instruction.opcode) {
        case 0x36: memory.storeI32(addr, asI32(value)); break;
        case 0x37: memory.storeI64(addr, asI64(value)); break;
        case 0x38: memory.storeF32(addr, asF32(value)); break;
        case 0x39: memory.storeF64(addr, asF64(value)); break;
        case 0x3a: memory.storeI32_8(addr, asI32(value)); break;
        case 0x3b: memory.storeI32_16(addr, asI32(value)); break;
        case 0x3c: memory.storeI64_8(addr, asI64(value)); break;
        case 0x3d: memory.storeI64_16(addr, asI64(value)); break;
        case 0x3e: memory.storeI64_32(addr, asI64(value)); break;
      }
      frame.pc += 1;
      return true;
    }
    final addr = _u32(asI32(stack.removeLast())) + memarg.offset;
    switch (instruction.opcode) {
      case 0x28: stack.add(i32(memory.loadI32(addr))); break;
      case 0x29: stack.add(i64(memory.loadI64(addr))); break;
      case 0x2a: stack.add(f32(memory.loadF32(addr))); break;
      case 0x2b: stack.add(f64(memory.loadF64(addr))); break;
      case 0x2c: stack.add(i32(memory.loadI32_8s(addr))); break;
      case 0x2d: stack.add(i32(memory.loadI32_8u(addr))); break;
      case 0x2e: stack.add(i32(memory.loadI32_16s(addr))); break;
      case 0x2f: stack.add(i32(memory.loadI32_16u(addr))); break;
      case 0x30: stack.add(i64(memory.loadI64_8s(addr))); break;
      case 0x31: stack.add(i64(memory.loadI64_8u(addr))); break;
      case 0x32: stack.add(i64(memory.loadI64_16s(addr))); break;
      case 0x33: stack.add(i64(memory.loadI64_16u(addr))); break;
      case 0x34: stack.add(i64(memory.loadI64_32s(addr))); break;
      case 0x35: stack.add(i64(memory.loadI64_32u(addr))); break;
      default: return false;
    }
    frame.pc += 1;
    return true;
  }

  bool _executeNumeric(DecodedInstruction instruction, _Frame frame, List<WasmValue> stack) {
    switch (instruction.opcode) {
      case 0x41: stack.add(i32(instruction.operand as int)); frame.pc += 1; return true;
      case 0x42: stack.add(i64(instruction.operand as int)); frame.pc += 1; return true;
      case 0x43: stack.add(f32(instruction.operand as double)); frame.pc += 1; return true;
      case 0x44: stack.add(f64(instruction.operand as double)); frame.pc += 1; return true;
      case 0x45: stack.add(i32(asI32(stack.removeLast()) == 0 ? 1 : 0)); frame.pc += 1; return true;
      case 0x46: case 0x47: case 0x48: case 0x49: case 0x4a: case 0x4b: case 0x4c: case 0x4d: case 0x4e: case 0x4f:
      case 0x67: case 0x68: case 0x69: case 0x6a: case 0x6b: case 0x6c: case 0x6d: case 0x6e: case 0x6f: case 0x70:
      case 0x71: case 0x72: case 0x73: case 0x74: case 0x75: case 0x76: case 0x77: case 0x78:
        _executeI32Op(instruction.opcode, stack); frame.pc += 1; return true;
      case 0x50: case 0x51: case 0x52: case 0x53: case 0x54: case 0x55: case 0x56: case 0x57: case 0x58: case 0x59: case 0x5a:
      case 0x79: case 0x7a: case 0x7b: case 0x7c: case 0x7d: case 0x7e: case 0x7f: case 0x80: case 0x81: case 0x82:
      case 0x83: case 0x84: case 0x85: case 0x86: case 0x87: case 0x88: case 0x89: case 0x8a:
        _executeI64Op(instruction.opcode, stack); frame.pc += 1; return true;
      case 0x5b: case 0x5c: case 0x5d: case 0x5e: case 0x5f: case 0x60: case 0x8b: case 0x8c: case 0x8d: case 0x8e: case 0x8f:
      case 0x90: case 0x91: case 0x92: case 0x93: case 0x94: case 0x95: case 0x96: case 0x97: case 0x98:
        _executeF32Op(instruction.opcode, stack); frame.pc += 1; return true;
      case 0x61: case 0x62: case 0x63: case 0x64: case 0x65: case 0x66: case 0x99: case 0x9a: case 0x9b: case 0x9c: case 0x9d:
      case 0x9e: case 0x9f: case 0xa0: case 0xa1: case 0xa2: case 0xa3: case 0xa4: case 0xa5: case 0xa6:
        _executeF64Op(instruction.opcode, stack); frame.pc += 1; return true;
      default: return false;
    }
  }

  bool _executeConversion(DecodedInstruction instruction, _Frame frame, List<WasmValue> stack) {
    switch (instruction.opcode) {
      case 0xa7: stack.add(i32(asI64(stack.removeLast()))); break;
      case 0xa8: stack.add(i32(_truncInt(asF32(stack.removeLast()), -2147483648, 2147483647))); break;
      case 0xa9: stack.add(i32(_truncInt(asF32(stack.removeLast()), 0, 4294967295))); break;
      case 0xaa: stack.add(i32(_truncInt(asF64(stack.removeLast()), -2147483648, 2147483647))); break;
      case 0xab: stack.add(i32(_truncInt(asF64(stack.removeLast()), 0, 4294967295))); break;
      case 0xac: stack.add(i64(asI32(stack.removeLast()))); break;
      case 0xad: stack.add(i64(_u32(asI32(stack.removeLast())))); break;
      case 0xae: stack.add(i64(_truncInt(asF32(stack.removeLast()), -0x8000000000000000, 0x7fffffffffffffff))); break;
      case 0xaf: stack.add(i64(_truncInt(asF32(stack.removeLast()), 0, 0xffffffffffffffff))); break;
      case 0xb0: stack.add(i64(_truncInt(asF64(stack.removeLast()), -0x8000000000000000, 0x7fffffffffffffff))); break;
      case 0xb1: stack.add(i64(_truncInt(asF64(stack.removeLast()), 0, 0xffffffffffffffff))); break;
      case 0xb2: stack.add(f32(asI32(stack.removeLast()).toDouble())); break;
      case 0xb3: stack.add(f32(_u32(asI32(stack.removeLast())).toDouble())); break;
      case 0xb4: stack.add(f32(asI64(stack.removeLast()).toDouble())); break;
      case 0xb5: stack.add(f32(_u64(asI64(stack.removeLast())).toDouble())); break;
      case 0xb6: stack.add(f32(asF64(stack.removeLast()))); break;
      case 0xb7: stack.add(f64(asI32(stack.removeLast()).toDouble())); break;
      case 0xb8: stack.add(f64(_u32(asI32(stack.removeLast())).toDouble())); break;
      case 0xb9: stack.add(f64(asI64(stack.removeLast()).toDouble())); break;
      case 0xba: stack.add(f64(_u64(asI64(stack.removeLast())).toDouble())); break;
      case 0xbb: stack.add(f64(asF32(stack.removeLast()))); break;
      case 0xbc: stack.add(i32(_reinterpretI32FromF32(asF32(stack.removeLast())))); break;
      case 0xbd: stack.add(i64(_reinterpretI64FromF64(asF64(stack.removeLast())))); break;
      case 0xbe: stack.add(f32(_reinterpretF32FromI32(asI32(stack.removeLast())))); break;
      case 0xbf: stack.add(f64(_reinterpretF64FromI64(asI64(stack.removeLast())))); break;
      default: return false;
    }
    frame.pc += 1;
    return true;
  }

  void _executeI32Op(int opcode, List<WasmValue> stack) {
    int pop() => asI32(stack.removeLast());
    switch (opcode) {
      case 0x46: final b = pop(), a = pop(); stack.add(i32(a == b ? 1 : 0)); return;
      case 0x47: final b = pop(), a = pop(); stack.add(i32(a != b ? 1 : 0)); return;
      case 0x48: final b = pop(), a = pop(); stack.add(i32(a < b ? 1 : 0)); return;
      case 0x49: final b = _u32(pop()), a = _u32(pop()); stack.add(i32(a < b ? 1 : 0)); return;
      case 0x4a: final b = pop(), a = pop(); stack.add(i32(a > b ? 1 : 0)); return;
      case 0x4b: final b = _u32(pop()), a = _u32(pop()); stack.add(i32(a > b ? 1 : 0)); return;
      case 0x4c: final b = pop(), a = pop(); stack.add(i32(a <= b ? 1 : 0)); return;
      case 0x4d: final b = _u32(pop()), a = _u32(pop()); stack.add(i32(a <= b ? 1 : 0)); return;
      case 0x4e: final b = pop(), a = pop(); stack.add(i32(a >= b ? 1 : 0)); return;
      case 0x4f: final b = _u32(pop()), a = _u32(pop()); stack.add(i32(a >= b ? 1 : 0)); return;
      case 0x67: stack.add(i32(_clz32(pop()))); return;
      case 0x68: stack.add(i32(_ctz32(pop()))); return;
      case 0x69: stack.add(i32(_popcnt32(pop()))); return;
      case 0x6a: final b = pop(), a = pop(); stack.add(i32(a + b)); return;
      case 0x6b: final b = pop(), a = pop(); stack.add(i32(a - b)); return;
      case 0x6c: final b = pop(), a = pop(); stack.add(i32(a * b)); return;
      case 0x6d: final b = pop(), a = pop(); if (b == 0) throw TrapError('integer divide by zero'); if (a == -2147483648 && b == -1) throw TrapError('integer overflow'); stack.add(i32(a ~/ b)); return;
      case 0x6e: final b = _u32(pop()), a = _u32(pop()); if (b == 0) throw TrapError('integer divide by zero'); stack.add(i32(a ~/ b)); return;
      case 0x6f: final b = pop(), a = pop(); if (b == 0) throw TrapError('integer divide by zero'); stack.add(i32(a == -2147483648 && b == -1 ? 0 : a % b)); return;
      case 0x70: final b = _u32(pop()), a = _u32(pop()); if (b == 0) throw TrapError('integer divide by zero'); stack.add(i32(a % b)); return;
      case 0x71: final b = pop(), a = pop(); stack.add(i32(a & b)); return;
      case 0x72: final b = pop(), a = pop(); stack.add(i32(a | b)); return;
      case 0x73: final b = pop(), a = pop(); stack.add(i32(a ^ b)); return;
      case 0x74: final b = pop(), a = pop(); stack.add(i32(a << (b & 31))); return;
      case 0x75: final b = pop(), a = pop(); stack.add(i32(a >> (b & 31))); return;
      case 0x76: final b = pop(), a = _u32(pop()); stack.add(i32(a >> (b & 31))); return;
      case 0x77: final b = pop(), a = _u32(pop()); final n = b & 31; stack.add(i32(n == 0 ? a : ((a << n) | (a >> (32 - n))))); return;
      case 0x78: final b = pop(), a = _u32(pop()); final n = b & 31; stack.add(i32(n == 0 ? a : ((a >> n) | (a << (32 - n))))); return;
    }
  }

  void _executeI64Op(int opcode, List<WasmValue> stack) {
    int pop() => asI64(stack.removeLast());
    switch (opcode) {
      case 0x50: stack.add(i32(pop() == 0 ? 1 : 0)); return;
      case 0x51: final b = pop(), a = pop(); stack.add(i32(a == b ? 1 : 0)); return;
      case 0x52: final b = pop(), a = pop(); stack.add(i32(a != b ? 1 : 0)); return;
      case 0x53: final b = pop(), a = pop(); stack.add(i32(a < b ? 1 : 0)); return;
      case 0x54: final b = _u64(pop()), a = _u64(pop()); stack.add(i32(a < b ? 1 : 0)); return;
      case 0x55: final b = pop(), a = pop(); stack.add(i32(a > b ? 1 : 0)); return;
      case 0x56: final b = _u64(pop()), a = _u64(pop()); stack.add(i32(a > b ? 1 : 0)); return;
      case 0x57: final b = pop(), a = pop(); stack.add(i32(a <= b ? 1 : 0)); return;
      case 0x58: final b = _u64(pop()), a = _u64(pop()); stack.add(i32(a <= b ? 1 : 0)); return;
      case 0x59: final b = pop(), a = pop(); stack.add(i32(a >= b ? 1 : 0)); return;
      case 0x5a: final b = _u64(pop()), a = _u64(pop()); stack.add(i32(a >= b ? 1 : 0)); return;
      case 0x79: stack.add(i64(_clz64(pop()))); return;
      case 0x7a: stack.add(i64(_ctz64(pop()))); return;
      case 0x7b: stack.add(i64(_popcnt64(pop()))); return;
      case 0x7c: final b = pop(), a = pop(); stack.add(i64(a + b)); return;
      case 0x7d: final b = pop(), a = pop(); stack.add(i64(a - b)); return;
      case 0x7e: final b = pop(), a = pop(); stack.add(i64(a * b)); return;
      case 0x7f: final b = pop(), a = pop(); if (b == 0) throw TrapError('integer divide by zero'); if (a == -0x8000000000000000 && b == -1) throw TrapError('integer overflow'); stack.add(i64(a ~/ b)); return;
      case 0x80: final b = _u64(pop()), a = _u64(pop()); if (b == 0) throw TrapError('integer divide by zero'); stack.add(i64(a ~/ b)); return;
      case 0x81: final b = pop(), a = pop(); if (b == 0) throw TrapError('integer divide by zero'); stack.add(i64(a == -0x8000000000000000 && b == -1 ? 0 : a % b)); return;
      case 0x82: final b = _u64(pop()), a = _u64(pop()); if (b == 0) throw TrapError('integer divide by zero'); stack.add(i64(a % b)); return;
      case 0x83: final b = pop(), a = pop(); stack.add(i64(a & b)); return;
      case 0x84: final b = pop(), a = pop(); stack.add(i64(a | b)); return;
      case 0x85: final b = pop(), a = pop(); stack.add(i64(a ^ b)); return;
      case 0x86: final b = _u64(pop()) & 63, a = pop(); stack.add(i64(a << b)); return;
      case 0x87: final b = _u64(pop()) & 63, a = pop(); stack.add(i64(a >> b)); return;
      case 0x88: final b = _u64(pop()) & 63, a = _u64(pop()); stack.add(i64(a >> b)); return;
      case 0x89: final b = _u64(pop()) & 63, a = _u64(pop()); stack.add(i64(b == 0 ? a : ((a << b) | (a >> (64 - b))))); return;
      case 0x8a: final b = _u64(pop()) & 63, a = _u64(pop()); stack.add(i64(b == 0 ? a : ((a >> b) | (a << (64 - b))))); return;
    }
  }
  void _executeF32Op(int opcode, List<WasmValue> stack) {
    double pop() => asF32(stack.removeLast());
    switch (opcode) {
      case 0x5b:
      case 0x5c:
      case 0x5d:
      case 0x5e:
      case 0x5f:
      case 0x60:
        final b = pop(), a = pop();
        final result = switch (opcode) { 0x5b => a == b, 0x5c => a != b, 0x5d => a < b, 0x5e => a > b, 0x5f => a <= b, _ => a >= b };
        stack.add(i32(result ? 1 : 0));
        return;
      case 0x8b: stack.add(f32(pop().abs())); return;
      case 0x8c: stack.add(f32(-pop())); return;
      case 0x8d: stack.add(f32(pop().ceilToDouble())); return;
      case 0x8e: stack.add(f32(pop().floorToDouble())); return;
      case 0x8f: stack.add(f32(pop().truncateToDouble())); return;
      case 0x90: stack.add(f32(_nearestF32(pop()))); return;
      case 0x91: stack.add(f32(math.sqrt(pop()))); return;
      case 0x92: final b = pop(), a = pop(); stack.add(f32(a + b)); return;
      case 0x93: final b = pop(), a = pop(); stack.add(f32(a - b)); return;
      case 0x94: final b = pop(), a = pop(); stack.add(f32(a * b)); return;
      case 0x95: final b = pop(), a = pop(); stack.add(f32(a / b)); return;
      case 0x96: final b = pop(), a = pop(); stack.add(f32(_minFloat(a, b))); return;
      case 0x97: final b = pop(), a = pop(); stack.add(f32(_maxFloat(a, b))); return;
      case 0x98: final b = pop(), a = pop(); stack.add(f32(_copySign(a, b))); return;
    }
  }

  void _executeF64Op(int opcode, List<WasmValue> stack) {
    double pop() => asF64(stack.removeLast());
    switch (opcode) {
      case 0x61:
      case 0x62:
      case 0x63:
      case 0x64:
      case 0x65:
      case 0x66:
        final b = pop(), a = pop();
        final result = switch (opcode) { 0x61 => a == b, 0x62 => a != b, 0x63 => a < b, 0x64 => a > b, 0x65 => a <= b, _ => a >= b };
        stack.add(i32(result ? 1 : 0));
        return;
      case 0x99: stack.add(f64(pop().abs())); return;
      case 0x9a: stack.add(f64(-pop())); return;
      case 0x9b: stack.add(f64(pop().ceilToDouble())); return;
      case 0x9c: stack.add(f64(pop().floorToDouble())); return;
      case 0x9d: stack.add(f64(pop().truncateToDouble())); return;
      case 0x9e: stack.add(f64(_nearestF64(pop()))); return;
      case 0x9f: stack.add(f64(math.sqrt(pop()))); return;
      case 0xa0: final b = pop(), a = pop(); stack.add(f64(a + b)); return;
      case 0xa1: final b = pop(), a = pop(); stack.add(f64(a - b)); return;
      case 0xa2: final b = pop(), a = pop(); stack.add(f64(a * b)); return;
      case 0xa3: final b = pop(), a = pop(); stack.add(f64(a / b)); return;
      case 0xa4: final b = pop(), a = pop(); stack.add(f64(_minFloat(a, b))); return;
      case 0xa5: final b = pop(), a = pop(); stack.add(f64(_maxFloat(a, b))); return;
      case 0xa6: final b = pop(), a = pop(); stack.add(f64(_copySign(a, b))); return;
    }
  }
}

class _Frame {
  _Frame({required this.funcIndex, required this.funcType, required this.locals, required this.instructions, required this.controlFlowMap});
  final int funcIndex;
  final FuncType funcType;
  final List<WasmValue> locals;
  final List<DecodedInstruction> instructions;
  final Map<int, ControlTarget> controlFlowMap;
  final List<Label> labels = <Label>[];
  int pc = 0;
}

void _ensureSameFuncType(FuncType actual, FuncType expected) {
  if (actual.params.length != expected.params.length || actual.results.length != expected.results.length) throw TrapError('indirect call type mismatch');
  for (var i = 0; i < actual.params.length; i += 1) { if (actual.params[i] != expected.params[i]) throw TrapError('indirect call type mismatch'); }
  for (var i = 0; i < actual.results.length; i += 1) { if (actual.results[i] != expected.results[i]) throw TrapError('indirect call type mismatch'); }
}

int _clz32(int value) => _u32(value) == 0 ? 32 : 32 - _u32(value).bitLength;
int _clz64(int value) => _u64(value) == 0 ? 64 : 64 - _u64(value).bitLength;

int _ctz32(int value) {
  var v = _u32(value);
  if (v == 0) return 32;
  var c = 0;
  while ((v & 1) == 0) { c += 1; v >>= 1; }
  return c;
}

int _ctz64(int value) {
  var v = _u64(value);
  if (v == 0) return 64;
  var c = 0;
  while ((v & 1) == 0) { c += 1; v >>= 1; }
  return c;
}

int _popcnt32(int value) {
  var v = _u32(value), c = 0;
  while (v != 0) { c += v & 1; v >>= 1; }
  return c;
}

int _popcnt64(int value) {
  var v = _u64(value), c = 0;
  while (v != 0) { c += v & 1; v >>= 1; }
  return c;
}

double _nearestF32(double value) {
  if (!value.isFinite || value == 0) return value;
  final floor = value.floorToDouble();
  final frac = value - floor;
  if (frac == 0.5) return _toF32(floor % 2 == 0 ? floor : floor + 1);
  return _toF32(value.roundToDouble());
}

double _nearestF64(double value) {
  if (!value.isFinite || value == 0) return value;
  final floor = value.floorToDouble();
  final frac = value - floor;
  if (frac == 0.5) return floor % 2 == 0 ? floor : floor + 1;
  return value.roundToDouble();
}

double _minFloat(double a, double b) {
  if (a.isNaN || b.isNaN) return double.nan;
  if (a == 0 && b == 0) return a.isNegative || b.isNegative ? -0.0 : 0.0;
  return math.min(a, b);
}

double _maxFloat(double a, double b) {
  if (a.isNaN || b.isNaN) return double.nan;
  if (a == 0 && b == 0) return !a.isNegative || !b.isNegative ? 0.0 : -0.0;
  return math.max(a, b);
}

double _copySign(double a, double b) => b.isNegative ? -a.abs() : a.abs();

int _truncInt(double value, int min, int max) {
  if (value.isNaN) throw TrapError('invalid conversion to integer');
  if (!value.isFinite) throw TrapError('integer overflow');
  final truncated = value.truncate();
  if (truncated < min || truncated > max) throw TrapError('integer overflow');
  return truncated;
}

int _reinterpretI32FromF32(double value) {
  final data = ByteData(4)..setFloat32(0, value, Endian.little);
  return data.getInt32(0, Endian.little);
}

int _reinterpretI64FromF64(double value) {
  final data = ByteData(8)..setFloat64(0, value, Endian.little);
  return data.getInt64(0, Endian.little);
}

double _reinterpretF32FromI32(int value) {
  final data = ByteData(4)..setInt32(0, value, Endian.little);
  return data.getFloat32(0, Endian.little);
}

double _reinterpretF64FromI64(int value) {
  final data = ByteData(8)..setInt64(0, value, Endian.little);
  return data.getFloat64(0, Endian.little);
}
