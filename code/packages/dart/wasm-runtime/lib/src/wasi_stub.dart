import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:coding_adventures_wasm_execution/wasm_execution.dart';
import 'package:coding_adventures_wasm_types/wasm_types.dart';

const int _enosys = 52;
const int _esuccess = 0;
const int _einval = 28;

abstract interface class WasiClock {
  int realtimeNs();
  int monotonicNs();
  int resolutionNs(int clockId);
}

abstract interface class WasiRandom {
  void fillBytes(Uint8List bytes);
}

class SystemClock implements WasiClock {
  final Stopwatch _monotonic = Stopwatch()..start();

  @override
  int monotonicNs() => _monotonic.elapsedMicroseconds * 1000;

  @override
  int realtimeNs() => DateTime.now().microsecondsSinceEpoch * 1000;

  @override
  int resolutionNs(int clockId) => 1000000;
}

class SystemRandom implements WasiRandom {
  final Random _random = Random.secure();

  @override
  void fillBytes(Uint8List bytes) {
    for (var i = 0; i < bytes.length; i += 1) {
      bytes[i] = _random.nextInt(256);
    }
  }
}

class ProcExitError implements Exception {
  ProcExitError(this.exitCode);

  final int exitCode;

  @override
  String toString() => 'proc_exit($exitCode)';
}

class WasiConfig {
  const WasiConfig({
    this.args = const [],
    this.env = const {},
    this.stdout,
    this.stderr,
    this.clock,
    this.random,
  });

  final List<String> args;
  final Map<String, String> env;
  final void Function(String text)? stdout;
  final void Function(String text)? stderr;
  final WasiClock? clock;
  final WasiRandom? random;
}

class WasiStub implements HostInterface {
  WasiStub([WasiConfig? options])
      : _stdout = options?.stdout ?? _noop,
        _stderr = options?.stderr ?? _noop,
        _args = List<String>.unmodifiable(options?.args ?? const []),
        _env = Map<String, String>.unmodifiable(options?.env ?? const {}),
        _clock = options?.clock ?? SystemClock(),
        _random = options?.random ?? SystemRandom();

  static void _noop(String _) {}

  final void Function(String text) _stdout;
  final void Function(String text) _stderr;
  final List<String> _args;
  final Map<String, String> _env;
  final WasiClock _clock;
  final WasiRandom _random;

  LinearMemory? _memory;

  void setMemory(LinearMemory memory) {
    _memory = memory;
  }

  @override
  HostFunction? resolveFunction(String moduleName, String name) {
    if (moduleName != 'wasi_snapshot_preview1') return null;
    return switch (name) {
      'fd_write' => _fdWrite(),
      'proc_exit' => _procExit(),
      'args_sizes_get' => _argsSizesGet(),
      'args_get' => _argsGet(),
      'environ_sizes_get' => _environSizesGet(),
      'environ_get' => _environGet(),
      'clock_res_get' => _clockResGet(),
      'clock_time_get' => _clockTimeGet(),
      'random_get' => _randomGet(),
      'sched_yield' => _schedYield(),
      _ => _stub(),
    };
  }

  @override
  ResolvedGlobalImport? resolveGlobal(String moduleName, String name) => null;

  @override
  LinearMemory? resolveMemory(String moduleName, String name) => null;

  @override
  Table? resolveTable(String moduleName, String name) => null;

  HostFunction _fdWrite() {
    return _InlineHostFunction(
      makeFuncType(
        [ValueType.i32, ValueType.i32, ValueType.i32, ValueType.i32],
        [ValueType.i32],
      ),
      (args) {
        final memory = _memory;
        if (memory == null) return [i32(_enosys)];
        final fd = asI32(args[0]);
        final iovsPtr = asI32(args[1]);
        final iovsLen = asI32(args[2]);
        final nwrittenPtr = asI32(args[3]);
        var totalWritten = 0;
        for (var i = 0; i < iovsLen; i += 1) {
          final bufPtr = memory.loadI32(iovsPtr + i * 8);
          final bufLen = memory.loadI32(iovsPtr + i * 8 + 4);
          final text = utf8.decode(memory.readBytes(bufPtr, bufLen), allowMalformed: true);
          totalWritten += bufLen;
          if (fd == 1) _stdout(text);
          if (fd == 2) _stderr(text);
        }
        memory.storeI32(nwrittenPtr, totalWritten);
        return [i32(_esuccess)];
      },
    );
  }

  HostFunction _procExit() {
    return _InlineHostFunction(
      makeFuncType([ValueType.i32], const []),
      (args) => throw ProcExitError(asI32(args[0])),
    );
  }

  HostFunction _argsSizesGet() {
    return _InlineHostFunction(
      makeFuncType([ValueType.i32, ValueType.i32], [ValueType.i32]),
      (args) {
        final memory = _memory;
        if (memory == null) return [i32(_enosys)];
        final argcPtr = asI32(args[0]);
        final argvBufSizePtr = asI32(args[1]);
        memory.storeI32(argcPtr, _args.length);
        final size = _args.fold<int>(0, (sum, arg) => sum + utf8.encode(arg).length + 1);
        memory.storeI32(argvBufSizePtr, size);
        return [i32(_esuccess)];
      },
    );
  }

  HostFunction _argsGet() {
    return _InlineHostFunction(
      makeFuncType([ValueType.i32, ValueType.i32], [ValueType.i32]),
      (args) {
        final memory = _memory;
        if (memory == null) return [i32(_enosys)];
        final argvPtr = asI32(args[0]);
        final argvBufPtr = asI32(args[1]);
        var offset = argvBufPtr;
        for (var i = 0; i < _args.length; i += 1) {
          memory.storeI32(argvPtr + i * 4, offset);
          final encoded = utf8.encode(_args[i]);
          memory.writeBytes(offset, encoded);
          offset += encoded.length;
          memory.storeI32_8(offset, 0);
          offset += 1;
        }
        return [i32(_esuccess)];
      },
    );
  }

  HostFunction _environSizesGet() {
    return _InlineHostFunction(
      makeFuncType([ValueType.i32, ValueType.i32], [ValueType.i32]),
      (args) {
        final memory = _memory;
        if (memory == null) return [i32(_enosys)];
        final countPtr = asI32(args[0]);
        final sizePtr = asI32(args[1]);
        memory.storeI32(countPtr, _env.length);
        final size = _env.entries.fold<int>(
          0,
          (sum, entry) => sum + utf8.encode('${entry.key}=${entry.value}').length + 1,
        );
        memory.storeI32(sizePtr, size);
        return [i32(_esuccess)];
      },
    );
  }

  HostFunction _environGet() {
    return _InlineHostFunction(
      makeFuncType([ValueType.i32, ValueType.i32], [ValueType.i32]),
      (args) {
        final memory = _memory;
        if (memory == null) return [i32(_enosys)];
        final environPtr = asI32(args[0]);
        final environBufPtr = asI32(args[1]);
        var offset = environBufPtr;
        final entries = _env.entries.toList(growable: false);
        for (var i = 0; i < entries.length; i += 1) {
          memory.storeI32(environPtr + i * 4, offset);
          final encoded = utf8.encode('${entries[i].key}=${entries[i].value}');
          memory.writeBytes(offset, encoded);
          offset += encoded.length;
          memory.storeI32_8(offset, 0);
          offset += 1;
        }
        return [i32(_esuccess)];
      },
    );
  }

  HostFunction _clockResGet() {
    return _InlineHostFunction(
      makeFuncType([ValueType.i32, ValueType.i32], [ValueType.i32]),
      (args) {
        final memory = _memory;
        if (memory == null) return [i32(_enosys)];
        memory.storeI64(asI32(args[1]), _clock.resolutionNs(asI32(args[0])));
        return [i32(_esuccess)];
      },
    );
  }

  HostFunction _clockTimeGet() {
    return _InlineHostFunction(
      makeFuncType([ValueType.i32, ValueType.i64, ValueType.i32], [ValueType.i32]),
      (args) {
        final memory = _memory;
        if (memory == null) return [i32(_enosys)];
        final clockId = asI32(args[0]);
        final timePtr = asI32(args[2]);
        final timeNs = switch (clockId) {
          0 => _clock.realtimeNs(),
          1 || 2 || 3 => _clock.monotonicNs(),
          _ => -1,
        };
        if (timeNs < 0) return [i32(_einval)];
        memory.storeI64(timePtr, timeNs);
        return [i32(_esuccess)];
      },
    );
  }

  HostFunction _randomGet() {
    return _InlineHostFunction(
      makeFuncType([ValueType.i32, ValueType.i32], [ValueType.i32]),
      (args) {
        final memory = _memory;
        if (memory == null) return [i32(_enosys)];
        final bufPtr = asI32(args[0]);
        final bufLen = asI32(args[1]);
        final bytes = Uint8List(bufLen);
        _random.fillBytes(bytes);
        memory.writeBytes(bufPtr, bytes);
        return [i32(_esuccess)];
      },
    );
  }

  HostFunction _schedYield() {
    return _InlineHostFunction(
      makeFuncType(const [], [ValueType.i32]),
      (_) => [i32(_esuccess)],
    );
  }

  HostFunction _stub() {
    return _InlineHostFunction(
      makeFuncType(const [], [ValueType.i32]),
      (_) => [i32(_enosys)],
    );
  }
}

class _InlineHostFunction implements HostFunction {
  const _InlineHostFunction(this.type, this._call);

  @override
  final FuncType type;

  final List<WasmValue> Function(List<WasmValue>) _call;

  @override
  List<WasmValue> call(List<WasmValue> args) => _call(args);
}
