// Selector.swift — memoised derived-state combinator.
//
// For derived state that's computed on the fly (e.g., a formula bar's
// displayed value depends on three state fields), recomputing on
// every selector call is wasteful.  `createSelector` memoises by
// input-equality: if every input selector returned the same value as
// last call, the cached output is reused.
//
// This is the same shape as Reselect / Redux Toolkit's
// `createSelector` and our TypeScript runtimes' `createSelector`.
// v0.1.0 provides two-input and three-input variants; higher arities
// are easy to add when a real use case appears.

/// Build a single-input memoised selector.
///
/// The combiner runs only when the input selector's result differs
/// from the last call (by `Equatable` equality if T is Equatable, or
/// reference identity for class types).
public func createSelector<State, A, R>(
    _ inputA: @escaping (State) -> A,
    _ combine: @escaping (A) -> R
) -> (State) -> R where A: Equatable {
    var lastInput: A?
    var lastResult: R?
    return { state in
        let a = inputA(state)
        if let last = lastInput, last == a, let result = lastResult {
            return result
        }
        lastInput = a
        let r = combine(a)
        lastResult = r
        return r
    }
}

/// Build a two-input memoised selector.
public func createSelector<State, A, B, R>(
    _ inputA: @escaping (State) -> A,
    _ inputB: @escaping (State) -> B,
    _ combine: @escaping (A, B) -> R
) -> (State) -> R where A: Equatable, B: Equatable {
    var lastInputs: (A, B)?
    var lastResult: R?
    return { state in
        let a = inputA(state)
        let b = inputB(state)
        if let last = lastInputs, last.0 == a, last.1 == b, let result = lastResult {
            return result
        }
        lastInputs = (a, b)
        let r = combine(a, b)
        lastResult = r
        return r
    }
}

/// Build a three-input memoised selector.
public func createSelector<State, A, B, C, R>(
    _ inputA: @escaping (State) -> A,
    _ inputB: @escaping (State) -> B,
    _ inputC: @escaping (State) -> C,
    _ combine: @escaping (A, B, C) -> R
) -> (State) -> R where A: Equatable, B: Equatable, C: Equatable {
    var lastInputs: (A, B, C)?
    var lastResult: R?
    return { state in
        let a = inputA(state)
        let b = inputB(state)
        let c = inputC(state)
        if let last = lastInputs,
           last.0 == a, last.1 == b, last.2 == c,
           let result = lastResult {
            return result
        }
        lastInputs = (a, b, c)
        let r = combine(a, b, c)
        lastResult = r
        return r
    }
}
