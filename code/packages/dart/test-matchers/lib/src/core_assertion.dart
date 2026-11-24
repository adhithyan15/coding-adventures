/// Re-exports Dart's own [AssertionError] for subclassing.
//
// `matchers.dart` declares its own `AssertionError`, which shadows the
// implicit `dart:core` import there — so it reaches the core class through
// this shim under a prefix instead. (Importing `dart:core` itself with a
// prefix would suppress the implicit import entirely.)
library;

export 'dart:core' show AssertionError;
