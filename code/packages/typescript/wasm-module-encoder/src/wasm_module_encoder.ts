import { encodeUnsigned } from "@coding-adventures/wasm-leb128";
import { ExternalKind } from "@coding-adventures/wasm-types";
import type {
  CustomSection,
  DataSegment,
  Element,
  Export,
  FuncType,
  FunctionBody,
  Global,
  GlobalType,
  Import,
  Limits,
  MemoryType,
  TableType,
  ValueType,
  WasmModule,
} from "@coding-adventures/wasm-types";

export const WASM_MAGIC = new Uint8Array([0x00, 0x61, 0x73, 0x6d]);
export const WASM_VERSION = new Uint8Array([0x01, 0x00, 0x00, 0x00]);

export class WasmEncodeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "WasmEncodeError";
  }
}

export function encodeModule(module: WasmModule): Uint8Array {
  const sections: Uint8Array[] = [];

  for (const custom of module.customs) {
    sections.push(section(0, encodeCustom(custom)));
  }
  if (module.types.length > 0) sections.push(section(1, vector(module.types, encodeFuncType)));
  if (module.imports.length > 0) sections.push(section(2, vector(module.imports, encodeImport)));
  if (module.functions.length > 0) sections.push(section(3, vector(module.functions, u32)));
  if (module.tables.length > 0) sections.push(section(4, vector(module.tables, encodeTableType)));
  if (module.memories.length > 0) sections.push(section(5, vector(module.memories, encodeMemoryType)));
  if (module.globals.length > 0) sections.push(section(6, vector(module.globals, encodeGlobal)));
  if (module.exports.length > 0) sections.push(section(7, vector(module.exports, encodeExport)));
  if (module.start !== null) sections.push(section(8, u32(module.start)));
  if (module.elements.length > 0) sections.push(section(9, vector(module.elements, encodeElement)));
  if (module.code.length > 0) sections.push(section(10, vector(module.code, encodeFunctionBody)));
  if (module.data.length > 0) sections.push(section(11, vector(module.data, encodeDataSegment)));

  return concatBytes(WASM_MAGIC, WASM_VERSION, ...sections);
}

function section(sectionId: number, payload: Uint8Array): Uint8Array {
  return concatBytes(Uint8Array.of(sectionId), u32(payload.length), payload);
}

function u32(value: number): Uint8Array {
  return encodeUnsigned(value);
}

function name(text: string): Uint8Array {
  const data = new TextEncoder().encode(text);
  return concatBytes(u32(data.length), data);
}

function vector<T>(values: readonly T[], encoder: (value: T) => Uint8Array): Uint8Array {
  const parts: Uint8Array[] = [u32(values.length)];
  for (const value of values) {
    parts.push(encoder(value));
  }
  return concatBytes(...parts);
}

function valueTypes(types: readonly ValueType[]): Uint8Array {
  return concatBytes(u32(types.length), Uint8Array.from(types));
}

function encodeFuncType(funcType: FuncType): Uint8Array {
  return concatBytes(Uint8Array.of(0x60), valueTypes(funcType.params), valueTypes(funcType.results));
}

function encodeLimits(limits: Limits): Uint8Array {
  if (limits.max === null) {
    return concatBytes(Uint8Array.of(0x00), u32(limits.min));
  }
  return concatBytes(Uint8Array.of(0x01), u32(limits.min), u32(limits.max));
}

function encodeMemoryType(memoryType: MemoryType): Uint8Array {
  return encodeLimits(memoryType.limits);
}

function encodeTableType(tableType: TableType): Uint8Array {
  return concatBytes(Uint8Array.of(tableType.elementType), encodeLimits(tableType.limits));
}

function encodeGlobalType(globalType: GlobalType): Uint8Array {
  return Uint8Array.of(globalType.valueType, globalType.mutable ? 0x01 : 0x00);
}

function encodeImport(importValue: Import): Uint8Array {
  const parts: Uint8Array[] = [
    name(importValue.moduleName),
    name(importValue.name),
    Uint8Array.of(importValue.kind),
  ];

  switch (importValue.kind) {
    case ExternalKind.FUNCTION:
      if (typeof importValue.typeInfo !== "number") {
        throw new WasmEncodeError("function imports require a numeric type index");
      }
      parts.push(u32(importValue.typeInfo));
      break;
    case ExternalKind.TABLE:
      parts.push(encodeTableType(importValue.typeInfo as TableType));
      break;
    case ExternalKind.MEMORY:
      parts.push(encodeMemoryType(importValue.typeInfo as MemoryType));
      break;
    case ExternalKind.GLOBAL:
      parts.push(encodeGlobalType(importValue.typeInfo as GlobalType));
      break;
    default:
      throw new WasmEncodeError(`unsupported import kind: ${String(importValue.kind)}`);
  }

  return concatBytes(...parts);
}

function encodeExport(exportValue: Export): Uint8Array {
  return concatBytes(name(exportValue.name), Uint8Array.of(exportValue.kind), u32(exportValue.index));
}

function encodeGlobal(globalValue: Global): Uint8Array {
  return concatBytes(encodeGlobalType(globalValue.globalType), globalValue.initExpr);
}

function encodeElement(element: Element): Uint8Array {
  return concatBytes(
    u32(element.tableIndex),
    element.offsetExpr,
    u32(element.functionIndices.length),
    ...element.functionIndices.map((index) => u32(index)),
  );
}

function encodeDataSegment(segment: DataSegment): Uint8Array {
  return concatBytes(u32(segment.memoryIndex), segment.offsetExpr, u32(segment.data.length), segment.data);
}

function encodeFunctionBody(body: FunctionBody): Uint8Array {
  const localGroups = groupLocals(body.locals);
  const localParts: Uint8Array[] = [u32(localGroups.length)];
  for (const [count, valueType] of localGroups) {
    localParts.push(u32(count), Uint8Array.of(valueType));
  }

  const payload = concatBytes(...localParts, body.code);
  return concatBytes(u32(payload.length), payload);
}

function groupLocals(locals: readonly ValueType[]): Array<[number, ValueType]> {
  if (locals.length === 0) return [];

  const groups: Array<[number, ValueType]> = [];
  let currentType = locals[0];
  let count = 1;

  for (let i = 1; i < locals.length; i++) {
    if (locals[i] === currentType) {
      count += 1;
      continue;
    }
    groups.push([count, currentType]);
    currentType = locals[i];
    count = 1;
  }
  groups.push([count, currentType]);
  return groups;
}

function encodeCustom(custom: CustomSection): Uint8Array {
  return concatBytes(name(custom.name), custom.data);
}

function concatBytes(...parts: Uint8Array[]): Uint8Array {
  const totalLength = parts.reduce((sum, part) => sum + part.length, 0);
  const result = new Uint8Array(totalLength);
  let offset = 0;
  for (const part of parts) {
    result.set(part, offset);
    offset += part.length;
  }
  return result;
}
