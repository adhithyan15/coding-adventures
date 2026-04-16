import 'dart:typed_data';

import 'package:coding_adventures_wasm_leb128/wasm_leb128.dart';
import 'package:coding_adventures_wasm_opcodes/wasm_opcodes.dart';
import 'package:coding_adventures_wasm_types/wasm_types.dart';

const Object _unknown = Object();
const int _maxMemoryPages = 65536;

typedef _StackValue = Object;

enum ValidationErrorKind {
  invalidTypeIndex,
  invalidFuncIndex,
  invalidTableIndex,
  invalidMemoryIndex,
  invalidGlobalIndex,
  invalidLocalIndex,
  invalidLabelIndex,
  invalidElementIndex,
  multipleMemories,
  multipleTables,
  memoryLimitExceeded,
  memoryLimitOrder,
  tableLimitOrder,
  duplicateExportName,
  exportIndexOutOfRange,
  startFunctionBadType,
  immutableGlobalWrite,
  initExprInvalid,
  typeMismatch,
  stackUnderflow,
  stackHeightMismatch,
  returnTypeMismatch,
  callIndirectTypeMismatch,
}

class ValidationError implements Exception {
  ValidationError(this.kind, this.message);

  final ValidationErrorKind kind;
  final String message;

  @override
  String toString() => 'ValidationError($kind): $message';
}

typedef ValidationException = ValidationError;

class ValidatedModule {
  ValidatedModule({
    required this.module,
    required List<FuncType> funcTypes,
    required List<List<ValueType>> funcLocals,
  })  : funcTypes = List<FuncType>.unmodifiable(funcTypes),
        funcLocals = List<List<ValueType>>.unmodifiable(
          funcLocals.map(List<ValueType>.unmodifiable),
        );

  final WasmModule module;
  final List<FuncType> funcTypes;
  final List<List<ValueType>> funcLocals;
}

class IndexSpaces {
  const IndexSpaces({
    required this.funcTypes,
    required this.numImportedFuncs,
    required this.tableTypes,
    required this.numImportedTables,
    required this.memoryTypes,
    required this.numImportedMemories,
    required this.globalTypes,
    required this.numImportedGlobals,
    required this.numTypes,
  });

  final List<FuncType> funcTypes;
  final int numImportedFuncs;
  final List<TableType> tableTypes;
  final int numImportedTables;
  final List<MemoryType> memoryTypes;
  final int numImportedMemories;
  final List<GlobalType> globalTypes;
  final int numImportedGlobals;
  final int numTypes;
}

ValidatedModule validate(WasmModule module) {
  final indexSpaces = validateStructure(module);
  final funcLocals = <List<ValueType>>[];

  for (var localFuncIndex = 0; localFuncIndex < module.code.length; localFuncIndex += 1) {
    final funcIndex = indexSpaces.numImportedFuncs + localFuncIndex;
    funcLocals.add(
      validateFunction(
        funcIndex: funcIndex,
        funcType: indexSpaces.funcTypes[funcIndex],
        body: module.code[localFuncIndex],
        indexSpaces: indexSpaces,
        module: module,
      ),
    );
  }

  return ValidatedModule(
    module: module,
    funcTypes: indexSpaces.funcTypes,
    funcLocals: funcLocals,
  );
}

IndexSpaces validateStructure(WasmModule module) {
  final indexSpaces = _buildIndexSpaces(module);

  if (indexSpaces.tableTypes.length > 1) {
    throw ValidationError(
      ValidationErrorKind.multipleTables,
      'WASM 1.0 allows at most one table, found ${indexSpaces.tableTypes.length}.',
    );
  }

  if (indexSpaces.memoryTypes.length > 1) {
    throw ValidationError(
      ValidationErrorKind.multipleMemories,
      'WASM 1.0 allows at most one memory, found ${indexSpaces.memoryTypes.length}.',
    );
  }

  for (final memoryType in indexSpaces.memoryTypes) {
    _validateMemoryLimits(memoryType.limits);
  }

  for (final tableType in indexSpaces.tableTypes) {
    _validateTableLimits(tableType.limits);
  }

  _validateExports(module, indexSpaces);
  _validateStartFunction(module, indexSpaces);

  for (final global in module.globals) {
    validateConstExpr(global.initExpr, global.globalType.valueType, indexSpaces);
  }

  for (final element in module.elements) {
    if (element.tableIndex != 0 || element.tableIndex >= indexSpaces.tableTypes.length) {
      throw ValidationError(
        ValidationErrorKind.invalidTableIndex,
        'Element segment references table index ${element.tableIndex}, but only ${indexSpaces.tableTypes.length} table(s) exist.',
      );
    }
    validateConstExpr(element.offsetExpr, ValueType.i32, indexSpaces);
    for (final funcIndex in element.functionIndices) {
      _ensureIndex(
        funcIndex,
        indexSpaces.funcTypes.length,
        ValidationErrorKind.invalidFuncIndex,
        'Element segment references function index $funcIndex, but only ${indexSpaces.funcTypes.length} function(s) exist.',
      );
    }
  }

  for (final dataSegment in module.data) {
    if (dataSegment.memoryIndex != 0 || dataSegment.memoryIndex >= indexSpaces.memoryTypes.length) {
      throw ValidationError(
        ValidationErrorKind.invalidMemoryIndex,
        'Data segment references memory index ${dataSegment.memoryIndex}, but only ${indexSpaces.memoryTypes.length} memory/memories exist.',
      );
    }
    validateConstExpr(dataSegment.offsetExpr, ValueType.i32, indexSpaces);
  }

  return indexSpaces;
}

List<ValueType> validateFunction({
  required int funcIndex,
  required FuncType funcType,
  required FunctionBody body,
  required IndexSpaces indexSpaces,
  required WasmModule module,
}) {
  final funcLocals = _buildFuncLocals(funcType, body);
  final reader = _CodeReader(body.code, 'function $funcIndex');
  final valueStack = <_StackValue>[];
  final controlStack = <_ControlFrame>[
    _ControlFrame(
      kind: _ControlFrameKind.function,
      startTypes: const [],
      endTypes: funcType.results,
      stackHeight: 0,
      unreachable: false,
      inheritedUnreachable: false,
      seenElse: false,
    ),
  ];

  var finished = false;

  while (!reader.eof) {
    final instruction = reader.readInstruction();
    final frame = _currentFrame(controlStack);

    switch (instruction.info.name) {
      case 'unreachable':
        _markFrameUnreachable(frame, valueStack);
        break;
      case 'nop':
        break;
      case 'block':
      case 'loop':
      case 'if':
        final endTypes = switch (instruction.blockType) {
          null => const <ValueType>[],
          final value => <ValueType>[value],
        };
        if (instruction.info.name == 'if') {
          _popExpectedValue(
            valueStack,
            frame,
            ValueType.i32,
            ValidationErrorKind.typeMismatch,
          );
        }
        controlStack.add(
          _ControlFrame(
            kind: switch (instruction.info.name) {
              'block' => _ControlFrameKind.block,
              'loop' => _ControlFrameKind.loop,
              _ => _ControlFrameKind.if_,
            },
            startTypes: const [],
            endTypes: endTypes,
            stackHeight: valueStack.length,
            unreachable: frame.unreachable,
            inheritedUnreachable: frame.unreachable,
            seenElse: false,
          ),
        );
        break;
      case 'else':
        final ifFrame = _currentFrame(controlStack);
        if (ifFrame.kind != _ControlFrameKind.if_ || ifFrame.seenElse) {
          throw ValidationError(
            ValidationErrorKind.typeMismatch,
            "'else' encountered without a matching 'if' at byte offset ${instruction.offset}.",
          );
        }
        _assertFrameResults(ifFrame, valueStack);
        valueStack.length = ifFrame.stackHeight;
        ifFrame.unreachable = ifFrame.inheritedUnreachable;
        ifFrame.seenElse = true;
        _pushValueTypes(valueStack, ifFrame, ifFrame.startTypes);
        break;
      case 'end':
        final endingFrame = _currentFrame(controlStack);
        _assertFrameResults(endingFrame, valueStack);
        controlStack.removeLast();
        if (endingFrame.kind == _ControlFrameKind.function) {
          if (!reader.eof) {
            throw ValidationError(
              ValidationErrorKind.typeMismatch,
              'Function $funcIndex has trailing bytes after its final end.',
            );
          }
          finished = true;
        } else {
          valueStack.length = endingFrame.stackHeight;
          final parentFrame = _currentFrame(controlStack);
          _pushValueTypes(valueStack, parentFrame, endingFrame.endTypes);
        }
        break;
      case 'br':
        final target = _resolveLabelFrame(controlStack, instruction.labelIndex);
        _consumeExpectedSequence(
          valueStack,
          frame,
          _labelTypesFor(target),
          ValidationErrorKind.typeMismatch,
        );
        _markFrameUnreachable(frame, valueStack);
        break;
      case 'br_if':
        _popExpectedValue(
          valueStack,
          frame,
          ValueType.i32,
          ValidationErrorKind.typeMismatch,
        );
        final target = _resolveLabelFrame(controlStack, instruction.labelIndex);
        final preserved = _consumeExpectedSequence(
          valueStack,
          frame,
          _labelTypesFor(target),
          ValidationErrorKind.typeMismatch,
        );
        _pushRawValues(valueStack, preserved);
        break;
      case 'br_table':
        _popExpectedValue(
          valueStack,
          frame,
          ValueType.i32,
          ValidationErrorKind.typeMismatch,
        );
        final defaultTarget = _resolveLabelFrame(controlStack, instruction.defaultLabelIndex);
        final expectedTypes = _labelTypesFor(defaultTarget);
        for (final labelIndex in instruction.labelTable ?? const <int>[]) {
          final target = _resolveLabelFrame(controlStack, labelIndex);
          if (!_sameTypes(_labelTypesFor(target), expectedTypes)) {
            throw ValidationError(
              ValidationErrorKind.typeMismatch,
              'All br_table targets must have matching label types.',
            );
          }
        }
        _consumeExpectedSequence(
          valueStack,
          frame,
          expectedTypes,
          ValidationErrorKind.typeMismatch,
        );
        _markFrameUnreachable(frame, valueStack);
        break;
      case 'return':
        _consumeExpectedSequence(
          valueStack,
          frame,
          controlStack.first.endTypes,
          ValidationErrorKind.returnTypeMismatch,
        );
        _markFrameUnreachable(frame, valueStack);
        break;
      case 'call':
        final calleeType = _resolveFuncType(indexSpaces, instruction.funcIndex);
        _consumeExpectedSequence(
          valueStack,
          frame,
          calleeType.params,
          ValidationErrorKind.typeMismatch,
        );
        _pushValueTypes(valueStack, frame, calleeType.results);
        break;
      case 'call_indirect':
        if (indexSpaces.tableTypes.isEmpty) {
          throw ValidationError(
            ValidationErrorKind.invalidTableIndex,
            'call_indirect requires a table, but the module declares none.',
          );
        }
        final tableIndex = instruction.tableIndex ?? 0;
        if (tableIndex != 0 || tableIndex >= indexSpaces.tableTypes.length) {
          throw ValidationError(
            ValidationErrorKind.invalidTableIndex,
            'call_indirect references table index $tableIndex, but only ${indexSpaces.tableTypes.length} table(s) exist.',
          );
        }
        final typeIndex = instruction.typeIndex ?? -1;
        _ensureIndex(
          typeIndex,
          indexSpaces.numTypes,
          ValidationErrorKind.invalidTypeIndex,
          'call_indirect references type index $typeIndex, but only ${indexSpaces.numTypes} type(s) exist.',
        );
        _popExpectedValue(
          valueStack,
          frame,
          ValueType.i32,
          ValidationErrorKind.typeMismatch,
        );
        _consumeExpectedSequence(
          valueStack,
          frame,
          module.types[typeIndex].params,
          ValidationErrorKind.typeMismatch,
        );
        _pushValueTypes(valueStack, frame, module.types[typeIndex].results);
        break;
      case 'drop':
        _popAnyValue(valueStack, frame);
        break;
      case 'select':
        _popExpectedValue(
          valueStack,
          frame,
          ValueType.i32,
          ValidationErrorKind.typeMismatch,
        );
        final right = _popAnyValue(valueStack, frame);
        final left = _popAnyValue(valueStack, frame);
        if (left != _unknown && right != _unknown && left != right) {
          throw ValidationError(
            ValidationErrorKind.typeMismatch,
            'select expects two values of the same type.',
          );
        }
        final selected = switch ((left, right)) {
          (_unknown, final r) => r,
          (final l, _unknown) => l,
          _ => left,
        };
        _pushValue(valueStack, frame, selected);
        break;
      case 'local.get':
        _pushValueType(
          valueStack,
          frame,
          _resolveLocalType(funcLocals, instruction.localIndex),
        );
        break;
      case 'local.set':
        _popExpectedValue(
          valueStack,
          frame,
          _resolveLocalType(funcLocals, instruction.localIndex),
          ValidationErrorKind.typeMismatch,
        );
        break;
      case 'local.tee':
        final localType = _resolveLocalType(funcLocals, instruction.localIndex);
        _popExpectedValue(
          valueStack,
          frame,
          localType,
          ValidationErrorKind.typeMismatch,
        );
        _pushValueType(valueStack, frame, localType);
        break;
      case 'global.get':
        _pushValueType(
          valueStack,
          frame,
          _resolveGlobalType(indexSpaces, instruction.globalIndex).valueType,
        );
        break;
      case 'global.set':
        final globalType = _resolveGlobalType(indexSpaces, instruction.globalIndex);
        if (!globalType.mutable) {
          throw ValidationError(
            ValidationErrorKind.immutableGlobalWrite,
            'global.set references immutable global ${instruction.globalIndex}.',
          );
        }
        _popExpectedValue(
          valueStack,
          frame,
          globalType.valueType,
          ValidationErrorKind.typeMismatch,
        );
        break;
      default:
        _applyByCategory(instruction, frame, valueStack, indexSpaces);
        break;
    }
  }

  if (!finished) {
    throw ValidationError(
      ValidationErrorKind.typeMismatch,
      'Function $funcIndex ended without a final end opcode.',
    );
  }

  return funcLocals;
}
void validateConstExpr(
  Uint8List expr,
  ValueType expectedType,
  IndexSpaces indexSpaces,
) {
  final reader = _CodeReader(expr, 'constant expression');
  final stack = <ValueType>[];

  try {
    while (!reader.eof) {
      final instruction = reader.readInstruction();
      switch (instruction.info.name) {
        case 'i32.const':
          stack.add(ValueType.i32);
          break;
        case 'i64.const':
          stack.add(ValueType.i64);
          break;
        case 'f32.const':
          stack.add(ValueType.f32);
          break;
        case 'f64.const':
          stack.add(ValueType.f64);
          break;
        case 'global.get':
          final globalIndex = instruction.globalIndex ?? -1;
          if (globalIndex < 0 || globalIndex >= indexSpaces.numImportedGlobals) {
            throw ValidationError(
              ValidationErrorKind.initExprInvalid,
              'Constant expressions may only reference imported globals, but saw global $globalIndex.',
            );
          }
          stack.add(indexSpaces.globalTypes[globalIndex].valueType);
          break;
        case 'end':
          if (!reader.eof) {
            throw ValidationError(
              ValidationErrorKind.initExprInvalid,
              'Constant expression terminated before the end of its byte sequence.',
            );
          }
          if (stack.length != 1 || stack.first != expectedType) {
            throw ValidationError(
              ValidationErrorKind.initExprInvalid,
              'Constant expression must leave exactly ${_typeName(expectedType)} on the stack.',
            );
          }
          return;
        default:
          throw ValidationError(
            ValidationErrorKind.initExprInvalid,
            "Opcode '${instruction.info.name}' is not allowed in a constant expression.",
          );
      }
    }
  } on ValidationError catch (error) {
    if (error.kind == ValidationErrorKind.initExprInvalid) {
      rethrow;
    }
    throw ValidationError(ValidationErrorKind.initExprInvalid, error.message);
  }

  throw ValidationError(
    ValidationErrorKind.initExprInvalid,
    "Constant expression did not terminate with 'end'.",
  );
}

IndexSpaces _buildIndexSpaces(WasmModule module) {
  if (module.functions.length != module.code.length) {
    throw ValidationError(
      ValidationErrorKind.invalidFuncIndex,
      'Function section declares ${module.functions.length} local function(s), but code section contains ${module.code.length} body/bodies.',
    );
  }

  final funcTypes = <FuncType>[];
  final tableTypes = <TableType>[];
  final memoryTypes = <MemoryType>[];
  final globalTypes = <GlobalType>[];
  var numImportedFuncs = 0;
  var numImportedTables = 0;
  var numImportedMemories = 0;
  var numImportedGlobals = 0;

  for (final entry in module.imports) {
    switch (entry.kind) {
      case ExternalKind.function:
        final typeIndex = _importTypeIndex(entry);
        _ensureIndex(
          typeIndex,
          module.types.length,
          ValidationErrorKind.invalidTypeIndex,
          "Imported function '${entry.moduleName}.${entry.name}' references invalid type index $typeIndex.",
        );
        funcTypes.add(module.types[typeIndex]);
        numImportedFuncs += 1;
        break;
      case ExternalKind.table:
        tableTypes.add(entry.typeInfo as TableType);
        numImportedTables += 1;
        break;
      case ExternalKind.memory:
        memoryTypes.add(entry.typeInfo as MemoryType);
        numImportedMemories += 1;
        break;
      case ExternalKind.global:
        globalTypes.add(entry.typeInfo as GlobalType);
        numImportedGlobals += 1;
        break;
    }
  }

  for (final typeIndex in module.functions) {
    _ensureIndex(
      typeIndex,
      module.types.length,
      ValidationErrorKind.invalidTypeIndex,
      'Local function references invalid type index $typeIndex.',
    );
    funcTypes.add(module.types[typeIndex]);
  }

  tableTypes.addAll(module.tables);
  memoryTypes.addAll(module.memories);
  globalTypes.addAll(module.globals.map((global) => global.globalType));

  return IndexSpaces(
    funcTypes: List<FuncType>.unmodifiable(funcTypes),
    numImportedFuncs: numImportedFuncs,
    tableTypes: List<TableType>.unmodifiable(tableTypes),
    numImportedTables: numImportedTables,
    memoryTypes: List<MemoryType>.unmodifiable(memoryTypes),
    numImportedMemories: numImportedMemories,
    globalTypes: List<GlobalType>.unmodifiable(globalTypes),
    numImportedGlobals: numImportedGlobals,
    numTypes: module.types.length,
  );
}

void _validateExports(WasmModule module, IndexSpaces indexSpaces) {
  final seen = <String>{};
  for (final exportEntry in module.exports) {
    if (!seen.add(exportEntry.name)) {
      throw ValidationError(
        ValidationErrorKind.duplicateExportName,
        "Duplicate export name '${exportEntry.name}'.",
      );
    }

    final upperBound = switch (exportEntry.kind) {
      ExternalKind.function => indexSpaces.funcTypes.length,
      ExternalKind.table => indexSpaces.tableTypes.length,
      ExternalKind.memory => indexSpaces.memoryTypes.length,
      ExternalKind.global => indexSpaces.globalTypes.length,
    };

    if (exportEntry.index < 0 || exportEntry.index >= upperBound) {
      throw ValidationError(
        ValidationErrorKind.exportIndexOutOfRange,
        "Export '${exportEntry.name}' references index ${exportEntry.index}, but only $upperBound definition(s) exist.",
      );
    }
  }
}

void _validateStartFunction(WasmModule module, IndexSpaces indexSpaces) {
  final start = module.start;
  if (start == null) return;
  _ensureIndex(
    start,
    indexSpaces.funcTypes.length,
    ValidationErrorKind.invalidFuncIndex,
    'Start function index $start is out of range for ${indexSpaces.funcTypes.length} function(s).',
  );
  final startType = indexSpaces.funcTypes[start];
  if (startType.params.isNotEmpty || startType.results.isNotEmpty) {
    throw ValidationError(
      ValidationErrorKind.startFunctionBadType,
      'Start function must have type [] -> [].',
    );
  }
}

void _validateMemoryLimits(Limits limits) {
  if (limits.max != null && limits.max! > _maxMemoryPages) {
    throw ValidationError(
      ValidationErrorKind.memoryLimitExceeded,
      'Memory maximum ${limits.max} exceeds the WASM 1.0 limit of $_maxMemoryPages pages.',
    );
  }
  if (limits.max != null && limits.min > limits.max!) {
    throw ValidationError(
      ValidationErrorKind.memoryLimitOrder,
      'Memory minimum ${limits.min} exceeds maximum ${limits.max}.',
    );
  }
}

void _validateTableLimits(Limits limits) {
  if (limits.max != null && limits.min > limits.max!) {
    throw ValidationError(
      ValidationErrorKind.tableLimitOrder,
      'Table minimum ${limits.min} exceeds maximum ${limits.max}.',
    );
  }
}

void _applyByCategory(
  _DecodedInstruction instruction,
  _ControlFrame frame,
  List<_StackValue> stack,
  IndexSpaces indexSpaces,
) {
  switch (instruction.info.category) {
    case 'memory':
      _applyMemoryInstruction(instruction, frame, stack, indexSpaces);
      return;
    case 'numeric_i32':
      _consumeExpectedSequence(
        stack,
        frame,
        _repeatValueType(ValueType.i32, instruction.info.stackPop),
        ValidationErrorKind.typeMismatch,
      );
      _pushValueTypes(
        stack,
        frame,
        _repeatValueType(ValueType.i32, instruction.info.stackPush),
      );
      return;
    case 'numeric_i64':
      _consumeExpectedSequence(
        stack,
        frame,
        _repeatValueType(ValueType.i64, instruction.info.stackPop),
        ValidationErrorKind.typeMismatch,
      );
      _pushValueTypes(
        stack,
        frame,
        _repeatValueType(
          _isComparisonInstruction(instruction.info.name) ? ValueType.i32 : ValueType.i64,
          instruction.info.stackPush,
        ),
      );
      return;
    case 'numeric_f32':
      _consumeExpectedSequence(
        stack,
        frame,
        _repeatValueType(ValueType.f32, instruction.info.stackPop),
        ValidationErrorKind.typeMismatch,
      );
      _pushValueTypes(
        stack,
        frame,
        _repeatValueType(
          _isComparisonInstruction(instruction.info.name) ? ValueType.i32 : ValueType.f32,
          instruction.info.stackPush,
        ),
      );
      return;
    case 'numeric_f64':
      _consumeExpectedSequence(
        stack,
        frame,
        _repeatValueType(ValueType.f64, instruction.info.stackPop),
        ValidationErrorKind.typeMismatch,
      );
      _pushValueTypes(
        stack,
        frame,
        _repeatValueType(
          _isComparisonInstruction(instruction.info.name) ? ValueType.i32 : ValueType.f64,
          instruction.info.stackPush,
        ),
      );
      return;
    case 'conversion':
      final signature = _conversionSignature(instruction.info.name);
      _popExpectedValue(
        stack,
        frame,
        signature.$1,
        ValidationErrorKind.typeMismatch,
      );
      _pushValueType(stack, frame, signature.$2);
      return;
    default:
      throw ValidationError(
        ValidationErrorKind.typeMismatch,
        "Unsupported instruction category '${instruction.info.category}' for opcode '${instruction.info.name}'.",
      );
  }
}

void _applyMemoryInstruction(
  _DecodedInstruction instruction,
  _ControlFrame frame,
  List<_StackValue> stack,
  IndexSpaces indexSpaces,
) {
  if (indexSpaces.memoryTypes.isEmpty) {
    throw ValidationError(
      ValidationErrorKind.invalidMemoryIndex,
      "Instruction '${instruction.info.name}' requires a memory, but the module declares none.",
    );
  }

  if (instruction.info.name == 'memory.size' || instruction.info.name == 'memory.grow') {
    final memIndex = instruction.memIndex ?? 0;
    if (memIndex != 0 || memIndex >= indexSpaces.memoryTypes.length) {
      throw ValidationError(
        ValidationErrorKind.invalidMemoryIndex,
        "Instruction '${instruction.info.name}' references memory index $memIndex, but only ${indexSpaces.memoryTypes.length} memory/memories exist.",
      );
    }
    if (instruction.info.name == 'memory.grow') {
      _popExpectedValue(
        stack,
        frame,
        ValueType.i32,
        ValidationErrorKind.typeMismatch,
      );
    }
    _pushValueType(stack, frame, ValueType.i32);
    return;
  }

  final memarg = instruction.memarg;
  if (memarg == null) {
    throw ValidationError(
      ValidationErrorKind.typeMismatch,
      "Instruction '${instruction.info.name}' is missing its memarg immediate.",
    );
  }

  final maxAlign = _naturalAlignment(instruction.info.name);
  if (memarg.align > maxAlign) {
    throw ValidationError(
      ValidationErrorKind.typeMismatch,
      "Instruction '${instruction.info.name}' uses alignment ${memarg.align}, but the natural maximum is $maxAlign.",
    );
  }

  final valueType = _memoryValueType(instruction.info.name);
  if (instruction.info.name.contains('.store')) {
    _consumeExpectedSequence(
      stack,
      frame,
      <ValueType>[ValueType.i32, valueType],
      ValidationErrorKind.typeMismatch,
    );
    return;
  }

  _popExpectedValue(
    stack,
    frame,
    ValueType.i32,
    ValidationErrorKind.typeMismatch,
  );
  _pushValueType(stack, frame, valueType);
}

List<ValueType> _buildFuncLocals(FuncType funcType, FunctionBody body) {
  return List<ValueType>.unmodifiable(<ValueType>[
    ...funcType.params,
    ...body.locals,
  ]);
}

FuncType _resolveFuncType(IndexSpaces indexSpaces, int? funcIndex) {
  final index = funcIndex ?? -1;
  _ensureIndex(
    index,
    indexSpaces.funcTypes.length,
    ValidationErrorKind.invalidFuncIndex,
    'Function index $index is out of range for ${indexSpaces.funcTypes.length} function(s).',
  );
  return indexSpaces.funcTypes[index];
}

GlobalType _resolveGlobalType(IndexSpaces indexSpaces, int? globalIndex) {
  final index = globalIndex ?? -1;
  _ensureIndex(
    index,
    indexSpaces.globalTypes.length,
    ValidationErrorKind.invalidGlobalIndex,
    'Global index $index is out of range for ${indexSpaces.globalTypes.length} global(s).',
  );
  return indexSpaces.globalTypes[index];
}

ValueType _resolveLocalType(List<ValueType> funcLocals, int? localIndex) {
  final index = localIndex ?? -1;
  _ensureIndex(
    index,
    funcLocals.length,
    ValidationErrorKind.invalidLocalIndex,
    'Local index $index is out of range for ${funcLocals.length} local(s).',
  );
  return funcLocals[index];
}

_ControlFrame _resolveLabelFrame(List<_ControlFrame> controlStack, int? labelIndex) {
  final depth = labelIndex ?? -1;
  if (depth < 0 || depth >= controlStack.length) {
    throw ValidationError(
      ValidationErrorKind.invalidLabelIndex,
      'Label index $depth is out of range for ${controlStack.length} active control frame(s).',
    );
  }
  return controlStack[controlStack.length - 1 - depth];
}

void _assertFrameResults(_ControlFrame frame, List<_StackValue> stack) {
  if (!frame.unreachable) {
    final available = stack.length - frame.stackHeight;
    final expected = frame.endTypes.length;
    if (available != expected) {
      throw ValidationError(
        frame.kind == _ControlFrameKind.function
            ? ValidationErrorKind.returnTypeMismatch
            : ValidationErrorKind.stackHeightMismatch,
        frame.kind == _ControlFrameKind.function
            ? 'Function expected $expected value(s) at end, found $available.'
            : "Frame '${frame.kind.name}' expected $expected value(s) at end, found $available.",
      );
    }
  }

  _consumeExpectedSequence(
    stack,
    frame,
    frame.endTypes,
    frame.kind == _ControlFrameKind.function
        ? ValidationErrorKind.returnTypeMismatch
        : ValidationErrorKind.typeMismatch,
  );
}

List<_StackValue> _consumeExpectedSequence(
  List<_StackValue> stack,
  _ControlFrame frame,
  List<ValueType> expected,
  ValidationErrorKind mismatchKind,
) {
  final popped = <_StackValue>[];
  for (var i = expected.length - 1; i >= 0; i -= 1) {
    popped.add(_popExpectedValue(stack, frame, expected[i], mismatchKind));
  }
  return popped.reversed.toList(growable: false);
}

_StackValue _popExpectedValue(
  List<_StackValue> stack,
  _ControlFrame frame,
  ValueType expected,
  ValidationErrorKind mismatchKind,
) {
  final actual = _popAnyValue(stack, frame);
  if (actual != _unknown && actual != expected) {
    throw ValidationError(
      mismatchKind,
      'Type mismatch: expected ${_typeName(expected)}, got ${_typeName(actual)}.',
    );
  }
  return actual;
}

_StackValue _popAnyValue(List<_StackValue> stack, _ControlFrame frame) {
  if (frame.unreachable && stack.length <= frame.stackHeight) {
    return _unknown;
  }
  if (stack.isEmpty) {
    throw ValidationError(ValidationErrorKind.stackUnderflow, 'Operand stack underflow.');
  }
  return stack.removeLast();
}

void _pushValueTypes(
  List<_StackValue> stack,
  _ControlFrame frame,
  List<ValueType> valueTypes,
) {
  for (final valueType in valueTypes) {
    _pushValueType(stack, frame, valueType);
  }
}

void _pushValueType(
  List<_StackValue> stack,
  _ControlFrame frame,
  ValueType valueType,
) {
  _pushValue(stack, frame, valueType);
}

void _pushValue(List<_StackValue> stack, _ControlFrame frame, _StackValue value) {
  stack.add(frame.unreachable ? _unknown : value);
}

void _pushRawValues(List<_StackValue> stack, List<_StackValue> values) {
  stack.addAll(values);
}

void _markFrameUnreachable(_ControlFrame frame, List<_StackValue> stack) {
  frame.unreachable = true;
  stack.length = frame.stackHeight;
}

_ControlFrame _currentFrame(List<_ControlFrame> controlStack) {
  if (controlStack.isEmpty) {
    throw ValidationError(ValidationErrorKind.typeMismatch, 'Control stack underflow.');
  }
  return controlStack.last;
}

List<ValueType> _labelTypesFor(_ControlFrame frame) {
  return frame.kind == _ControlFrameKind.loop ? frame.startTypes : frame.endTypes;
}

ValueType _memoryValueType(String name) {
  if (name.startsWith('i32.')) return ValueType.i32;
  if (name.startsWith('i64.')) return ValueType.i64;
  if (name.startsWith('f32.')) return ValueType.f32;
  return ValueType.f64;
}

int _naturalAlignment(String name) {
  if (name == 'i32.load' || name == 'i32.store' || name == 'f32.load' || name == 'f32.store') {
    return 2;
  }
  if (name == 'i64.load' || name == 'i64.store' || name == 'f64.load' || name == 'f64.store') {
    return 3;
  }
  if (name.contains('8')) return 0;
  if (name.contains('16')) return 1;
  return 2;
}

(ValueType, ValueType) _conversionSignature(String name) {
  final signature = _conversionSignatures[name];
  if (signature == null) {
    throw ValidationError(
      ValidationErrorKind.typeMismatch,
      "Unsupported conversion instruction '$name'.",
    );
  }
  return signature;
}

List<ValueType> _repeatValueType(ValueType valueType, int count) {
  return List<ValueType>.filled(count, valueType);
}

bool _isComparisonInstruction(String name) {
  return name.endsWith('.eqz') ||
      name.endsWith('.eq') ||
      name.endsWith('.ne') ||
      name.endsWith('.lt_s') ||
      name.endsWith('.lt_u') ||
      name.endsWith('.lt') ||
      name.endsWith('.gt_s') ||
      name.endsWith('.gt_u') ||
      name.endsWith('.gt') ||
      name.endsWith('.le_s') ||
      name.endsWith('.le_u') ||
      name.endsWith('.le') ||
      name.endsWith('.ge_s') ||
      name.endsWith('.ge_u') ||
      name.endsWith('.ge');
}

bool _sameTypes(List<ValueType> a, List<ValueType> b) {
  if (a.length != b.length) return false;
  for (var index = 0; index < a.length; index += 1) {
    if (a[index] != b[index]) return false;
  }
  return true;
}

void _ensureIndex(
  int index,
  int length,
  ValidationErrorKind kind,
  String message,
) {
  if (index < 0 || index >= length) {
    throw ValidationError(kind, message);
  }
}

int _importTypeIndex(ImportEntry entry) {
  if (entry.typeInfo is! int) {
    throw ValidationError(
      ValidationErrorKind.invalidTypeIndex,
      "Import '${entry.moduleName}.${entry.name}' is not carrying a function type index.",
    );
  }
  return entry.typeInfo as int;
}

bool _isValueType(int value) {
  return value == ValueType.i32.code ||
      value == ValueType.i64.code ||
      value == ValueType.f32.code ||
      value == ValueType.f64.code;
}

String _typeName(Object? value) {
  if (identical(value, _unknown)) return 'unknown';
  if (value is ValueType) return value.name;
  return '$value';
}

final Map<String, (ValueType, ValueType)> _conversionSignatures =
    <String, (ValueType, ValueType)>{
  'i32.wrap_i64': (ValueType.i64, ValueType.i32),
  'i32.trunc_f32_s': (ValueType.f32, ValueType.i32),
  'i32.trunc_f32_u': (ValueType.f32, ValueType.i32),
  'i32.trunc_f64_s': (ValueType.f64, ValueType.i32),
  'i32.trunc_f64_u': (ValueType.f64, ValueType.i32),
  'i64.extend_i32_s': (ValueType.i32, ValueType.i64),
  'i64.extend_i32_u': (ValueType.i32, ValueType.i64),
  'i64.trunc_f32_s': (ValueType.f32, ValueType.i64),
  'i64.trunc_f32_u': (ValueType.f32, ValueType.i64),
  'i64.trunc_f64_s': (ValueType.f64, ValueType.i64),
  'i64.trunc_f64_u': (ValueType.f64, ValueType.i64),
  'f32.convert_i32_s': (ValueType.i32, ValueType.f32),
  'f32.convert_i32_u': (ValueType.i32, ValueType.f32),
  'f32.convert_i64_s': (ValueType.i64, ValueType.f32),
  'f32.convert_i64_u': (ValueType.i64, ValueType.f32),
  'f32.demote_f64': (ValueType.f64, ValueType.f32),
  'f64.convert_i32_s': (ValueType.i32, ValueType.f64),
  'f64.convert_i32_u': (ValueType.i32, ValueType.f64),
  'f64.convert_i64_s': (ValueType.i64, ValueType.f64),
  'f64.convert_i64_u': (ValueType.i64, ValueType.f64),
  'f64.promote_f32': (ValueType.f32, ValueType.f64),
  'i32.reinterpret_f32': (ValueType.f32, ValueType.i32),
  'i64.reinterpret_f64': (ValueType.f64, ValueType.i64),
  'f32.reinterpret_i32': (ValueType.i32, ValueType.f32),
  'f64.reinterpret_i64': (ValueType.i64, ValueType.f64),
};

enum _ControlFrameKind { function, block, loop, if_ }

class _ControlFrame {
  _ControlFrame({
    required this.kind,
    required List<ValueType> startTypes,
    required List<ValueType> endTypes,
    required this.stackHeight,
    required this.unreachable,
    required this.inheritedUnreachable,
    required this.seenElse,
  })  : startTypes = List<ValueType>.unmodifiable(startTypes),
        endTypes = List<ValueType>.unmodifiable(endTypes);

  final _ControlFrameKind kind;
  final List<ValueType> startTypes;
  final List<ValueType> endTypes;
  final int stackHeight;
  bool unreachable;
  final bool inheritedUnreachable;
  bool seenElse;
}

class _MemArg {
  const _MemArg({required this.align, required this.offset});

  final int align;
  final int offset;
}

class _DecodedInstruction {
  const _DecodedInstruction({
    required this.info,
    required this.offset,
    this.blockType,
    this.labelIndex,
    this.labelTable,
    this.defaultLabelIndex,
    this.funcIndex,
    this.typeIndex,
    this.tableIndex,
    this.localIndex,
    this.globalIndex,
    this.memarg,
    this.memIndex,
  });

  final OpcodeInfo info;
  final int offset;
  final ValueType? blockType;
  final int? labelIndex;
  final List<int>? labelTable;
  final int? defaultLabelIndex;
  final int? funcIndex;
  final int? typeIndex;
  final int? tableIndex;
  final int? localIndex;
  final int? globalIndex;
  final _MemArg? memarg;
  final int? memIndex;
}

class _CodeReader {
  _CodeReader(this.bytes, this.context);

  final Uint8List bytes;
  final String context;
  int offset = 0;

  bool get eof => offset >= bytes.length;

  _DecodedInstruction readInstruction() {
    final start = offset;
    final opcode = _readByte();
    final info = getOpcode(opcode);
    if (info == null) {
      throw ValidationError(
        ValidationErrorKind.typeMismatch,
        'Unknown opcode 0x${opcode.toRadixString(16).padLeft(2, '0')} in $context at byte $start.',
      );
    }

    ValueType? blockType;
    int? labelIndex;
    List<int>? labelTable;
    int? defaultLabelIndex;
    int? funcIndex;
    int? typeIndex;
    int? tableIndex;
    int? localIndex;
    int? globalIndex;
    _MemArg? memarg;
    int? memIndex;

    for (final immediate in info.immediates) {
      switch (immediate) {
        case 'blocktype':
          blockType = _readBlockType();
          break;
        case 'labelidx':
          labelIndex = _readU32();
          break;
        case 'vec_labelidx':
          final count = _readU32();
          labelTable = <int>[];
          for (var i = 0; i < count; i += 1) {
            labelTable.add(_readU32());
          }
          defaultLabelIndex = _readU32();
          break;
        case 'funcidx':
          funcIndex = _readU32();
          break;
        case 'typeidx':
          typeIndex = _readU32();
          break;
        case 'tableidx':
          tableIndex = _readU32();
          break;
        case 'localidx':
          localIndex = _readU32();
          break;
        case 'globalidx':
          globalIndex = _readU32();
          break;
        case 'memarg':
          memarg = _MemArg(align: _readU32(), offset: _readU32());
          break;
        case 'memidx':
          memIndex = _readU32();
          break;
        case 'i32':
        case 'i64':
          _readSigned();
          break;
        case 'f32':
          _readBytes(4);
          break;
        case 'f64':
          _readBytes(8);
          break;
        default:
          throw ValidationError(
            ValidationErrorKind.typeMismatch,
            "Unsupported immediate '$immediate' in $context.",
          );
      }
    }

    return _DecodedInstruction(
      info: info,
      offset: start,
      blockType: blockType,
      labelIndex: labelIndex,
      labelTable: labelTable == null ? null : List<int>.unmodifiable(labelTable),
      defaultLabelIndex: defaultLabelIndex,
      funcIndex: funcIndex,
      typeIndex: typeIndex,
      tableIndex: tableIndex,
      localIndex: localIndex,
      globalIndex: globalIndex,
      memarg: memarg,
      memIndex: memIndex,
    );
  }

  ValueType? _readBlockType() {
    final byte = _readByte();
    if (byte == emptyBlockType) return null;
    if (_isValueType(byte)) return ValueType.fromCode(byte);
    throw ValidationError(
      ValidationErrorKind.typeMismatch,
      'Unsupported blocktype byte 0x${byte.toRadixString(16).padLeft(2, '0')} in $context.',
    );
  }

  int _readByte() {
    if (offset >= bytes.length) {
      throw ValidationError(
        ValidationErrorKind.typeMismatch,
        'Unexpected end of $context at byte $offset.',
      );
    }
    return bytes[offset++];
  }

  Uint8List _readBytes(int length) {
    if (offset + length > bytes.length) {
      throw ValidationError(
        ValidationErrorKind.typeMismatch,
        'Unexpected end of $context at byte $offset.',
      );
    }
    final slice = bytes.sublist(offset, offset + length);
    offset += length;
    return Uint8List.fromList(slice);
  }

  int _readU32() {
    try {
      final decoded = decodeUnsigned(bytes, offset: offset, maxBytes: 5);
      offset += decoded.bytesRead;
      return decoded.value;
    } catch (error) {
      throw ValidationError(
        ValidationErrorKind.typeMismatch,
        'Invalid unsigned LEB128 in $context at byte $offset: $error',
      );
    }
  }

  int _readSigned() {
    try {
      final decoded = decodeSigned(bytes, offset: offset, maxBytes: 10, bitWidth: 64);
      offset += decoded.bytesRead;
      return decoded.value;
    } catch (error) {
      throw ValidationError(
        ValidationErrorKind.typeMismatch,
        'Invalid signed LEB128 in $context at byte $offset: $error',
      );
    }
  }
}
