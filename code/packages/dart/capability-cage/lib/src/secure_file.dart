/// Secure-wrapper functions for the `fs` capability category.
///
/// Every call validates the manifest first, then delegates to the
/// current [Backend]. Denial throws the [CapabilityViolationError]
/// directly: the reference wraps it in an `io::Error` so callers'
/// existing `io::Result` propagation keeps working, but Dart has no
/// such channel to preserve, so the violation itself is the honest
/// failure.
///
/// Paths are checked against the manifest exactly as given (no
/// normalization), matching the reference's lossy display of the path.
library;

import 'dart:typed_data';

import 'backend.dart';
import 'category.dart';
import 'errors.dart';
import 'manifest.dart';

/// Reads the entire contents of [path].
Future<Uint8List> readFile(Manifest manifest, String path) async {
  manifest.check(Category.fs, Action.read, path);
  return currentBackend().readFile(path);
}

/// Writes [data] to [path], replacing any existing contents.
Future<void> writeFile(
  Manifest manifest,
  String path,
  Uint8List data,
) async {
  manifest.check(Category.fs, Action.write, path);
  return currentBackend().writeFile(path, data);
}

/// Creates a new empty file at [path]. Fails if it already exists.
Future<void> createFile(Manifest manifest, String path) async {
  manifest.check(Category.fs, Action.create, path);
  return currentBackend().createFile(path);
}

/// Removes the file at [path].
Future<void> deleteFile(Manifest manifest, String path) async {
  manifest.check(Category.fs, Action.delete, path);
  return currentBackend().deleteFile(path);
}

/// Lists the entries in the directory at [path].
Future<List<String>> listDir(Manifest manifest, String path) async {
  manifest.check(Category.fs, Action.list, path);
  return currentBackend().listDir(path);
}
