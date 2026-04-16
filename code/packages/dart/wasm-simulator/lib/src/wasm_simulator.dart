import 'package:coding_adventures_wasm_opcodes/wasm_opcodes.dart';
import 'package:coding_adventures_wasm_runtime/wasm_runtime.dart';
import 'package:coding_adventures_wasm_types/wasm_types.dart';

class SimulationResult {
  const SimulationResult({required this.trace, required this.result});

  final List<String> trace;
  final List<num> result;
}

class WasmSimulator {
  WasmSimulator({WasmRuntime? runtime}) : _runtime = runtime ?? WasmRuntime();

  final WasmRuntime _runtime;

  SimulationResult simulate(
    List<int> wasmBytes, {
    required String exportName,
    List<num> args = const [],
  }) {
    final module = _runtime.load(wasmBytes);
    final instance = _runtime.instantiate(module);
    final export = instance.exports[exportName];
    if (export == null || export.kind != ExternalKind.function) {
      throw StateError('Export "$exportName" is not a function.');
    }

    final trace = <String>[];
    final body = instance.funcBodies[export.index];
    if (body != null) {
      for (final byte in body.code) {
        final info = opcodeByCode(byte);
        if (info != null) {
          trace.add(info.mnemonic);
        }
      }
    }

    final result = _runtime.call(instance, exportName, args);
    return SimulationResult(trace: trace, result: result);
  }
}
