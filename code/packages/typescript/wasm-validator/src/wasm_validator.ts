import { decodeSigned, decodeUnsigned } from "@coding-adventures/wasm-leb128";
import { getOpcode } from "@coding-adventures/wasm-opcodes";
import {
  BlockType,
  ExternalKind,
  ValueType,
  WasmModule,
} from "@coding-adventures/wasm-types";
import type {
  FuncType,
  FunctionBody,
  GlobalType,
  Import,
  Limits,
  MemoryType,
  TableType,
} from "@coding-adventures/wasm-types";

const UNKNOWN = Symbol("wasm-validator-unknown");
const MAX_MEMORY_PAGES = 65536;

type StackValue = ValueType | typeof UNKNOWN;

type ControlFrameKind = "function" | "block" | "loop" | "if";

interface ControlFrame {
  kind: ControlFrameKind;
  startTypes: readonly ValueType[];
  endTypes: readonly ValueType[];
  stackHeight: number;
  unreachable: boolean;
  inheritedUnreachable: boolean;
  seenElse: boolean;
}

interface MemArg {
  align: number;
  offset: number;
}

interface DecodedInstruction {
  info: { name: string; opcode: number; category: string; stackPop: number; stackPush: number };
  offset: number;
  blockType?: ValueType | null;
  labelIndex?: number;
  labelTable?: readonly number[];
  defaultLabelIndex?: number;
  funcIndex?: number;
  typeIndex?: number;
  tableIndex?: number;
  localIndex?: number;
  globalIndex?: number;
  memarg?: MemArg;
  memIndex?: number;
}

export interface ValidatedModule {
  readonly module: WasmModule;
  readonly funcTypes: readonly FuncType[];
  readonly funcLocals: readonly (readonly ValueType[])[];
}

export interface IndexSpaces {
  readonly funcTypes: readonly FuncType[];
  readonly numImportedFuncs: number;
  readonly tableTypes: readonly TableType[];
  readonly numImportedTables: number;
  readonly memoryTypes: readonly MemoryType[];
  readonly numImportedMemories: number;
  readonly globalTypes: readonly GlobalType[];
  readonly numImportedGlobals: number;
  readonly numTypes: number;
}

export enum ValidationErrorKind {
  INVALID_TYPE_INDEX = "invalid_type_index",
  INVALID_FUNC_INDEX = "invalid_func_index",
  INVALID_TABLE_INDEX = "invalid_table_index",
  INVALID_MEMORY_INDEX = "invalid_memory_index",
  INVALID_GLOBAL_INDEX = "invalid_global_index",
  INVALID_LOCAL_INDEX = "invalid_local_index",
  INVALID_LABEL_INDEX = "invalid_label_index",
  INVALID_ELEMENT_INDEX = "invalid_element_index",
  MULTIPLE_MEMORIES = "multiple_memories",
  MULTIPLE_TABLES = "multiple_tables",
  MEMORY_LIMIT_EXCEEDED = "memory_limit_exceeded",
  MEMORY_LIMIT_ORDER = "memory_limit_order",
  TABLE_LIMIT_ORDER = "table_limit_order",
  DUPLICATE_EXPORT_NAME = "duplicate_export_name",
  EXPORT_INDEX_OUT_OF_RANGE = "export_index_out_of_range",
  START_FUNCTION_BAD_TYPE = "start_function_bad_type",
  IMMUTABLE_GLOBAL_WRITE = "immutable_global_write",
  INIT_EXPR_INVALID = "init_expr_invalid",
  TYPE_MISMATCH = "type_mismatch",
  STACK_UNDERFLOW = "stack_underflow",
  STACK_HEIGHT_MISMATCH = "stack_height_mismatch",
  RETURN_TYPE_MISMATCH = "return_type_mismatch",
  CALL_INDIRECT_TYPE_MISMATCH = "call_indirect_type_mismatch",
}

export class ValidationError extends Error {
  constructor(
    public readonly kind: ValidationErrorKind,
    message: string
  ) {
    super(message);
    this.name = "ValidationError";
  }
}

export function validate(module: WasmModule): ValidatedModule {
  const indexSpaces = validateStructure(module);
  const funcLocals = module.code.map((body, localFuncIndex) =>
    validateFunction(
      indexSpaces.numImportedFuncs + localFuncIndex,
      indexSpaces.funcTypes[indexSpaces.numImportedFuncs + localFuncIndex],
      body,
      indexSpaces,
      module
    )
  );

  return Object.freeze({
    module,
    funcTypes: Object.freeze([...indexSpaces.funcTypes]),
    funcLocals: Object.freeze(funcLocals.map((locals) => Object.freeze([...locals]))),
  });
}

export function validateStructure(module: WasmModule): IndexSpaces {
  const indexSpaces = buildIndexSpaces(module);

  if (indexSpaces.tableTypes.length > 1) {
    throw new ValidationError(
      ValidationErrorKind.MULTIPLE_TABLES,
      `WASM 1.0 allows at most one table, found ${indexSpaces.tableTypes.length}`
    );
  }

  if (indexSpaces.memoryTypes.length > 1) {
    throw new ValidationError(
      ValidationErrorKind.MULTIPLE_MEMORIES,
      `WASM 1.0 allows at most one memory, found ${indexSpaces.memoryTypes.length}`
    );
  }

  for (const memoryType of indexSpaces.memoryTypes) {
    validateMemoryLimits(memoryType.limits);
  }
  for (const tableType of indexSpaces.tableTypes) {
    validateTableLimits(tableType.limits);
  }

  validateExports(module, indexSpaces);
  validateStartFunction(module, indexSpaces);

  for (const global of module.globals) {
    validateConstExpr(global.initExpr, global.globalType.valueType, indexSpaces);
  }

  for (const element of module.elements) {
    if (element.tableIndex !== 0 || element.tableIndex >= indexSpaces.tableTypes.length) {
      throw new ValidationError(
        ValidationErrorKind.INVALID_TABLE_INDEX,
        `Element segment references table index ${element.tableIndex}, but only ${indexSpaces.tableTypes.length} table(s) exist`
      );
    }
    validateConstExpr(element.offsetExpr, ValueType.I32, indexSpaces);
    for (const funcIndex of element.functionIndices) {
      ensureIndex(
        funcIndex,
        indexSpaces.funcTypes.length,
        ValidationErrorKind.INVALID_FUNC_INDEX,
        `Element segment references function index ${funcIndex}, but only ${indexSpaces.funcTypes.length} function(s) exist`
      );
    }
  }

  for (const dataSegment of module.data) {
    if (dataSegment.memoryIndex !== 0 || dataSegment.memoryIndex >= indexSpaces.memoryTypes.length) {
      throw new ValidationError(
        ValidationErrorKind.INVALID_MEMORY_INDEX,
        `Data segment references memory index ${dataSegment.memoryIndex}, but only ${indexSpaces.memoryTypes.length} memory/memories exist`
      );
    }
    validateConstExpr(dataSegment.offsetExpr, ValueType.I32, indexSpaces);
  }

  return indexSpaces;
}

export function validateFunction(
  funcIndex: number,
  funcType: FuncType,
  body: FunctionBody,
  indexSpaces: IndexSpaces,
  module: WasmModule
): readonly ValueType[] {
  const funcLocals = buildFuncLocals(funcType, body);
  const reader = new CodeReader(body.code, `function ${funcIndex}`);
  const valueStack: StackValue[] = [];
  const controlStack: ControlFrame[] = [
    {
      kind: "function",
      startTypes: Object.freeze([]),
      endTypes: Object.freeze([...funcType.results]),
      stackHeight: 0,
      unreachable: false,
      inheritedUnreachable: false,
      seenElse: false,
    },
  ];

  let finished = false;

  while (!reader.eof()) {
    const instruction = reader.readInstruction();
    const frame = currentFrame(controlStack);

    switch (instruction.info.name) {
      case "unreachable":
        markFrameUnreachable(frame, valueStack);
        break;
      case "nop":
        break;
      case "block":
      case "loop":
      case "if": {
        const endTypes = instruction.blockType === null || instruction.blockType === undefined
          ? Object.freeze([] as ValueType[])
          : Object.freeze([instruction.blockType]);
        if (instruction.info.name === "if") {
          popExpectedValue(valueStack, frame, ValueType.I32, ValidationErrorKind.TYPE_MISMATCH);
        }
        controlStack.push({
          kind: instruction.info.name,
          startTypes: Object.freeze([]),
          endTypes,
          stackHeight: valueStack.length,
          unreachable: frame.unreachable,
          inheritedUnreachable: frame.unreachable,
          seenElse: false,
        });
        break;
      }
      case "else": {
        const ifFrame = currentFrame(controlStack);
        if (ifFrame.kind !== "if" || ifFrame.seenElse) {
          throw new ValidationError(
            ValidationErrorKind.TYPE_MISMATCH,
            `'else' encountered without a matching 'if' at byte offset ${instruction.offset}`
          );
        }
        assertFrameResults(ifFrame, valueStack);
        valueStack.length = ifFrame.stackHeight;
        ifFrame.unreachable = ifFrame.inheritedUnreachable;
        ifFrame.seenElse = true;
        pushValueTypes(valueStack, ifFrame, ifFrame.startTypes);
        break;
      }
      case "end": {
        const endingFrame = currentFrame(controlStack);
        assertFrameResults(endingFrame, valueStack);
        controlStack.pop();
        if (endingFrame.kind === "function") {
          if (!reader.eof()) {
            throw new ValidationError(
              ValidationErrorKind.TYPE_MISMATCH,
              `Function ${funcIndex} has trailing bytes after its final 'end'`
            );
          }
          finished = true;
        } else {
          valueStack.length = endingFrame.stackHeight;
          const parentFrame = currentFrame(controlStack);
          pushValueTypes(valueStack, parentFrame, endingFrame.endTypes);
        }
        break;
      }
      case "br": {
        const target = resolveLabelFrame(controlStack, instruction.labelIndex);
        consumeExpectedSequence(
          valueStack,
          frame,
          labelTypesFor(target),
          ValidationErrorKind.TYPE_MISMATCH
        );
        markFrameUnreachable(frame, valueStack);
        break;
      }
      case "br_if": {
        popExpectedValue(valueStack, frame, ValueType.I32, ValidationErrorKind.TYPE_MISMATCH);
        const target = resolveLabelFrame(controlStack, instruction.labelIndex);
        const preserved = consumeExpectedSequence(
          valueStack,
          frame,
          labelTypesFor(target),
          ValidationErrorKind.TYPE_MISMATCH
        );
        pushRawValues(valueStack, preserved);
        break;
      }
      case "br_table": {
        popExpectedValue(valueStack, frame, ValueType.I32, ValidationErrorKind.TYPE_MISMATCH);
        const defaultTarget = resolveLabelFrame(controlStack, instruction.defaultLabelIndex);
        const expectedTypes = labelTypesFor(defaultTarget);
        for (const labelIndex of instruction.labelTable ?? []) {
          const target = resolveLabelFrame(controlStack, labelIndex);
          if (!sameTypes(labelTypesFor(target), expectedTypes)) {
            throw new ValidationError(
              ValidationErrorKind.TYPE_MISMATCH,
              `All br_table targets must have matching label types, but label ${labelIndex} has ${formatTypes(labelTypesFor(target))} and default target has ${formatTypes(expectedTypes)}`
            );
          }
        }
        consumeExpectedSequence(
          valueStack,
          frame,
          expectedTypes,
          ValidationErrorKind.TYPE_MISMATCH
        );
        markFrameUnreachable(frame, valueStack);
        break;
      }
      case "return":
        consumeExpectedSequence(
          valueStack,
          frame,
          controlStack[0].endTypes,
          ValidationErrorKind.RETURN_TYPE_MISMATCH
        );
        markFrameUnreachable(frame, valueStack);
        break;
      case "call": {
        const calleeType = resolveFuncType(indexSpaces, instruction.funcIndex);
        consumeExpectedSequence(
          valueStack,
          frame,
          calleeType.params,
          ValidationErrorKind.TYPE_MISMATCH
        );
        pushValueTypes(valueStack, frame, calleeType.results);
        break;
      }
      case "call_indirect": {
        if (indexSpaces.tableTypes.length === 0) {
          throw new ValidationError(
            ValidationErrorKind.INVALID_TABLE_INDEX,
            "call_indirect requires a table, but the module declares none"
          );
        }
        const tableIndex = instruction.tableIndex ?? 0;
        if (tableIndex !== 0 || tableIndex >= indexSpaces.tableTypes.length) {
          throw new ValidationError(
            ValidationErrorKind.INVALID_TABLE_INDEX,
            `call_indirect references table index ${tableIndex}, but only ${indexSpaces.tableTypes.length} table(s) exist`
          );
        }
        const typeIndex = instruction.typeIndex ?? -1;
        ensureIndex(
          typeIndex,
          indexSpaces.numTypes,
          ValidationErrorKind.INVALID_TYPE_INDEX,
          `call_indirect references type index ${typeIndex}, but only ${indexSpaces.numTypes} type(s) exist`
        );
        popExpectedValue(valueStack, frame, ValueType.I32, ValidationErrorKind.TYPE_MISMATCH);
        consumeExpectedSequence(
          valueStack,
          frame,
          module.types[typeIndex].params,
          ValidationErrorKind.TYPE_MISMATCH
        );
        pushValueTypes(valueStack, frame, module.types[typeIndex].results);
        break;
      }
      case "drop":
        popAnyValue(valueStack, frame);
        break;
      case "select": {
        popExpectedValue(valueStack, frame, ValueType.I32, ValidationErrorKind.TYPE_MISMATCH);
        const right = popAnyValue(valueStack, frame);
        const left = popAnyValue(valueStack, frame);
        if (left !== UNKNOWN && right !== UNKNOWN && left !== right) {
          throw new ValidationError(
            ValidationErrorKind.TYPE_MISMATCH,
            `select expects two values of the same type, got ${typeName(left)} and ${typeName(right)}`
          );
        }
        const selected =
          left === UNKNOWN ? right :
          right === UNKNOWN ? left :
          left;
        pushValue(valueStack, frame, selected);
        break;
      }
      case "local.get": {
        const localType = resolveLocalType(funcLocals, instruction.localIndex);
        pushValueType(valueStack, frame, localType);
        break;
      }
      case "local.set": {
        const localType = resolveLocalType(funcLocals, instruction.localIndex);
        popExpectedValue(valueStack, frame, localType, ValidationErrorKind.TYPE_MISMATCH);
        break;
      }
      case "local.tee": {
        const localType = resolveLocalType(funcLocals, instruction.localIndex);
        popExpectedValue(valueStack, frame, localType, ValidationErrorKind.TYPE_MISMATCH);
        pushValueType(valueStack, frame, localType);
        break;
      }
      case "global.get": {
        const globalType = resolveGlobalType(indexSpaces, instruction.globalIndex);
        pushValueType(valueStack, frame, globalType.valueType);
        break;
      }
      case "global.set": {
        const globalType = resolveGlobalType(indexSpaces, instruction.globalIndex);
        if (!globalType.mutable) {
          throw new ValidationError(
            ValidationErrorKind.IMMUTABLE_GLOBAL_WRITE,
            `global.set references immutable global ${instruction.globalIndex}`
          );
        }
        popExpectedValue(valueStack, frame, globalType.valueType, ValidationErrorKind.TYPE_MISMATCH);
        break;
      }
      default:
        applyByCategory(instruction, frame, valueStack, indexSpaces);
        break;
    }
  }

  if (!finished) {
    throw new ValidationError(
      ValidationErrorKind.TYPE_MISMATCH,
      `Function ${funcIndex} ended without a final 'end' opcode`
    );
  }

  return Object.freeze([...funcLocals]);
}

export function validateConstExpr(
  expr: Uint8Array,
  expectedType: ValueType,
  indexSpaces: IndexSpaces
): void {
  const reader = new CodeReader(expr, "constant expression");
  const stack: ValueType[] = [];

  try {
    while (!reader.eof()) {
      const instruction = reader.readInstruction();
      switch (instruction.info.name) {
        case "i32.const":
          stack.push(ValueType.I32);
          break;
        case "i64.const":
          stack.push(ValueType.I64);
          break;
        case "f32.const":
          stack.push(ValueType.F32);
          break;
        case "f64.const":
          stack.push(ValueType.F64);
          break;
        case "global.get": {
          const globalIndex = instruction.globalIndex ?? -1;
          if (globalIndex < 0 || globalIndex >= indexSpaces.numImportedGlobals) {
            throw new ValidationError(
              ValidationErrorKind.INIT_EXPR_INVALID,
              `Constant expressions may only reference imported globals, but saw global ${globalIndex}`
            );
          }
          stack.push(indexSpaces.globalTypes[globalIndex].valueType);
          break;
        }
        case "end":
          if (!reader.eof()) {
            throw new ValidationError(
              ValidationErrorKind.INIT_EXPR_INVALID,
              "Constant expression terminated before the end of its byte sequence"
            );
          }
          if (stack.length !== 1 || stack[0] !== expectedType) {
            throw new ValidationError(
              ValidationErrorKind.INIT_EXPR_INVALID,
              `Constant expression must leave exactly ${typeName(expectedType)} on the stack, found ${formatTypes(stack)}`
            );
          }
          return;
        default:
          throw new ValidationError(
            ValidationErrorKind.INIT_EXPR_INVALID,
            `Opcode '${instruction.info.name}' is not allowed in a constant expression`
          );
      }
    }
  } catch (error) {
    if (error instanceof ValidationError) {
      if (error.kind === ValidationErrorKind.INIT_EXPR_INVALID) {
        throw error;
      }
      throw new ValidationError(ValidationErrorKind.INIT_EXPR_INVALID, error.message);
    }
    throw error;
  }

  throw new ValidationError(
    ValidationErrorKind.INIT_EXPR_INVALID,
    "Constant expression did not terminate with 'end'"
  );
}

function buildIndexSpaces(module: WasmModule): IndexSpaces {
  if (module.functions.length !== module.code.length) {
    throw new ValidationError(
      ValidationErrorKind.INVALID_FUNC_INDEX,
      `Function section declares ${module.functions.length} local function(s), but code section contains ${module.code.length} body/bodies`
    );
  }

  const funcTypes: FuncType[] = [];
  const tableTypes: TableType[] = [];
  const memoryTypes: MemoryType[] = [];
  const globalTypes: GlobalType[] = [];
  let numImportedFuncs = 0;
  let numImportedTables = 0;
  let numImportedMemories = 0;
  let numImportedGlobals = 0;

  for (const entry of module.imports) {
    switch (entry.kind) {
      case ExternalKind.FUNCTION: {
        const typeIndex = importTypeIndex(entry);
        ensureIndex(
          typeIndex,
          module.types.length,
          ValidationErrorKind.INVALID_TYPE_INDEX,
          `Imported function '${entry.moduleName}.${entry.name}' references type index ${typeIndex}, but only ${module.types.length} type(s) exist`
        );
        funcTypes.push(module.types[typeIndex]);
        numImportedFuncs += 1;
        break;
      }
      case ExternalKind.TABLE:
        tableTypes.push(entry.typeInfo as TableType);
        numImportedTables += 1;
        break;
      case ExternalKind.MEMORY:
        memoryTypes.push(entry.typeInfo as MemoryType);
        numImportedMemories += 1;
        break;
      case ExternalKind.GLOBAL:
        globalTypes.push(entry.typeInfo as GlobalType);
        numImportedGlobals += 1;
        break;
      default:
        break;
    }
  }

  for (const typeIndex of module.functions) {
    ensureIndex(
      typeIndex,
      module.types.length,
      ValidationErrorKind.INVALID_TYPE_INDEX,
      `Local function references type index ${typeIndex}, but only ${module.types.length} type(s) exist`
    );
    funcTypes.push(module.types[typeIndex]);
  }

  tableTypes.push(...module.tables);
  memoryTypes.push(...module.memories);
  globalTypes.push(...module.globals.map((global) => global.globalType));

  return Object.freeze({
    funcTypes: Object.freeze([...funcTypes]),
    numImportedFuncs,
    tableTypes: Object.freeze([...tableTypes]),
    numImportedTables,
    memoryTypes: Object.freeze([...memoryTypes]),
    numImportedMemories,
    globalTypes: Object.freeze([...globalTypes]),
    numImportedGlobals,
    numTypes: module.types.length,
  });
}

function validateExports(module: WasmModule, indexSpaces: IndexSpaces): void {
  const seen = new Set<string>();
  for (const exportEntry of module.exports) {
    if (seen.has(exportEntry.name)) {
      throw new ValidationError(
        ValidationErrorKind.DUPLICATE_EXPORT_NAME,
        `Duplicate export name '${exportEntry.name}'`
      );
    }
    seen.add(exportEntry.name);

    const upperBound =
      exportEntry.kind === ExternalKind.FUNCTION ? indexSpaces.funcTypes.length :
      exportEntry.kind === ExternalKind.TABLE ? indexSpaces.tableTypes.length :
      exportEntry.kind === ExternalKind.MEMORY ? indexSpaces.memoryTypes.length :
      exportEntry.kind === ExternalKind.GLOBAL ? indexSpaces.globalTypes.length :
      0;

    if (exportEntry.index < 0 || exportEntry.index >= upperBound) {
      throw new ValidationError(
        ValidationErrorKind.EXPORT_INDEX_OUT_OF_RANGE,
        `Export '${exportEntry.name}' references index ${exportEntry.index}, but only ${upperBound} definition(s) exist for kind ${exportEntry.kind}`
      );
    }
  }
}

function validateStartFunction(module: WasmModule, indexSpaces: IndexSpaces): void {
  if (module.start === null) {
    return;
  }
  ensureIndex(
    module.start,
    indexSpaces.funcTypes.length,
    ValidationErrorKind.INVALID_FUNC_INDEX,
    `Start function index ${module.start} is out of range for ${indexSpaces.funcTypes.length} function(s)`
  );
  const startType = indexSpaces.funcTypes[module.start];
  if (startType.params.length !== 0 || startType.results.length !== 0) {
    throw new ValidationError(
      ValidationErrorKind.START_FUNCTION_BAD_TYPE,
      `Start function must have type () -> (), got ${formatFuncType(startType)}`
    );
  }
}

function validateMemoryLimits(limits: Limits): void {
  if (limits.max !== null && limits.max > MAX_MEMORY_PAGES) {
    throw new ValidationError(
      ValidationErrorKind.MEMORY_LIMIT_EXCEEDED,
      `Memory maximum ${limits.max} exceeds the WASM 1.0 limit of ${MAX_MEMORY_PAGES} pages`
    );
  }
  if (limits.max !== null && limits.min > limits.max) {
    throw new ValidationError(
      ValidationErrorKind.MEMORY_LIMIT_ORDER,
      `Memory minimum ${limits.min} exceeds maximum ${limits.max}`
    );
  }
}

function validateTableLimits(limits: Limits): void {
  if (limits.max !== null && limits.min > limits.max) {
    throw new ValidationError(
      ValidationErrorKind.TABLE_LIMIT_ORDER,
      `Table minimum ${limits.min} exceeds maximum ${limits.max}`
    );
  }
}

function applyByCategory(
  instruction: DecodedInstruction,
  frame: ControlFrame,
  stack: StackValue[],
  indexSpaces: IndexSpaces
): void {
  switch (instruction.info.category) {
    case "memory":
      applyMemoryInstruction(instruction, frame, stack, indexSpaces);
      return;
    case "numeric_i32":
      consumeExpectedSequence(
        stack,
        frame,
        repeatValueType(ValueType.I32, instruction.info.stackPop),
        ValidationErrorKind.TYPE_MISMATCH
      );
      pushValueTypes(stack, frame, repeatValueType(ValueType.I32, instruction.info.stackPush));
      return;
    case "numeric_i64":
      consumeExpectedSequence(
        stack,
        frame,
        repeatValueType(ValueType.I64, instruction.info.stackPop),
        ValidationErrorKind.TYPE_MISMATCH
      );
      pushValueTypes(
        stack,
        frame,
        repeatValueType(
          isComparisonInstruction(instruction.info.name) ? ValueType.I32 : ValueType.I64,
          instruction.info.stackPush
        )
      );
      return;
    case "numeric_f32":
      consumeExpectedSequence(
        stack,
        frame,
        repeatValueType(ValueType.F32, instruction.info.stackPop),
        ValidationErrorKind.TYPE_MISMATCH
      );
      pushValueTypes(
        stack,
        frame,
        repeatValueType(
          isComparisonInstruction(instruction.info.name) ? ValueType.I32 : ValueType.F32,
          instruction.info.stackPush
        )
      );
      return;
    case "numeric_f64":
      consumeExpectedSequence(
        stack,
        frame,
        repeatValueType(ValueType.F64, instruction.info.stackPop),
        ValidationErrorKind.TYPE_MISMATCH
      );
      pushValueTypes(
        stack,
        frame,
        repeatValueType(
          isComparisonInstruction(instruction.info.name) ? ValueType.I32 : ValueType.F64,
          instruction.info.stackPush
        )
      );
      return;
    case "conversion": {
      const [inputType, outputType] = conversionSignature(instruction.info.name);
      popExpectedValue(stack, frame, inputType, ValidationErrorKind.TYPE_MISMATCH);
      pushValueType(stack, frame, outputType);
      return;
    }
    default:
      throw new ValidationError(
        ValidationErrorKind.TYPE_MISMATCH,
        `Unsupported instruction category '${instruction.info.category}' for opcode '${instruction.info.name}'`
      );
  }
}

function applyMemoryInstruction(
  instruction: DecodedInstruction,
  frame: ControlFrame,
  stack: StackValue[],
  indexSpaces: IndexSpaces
): void {
  if (indexSpaces.memoryTypes.length === 0) {
    throw new ValidationError(
      ValidationErrorKind.INVALID_MEMORY_INDEX,
      `Instruction '${instruction.info.name}' requires a memory, but the module declares none`
    );
  }

  if (instruction.info.name === "memory.size" || instruction.info.name === "memory.grow") {
    const memIndex = instruction.memIndex ?? 0;
    if (memIndex !== 0 || memIndex >= indexSpaces.memoryTypes.length) {
      throw new ValidationError(
        ValidationErrorKind.INVALID_MEMORY_INDEX,
        `Instruction '${instruction.info.name}' references memory index ${memIndex}, but only ${indexSpaces.memoryTypes.length} memory/memories exist`
      );
    }
    if (instruction.info.name === "memory.grow") {
      popExpectedValue(stack, frame, ValueType.I32, ValidationErrorKind.TYPE_MISMATCH);
    }
    pushValueType(stack, frame, ValueType.I32);
    return;
  }

  const memarg = instruction.memarg;
  if (!memarg) {
    throw new ValidationError(
      ValidationErrorKind.TYPE_MISMATCH,
      `Instruction '${instruction.info.name}' is missing its memarg immediate`
    );
  }

  const maxAlign = naturalAlignment(instruction.info.name);
  if (memarg.align > maxAlign) {
    throw new ValidationError(
      ValidationErrorKind.TYPE_MISMATCH,
      `Instruction '${instruction.info.name}' uses alignment ${memarg.align}, but the natural maximum is ${maxAlign}`
    );
  }

  const valueType = memoryValueType(instruction.info.name);
  if (instruction.info.name.includes(".store")) {
    consumeExpectedSequence(
      stack,
      frame,
      [ValueType.I32, valueType],
      ValidationErrorKind.TYPE_MISMATCH
    );
    return;
  }

  popExpectedValue(stack, frame, ValueType.I32, ValidationErrorKind.TYPE_MISMATCH);
  pushValueType(stack, frame, valueType);
}

function buildFuncLocals(funcType: FuncType, body: FunctionBody): readonly ValueType[] {
  return Object.freeze([...funcType.params, ...body.locals]);
}

function resolveFuncType(indexSpaces: IndexSpaces, funcIndex: number | undefined): FuncType {
  ensureIndex(
    funcIndex ?? -1,
    indexSpaces.funcTypes.length,
    ValidationErrorKind.INVALID_FUNC_INDEX,
    `Function index ${funcIndex ?? -1} is out of range for ${indexSpaces.funcTypes.length} function(s)`
  );
  return indexSpaces.funcTypes[funcIndex as number];
}

function resolveGlobalType(indexSpaces: IndexSpaces, globalIndex: number | undefined): GlobalType {
  ensureIndex(
    globalIndex ?? -1,
    indexSpaces.globalTypes.length,
    ValidationErrorKind.INVALID_GLOBAL_INDEX,
    `Global index ${globalIndex ?? -1} is out of range for ${indexSpaces.globalTypes.length} global(s)`
  );
  return indexSpaces.globalTypes[globalIndex as number];
}

function resolveLocalType(funcLocals: readonly ValueType[], localIndex: number | undefined): ValueType {
  ensureIndex(
    localIndex ?? -1,
    funcLocals.length,
    ValidationErrorKind.INVALID_LOCAL_INDEX,
    `Local index ${localIndex ?? -1} is out of range for ${funcLocals.length} local(s)`
  );
  return funcLocals[localIndex as number];
}

function resolveLabelFrame(controlStack: readonly ControlFrame[], labelIndex: number | undefined): ControlFrame {
  const depth = labelIndex ?? -1;
  if (depth < 0 || depth >= controlStack.length) {
    throw new ValidationError(
      ValidationErrorKind.INVALID_LABEL_INDEX,
      `Label index ${depth} is out of range for ${controlStack.length} active control frame(s)`
    );
  }
  return controlStack[controlStack.length - 1 - depth];
}

function assertFrameResults(frame: ControlFrame, stack: StackValue[]): void {
  if (!frame.unreachable) {
    const available = stack.length - frame.stackHeight;
    const expected = frame.endTypes.length;
    if (available !== expected) {
      throw new ValidationError(
        frame.kind === "function"
          ? ValidationErrorKind.RETURN_TYPE_MISMATCH
          : ValidationErrorKind.STACK_HEIGHT_MISMATCH,
        `${frame.kind === "function" ? "Function" : `Frame '${frame.kind}'`} expected ${expected} value(s) at end, found ${available}`
      );
    }
  }

  consumeExpectedSequence(
    stack,
    frame,
    frame.endTypes,
    frame.kind === "function"
      ? ValidationErrorKind.RETURN_TYPE_MISMATCH
      : ValidationErrorKind.TYPE_MISMATCH
  );
}

function consumeExpectedSequence(
  stack: StackValue[],
  frame: ControlFrame,
  expected: readonly ValueType[],
  mismatchKind: ValidationErrorKind
): StackValue[] {
  const popped: StackValue[] = [];
  for (let i = expected.length - 1; i >= 0; i -= 1) {
    popped.push(popExpectedValue(stack, frame, expected[i], mismatchKind));
  }
  return popped.reverse();
}

function popExpectedValue(
  stack: StackValue[],
  frame: ControlFrame,
  expected: ValueType,
  mismatchKind: ValidationErrorKind
): StackValue {
  const actual = popAnyValue(stack, frame);
  if (actual !== UNKNOWN && actual !== expected) {
    throw new ValidationError(
      mismatchKind,
      `Type mismatch: expected ${typeName(expected)}, got ${typeName(actual)}`
    );
  }
  return actual;
}

function popAnyValue(stack: StackValue[], frame: ControlFrame): StackValue {
  if (frame.unreachable && stack.length <= frame.stackHeight) {
    return UNKNOWN;
  }
  const value = stack.pop();
  if (value === undefined) {
    throw new ValidationError(
      ValidationErrorKind.STACK_UNDERFLOW,
      "Operand stack underflow"
    );
  }
  return value;
}

function pushValueTypes(
  stack: StackValue[],
  frame: ControlFrame,
  valueTypes: readonly ValueType[]
): void {
  for (const valueType of valueTypes) {
    pushValueType(stack, frame, valueType);
  }
}

function pushValueType(stack: StackValue[], frame: ControlFrame, valueType: ValueType): void {
  pushValue(stack, frame, valueType);
}

function pushValue(stack: StackValue[], frame: ControlFrame, value: StackValue): void {
  stack.push(frame.unreachable ? UNKNOWN : value);
}

function pushRawValues(stack: StackValue[], values: readonly StackValue[]): void {
  stack.push(...values);
}

function markFrameUnreachable(frame: ControlFrame, stack: StackValue[]): void {
  frame.unreachable = true;
  stack.length = frame.stackHeight;
}

function currentFrame(controlStack: readonly ControlFrame[]): ControlFrame {
  const frame = controlStack[controlStack.length - 1];
  if (!frame) {
    throw new ValidationError(
      ValidationErrorKind.TYPE_MISMATCH,
      "Control stack underflow"
    );
  }
  return frame;
}

function labelTypesFor(frame: ControlFrame): readonly ValueType[] {
  return frame.kind === "loop" ? frame.startTypes : frame.endTypes;
}

function memoryValueType(name: string): ValueType {
  if (name.startsWith("i32.")) {
    return ValueType.I32;
  }
  if (name.startsWith("i64.")) {
    return ValueType.I64;
  }
  if (name.startsWith("f32.")) {
    return ValueType.F32;
  }
  return ValueType.F64;
}

function naturalAlignment(name: string): number {
  if (
    name === "i32.load" ||
    name === "i32.store" ||
    name === "f32.load" ||
    name === "f32.store"
  ) {
    return 2;
  }
  if (
    name === "i64.load" ||
    name === "i64.store" ||
    name === "f64.load" ||
    name === "f64.store"
  ) {
    return 3;
  }
  if (name.includes("8")) {
    return 0;
  }
  if (name.includes("16")) {
    return 1;
  }
  return 2;
}

function conversionSignature(name: string): readonly [ValueType, ValueType] {
  const signature = CONVERSION_SIGNATURES[name];
  if (!signature) {
    throw new ValidationError(
      ValidationErrorKind.TYPE_MISMATCH,
      `Unsupported conversion instruction '${name}'`
    );
  }
  return signature;
}

function repeatValueType(valueType: ValueType, count: number): ValueType[] {
  return Array.from({ length: count }, () => valueType);
}

function isComparisonInstruction(name: string): boolean {
  return /\.(eqz|eq|ne|lt(?:_[su])?|gt(?:_[su])?|le(?:_[su])?|ge(?:_[su])?)$/.test(name);
}

function sameTypes(a: readonly ValueType[], b: readonly ValueType[]): boolean {
  return a.length === b.length && a.every((value, index) => value === b[index]);
}

function ensureIndex(
  index: number,
  length: number,
  kind: ValidationErrorKind,
  message: string
): void {
  if (!Number.isInteger(index) || index < 0 || index >= length) {
    throw new ValidationError(kind, message);
  }
}

function importTypeIndex(entry: Import): number {
  if (typeof entry.typeInfo !== "number") {
    throw new ValidationError(
      ValidationErrorKind.INVALID_TYPE_INDEX,
      `Import '${entry.moduleName}.${entry.name}' is not carrying a function type index`
    );
  }
  return entry.typeInfo;
}

function isValueType(value: number): value is ValueType {
  return (
    value === ValueType.I32 ||
    value === ValueType.I64 ||
    value === ValueType.F32 ||
    value === ValueType.F64
  );
}

function typeName(value: StackValue): string {
  if (value === UNKNOWN) {
    return "unknown";
  }
  return (
    value === ValueType.I32 ? "i32" :
    value === ValueType.I64 ? "i64" :
    value === ValueType.F32 ? "f32" :
    "f64"
  );
}

function formatTypes(values: readonly ValueType[]): string {
  return `[${values.map((value) => typeName(value)).join(", ")}]`;
}

function formatFuncType(funcType: FuncType): string {
  return `${formatTypes(funcType.params)} -> ${formatTypes(funcType.results)}`;
}

const CONVERSION_SIGNATURES: Record<string, readonly [ValueType, ValueType]> = {
  "i32.wrap_i64": [ValueType.I64, ValueType.I32],
  "i32.trunc_f32_s": [ValueType.F32, ValueType.I32],
  "i32.trunc_f32_u": [ValueType.F32, ValueType.I32],
  "i32.trunc_f64_s": [ValueType.F64, ValueType.I32],
  "i32.trunc_f64_u": [ValueType.F64, ValueType.I32],
  "i64.extend_i32_s": [ValueType.I32, ValueType.I64],
  "i64.extend_i32_u": [ValueType.I32, ValueType.I64],
  "i64.trunc_f32_s": [ValueType.F32, ValueType.I64],
  "i64.trunc_f32_u": [ValueType.F32, ValueType.I64],
  "i64.trunc_f64_s": [ValueType.F64, ValueType.I64],
  "i64.trunc_f64_u": [ValueType.F64, ValueType.I64],
  "f32.convert_i32_s": [ValueType.I32, ValueType.F32],
  "f32.convert_i32_u": [ValueType.I32, ValueType.F32],
  "f32.convert_i64_s": [ValueType.I64, ValueType.F32],
  "f32.convert_i64_u": [ValueType.I64, ValueType.F32],
  "f32.demote_f64": [ValueType.F64, ValueType.F32],
  "f64.convert_i32_s": [ValueType.I32, ValueType.F64],
  "f64.convert_i32_u": [ValueType.I32, ValueType.F64],
  "f64.convert_i64_s": [ValueType.I64, ValueType.F64],
  "f64.convert_i64_u": [ValueType.I64, ValueType.F64],
  "f64.promote_f32": [ValueType.F32, ValueType.F64],
  "i32.reinterpret_f32": [ValueType.F32, ValueType.I32],
  "i64.reinterpret_f64": [ValueType.F64, ValueType.I64],
  "f32.reinterpret_i32": [ValueType.I32, ValueType.F32],
  "f64.reinterpret_i64": [ValueType.I64, ValueType.F64],
};

class CodeReader {
  private offset = 0;

  constructor(
    private readonly bytes: Uint8Array,
    private readonly context: string
  ) {}

  eof(): boolean {
    return this.offset >= this.bytes.length;
  }

  readInstruction(): DecodedInstruction {
    const start = this.offset;
    const opcode = this.readByte();
    const info = getOpcode(opcode);
    if (!info) {
      throw new ValidationError(
        ValidationErrorKind.TYPE_MISMATCH,
        `Unknown opcode 0x${opcode.toString(16).padStart(2, "0")} in ${this.context} at byte ${start}`
      );
    }

    const instruction: DecodedInstruction = {
      info,
      offset: start,
    };

    for (const immediate of info.immediates) {
      switch (immediate) {
        case "blocktype":
          instruction.blockType = this.readBlockType();
          break;
        case "labelidx":
          instruction.labelIndex = this.readU32();
          break;
        case "vec_labelidx": {
          const count = this.readU32();
          const labels: number[] = [];
          for (let i = 0; i < count; i += 1) {
            labels.push(this.readU32());
          }
          instruction.labelTable = Object.freeze(labels);
          instruction.defaultLabelIndex = this.readU32();
          break;
        }
        case "funcidx":
          instruction.funcIndex = this.readU32();
          break;
        case "typeidx":
          instruction.typeIndex = this.readU32();
          break;
        case "tableidx":
          instruction.tableIndex = this.readU32();
          break;
        case "localidx":
          instruction.localIndex = this.readU32();
          break;
        case "globalidx":
          instruction.globalIndex = this.readU32();
          break;
        case "memarg":
          instruction.memarg = { align: this.readU32(), offset: this.readU32() };
          break;
        case "memidx":
          instruction.memIndex = this.readU32();
          break;
        case "i32":
        case "i64":
          this.readSigned();
          break;
        case "f32":
          this.readBytes(4);
          break;
        case "f64":
          this.readBytes(8);
          break;
        default:
          throw new ValidationError(
            ValidationErrorKind.TYPE_MISMATCH,
            `Unsupported immediate '${immediate}' in ${this.context}`
          );
      }
    }

    return instruction;
  }

  private readBlockType(): ValueType | null {
    const byte = this.readByte();
    if (byte === BlockType.EMPTY) {
      return null;
    }
    if (isValueType(byte)) {
      return byte;
    }
    throw new ValidationError(
      ValidationErrorKind.TYPE_MISMATCH,
      `Unsupported blocktype byte 0x${byte.toString(16).padStart(2, "0")} in ${this.context}`
    );
  }

  private readByte(): number {
    if (this.offset >= this.bytes.length) {
      throw new ValidationError(
        ValidationErrorKind.TYPE_MISMATCH,
        `Unexpected end of ${this.context} at byte ${this.offset}`
      );
    }
    return this.bytes[this.offset++];
  }

  private readBytes(length: number): Uint8Array {
    if (this.offset + length > this.bytes.length) {
      throw new ValidationError(
        ValidationErrorKind.TYPE_MISMATCH,
        `Unexpected end of ${this.context} at byte ${this.offset}`
      );
    }
    const slice = this.bytes.slice(this.offset, this.offset + length);
    this.offset += length;
    return slice;
  }

  private readU32(): number {
    try {
      const [value, consumed] = decodeUnsigned(this.bytes, this.offset);
      this.offset += consumed;
      return value;
    } catch (error) {
      throw new ValidationError(
        ValidationErrorKind.TYPE_MISMATCH,
        `Invalid unsigned LEB128 in ${this.context} at byte ${this.offset}: ${(error as Error).message}`
      );
    }
  }

  private readSigned(): number {
    try {
      const [value, consumed] = decodeSigned(this.bytes, this.offset);
      this.offset += consumed;
      return value;
    } catch (error) {
      throw new ValidationError(
        ValidationErrorKind.TYPE_MISMATCH,
        `Invalid signed LEB128 in ${this.context} at byte ${this.offset}: ${(error as Error).message}`
      );
    }
  }
}
