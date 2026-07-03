// selector.dart — memoised derived-state combinator.
//
// For derived state computed from multiple slots of the same state,
// recomputing on every call is wasteful. createSelector1/2/3
// memoise by input equality: if every input returned the same
// value as last call, the cached output is reused.

/// Build a single-input memoised selector.
R Function(S) createSelector1<S, A, R>(
  A Function(S) inputA,
  R Function(A) combine,
) {
  A? _lastInput;
  bool _hasLast = false;
  late R _lastResult;
  return (state) {
    final a = inputA(state);
    if (_hasLast && _lastInput == a) {
      return _lastResult;
    }
    _lastInput = a;
    _hasLast = true;
    _lastResult = combine(a);
    return _lastResult;
  };
}

/// Build a two-input memoised selector.
R Function(S) createSelector2<S, A, B, R>(
  A Function(S) inputA,
  B Function(S) inputB,
  R Function(A, B) combine,
) {
  A? _lastA;
  B? _lastB;
  bool _hasLast = false;
  late R _lastResult;
  return (state) {
    final a = inputA(state);
    final b = inputB(state);
    if (_hasLast && _lastA == a && _lastB == b) {
      return _lastResult;
    }
    _lastA = a;
    _lastB = b;
    _hasLast = true;
    _lastResult = combine(a, b);
    return _lastResult;
  };
}

/// Build a three-input memoised selector.
R Function(S) createSelector3<S, A, B, C, R>(
  A Function(S) inputA,
  B Function(S) inputB,
  C Function(S) inputC,
  R Function(A, B, C) combine,
) {
  A? _lastA;
  B? _lastB;
  C? _lastC;
  bool _hasLast = false;
  late R _lastResult;
  return (state) {
    final a = inputA(state);
    final b = inputB(state);
    final c = inputC(state);
    if (_hasLast && _lastA == a && _lastB == b && _lastC == c) {
      return _lastResult;
    }
    _lastA = a;
    _lastB = b;
    _lastC = c;
    _hasLast = true;
    _lastResult = combine(a, b, c);
    return _lastResult;
  };
}
