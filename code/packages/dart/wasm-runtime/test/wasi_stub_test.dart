import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_wasm_execution/wasm_execution.dart';
import 'package:coding_adventures_wasm_runtime/wasm_runtime.dart';
import 'package:test/test.dart';

class _FixedRandom implements WasiRandom {
  @override
  void fillBytes(Uint8List bytes) {
    for (var i = 0; i < bytes.length; i += 1) {
      bytes[i] = i + 1;
    }
  }
}

void main() {
  test('fd_write captures stdout', () {
    final output = StringBuffer();
    final stub = WasiStub(WasiConfig(stdout: output.write));
    final memory = LinearMemory(1);
    stub.setMemory(memory);

    memory.writeBytes(64, utf8.encode('hello'));
    memory.storeI32(32, 64);
    memory.storeI32(36, 5);
    final fdWrite = stub.resolveFunction('wasi_snapshot_preview1', 'fd_write')!;
    final result = fdWrite.call([i32(1), i32(32), i32(1), i32(48)]);

    expect(asI32(result.single), 0);
    expect(memory.loadI32(48), 5);
    expect(output.toString(), 'hello');
  });

  test('args_get and environ_get materialize strings into memory', () {
    final stub = WasiStub(
      const WasiConfig(
        args: ['tool', '--flag'],
        env: {'HOME': '/tmp'},
      ),
    );
    final memory = LinearMemory(1);
    stub.setMemory(memory);

    final argsSizes = stub.resolveFunction('wasi_snapshot_preview1', 'args_sizes_get')!;
    final argsGet = stub.resolveFunction('wasi_snapshot_preview1', 'args_get')!;
    final envSizes = stub.resolveFunction('wasi_snapshot_preview1', 'environ_sizes_get')!;
    final envGet = stub.resolveFunction('wasi_snapshot_preview1', 'environ_get')!;

    argsSizes.call([i32(0), i32(4)]);
    expect(memory.loadI32(0), 2);
    expect(memory.loadI32(4), greaterThan(0));
    argsGet.call([i32(16), i32(64)]);
    final arg0Ptr = memory.loadI32(16);
    final arg1Ptr = memory.loadI32(20);
    expect(utf8.decode(memory.readBytes(arg0Ptr, 4)), 'tool');
    expect(utf8.decode(memory.readBytes(arg1Ptr, 6)), '--flag');

    envSizes.call([i32(8), i32(12)]);
    expect(memory.loadI32(8), 1);
    envGet.call([i32(24), i32(96)]);
    final envPtr = memory.loadI32(24);
    expect(utf8.decode(memory.readBytes(envPtr, 'HOME=/tmp'.length)), 'HOME=/tmp');
  });

  test('random_get fills a buffer', () {
    final stub = WasiStub(WasiConfig(random: _FixedRandom()));
    final memory = LinearMemory(1);
    stub.setMemory(memory);

    final randomGet = stub.resolveFunction('wasi_snapshot_preview1', 'random_get')!;
    final result = randomGet.call([i32(128), i32(4)]);

    expect(asI32(result.single), 0);
    expect(memory.readBytes(128, 4), [1, 2, 3, 4]);
  });
}
