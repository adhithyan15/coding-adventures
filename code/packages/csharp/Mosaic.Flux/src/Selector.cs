// Selector.cs — memoised derived-state combinator.
//
// For derived state computed from multiple slots of the same state,
// recomputing on every call is wasteful.  CreateSelector* methods
// memoise by input equality: if every input returned the same value
// as last call, the cached output is reused.

namespace Mosaic.Flux;

/// <summary>
/// Memoised derived-state selector combinators (1-, 2-, and 3-input
/// variants).
/// </summary>
public static class Selector
{
    /// <summary>
    /// Build a single-input memoised selector.
    /// </summary>
    public static Func<TState, TResult> Create<TState, TA, TResult>(
        Func<TState, TA> inputA,
        Func<TA, TResult> combine)
        where TA : notnull
    {
        TA? lastInput = default;
        bool hasLast = false;
        TResult lastResult = default!;
        return state =>
        {
            var a = inputA(state);
            if (hasLast && EqualityComparer<TA>.Default.Equals(lastInput!, a))
            {
                return lastResult;
            }
            lastInput = a;
            hasLast = true;
            lastResult = combine(a);
            return lastResult;
        };
    }

    /// <summary>
    /// Build a two-input memoised selector.
    /// </summary>
    public static Func<TState, TResult> Create<TState, TA, TB, TResult>(
        Func<TState, TA> inputA,
        Func<TState, TB> inputB,
        Func<TA, TB, TResult> combine)
        where TA : notnull
        where TB : notnull
    {
        TA? lastA = default;
        TB? lastB = default;
        bool hasLast = false;
        TResult lastResult = default!;
        return state =>
        {
            var a = inputA(state);
            var b = inputB(state);
            if (hasLast
                && EqualityComparer<TA>.Default.Equals(lastA!, a)
                && EqualityComparer<TB>.Default.Equals(lastB!, b))
            {
                return lastResult;
            }
            lastA = a;
            lastB = b;
            hasLast = true;
            lastResult = combine(a, b);
            return lastResult;
        };
    }

    /// <summary>
    /// Build a three-input memoised selector.
    /// </summary>
    public static Func<TState, TResult> Create<TState, TA, TB, TC, TResult>(
        Func<TState, TA> inputA,
        Func<TState, TB> inputB,
        Func<TState, TC> inputC,
        Func<TA, TB, TC, TResult> combine)
        where TA : notnull
        where TB : notnull
        where TC : notnull
    {
        TA? lastA = default;
        TB? lastB = default;
        TC? lastC = default;
        bool hasLast = false;
        TResult lastResult = default!;
        return state =>
        {
            var a = inputA(state);
            var b = inputB(state);
            var c = inputC(state);
            if (hasLast
                && EqualityComparer<TA>.Default.Equals(lastA!, a)
                && EqualityComparer<TB>.Default.Equals(lastB!, b)
                && EqualityComparer<TC>.Default.Equals(lastC!, c))
            {
                return lastResult;
            }
            lastA = a;
            lastB = b;
            lastC = c;
            hasLast = true;
            lastResult = combine(a, b, c);
            return lastResult;
        };
    }
}
