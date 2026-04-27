import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_wasm_leb128/wasm_leb128.dart';
import 'package:coding_adventures_wasm_types/wasm_types.dart';

Uint8List encodeModule(WasmModule module) {
  final bytes = <int>[
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
  ];

  if (module.types.isNotEmpty) {
    final payload = <int>[...encodeUnsigned(module.types.length)];
    for (final type in module.types) {
      payload.add(0x60);
      payload.addAll(encodeUnsigned(type.params.length));
      payload.addAll(type.params.map((value) => value.code));
      payload.addAll(encodeUnsigned(type.results.length));
      payload.addAll(type.results.map((value) => value.code));
    }
    _appendSection(bytes, 0x01, payload);
  }

  if (module.functions.isNotEmpty) {
    final payload = <int>[...encodeUnsigned(module.functions.length)];
    for (final typeIndex in module.functions) {
      payload.addAll(encodeUnsigned(typeIndex));
    }
    _appendSection(bytes, 0x03, payload);
  }

  if (module.memories.isNotEmpty) {
    final payload = <int>[...encodeUnsigned(module.memories.length)];
    for (final memory in module.memories) {
      payload.addAll(_encodeLimits(memory.limits));
    }
    _appendSection(bytes, 0x05, payload);
  }

  if (module.globals.isNotEmpty) {
    final payload = <int>[...encodeUnsigned(module.globals.length)];
    for (final global in module.globals) {
      payload
        ..add(global.globalType.valueType.code)
        ..add(global.globalType.mutable ? 0x01 : 0x00)
        ..addAll(global.initExpr);
    }
    _appendSection(bytes, 0x06, payload);
  }

  if (module.exports.isNotEmpty) {
    final payload = <int>[...encodeUnsigned(module.exports.length)];
    for (final export in module.exports) {
      final nameBytes = utf8.encode(export.name);
      payload
        ..addAll(encodeUnsigned(nameBytes.length))
        ..addAll(nameBytes)
        ..add(export.kind.code)
        ..addAll(encodeUnsigned(export.index));
    }
    _appendSection(bytes, 0x07, payload);
  }

  if (module.code.isNotEmpty) {
    final payload = <int>[...encodeUnsigned(module.code.length)];
    for (final body in module.code) {
      final locals = _encodeLocals(body.locals);
      final encodedBody = <int>[...locals, ...body.code];
      payload
        ..addAll(encodeUnsigned(encodedBody.length))
        ..addAll(encodedBody);
    }
    _appendSection(bytes, 0x0a, payload);
  }

  return Uint8List.fromList(bytes);
}

void _appendSection(List<int> bytes, int sectionId, List<int> payload) {
  bytes
    ..add(sectionId)
    ..addAll(encodeUnsigned(payload.length))
    ..addAll(payload);
}

List<int> _encodeLimits(Limits limits) {
  return [
    limits.max == null ? 0x00 : 0x01,
    ...encodeUnsigned(limits.min),
    if (limits.max != null) ...encodeUnsigned(limits.max!),
  ];
}

List<int> _encodeLocals(List<ValueType> locals) {
  if (locals.isEmpty) {
    return [0x00];
  }

  final groups = <List<Object>>[];
  ValueType? current;
  var count = 0;
  for (final local in locals) {
    if (current == null || local != current) {
      if (current != null) {
        groups.add([count, current]);
      }
      current = local;
      count = 1;
    } else {
      count += 1;
    }
  }
  if (current != null) {
    groups.add([count, current]);
  }

  final encoded = <int>[...encodeUnsigned(groups.length)];
  for (final group in groups) {
    encoded
      ..addAll(encodeUnsigned(group[0] as int))
      ..add((group[1] as ValueType).code);
  }
  return encoded;
}
