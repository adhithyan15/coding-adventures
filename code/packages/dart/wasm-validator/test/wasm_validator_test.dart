import 'dart:typed_data';

import 'package:coding_adventures_wasm_types/wasm_types.dart';
import 'package:coding_adventures_wasm_validator/wasm_validator.dart';
import 'package:test/test.dart';

Uint8List _bytes(List<int> values) => Uint8List.fromList(values);

FunctionBody _body(List<int> code, {List<ValueType> locals = const []}) {
  return FunctionBody(locals: locals, code: _bytes(code));
}

WasmModule _module() => WasmModule();

MemoryType _memory(int min, [int? max]) => MemoryType(Limits(min: min, max: max));

TableType _table(int min, [int? max]) => TableType(
      elementType: funcRef,
      limits: Limits(min: min, max: max),
    );

void _expectValidationError(void Function() action, ValidationErrorKind kind) {
  try {
    action();
    fail('Expected ValidationError($kind)');
  } on ValidationError catch (error) {
    expect(error.kind, kind);
  }
}

void main() {
  test('validates an empty module', () {
    expect(() => validate(_module()), returnsNormally);
  });

  test('validates a simple i32.add function and caches locals', () {
    final module = _module();
    module.types.add(makeFuncType([ValueType.i32, ValueType.i32], [ValueType.i32]));
    module.functions.add(0);
    module.code.add(
      _body([0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b], locals: [ValueType.i32]),
    );

    final validated = validate(module);
    expect(validated.funcTypes.length, 1);
    expect(
      validated.funcLocals.single,
      [ValueType.i32, ValueType.i32, ValueType.i32],
    );
  });

  test('accepts unreachable dead code after br', () {
    final module = _module();
    module.types.add(makeFuncType(const [], [ValueType.i32]));
    module.functions.add(0);
    module.code.add(
      _body([
        0x02, 0x7f,
        0x41, 0x07,
        0x0c, 0x00,
        0x43, 0x00, 0x00, 0x00, 0x00,
        0x7c,
        0x0b,
        0x0b,
      ]),
    );

    expect(() => validate(module), returnsNormally);
  });

  test('rejects duplicate exports', () {
    final module = _module();
    module.types.add(makeFuncType(const [], const []));
    module.functions.add(0);
    module.code.add(_body([0x0b]));
    module.exports.addAll([
      ExportEntry(name: 'main', kind: ExternalKind.function, index: 0),
      ExportEntry(name: 'main', kind: ExternalKind.function, index: 0),
    ]);

    _expectValidationError(() => validate(module), ValidationErrorKind.duplicateExportName);
  });

  test('rejects multiple memories', () {
    final module = _module();
    module.memories.addAll([_memory(1), _memory(1)]);

    _expectValidationError(
      () => validateStructure(module),
      ValidationErrorKind.multipleMemories,
    );
  });

  test('rejects bad memory limits and bad table limits', () {
    final memoryModule = _module()..memories.add(_memory(5, 3));
    _expectValidationError(
      () => validateStructure(memoryModule),
      ValidationErrorKind.memoryLimitOrder,
    );

    final tableModule = _module()..tables.add(_table(5, 3));
    _expectValidationError(
      () => validateStructure(tableModule),
      ValidationErrorKind.tableLimitOrder,
    );
  });

  test('rejects start functions that are not () -> ()', () {
    final module = _module();
    module.types.add(makeFuncType([ValueType.i32], const []));
    module.functions.add(0);
    module.code.add(_body([0x0b]));
    module.start = 0;

    _expectValidationError(
      () => validate(module),
      ValidationErrorKind.startFunctionBadType,
    );
  });

  test('validates local.set and local.tee', () {
    final module = _module();
    module.types.add(makeFuncType(const [], [ValueType.i32]));
    module.functions.add(0);
    module.code.add(
      _body(
        [
          0x41, 0x05,
          0x21, 0x00,
          0x41, 0x07,
          0x22, 0x00,
          0x1a,
          0x20, 0x00,
          0x0b,
        ],
        locals: [ValueType.i32],
      ),
    );

    expect(() => validate(module), returnsNormally);
  });

  test('rejects stack underflow in numeric instructions', () {
    final module = _module();
    module.types.add(makeFuncType(const [], const []));
    module.functions.add(0);
    module.code.add(_body([0x41, 0x01, 0x6a, 0x0b]));

    _expectValidationError(
      () => validate(module),
      ValidationErrorKind.stackUnderflow,
    );
  });

  test('rejects memory instructions when no memory exists', () {
    final module = _module();
    module.types.add(makeFuncType(const [], const []));
    module.functions.add(0);
    module.code.add(
      _body([
        0x41, 0x00,
        0x28, 0x02, 0x00,
        0x1a,
        0x0b,
      ]),
    );

    _expectValidationError(
      () => validate(module),
      ValidationErrorKind.invalidMemoryIndex,
    );
  });

  test('validates memory store, load, size, and grow when a memory exists', () {
    final module = _module();
    module.types.add(makeFuncType(const [], const []));
    module.memories.add(_memory(1, 2));
    module.functions.add(0);
    module.code.add(
      _body([
        0x41, 0x00,
        0x41, 0x2a,
        0x36, 0x02, 0x00,
        0x41, 0x00,
        0x28, 0x02, 0x00,
        0x1a,
        0x3f, 0x00,
        0x1a,
        0x41, 0x00,
        0x40, 0x00,
        0x1a,
        0x0b,
      ]),
    );

    expect(() => validate(module), returnsNormally);
  });

  test('rejects global.set on immutable globals', () {
    final module = _module();
    module.types.add(makeFuncType(const [], const []));
    module.imports.add(
      ImportEntry(
        moduleName: 'env',
        name: 'flag',
        kind: ExternalKind.global,
        typeInfo: GlobalType(valueType: ValueType.i32, mutable: false),
      ),
    );
    module.functions.add(0);
    module.code.add(_body([0x41, 0x01, 0x24, 0x00, 0x0b]));

    _expectValidationError(
      () => validate(module),
      ValidationErrorKind.immutableGlobalWrite,
    );
  });

  test('validates call_indirect when the table and type exist', () {
    final module = _module();
    module.types.addAll([
      makeFuncType([ValueType.i32], [ValueType.i32]),
      makeFuncType(const [], [ValueType.i32]),
    ]);
    module.tables.add(_table(1, 1));
    module.functions.add(1);
    module.code.add(
      _body([
        0x41, 0x29,
        0x41, 0x00,
        0x11, 0x00, 0x00,
        0x0b,
      ]),
    );

    expect(() => validate(module), returnsNormally);
  });

  test('rejects malformed constant expressions', () {
    final module = _module();
    module.globals.add(
      GlobalEntry(
        globalType: const GlobalType(valueType: ValueType.i32, mutable: false),
        initExpr: _bytes([0x41, 0x01, 0x41, 0x02, 0x6a, 0x0b]),
      ),
    );

    _expectValidationError(
      () => validate(module),
      ValidationErrorKind.initExprInvalid,
    );
  });
}
