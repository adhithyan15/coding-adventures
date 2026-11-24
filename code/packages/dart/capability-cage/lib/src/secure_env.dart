/// Secure-wrapper functions for the `env` capability category.
///
/// These wrappers mirror `secure_file.dart`: every call checks the
/// manifest first, then delegates to the current backend. Environment
/// variable names are the manifest targets, so callers grant individual
/// variables or globbed variable families.
///
/// Denial throws the [CapabilityViolationError] directly (see
/// `secure_file.dart` for why the reference's `io::Error` wrapping is
/// not mirrored).
library;

import 'backend.dart';
import 'category.dart';
import 'manifest.dart';

/// Reads one environment variable as a string.
///
/// Missing variables return null.
Future<String?> readVar(Manifest manifest, String name) async {
  manifest.check(Category.env, Action.read, name);
  return currentBackend().readEnv(name);
}

/// Sets one environment variable.
///
/// Note the platform limit documented on [OpenBackend]: the live
/// backend always throws [UnsupportedError] for writes, because
/// `dart:io` exposes the process environment read-only.
Future<void> writeVar(Manifest manifest, String name, String value) async {
  manifest.check(Category.env, Action.write, name);
  return currentBackend().writeEnv(name, value);
}
