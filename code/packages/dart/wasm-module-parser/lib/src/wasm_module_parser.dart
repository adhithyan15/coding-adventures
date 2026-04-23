import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_wasm_leb128/wasm_leb128.dart';
import 'package:coding_adventures_wasm_types/wasm_types.dart';

class WasmParseException implements Exception {
  WasmParseException(this.message, this.offset);

  final String message;
  final int offset;

  @override
  String toString() => 'WasmParseException(offset=$offset): $message';
}

class WasmModuleParser {
  WasmModule parse(List<int> bytes) {
    final reader = _BinaryReader(Uint8List.fromList(bytes));
    return reader.parseModule();
  }
}

class _BinaryReader {
  _BinaryReader(this.data);

  final Uint8List data;
  int offset = 0;

  static const List<int> _magic = [0x00, 0x61, 0x73, 0x6d];
  static const List<int> _version = [0x01, 0x00, 0x00, 0x00];

  WasmModule parseModule() {
    _validateHeader();
    final module = WasmModule();
    var lastSection = 0;

    while (!isAtEnd) {
      final sectionOffset = offset;
      final sectionId = readByte();
      final payloadSize = readU32();
      final payloadStart = offset;
      final payloadEnd = payloadStart + payloadSize;
      if (payloadEnd > data.length) {
        throw WasmParseException('Section payload exceeds end of file.', payloadStart);
      }

      if (sectionId != 0) {
        if (sectionId < lastSection) {
          throw WasmParseException('Section $sectionId is out of order.', sectionOffset);
        }
        lastSection = sectionId;
      }

      switch (sectionId) {
        case 0:
          _parseCustomSection(module, payloadEnd);
          break;
        case 1:
          _parseTypeSection(module, payloadEnd);
          break;
        case 2:
          _parseImportSection(module, payloadEnd);
          break;
        case 3:
          _parseFunctionSection(module, payloadEnd);
          break;
        case 4:
          _parseTableSection(module, payloadEnd);
          break;
        case 5:
          _parseMemorySection(module, payloadEnd);
          break;
        case 6:
          _parseGlobalSection(module, payloadEnd);
          break;
        case 7:
          _parseExportSection(module, payloadEnd);
          break;
        case 8:
          module.start = readU32();
          break;
        case 9:
          _parseElementSection(module, payloadEnd);
          break;
        case 10:
          _parseCodeSection(module, payloadEnd);
          break;
        case 11:
          _parseDataSection(module, payloadEnd);
          break;
        default:
          break;
      }

      offset = payloadEnd;
    }

    return module;
  }

  bool get isAtEnd => offset >= data.length;

  int readByte() {
    if (offset >= data.length) {
      throw WasmParseException('Unexpected end of input.', offset);
    }
    final byte = data[offset];
    offset += 1;
    return byte;
  }

  Uint8List readBytes(int length) {
    final end = offset + length;
    if (end > data.length) {
      throw WasmParseException('Unexpected end of input while reading bytes.', offset);
    }
    final bytes = Uint8List.sublistView(data, offset, end);
    offset = end;
    return Uint8List.fromList(bytes);
  }

  int readU32() {
    final decoded = decodeUnsigned(data, offset: offset, maxBytes: 5);
    offset += decoded.bytesRead;
    return decoded.value;
  }

  int readS32() {
    final decoded = decodeSigned(data, offset: offset, maxBytes: 5, bitWidth: 32);
    offset += decoded.bytesRead;
    return decoded.value;
  }

  int readS64() {
    final decoded = decodeSigned(data, offset: offset, maxBytes: 10, bitWidth: 64);
    offset += decoded.bytesRead;
    return decoded.value;
  }

  String readName() {
    final length = readU32();
    return utf8.decode(readBytes(length));
  }

  Limits _readLimits() {
    final flags = readByte();
    final min = readU32();
    final max = flags == 0x01 ? readU32() : null;
    return Limits(min: min, max: max);
  }

  GlobalType _readGlobalType() {
    final valueType = ValueType.fromCode(readByte());
    final mutable = readByte() == 0x01;
    return GlobalType(valueType: valueType, mutable: mutable);
  }

  Uint8List _readInitExpr(int sectionEnd) {
    final bytes = <int>[];

    while (true) {
      if (offset >= sectionEnd) {
        throw WasmParseException('Constant expression is missing its end opcode.', offset);
      }
      final opcode = readByte();
      bytes.add(opcode);
      switch (opcode) {
        case 0x0b:
          return Uint8List.fromList(bytes);
        case 0x23:
          bytes.addAll(_readRawUnsigned(maxBytes: 5));
          break;
        case 0x41:
          bytes.addAll(_readRawSigned(maxBytes: 5, bitWidth: 32));
          break;
        case 0x42:
          bytes.addAll(_readRawSigned(maxBytes: 10, bitWidth: 64));
          break;
        case 0x43:
          bytes.addAll(readBytes(4));
          break;
        case 0x44:
          bytes.addAll(readBytes(8));
          break;
        default:
          throw WasmParseException('Unsupported constant expression opcode 0x${opcode.toRadixString(16)}.', offset - 1);
      }
    }
  }

  Uint8List _readRawUnsigned({required int maxBytes}) {
    final start = offset;
    final decoded = decodeUnsigned(data, offset: offset, maxBytes: maxBytes);
    offset += decoded.bytesRead;
    return Uint8List.fromList(data.sublist(start, offset));
  }

  Uint8List _readRawSigned({required int maxBytes, required int bitWidth}) {
    final start = offset;
    final decoded = decodeSigned(
      data,
      offset: offset,
      maxBytes: maxBytes,
      bitWidth: bitWidth,
    );
    offset += decoded.bytesRead;
    return Uint8List.fromList(data.sublist(start, offset));
  }

  void _parseTypeSection(WasmModule module, int payloadEnd) {
    final count = readU32();
    for (var index = 0; index < count; index += 1) {
      final marker = readByte();
      if (marker != 0x60) {
        throw WasmParseException('Invalid function type marker.', offset - 1);
      }
      final paramCount = readU32();
      final params = <ValueType>[];
      for (var i = 0; i < paramCount; i += 1) {
        params.add(ValueType.fromCode(readByte()));
      }
      final resultCount = readU32();
      final results = <ValueType>[];
      for (var i = 0; i < resultCount; i += 1) {
        results.add(ValueType.fromCode(readByte()));
      }
      module.types.add(FuncType(params: params, results: results));
    }
    offset = payloadEnd;
  }

  void _parseImportSection(WasmModule module, int payloadEnd) {
    final count = readU32();
    for (var index = 0; index < count; index += 1) {
      final moduleName = readName();
      final name = readName();
      final kindOffset = offset;
      late final ExternalKind kind;
      try {
        kind = ExternalKind.fromCode(readByte());
      } on ArgumentError {
        throw WasmParseException('Unknown import kind.', kindOffset);
      }

      late final Object typeInfo;
      switch (kind) {
        case ExternalKind.function:
          typeInfo = readU32();
          break;
        case ExternalKind.table:
          final elementType = readByte();
          if (elementType != funcRef) {
            throw WasmParseException('Invalid table element type.', offset - 1);
          }
          typeInfo = TableType(elementType: elementType, limits: _readLimits());
          break;
        case ExternalKind.memory:
          typeInfo = MemoryType(_readLimits());
          break;
        case ExternalKind.global:
          typeInfo = _readGlobalType();
          break;
      }

      module.imports.add(
        ImportEntry(
          moduleName: moduleName,
          name: name,
          kind: kind,
          typeInfo: typeInfo,
        ),
      );
    }
    offset = payloadEnd;
  }

  void _parseFunctionSection(WasmModule module, int payloadEnd) {
    final count = readU32();
    for (var index = 0; index < count; index += 1) {
      module.functions.add(readU32());
    }
    offset = payloadEnd;
  }

  void _parseTableSection(WasmModule module, int payloadEnd) {
    final count = readU32();
    for (var index = 0; index < count; index += 1) {
      final elementType = readByte();
      if (elementType != funcRef) {
        throw WasmParseException('Invalid table element type.', offset - 1);
      }
      module.tables.add(
        TableType(
          elementType: elementType,
          limits: _readLimits(),
        ),
      );
    }
    offset = payloadEnd;
  }

  void _parseMemorySection(WasmModule module, int payloadEnd) {
    final count = readU32();
    for (var index = 0; index < count; index += 1) {
      module.memories.add(MemoryType(_readLimits()));
    }
    offset = payloadEnd;
  }

  void _parseGlobalSection(WasmModule module, int payloadEnd) {
    final count = readU32();
    for (var index = 0; index < count; index += 1) {
      module.globals.add(
        GlobalEntry(
          globalType: _readGlobalType(),
          initExpr: _readInitExpr(payloadEnd),
        ),
      );
    }
    offset = payloadEnd;
  }

  void _parseExportSection(WasmModule module, int payloadEnd) {
    final count = readU32();
    for (var index = 0; index < count; index += 1) {
      final name = readName();
      final kindOffset = offset;
      late final ExternalKind kind;
      try {
        kind = ExternalKind.fromCode(readByte());
      } on ArgumentError {
        throw WasmParseException('Unknown export kind.', kindOffset);
      }
      module.exports.add(
        ExportEntry(
          name: name,
          kind: kind,
          index: readU32(),
        ),
      );
    }
    offset = payloadEnd;
  }

  void _parseElementSection(WasmModule module, int payloadEnd) {
    final count = readU32();
    for (var index = 0; index < count; index += 1) {
      final tableIndex = readU32();
      final offsetExpr = _readInitExpr(payloadEnd);
      final functionCount = readU32();
      final functionIndices = <int>[];
      for (var i = 0; i < functionCount; i += 1) {
        functionIndices.add(readU32());
      }
      module.elements.add(
        ElementSegment(
          tableIndex: tableIndex,
          offsetExpr: offsetExpr,
          functionIndices: functionIndices,
        ),
      );
    }
    offset = payloadEnd;
  }

  void _parseCodeSection(WasmModule module, int payloadEnd) {
    final count = readU32();
    for (var index = 0; index < count; index += 1) {
      final bodySize = readU32();
      final bodyEnd = offset + bodySize;
      final localGroupCount = readU32();
      final locals = <ValueType>[];

      for (var group = 0; group < localGroupCount; group += 1) {
        final localCount = readU32();
        final type = ValueType.fromCode(readByte());
        for (var i = 0; i < localCount; i += 1) {
          locals.add(type);
        }
      }

      final codeLength = bodyEnd - offset;
      module.code.add(
        FunctionBody(
          locals: locals,
          code: readBytes(codeLength),
        ),
      );
    }
    offset = payloadEnd;
  }

  void _parseDataSection(WasmModule module, int payloadEnd) {
    final count = readU32();
    for (var index = 0; index < count; index += 1) {
      module.data.add(
        DataSegment(
          memoryIndex: readU32(),
          offsetExpr: _readInitExpr(payloadEnd),
          data: readBytes(readU32()),
        ),
      );
    }
    offset = payloadEnd;
  }

  void _parseCustomSection(WasmModule module, int payloadEnd) {
    final name = readName();
    final remaining = payloadEnd - offset;
    module.customs.add(CustomSection(name: name, data: readBytes(remaining)));
    offset = payloadEnd;
  }

  void _validateHeader() {
    if (data.length < 8) {
      throw WasmParseException('Input is too short for a WASM header.', 0);
    }
    for (var i = 0; i < _magic.length; i += 1) {
      if (data[i] != _magic[i]) {
        throw WasmParseException('Invalid WASM magic header.', i);
      }
    }
    for (var i = 0; i < _version.length; i += 1) {
      if (data[4 + i] != _version[i]) {
        throw WasmParseException('Unsupported WASM version.', 4 + i);
      }
    }
    offset = 8;
  }
}
