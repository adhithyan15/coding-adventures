/// Best-effort in-memory wiping for secrets.
///
/// ## Why this exists
///
/// When a program holds a secret (a password, a master key, a session
/// token) in a buffer, wiping the buffer when done shrinks the window
/// in which the secret sits in RAM — relevant if the machine swaps or
/// hibernates, the process core-dumps, or freed pages are later
/// inspected.
///
/// ## Honest platform limits — read before relying on this
///
/// The reference achieves an *observable* wipe with volatile stores
/// (the compiler must emit them) plus a compiler fence (they cannot
/// be reordered past the buffer's lifetime), fired automatically by
/// `Drop` on every exit path including panics. Dart provides **none**
/// of those:
///
/// - No volatile stores or compiler fences exist: [zeroizeBytes] is a
///   plain overwrite. A sufficiently aggressive compiler could
///   theoretically elide it; current Dart compilers emit the stores.
/// - No destructors exist: [ZeroizingBytes.dispose] must be called
///   explicitly (use `try`/`finally`). There is no automatic wipe on
///   scope exit, exception, or garbage collection.
/// - The garbage collector may copy survivors: stale copies of the
///   buffer can linger in from-space after the live buffer is wiped.
/// - Strings are immutable and cannot be wiped at all: never hold a
///   secret in a [String] expecting this package to clear it. There is
///   deliberately no string API here.
/// - Integers are immutable values: "wiping" an `int` is meaningless
///   (the caller's variable and every copy keep their value), so the
///   reference's integer impls have no equivalent either.
///
/// What this package *does* guarantee: after [zeroizeBytes] (or
/// [ZeroizingBytes.dispose]) returns, every byte of the live buffer
/// reads back as zero, and the operation is idempotent. That is a
/// real, tested reduction of the live-secret window — and nothing more.
/// Callers with stronger needs (mlock, swap/paging defence) must solve
/// those at the OS-container level, exactly as the reference demands.
library;

import 'dart:typed_data';

/// Overwrites every byte of [bytes] with zero.
///
/// Best-effort (see the library docs for the platform limits): a plain
/// overwrite with no compiler fence. Idempotent; empty input is a noop.
void zeroizeBytes(Uint8List bytes) {
  bytes.fillRange(0, bytes.length, 0);
}

/// Extension so buffers wipe themselves: `key.zeroize()`.
extension ZeroizeBytes on Uint8List {
  /// Overwrites every byte of this buffer with zero.
  void zeroize() => zeroizeBytes(this);
}

/// An owning wrapper for secret bytes with explicit disposal.
///
/// Use this for any local holding a secret. Call [dispose] (ideally in
/// a `finally` block) to wipe the bytes; unlike the reference's
/// `Zeroizing<T>`, nothing runs automatically — Dart has no `Drop`.
///
/// ```dart
/// final key = ZeroizingBytes(secretBytes);
/// try {
///   // ... use key.bytes ...
/// } finally {
///   key.dispose(); // wipes secretBytes
/// }
/// ```
///
/// There is deliberately no `==`, `hashCode`, or debug rendering that
/// exposes the bytes: printing or duplicating secret material is
/// exactly the mistake this wrapper exists to prevent.
class ZeroizingBytes {
  Uint8List? _bytes;

  /// Takes ownership of [bytes] for later [dispose].
  ///
  /// The wrapper aliases (never copies) the buffer, so disposing wipes
  /// the caller's bytes too.
  ZeroizingBytes(Uint8List bytes) : _bytes = bytes;

  /// Whether [dispose] (or [takeBytes]) has already run.
  bool get isDisposed => _bytes == null;

  /// The live bytes. Throws [StateError] after disposal.
  Uint8List get bytes {
    final current = _bytes;
    if (current == null) {
      throw StateError('ZeroizingBytes used after dispose()');
    }
    return current;
  }

  /// Wipes the bytes and releases them. Idempotent: disposing twice is
  /// safe and wipes once.
  void dispose() {
    final current = _bytes;
    _bytes = null;
    current?.fillRange(0, current.length, 0);
  }

  /// Releases the bytes *without* wiping and returns them.
  ///
  /// This is the escape hatch for cases where the secret must outlive
  /// the wrapper. The caller then owns the wipe responsibility.
  /// Throws [StateError] if already disposed or taken.
  Uint8List takeBytes() {
    final current = _bytes;
    if (current == null) {
      throw StateError('ZeroizingBytes already disposed');
    }
    _bytes = null;
    return current;
  }
}
