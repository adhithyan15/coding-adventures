import 'package:coding_adventures_wasm_execution/wasm_execution.dart';
import 'package:coding_adventures_wasm_module_parser/wasm_module_parser.dart';
import 'package:coding_adventures_wasm_types/wasm_types.dart';
import 'package:coding_adventures_wasm_validator/wasm_validator.dart' as validator;

import 'wasm_instance.dart';
import 'wasi_stub.dart';

class WasmRuntime {
  WasmRuntime([HostInterface? host]) : _host = host;

  final WasmModuleParser _parser = WasmModuleParser();
  final HostInterface? _host;

  WasmModule load(List<int> wasmBytes) => _parser.parse(wasmBytes);

  validator.ValidatedModule validate(WasmModule module) => validateModule(module);

  validator.ValidatedModule validateModule(WasmModule module) => validator.validate(module);

  WasmInstance instantiate(WasmModule module) {
    final validated = validator.validate(module);
    final funcTypes = <FuncType>[];
    final funcBodies = <FunctionBody?>[];
    final hostFunctions = <HostFunction?>[];
    final globalTypes = <GlobalType>[];
    final globals = <WasmValue>[];
    final tables = <Table>[];
    LinearMemory? memory;

    for (final imp in module.imports) {
      switch (imp.kind) {
        case ExternalKind.function:
          final typeIndex = imp.typeInfo as int;
          funcTypes.add(module.types[typeIndex]);
          funcBodies.add(null);
          hostFunctions.add(_host?.resolveFunction(imp.moduleName, imp.name));
          break;
        case ExternalKind.memory:
          final importedMemory = _host?.resolveMemory(imp.moduleName, imp.name);
          if (importedMemory != null) {
            memory = importedMemory;
          }
          break;
        case ExternalKind.table:
          final importedTable = _host?.resolveTable(imp.moduleName, imp.name);
          if (importedTable != null) {
            tables.add(importedTable);
          }
          break;
        case ExternalKind.global:
          final importedGlobal = _host?.resolveGlobal(imp.moduleName, imp.name);
          if (importedGlobal != null) {
            globalTypes.add(importedGlobal.type);
            globals.add(importedGlobal.value);
          }
          break;
      }
    }

    for (var index = 0; index < module.functions.length; index += 1) {
      funcTypes.add(module.types[module.functions[index]]);
      funcBodies.add(module.code[index]);
      hostFunctions.add(null);
    }

    if (memory == null && module.memories.isNotEmpty) {
      final memType = module.memories.first;
      memory = LinearMemory(memType.limits.min, memType.limits.max);
    }

    for (final tableType in module.tables) {
      tables.add(Table(tableType.limits.min, tableType.limits.max));
    }

    for (final global in module.globals) {
      globalTypes.add(global.globalType);
      globals.add(evaluateConstExpr(global.initExpr, globals: globals));
    }

    if (memory != null) {
      for (final segment in module.data) {
        final offset = evaluateConstExpr(segment.offsetExpr, globals: globals);
        memory.writeBytes(offset.value as int, segment.data);
      }
    }

    for (final element in module.elements) {
      final table = tables[element.tableIndex];
      final offset = evaluateConstExpr(element.offsetExpr, globals: globals).value as int;
      for (var index = 0; index < element.functionIndices.length; index += 1) {
        table.set(offset + index, element.functionIndices[index]);
      }
    }

    final instance = WasmInstance(
      module: validated.module,
      memory: memory,
      tables: tables,
      globals: globals,
      globalTypes: globalTypes,
      funcTypes: funcTypes,
      funcBodies: funcBodies,
      hostFunctions: hostFunctions,
      exports: {
        for (final exp in module.exports) exp.name: (kind: exp.kind, index: exp.index),
      },
      host: _host,
    );

    if (_host is WasiStub && instance.memory != null) {
      (_host as WasiStub).setMemory(instance.memory!);
    }

    if (module.start case final start?) {
      _engine(instance).callFunction(start, const []);
    }

    return instance;
  }

  List<num> call(WasmInstance instance, String name, List<num> args) {
    final exp = instance.exports[name];
    if (exp == null) {
      throw TrapError('export "$name" not found');
    }
    if (exp.kind != ExternalKind.function) {
      throw TrapError('export "$name" is not a function');
    }

    final funcType = instance.funcTypes[exp.index];
    if (args.length != funcType.params.length) {
      throw TrapError('export "$name" expects ${funcType.params.length} args, got ${args.length}');
    }

    final wasmArgs = <WasmValue>[
      for (var i = 0; i < args.length; i += 1)
        switch (funcType.params[i]) {
          ValueType.i32 => i32(args[i].toInt()),
          ValueType.i64 => i64(args[i].toInt()),
          ValueType.f32 => f32(args[i].toDouble()),
          ValueType.f64 => f64(args[i].toDouble()),
        },
    ];

    return _engine(instance).callFunction(exp.index, wasmArgs).map((result) {
      final value = result.value;
      return value is num ? value : (value as int);
    }).toList(growable: false);
  }

  List<num> loadAndRun(
    List<int> wasmBytes, {
    String exportName = '_start',
    List<num> args = const [],
  }) {
    final module = load(wasmBytes);
    validate(module);
    final instance = instantiate(module);
    return call(instance, exportName, args);
  }

  WasmExecutionEngine _engine(WasmInstance instance) {
    return WasmExecutionEngine(
      WasmExecutionContext(
        memory: instance.memory,
        tables: instance.tables,
        globals: instance.globals,
        globalTypes: instance.globalTypes,
        funcTypes: instance.funcTypes,
        funcBodies: instance.funcBodies,
        hostFunctions: instance.hostFunctions,
      ),
    );
  }
}
