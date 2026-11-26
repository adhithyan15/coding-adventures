/// Shared type-checker contract and generic checker framework.
///
/// The Dart lane of the TypeScript `type-checker-protocol` package: compiler
/// toolchains get a common semantic-analysis shape — the [TypeChecker]
/// contract, [TypeCheckResult] reporting, and [GenericTypeChecker] with
/// reusable phase/kind hook dispatch, diagnostics, and normalization.
///
/// One adaptation: TypeScript's variadic hook arguments (`...args`) become an
/// explicit argument list, since Dart has no rest parameters. Hooks take
/// `(node, args)` and [GenericTypeChecker.dispatch] takes an optional
/// positional argument list.
library;

import 'package:meta/meta.dart';

/// A single type error with its source position.
///
/// A value type: equality compares all three fields, so test assertions can
/// compare diagnostics structurally.
class TypeErrorDiagnostic {
  /// Human-readable error message.
  final String message;

  /// 1-based line number.
  final int line;

  /// 1-based column number.
  final int column;

  /// Creates a diagnostic.
  const TypeErrorDiagnostic({
    required this.message,
    required this.line,
    required this.column,
  });

  @override
  bool operator ==(Object other) =>
      other is TypeErrorDiagnostic &&
      message == other.message &&
      line == other.line &&
      column == other.column;

  @override
  int get hashCode => Object.hash(message, line, column);

  @override
  String toString() =>
      'TypeErrorDiagnostic(message: $message, line: $line, column: $column)';
}

/// The outcome of checking one AST: the (possibly rewritten) tree, the
/// collected diagnostics, and whether checking succeeded.
class TypeCheckResult<AST> {
  /// The checked AST. [GenericTypeChecker] returns the input tree itself;
  /// custom checkers may return a rewritten one.
  final AST typedAst;

  /// The diagnostics collected during checking (a copy per [TypeChecker.check]).
  final List<TypeErrorDiagnostic> errors;

  /// True iff [errors] is empty.
  final bool ok;

  /// Creates a result.
  const TypeCheckResult({
    required this.typedAst,
    required this.errors,
    required this.ok,
  });
}

/// The public contract every checker implements.
abstract class TypeChecker<ASTIn, ASTOut> {
  /// Checks [ast], returning the typed tree plus diagnostics.
  TypeCheckResult<ASTOut> check(ASTIn ast);
}

/// A hook's return value: anything, with [GenericTypeChecker.notHandled]'s
/// sentinel meaning "this hook declines; try the next one".
typedef HookResult = Object?;

/// A phase/kind hook: observes (or rewrites) [node] with extra [args].
///
/// Dart has no rest parameters, so the reference's `...args` arrives as one
/// explicit list.
typedef Hook<AST> = HookResult Function(AST node, List<Object?> args);

/// Unforgeable "not handled" sentinel (a private type with one instance).
class _NotHandled {
  const _NotHandled();
}

/// Reusable checker base: hook registration and dispatch, diagnostics, and
/// kind normalization. Subclasses implement [run] and [nodeKind], register
/// hooks in their constructor, and optionally override [locate].
///
/// The `run`/`nodeKind`/`locate`/`registerHook`/`dispatch`/`notHandled`/
/// `error` members are the subclass extension points, enforced by
/// `@protected` from `package:meta`.
abstract class GenericTypeChecker<AST> implements TypeChecker<AST, AST> {
  /// Hooks by `"phase:normalizedKind"` (`*` for the wildcard kind).
  final Map<String, List<Hook<AST>>> _hooks = {};

  /// Diagnostics collected by the in-progress [check] call.
  List<TypeErrorDiagnostic> _errors = [];

  /// Runs [run] against a fresh diagnostic list and reports the outcome.
  ///
  /// The returned [TypeCheckResult.errors] is a copy: later [check] calls
  /// never mutate an earlier result.
  @override
  TypeCheckResult<AST> check(AST ast) {
    _errors = [];
    run(ast);
    return TypeCheckResult<AST>(
      typedAst: ast,
      errors: List.of(_errors),
      ok: _errors.isEmpty,
    );
  }

  /// The checking pass: dispatches [ast] (and, by recursion, its children)
  /// through the registered hooks.
  @protected
  void run(AST ast);

  /// The dispatch kind of [node], or null when the node has none (null
  /// normalizes to `""`, which only wildcard hooks can match).
  @protected
  String? nodeKind(AST node);

  /// The source position of [subject] for diagnostics. The default pins
  /// everything to (1, 1); real checkers override it to read node spans.
  @protected
  (int, int) locate(Object? subject) => (1, 1);

  /// Registers [hook] for [phase] and [kind] (`"*"` for all kinds of the
  /// phase). Hooks run in registration order.
  ///
  /// Deliberate fix over the reference: the reference normalizes `'*'` to
  /// `""` at registration (storing under `"phase:"`) while [dispatch] looks
  /// up the literal `"phase:*"` tier, so wildcard hooks never fire there
  /// (uncovered by the reference suite). Here the exact kind `'*'` is stored
  /// under the literal `"phase:*"` key, making the tier work as [dispatch]
  /// intends. All other kinds normalize exactly as in the reference.
  @protected
  void registerHook(String phase, String kind, Hook<AST> hook) {
    final key = kind == '*' ? '$phase:*' : '$phase:${_normalizeKind(kind)}';
    (_hooks[key] ??= []).add(hook);
  }

  /// Runs [node] through the `phase:kind` hooks, then the `phase:*` hooks,
  /// returning the first result that is not [notHandled]. Extra [args] are
  /// forwarded to every hook. Returns null when no hook handles the node.
  @protected
  HookResult dispatch(String phase, AST node, [List<Object?> args = const []]) {
    final normalizedKind = _normalizeKind(nodeKind(node) ?? '');

    for (final key in ['$phase:$normalizedKind', '$phase:*']) {
      for (final hook in _hooks[key] ?? const []) {
        final result = hook(node, args);
        if (result is! _NotHandled) {
          return result;
        }
      }
    }

    return null;
  }

  /// The "not handled" sentinel: return it from a hook to decline and let
  /// the next hook (or the wildcard phase) try.
  @protected
  Object notHandled() => const _NotHandled();

  /// Records a diagnostic for [subject] at [locate]'s position.
  @protected
  void error(String message, Object? subject) {
    final (line, column) = locate(subject);
    _errors
        .add(TypeErrorDiagnostic(message: message, line: line, column: column));
  }

  /// Normalizes a kind for hook-key matching: ASCII alphanumerics survive,
  /// every other run collapses to one `_`, and leading/trailing `_` is
  /// trimmed. Runs over code points, as the reference iterates characters.
  static String _normalizeKind(String kind) {
    final buffer = StringBuffer();
    var lastWasUnderscore = false;

    for (final rune in kind.runes) {
      final isAlphaNumeric = (rune >= 0x61 && rune <= 0x7a) ||
          (rune >= 0x41 && rune <= 0x5a) ||
          (rune >= 0x30 && rune <= 0x39);
      if (isAlphaNumeric) {
        buffer.writeCharCode(rune);
        lastWasUnderscore = false;
        continue;
      }
      if (!lastWasUnderscore) {
        buffer.write('_');
        lastWasUnderscore = true;
      }
    }

    var normalized = buffer.toString();
    var start = 0;
    while (start < normalized.length && normalized[start] == '_') {
      start += 1;
    }
    var end = normalized.length;
    while (end > start && normalized[end - 1] == '_') {
      end -= 1;
    }
    return normalized.substring(start, end);
  }
}
