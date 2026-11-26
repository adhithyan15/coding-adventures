/// Backend trait and the three default implementations.
///
/// The cage performs the manifest check, then delegates the actual OS
/// call to a [Backend] implementation. The default is [OpenBackend]
/// (calls `dart:io`). [TestBackend] records calls for tests; a future
/// host runtime installs its own backend that routes secure calls over
/// the host channel.
///
/// V1 scope started with file-system methods. Environment access has the
/// same backend seam. Network, process, time, and stdio methods land in
/// subsequent ports.
///
/// ## Async by design
///
/// The reference backend is synchronous (`std::fs`). The Dart port is
/// asynchronous (`Future`-based): async is the idiomatic `dart:io` style,
/// and a host-channel backend is inherently asynchronous. [TestBackend]
/// answers synchronously-completed futures, so tests stay deterministic.
library;

import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

/// The contract the cage delegates OS work to.
///
/// Implementations are responsible only for performing the operation;
/// the manifest check happens before the call in the secure-wrapper
/// functions.
abstract class Backend {
  /// Reads the file at [path] as bytes.
  Future<Uint8List> readFile(String path);

  /// Writes [data] to the file at [path], creating or truncating it.
  Future<void> writeFile(String path, Uint8List data);

  /// Creates an empty file at [path]; fails if it already exists.
  Future<void> createFile(String path);

  /// Deletes the file at [path].
  Future<void> deleteFile(String path);

  /// Lists the entry names in the directory at [path].
  Future<List<String>> listDir(String path);

  /// Reads the environment variable [name], or null when absent.
  Future<String?> readEnv(String name);

  /// Sets the environment variable [name] to [value].
  Future<void> writeEnv(String name, String value);
}

/// A backend call failed: refused ([DenyAllBackend]) or otherwise unable
/// to perform the operation.
final class BackendError implements Exception {
  /// Human-readable diagnostic.
  final String message;

  /// Creates the error.
  const BackendError(this.message);

  @override
  bool operator ==(Object other) =>
      other is BackendError && message == other.message;

  @override
  int get hashCode => message.hashCode;

  @override
  String toString() => message;
}

/// The default backend. Delegates straight to `dart:io`.
///
/// OS failures propagate as [FileSystemException]. Three platform notes:
///
/// - Paths containing NUL are rejected with [BackendError] before any
///   OS call. `dart:io` silently truncates paths at the first NUL
///   (C-string semantics: writing `a\x00b` creates file `a`), while the
///   reference's `std::fs` fails on NUL — and a manifest check on the
///   full string combined with a truncated OS act would be a
///   truncation/confusion gap. Failing here keeps the port fail-closed
///   like the reference.
/// - [writeEnv] always throws [UnsupportedError]: `dart:io` exposes the
///   process environment read-only ([Platform.environment] has no
///   setter). The method upholds the contract shape so wrappers and
///   [TestBackend] scripts stay uniform; only the live OS call is
///   impossible.
/// - [listDir] returns basenames as reported by the VM. The reference
///   skips names that are not valid Unicode; Dart path strings are
///   always valid strings, so nothing is skipped.
class OpenBackend implements Backend {
  /// Creates the default backend.
  const OpenBackend();

  @override
  Future<Uint8List> readFile(String path) async {
    _checkNoNul(path);
    return File(path).readAsBytes();
  }

  @override
  Future<void> writeFile(String path, Uint8List data) async {
    _checkNoNul(path);
    await File(path).writeAsBytes(data);
  }

  @override
  Future<void> createFile(String path) async {
    _checkNoNul(path);
    await File(path).create(exclusive: true);
  }

  @override
  Future<void> deleteFile(String path) async {
    _checkNoNul(path);
    await File(path).delete();
  }

  @override
  Future<List<String>> listDir(String path) async {
    _checkNoNul(path);
    final names = <String>[];
    await for (final entity in Directory(path).list()) {
      names.add(_basename(entity.path));
    }
    return names;
  }

  @override
  Future<String?> readEnv(String name) async => Platform.environment[name];

  @override
  Future<void> writeEnv(String name, String value) async {
    throw UnsupportedError(
      'OpenBackend cannot set environment variable "$name": '
      'dart:io exposes the process environment read-only',
    );
  }
}

/// Rejects paths containing NUL (see [OpenBackend]).
void _checkNoNul(String path) {
  if (path.codeUnits.contains(0)) {
    throw const BackendError('path contains a NUL byte');
  }
}

/// Returns the final path segment of [path].
String _basename(String path) {
  final slash = path.lastIndexOf('/');
  final backslash = path.lastIndexOf(r'\');
  final cut = slash > backslash ? slash : backslash;
  return cut < 0 ? path : path.substring(cut + 1);
}

/// A backend that refuses every call.
///
/// Useful as a default for tests of pure-computation packages: any
/// accidental OS access fails loud. Refusals throw [BackendError] with
/// the reference's exact messages.
class DenyAllBackend implements Backend {
  /// Creates the refusing backend.
  const DenyAllBackend();

  @override
  Future<Uint8List> readFile(String path) async {
    throw const BackendError('DenyAllBackend refused fs.read');
  }

  @override
  Future<void> writeFile(String path, Uint8List data) async {
    throw const BackendError('DenyAllBackend refused fs.write');
  }

  @override
  Future<void> createFile(String path) async {
    throw const BackendError('DenyAllBackend refused fs.create');
  }

  @override
  Future<void> deleteFile(String path) async {
    throw const BackendError('DenyAllBackend refused fs.delete');
  }

  @override
  Future<List<String>> listDir(String path) async {
    throw const BackendError('DenyAllBackend refused fs.list');
  }

  @override
  Future<String?> readEnv(String name) async {
    throw const BackendError('DenyAllBackend refused env.read');
  }

  @override
  Future<void> writeEnv(String name, String value) async {
    throw const BackendError('DenyAllBackend refused env.write');
  }
}

/// One recorded [TestBackend] call.
sealed class TestBackendCall {
  const TestBackendCall();
}

/// A [Backend.readFile] call.
final class ReadFileCall extends TestBackendCall {
  /// The requested path.
  final String path;

  /// Creates the record.
  const ReadFileCall(this.path);

  @override
  bool operator ==(Object other) => other is ReadFileCall && path == other.path;

  @override
  int get hashCode => path.hashCode;

  @override
  String toString() => 'ReadFileCall($path)';
}

/// A [Backend.writeFile] call.
final class WriteFileCall extends TestBackendCall {
  /// The written path.
  final String path;

  /// The written bytes.
  final Uint8List data;

  /// Creates the record.
  const WriteFileCall(this.path, this.data);

  @override
  bool operator ==(Object other) =>
      other is WriteFileCall &&
      path == other.path &&
      _bytesEqual(data, other.data);

  @override
  int get hashCode => Object.hash(path, Object.hashAll(data));

  @override
  String toString() => 'WriteFileCall($path, ${data.length} bytes)';
}

/// A [Backend.createFile] call.
final class CreateFileCall extends TestBackendCall {
  /// The created path.
  final String path;

  /// Creates the record.
  const CreateFileCall(this.path);

  @override
  bool operator ==(Object other) =>
      other is CreateFileCall && path == other.path;

  @override
  int get hashCode => path.hashCode;

  @override
  String toString() => 'CreateFileCall($path)';
}

/// A [Backend.deleteFile] call.
final class DeleteFileCall extends TestBackendCall {
  /// The deleted path.
  final String path;

  /// Creates the record.
  const DeleteFileCall(this.path);

  @override
  bool operator ==(Object other) =>
      other is DeleteFileCall && path == other.path;

  @override
  int get hashCode => path.hashCode;

  @override
  String toString() => 'DeleteFileCall($path)';
}

/// A [Backend.listDir] call.
final class ListDirCall extends TestBackendCall {
  /// The listed path.
  final String path;

  /// Creates the record.
  const ListDirCall(this.path);

  @override
  bool operator ==(Object other) => other is ListDirCall && path == other.path;

  @override
  int get hashCode => path.hashCode;

  @override
  String toString() => 'ListDirCall($path)';
}

/// A [Backend.readEnv] call.
final class ReadEnvCall extends TestBackendCall {
  /// The requested variable name.
  final String name;

  /// Creates the record.
  const ReadEnvCall(this.name);

  @override
  bool operator ==(Object other) => other is ReadEnvCall && name == other.name;

  @override
  int get hashCode => name.hashCode;

  @override
  String toString() => 'ReadEnvCall($name)';
}

/// A [Backend.writeEnv] call.
final class WriteEnvCall extends TestBackendCall {
  /// The written variable name.
  final String name;

  /// The written value.
  final String value;

  /// Creates the record.
  const WriteEnvCall(this.name, this.value);

  @override
  bool operator ==(Object other) =>
      other is WriteEnvCall && name == other.name && value == other.value;

  @override
  int get hashCode => Object.hash(name, value);

  @override
  String toString() => 'WriteEnvCall($name)';
}

bool _bytesEqual(Uint8List a, Uint8List b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

/// A backend that records every call into an internal log. Useful in
/// tests that assert on the sequence of secure operations.
///
/// Reads and lists return scripted responses provided via [withResponse],
/// [withDirResponse], and [withEnvResponse]; if no response is scripted,
/// reads return empty bytes, lists return an empty vector, and env reads
/// return null. Writes / creates / deletes always succeed and are
/// recorded.
///
/// The reference guards the log and response tables with mutexes; Dart
/// is single-threaded per isolate, so plain fields are equivalent.
class TestBackend implements Backend {
  final List<TestBackendCall> _log = [];
  final List<(String, Uint8List)> _fileResponses = [];
  final List<(String, List<String>)> _dirResponses = [];
  final List<(String, String)> _envResponses = [];

  /// Creates an empty test backend.
  TestBackend();

  /// Scripts a response for a given path. The next [readFile] call
  /// against this path returns these bytes.
  ///
  /// The reference consumes the backend (`self`) and returns it; without
  /// move semantics this mutates and returns `this`, which chains the
  /// same way.
  TestBackend withResponse(String path, Uint8List data) {
    _fileResponses.add((path, data));
    return this;
  }

  /// Scripts a response for a directory listing.
  TestBackend withDirResponse(String path, List<String> entries) {
    _dirResponses.add((path, entries));
    return this;
  }

  /// Scripts a response for an environment variable read.
  TestBackend withEnvResponse(String name, String value) {
    _envResponses.add((name, value));
    return this;
  }

  /// Snapshots the call log.
  List<TestBackendCall> calls() => List<TestBackendCall>.of(_log);

  @override
  Future<Uint8List> readFile(String path) async {
    _log.add(ReadFileCall(path));
    for (final response in _fileResponses) {
      if (response.$1 == path) return response.$2;
    }
    return Uint8List(0);
  }

  @override
  Future<void> writeFile(String path, Uint8List data) async {
    _log.add(WriteFileCall(path, data));
  }

  @override
  Future<void> createFile(String path) async {
    _log.add(CreateFileCall(path));
  }

  @override
  Future<void> deleteFile(String path) async {
    _log.add(DeleteFileCall(path));
  }

  @override
  Future<List<String>> listDir(String path) async {
    _log.add(ListDirCall(path));
    for (final response in _dirResponses) {
      if (response.$1 == path) return response.$2;
    }
    return [];
  }

  @override
  Future<String?> readEnv(String name) async {
    _log.add(ReadEnvCall(name));
    for (final response in _envResponses) {
      if (response.$1 == name) return response.$2;
    }
    return null;
  }

  @override
  Future<void> writeEnv(String name, String value) async {
    _log.add(WriteEnvCall(name, value));
  }
}

// ---------------------------------------------------------------------------
// Scoped default backend.
// ---------------------------------------------------------------------------

/// Zone key carrying the ambient backend override, if any.
const Symbol _backendZoneKey = #coding_adventures_cage_backend;

/// The process-wide default backend (the reference's `DEFAULT_BACKEND`).
const Backend _defaultBackend = OpenBackend();

/// The backend secure wrappers delegate to: the innermost
/// [runWithBackend] override, or the process-wide [OpenBackend] default.
///
/// Internal accessor for secure-wrapper modules (mirrors the
/// reference's `pub(crate) current_backend`).
Backend currentBackend() =>
    (Zone.current[_backendZoneKey] as Backend?) ?? _defaultBackend;

/// Runs [body] with [backend] installed as the ambient backend.
///
/// This maps the reference's `with_backend` guard. Dart has no
/// destructors, so instead of a RAII guard this uses a [Zone]: the
/// override is visible to [currentBackend] throughout [body] —
/// including across `await`s with [runWithBackendAsync] — and the
/// previous backend is restored when [body] returns or throws.
///
/// Zones nest: an inner override shadows the outer one and restores it
/// on exit. The reference serializes overrides with a mutex so parallel
/// tests cannot restore each other's backend mid-call; zones give that
/// isolation structurally, one ambient backend per execution context.
T runWithBackend<T>(Backend backend, T Function() body) =>
    runZoned(body, zoneValues: {_backendZoneKey: backend});

/// Async variant of [runWithBackend].
Future<T> runWithBackendAsync<T>(
  Backend backend,
  Future<T> Function() body,
) =>
    runZoned(body, zoneValues: {_backendZoneKey: backend});
