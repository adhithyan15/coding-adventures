/// A small, read-only Unix teaching image with reusable kernel components.
///
/// Directory entries keep their presentation order, while inode identity and
/// file bytes live in the node map. This makes path lookup independent of the
/// shell adapter and gives each open descriptor its own byte offset.
sealed class Node {}

class DirectoryNode extends Node {
  DirectoryNode(Map<String, int> entries) : entries = Map.unmodifiable(entries);
  final Map<String, int> entries;
}

class FileNode extends Node {
  FileNode(List<int> bytes) : bytes = List.unmodifiable(bytes);
  final List<int> bytes;
}

class FileSystem {
  FileSystem(Map<int, Node> nodes) : nodes = Map.unmodifiable(nodes);
  final Map<int, Node> nodes;

  factory FileSystem.demo() => FileSystem({
        1: DirectoryNode({'hello': 2, 'empty': 3, 'etc': 4}),
        2: FileNode('Hello, Unix!\n'.codeUnits),
        3: FileNode(const []),
        4: DirectoryNode({'notes': 5}),
        5: FileNode(
            'A directory maps a name to an inode.\nAn open descriptor tracks a byte offset.\n'
                .codeUnits),
      });

  (int, Node) resolve(String path) {
    if (path.isEmpty || path.contains('\u0000')) throw const FsError('EINVAL');
    var inode = 1;
    final parents = <int>[];
    for (final part in path.split('/').where((part) => part.isNotEmpty)) {
      final node = nodes[inode];
      if (node == null) throw const FsError('ENOENT');
      if (node is! DirectoryNode) throw const FsError('ENOTDIR');
      if (part == '.') continue;
      if (part == '..') {
        inode = parents.isEmpty ? 1 : parents.removeLast();
        continue;
      }
      final child = node.entries[part];
      if (child == null) throw const FsError('ENOENT');
      parents.add(inode);
      inode = child;
    }
    final node = nodes[inode];
    if (node == null) throw const FsError('ENOENT');
    if (path.endsWith('/') && node is FileNode) throw const FsError('ENOTDIR');
    return (inode, node);
  }

  List<int> read(int inode, int offset, int count) {
    final node = nodes[inode];
    if (node == null) throw const FsError('ENOENT');
    if (node is! FileNode) throw const FsError('EISDIR');
    if (offset < 0 || count < 0) throw const FsError('EINVAL');
    final start = offset.clamp(0, node.bytes.length);
    final end = start + count.clamp(0, node.bytes.length - start);
    return node.bytes.sublist(start, end);
  }
}

class FsError implements Exception {
  const FsError(this.code);
  final String code;
  @override
  String toString() => code;
}

class Descriptor {
  const Descriptor(this.inode, this.offset);
  final int inode;
  final int offset;
}

class DescriptorTable {
  DescriptorTable(int capacity)
      : _slots = List<Descriptor?>.filled(capacity, null) {
    if (capacity < 0) throw ArgumentError.value(capacity, 'capacity');
  }
  final List<Descriptor?> _slots;
  Descriptor? get(int fd) => fd >= 0 && fd < _slots.length ? _slots[fd] : null;
  int? open(int inode) {
    final fd = _slots.indexWhere((slot) => slot == null);
    if (fd < 0) return null;
    _slots[fd] = Descriptor(inode, 0);
    return fd;
  }

  bool advance(int fd, int count) {
    final slot = get(fd);
    if (slot == null || count < 0 || count > 0x7fffffffffffffff - slot.offset)
      return false;
    _slots[fd] = Descriptor(slot.inode, slot.offset + count);
    return true;
  }

  bool close(int fd) {
    if (get(fd) == null) return false;
    _slots[fd] = null;
    return true;
  }
}

class Kernel {
  Kernel(this.files, this.descriptors);
  final FileSystem files;
  final DescriptorTable descriptors;
  Map<String, Object?> open(String path) {
    try {
      final (inode, node) = files.resolve(path);
      if (node is DirectoryNode) throw const FsError('EISDIR');
      final fd = descriptors.open(inode);
      if (fd == null) throw const FsError('EMFILE');
      return {
        'call': 'open',
        'path': path,
        'fd': fd,
        'inode': inode,
        'offset': 0
      };
    } on FsError catch (error) {
      return {'call': 'open', 'path': path, 'error': error.code};
    }
  }

  (Map<String, Object?>, List<int>?) read(int fd, int count) {
    final slot = descriptors.get(fd);
    if (slot == null)
      return ({'call': 'read', 'fd': fd, 'error': 'EBADF'}, null);
    try {
      final bytes = files.read(slot.inode, slot.offset, count);
      descriptors.advance(fd, bytes.length);
      return (
        {
          'call': 'read',
          'fd': fd,
          'inode': slot.inode,
          'offset': slot.offset,
          'bytes': bytes.length,
          'nextOffset': slot.offset + bytes.length
        },
        bytes
      );
    } on FsError catch (error) {
      return ({'call': 'read', 'fd': fd, 'error': error.code}, null);
    }
  }

  Map<String, Object?> close(int fd) {
    final event = <String, Object?>{'call': 'close', 'fd': fd};
    if (!descriptors.close(fd)) event['error'] = 'EBADF';
    return event;
  }

  Map<String, Object?> list(String path) {
    try {
      final (inode, node) = files.resolve(path);
      if (node is! DirectoryNode) throw const FsError('ENOTDIR');
      return {
        'call': 'list',
        'path': path,
        'inode': inode,
        'names': node.entries.keys.toList()
      };
    } on FsError catch (error) {
      return {'call': 'list', 'path': path, 'error': error.code};
    }
  }
}

Map<String, Object?> runCommand(String command) {
  final kernel = Kernel(FileSystem.demo(), DescriptorTable(4));
  final events = <Map<String, Object?>>[];
  final output = StringBuffer();
  final words = command
      .split(RegExp(r'[ \t\n\r\f\v]+'))
      .where((word) => word.isNotEmpty)
      .toList();
  void record(Map<String, Object?> event) {
    events.add(event);
    if (event['error'] != null) output.write('error: ${event['error']}\n');
  }

  if (words.isNotEmpty && words[0] == 'cat' && words.length == 2) {
    final opened = kernel.open(words[1]);
    record(opened);
    final fd = opened['fd'];
    if (fd is int) {
      while (true) {
        final (event, bytes) = kernel.read(fd, 8);
        record(event);
        if (bytes == null || bytes.isEmpty) break;
        output.write(String.fromCharCodes(bytes));
      }
      record(kernel.close(fd));
    }
  } else if (words.isNotEmpty && words[0] == 'ls' && words.length <= 2) {
    final listed = kernel.list(words.length == 2 ? words[1] : '/');
    record(listed);
    for (final name in (listed['names'] as List<String>? ?? const [])) {
      output.write('$name\n');
    }
  } else if (words.isNotEmpty) {
    output.write('usage: cat PATH or ls [PATH]\n');
  }
  return {'output': output.toString(), 'events': events};
}
