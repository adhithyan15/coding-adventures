import 'dart:typed_data';

import 'package:coding_adventures_wasm_execution/wasm_execution.dart';
import 'package:coding_adventures_wasm_types/wasm_types.dart';
import 'package:test/test.dart';

FunctionBody _body(List<int> bytecodes, {List<ValueType> locals = const []}) {
  return FunctionBody(
    locals: locals,
    code: Uint8List.fromList(<int>[...bytecodes, 0x0b]),
  );
}

WasmExecutionContext _context({
  LinearMemory? memory,
  List<Table> tables = const [],
  List<WasmValue> globals = const [],
  List<GlobalType> globalTypes = const [],
  required List<FuncType> funcTypes,
  required List<FunctionBody?> funcBodies,
  required List<HostFunction?> hostFunctions,
}) {
  return WasmExecutionContext(
    memory: memory,
    tables: tables,
    globals: [...globals],
    globalTypes: [...globalTypes],
    funcTypes: funcTypes,
    funcBodies: funcBodies,
    hostFunctions: hostFunctions,
  );
}

class _InlineHostFunction implements HostFunction {
  const _InlineHostFunction(this.type, this._call);

  @override
  final FuncType type;

  final List<WasmValue> Function(List<WasmValue>) _call;

  @override
  List<WasmValue> call(List<WasmValue> args) => _call(args);
}

void main() {
  test('evaluates i32 constant expressions', () {
    final value = evaluateConstExpr(Uint8List.fromList([0x41, 0x2a, 0x0b]));
    expect(value.type, ValueType.i32);
    expect(value.value, 42);
  });

  test('executes an i32 square function body', () {
    final engine = WasmExecutionEngine(
      _context(
        funcTypes: [
          FuncType(params: [ValueType.i32], results: [ValueType.i32]),
        ],
        funcBodies: [
          _body([0x20, 0x00, 0x20, 0x00, 0x6c]),
        ],
        hostFunctions: [null],
      ),
    );

    final result = engine.callFunction(0, [i32(7)]);
    expect(result.single.value, 49);
  });

  test('uses declared locals with local.set and local.get', () {
    final engine = WasmExecutionEngine(
      _context(
        funcTypes: [
          FuncType(params: [ValueType.i32], results: [ValueType.i32]),
        ],
        funcBodies: [
          _body(
            [0x20, 0x00, 0x21, 0x01, 0x20, 0x01],
            locals: [ValueType.i32],
          ),
        ],
        hostFunctions: [null],
      ),
    );

    final result = engine.callFunction(0, [i32(99)]);
    expect(result.single.value, 99);
  });

  test('calls a host function', () {
    final type = FuncType(params: [ValueType.i32], results: [ValueType.i32]);
    final engine = WasmExecutionEngine(
      _context(
        funcTypes: [type],
        funcBodies: [null],
        hostFunctions: [
          _InlineHostFunction(type, (args) => [i32(asI32(args.single) * 2)]),
        ],
      ),
    );

    final result = engine.callFunction(0, [i32(21)]);
    expect(result.single.value, 42);
  });

  test('supports direct calls between module functions', () {
    final calleeType = FuncType(params: const [], results: [ValueType.i32]);
    final callerType = FuncType(params: const [], results: [ValueType.i32]);
    final engine = WasmExecutionEngine(
      _context(
        funcTypes: [calleeType, callerType],
        funcBodies: [
          _body([0x41, 0x07]),
          _body([0x10, 0x00]),
        ],
        hostFunctions: [null, null],
      ),
    );

    final result = engine.callFunction(1, const []);
    expect(result.single.value, 7);
  });

  test('supports call_indirect through a function table', () {
    final table = Table(1)..set(0, 0);
    final signature = FuncType(params: [ValueType.i32], results: [ValueType.i32]);
    final callerType = FuncType(params: const [], results: [ValueType.i32]);
    final engine = WasmExecutionEngine(
      _context(
        tables: [table],
        funcTypes: [signature, callerType],
        funcBodies: [
          _body([0x20, 0x00, 0x20, 0x00, 0x6c]),
          _body([0x41, 0x05, 0x41, 0x00, 0x11, 0x00, 0x00]),
        ],
        hostFunctions: [null, null],
      ),
    );

    final result = engine.callFunction(1, const []);
    expect(result.single.value, 25);
  });

  test('reads and writes linear memory', () {
    final memory = LinearMemory(1, 2);
    final engine = WasmExecutionEngine(
      _context(
        memory: memory,
        funcTypes: [
          FuncType(params: const [], results: [ValueType.i32]),
        ],
        funcBodies: [
          _body([
            0x41, 0x00,
            0x41, 0x2a,
            0x36, 0x02, 0x00,
            0x41, 0x00,
            0x28, 0x02, 0x00,
          ]),
        ],
        hostFunctions: [null],
      ),
    );

    final result = engine.callFunction(0, const []);
    expect(result.single.value, 42);
  });

  test('supports simple block branching', () {
    final engine = WasmExecutionEngine(
      _context(
        funcTypes: [
          FuncType(params: const [], results: [ValueType.i32]),
        ],
        funcBodies: [
          _body([
            0x02, 0x7f,
            0x41, 0x07,
            0x0c, 0x00,
            0x41, 0x2a,
          ]),
        ],
        hostFunctions: [null],
      ),
    );

    final result = engine.callFunction(0, const []);
    expect(result.single.value, 7);
  });

  test('traps on an invalid function index', () {
    final engine = WasmExecutionEngine(
      _context(
        funcTypes: const [],
        funcBodies: const [],
        hostFunctions: const [],
      ),
    );

    expect(() => engine.callFunction(99, const []), throwsA(isA<TrapError>()));
  });

  test('traps on argument count mismatch', () {
    final engine = WasmExecutionEngine(
      _context(
        funcTypes: [
          FuncType(params: [ValueType.i32, ValueType.i32], results: [ValueType.i32]),
        ],
        funcBodies: [
          _body([0x20, 0x00]),
        ],
        hostFunctions: [null],
      ),
    );

    expect(() => engine.callFunction(0, [i32(1)]), throwsA(isA<TrapError>()));
  });
}
