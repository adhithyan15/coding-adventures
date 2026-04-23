import 'dart:typed_data';

enum ValueType {
  i32(0x7f),
  i64(0x7e),
  f32(0x7d),
  f64(0x7c);

  const ValueType(this.code);

  final int code;

  static ValueType fromCode(int code) {
    for (final value in ValueType.values) {
      if (value.code == code) {
        return value;
      }
    }
    throw ArgumentError('Unknown value type byte: 0x${code.toRadixString(16)}');
  }
}

enum ExternalKind {
  function(0x00),
  table(0x01),
  memory(0x02),
  global(0x03);

  const ExternalKind(this.code);

  final int code;

  static ExternalKind fromCode(int code) {
    for (final value in ExternalKind.values) {
      if (value.code == code) {
        return value;
      }
    }
    throw ArgumentError('Unknown external kind byte: 0x${code.toRadixString(16)}');
  }
}

const int funcRef = 0x70;
const int emptyBlockType = 0x40;

FuncType makeFuncType(List<ValueType> params, List<ValueType> results) {
  return FuncType(params: params, results: results);
}

class FuncType {
  FuncType({required List<ValueType> params, required List<ValueType> results})
      : params = List<ValueType>.unmodifiable(params),
        results = List<ValueType>.unmodifiable(results);

  final List<ValueType> params;
  final List<ValueType> results;
}

class Limits {
  const Limits({required this.min, this.max});

  final int min;
  final int? max;
}

class MemoryType {
  const MemoryType(this.limits);

  final Limits limits;
}

class TableType {
  const TableType({required this.elementType, required this.limits});

  final int elementType;
  final Limits limits;
}

class GlobalType {
  const GlobalType({required this.valueType, required this.mutable});

  final ValueType valueType;
  final bool mutable;
}

class ImportEntry {
  const ImportEntry({
    required this.moduleName,
    required this.name,
    required this.kind,
    required this.typeInfo,
  });

  final String moduleName;
  final String name;
  final ExternalKind kind;
  final Object typeInfo;
}

class ExportEntry {
  const ExportEntry({
    required this.name,
    required this.kind,
    required this.index,
  });

  final String name;
  final ExternalKind kind;
  final int index;
}

class GlobalEntry {
  const GlobalEntry({
    required this.globalType,
    required this.initExpr,
  });

  final GlobalType globalType;
  final Uint8List initExpr;
}

class ElementSegment {
  ElementSegment({
    required this.tableIndex,
    required this.offsetExpr,
    required List<int> functionIndices,
  }) : functionIndices = List<int>.unmodifiable(functionIndices);

  final int tableIndex;
  final Uint8List offsetExpr;
  final List<int> functionIndices;
}

class DataSegment {
  const DataSegment({
    required this.memoryIndex,
    required this.offsetExpr,
    required this.data,
  });

  final int memoryIndex;
  final Uint8List offsetExpr;
  final Uint8List data;
}

class FunctionBody {
  FunctionBody({
    required List<ValueType> locals,
    required this.code,
  }) : locals = List<ValueType>.unmodifiable(locals);

  final List<ValueType> locals;
  final Uint8List code;
}

class CustomSection {
  const CustomSection({
    required this.name,
    required this.data,
  });

  final String name;
  final Uint8List data;
}

class WasmModule {
  final List<FuncType> types = [];
  final List<ImportEntry> imports = [];
  final List<int> functions = [];
  final List<TableType> tables = [];
  final List<MemoryType> memories = [];
  final List<GlobalEntry> globals = [];
  final List<ExportEntry> exports = [];
  int? start;
  final List<ElementSegment> elements = [];
  final List<FunctionBody> code = [];
  final List<DataSegment> data = [];
  final List<CustomSection> customs = [];
}
