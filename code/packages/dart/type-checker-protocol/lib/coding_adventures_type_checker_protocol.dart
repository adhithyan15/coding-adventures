/// Shared type-checker contract and generic checker framework.
///
/// The Dart lane of `type-checker-protocol`: the [TypeChecker] contract,
/// [TypeCheckResult] reporting with [TypeErrorDiagnostic] diagnostics, and
/// the [GenericTypeChecker] base with phase/kind hook dispatch.
library coding_adventures_type_checker_protocol;

export 'src/type_checker_protocol.dart'
    show GenericTypeChecker, TypeCheckResult, TypeChecker, TypeErrorDiagnostic;
