import 'package:coding_adventures_wasm_execution/wasm_execution.dart';
import 'package:coding_adventures_wasm_types/wasm_types.dart';

class WasmInstance {
  WasmInstance({
    required this.module,
    required this.memory,
    required this.tables,
    required this.globals,
    required this.globalTypes,
    required this.funcTypes,
    required this.funcBodies,
    required this.hostFunctions,
    required this.exports,
    required this.host,
  });

  final WasmModule module;
  final LinearMemory? memory;
  final List<Table> tables;
  final List<WasmValue> globals;
  final List<GlobalType> globalTypes;
  final List<FuncType> funcTypes;
  final List<FunctionBody?> funcBodies;
  final List<HostFunction?> hostFunctions;
  final Map<String, ({ExternalKind kind, int index})> exports;
  final HostInterface? host;
}
