import 'package:unix_history_simulator/unix_history.dart';

void check(bool condition, String message) {
  if (!condition) throw StateError(message);
}

void main() {
  final fs = FileSystem.demo();
  check(fs.resolve('//etc/../').$1 == 1, 'dot-dot resolves to root');
  try {
    fs.resolve('/hello/child');
    throw StateError('file traversal accepted');
  } on FsError catch (e) {
    check(e.code == 'ENOTDIR', 'file traversal error');
  }
  final descriptors = DescriptorTable(2);
  final first = descriptors.open(2)!;
  final second = descriptors.open(2)!;
  check(first == 0 && second == 1, 'lowest free descriptor');
  check(descriptors.advance(first, 3), 'advance succeeds');
  check(
    descriptors.get(first)!.offset == 3 && descriptors.get(second)!.offset == 0,
    'offsets are independent',
  );
  check(
    descriptors.close(first) && descriptors.open(3) == first,
    'closed slot is reused',
  );

  final kernel = Kernel(FileSystem.demo(), DescriptorTable(1));
  check(kernel.open('/hello')['fd'] == 0, 'open returns lowest free fd');
  check(
    kernel.open('/hello')['error'] == 'EMFILE',
    'full table reports EMFILE',
  );
  final (read, bytes) = kernel.read(0, 5);
  check(
    read['bytes'] == 5 && String.fromCharCodes(bytes!) == 'Hello',
    'read returns bytes and advances the descriptor',
  );
  check(kernel.descriptors.get(0)!.offset == 5, 'read advances offset');
  check(kernel.read(7, 1).$1['error'] == 'EBADF', 'invalid fd reports EBADF');
  check(kernel.open('/')['error'] == 'EISDIR', 'opening a directory fails');
  check(kernel.list('/hello')['error'] == 'ENOTDIR', 'listing a file fails');
  check(
    runCommand('cat /missing')['output'] == 'error: ENOENT\n',
    'shell reports kernel errors',
  );
}
