// Store.cs — MosaicStore: state container + dispatcher.
//
// The MosaicStore is the runtime's center of gravity.  It holds the
// current state, accepts action dispatches, runs middleware, and
// notifies subscribers when state changes.
//
// Design choices (per UI33-rewrite §6):
//
//   1. No central reducer.  Dispatch calls action.Apply(state)
//      directly — Command Pattern.
//
//   2. Fine-grained subscription via Subscribe(selector,
//      equality, callback).  Callback fires only when the
//      projected slice changes.
//
//   3. INotifyPropertyChanged integration so XAML hosts can bind
//      directly to <c>store.State</c> via {x:Bind State.Foo,
//      Mode=OneWay} bindings.  PropertyChanged fires with
//      property name "State" on every change.
//
//   4. Synchronous dispatch.  Apply runs immediately; subscribers
//      fire; middleware runs.

using System.Collections.Concurrent;
using System.ComponentModel;
using System.Runtime.CompilerServices;

namespace Mosaic.Flux;

/// <summary>
/// The Mosaic state container and dispatcher.  Generic over the
/// state type.  Implements INotifyPropertyChanged so XAML hosts can
/// bind to <c>store.State</c> directly.
/// </summary>
public sealed class MosaicStore<TState> : INotifyPropertyChanged
{
    private TState _state;
    private readonly Middleware<TState> _middleware;
    // ConcurrentDictionary guards against the case where Subscribe /
    // Unsubscribe / Dispatch are called from different threads (e.g.
    // a background data load dispatching while the UI thread is
    // subscribing).  Dictionary<,> is not thread-safe and corruption
    // is observable as InvalidOperationException or an infinite loop.
    private readonly ConcurrentDictionary<int, ISubscription> _subscriptions = new();
    private int _nextSubscriptionId;

    public MosaicStore(
        TState initialState,
        IEnumerable<Middleware<TState>>? middleware = null)
    {
        _state = initialState;
        _middleware = MiddlewareHelpers.Compose(
            middleware ?? Array.Empty<Middleware<TState>>());
    }

    /// <summary>
    /// The current state.  Read-only from the consumer's
    /// perspective.  XAML bindings to State.{property} will fire
    /// the PropertyChanged event when the state is swapped.
    /// </summary>
    public TState State
    {
        get => _state;
        private set
        {
            _state = value;
            OnPropertyChanged();
        }
    }

    /// <inheritdoc/>
    public event PropertyChangedEventHandler? PropertyChanged;

    private void OnPropertyChanged(
        [CallerMemberName] string? propertyName = null)
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }

    /// <summary>
    /// Dispatch an action.  Runs action.Apply(State), swaps state,
    /// notifies subscribers whose projected slice changed, raises
    /// PropertyChanged for "State", then runs middleware.
    /// </summary>
    public void Dispatch<TAction>(TAction action)
        where TAction : IMosaicAction<TState>
    {
        var prev = _state;
        var next = action.Apply(prev);
        if (ReferenceEqualsOrValueEquals(prev, next))
        {
            // No-op transform; middleware still runs.
            _middleware(action, prev, next);
            return;
        }
        State = next;  // sets _state AND raises PropertyChanged

        // Snapshot subscriptions so a callback that unsubscribes
        // can't perturb iteration.
        var snapshot = _subscriptions.Values.ToList();
        foreach (var sub in snapshot)
        {
            sub.NotifyIfChanged(next);
        }
        _middleware(action, prev, next);
    }

    /// <summary>
    /// Subscribe to a slice of state via a selector.  The callback
    /// fires when the projected slice changes by the supplied
    /// equality function (default: object equality).
    /// </summary>
    /// <returns>A disposable that unsubscribes when disposed.</returns>
    public IDisposable Subscribe<T>(
        Func<TState, T> selector,
        Action<T> callback,
        Func<T, T, bool>? equality = null)
    {
        var id = Interlocked.Increment(ref _nextSubscriptionId);
        var eq = equality ?? ((a, b) => Equals(a, b));
        _subscriptions[id] = new Subscription<T>(
            selector, callback, eq, selector(_state));
        return new Unsubscribe(() => _subscriptions.TryRemove(id, out _));
    }

    /// <summary>
    /// One-shot read of a slice without subscription.
    /// </summary>
    public T Select<T>(Func<TState, T> selector) => selector(_state);

    // Choose value-equality for value-type states (so they compare
    // structurally) and reference-equality for class-typed states.
    // Records' value equality is captured automatically because
    // their Equals override forwards to value comparison.
    private static bool ReferenceEqualsOrValueEquals(TState a, TState b)
    {
        if (typeof(TState).IsValueType)
        {
            return Equals(a, b);
        }
        return ReferenceEquals(a, b);
    }

    private interface ISubscription
    {
        void NotifyIfChanged(TState nextState);
    }

    private sealed class Subscription<T> : ISubscription
    {
        private readonly Func<TState, T> _selector;
        private readonly Action<T> _callback;
        private readonly Func<T, T, bool> _equality;
        private T _lastValue;

        public Subscription(
            Func<TState, T> selector,
            Action<T> callback,
            Func<T, T, bool> equality,
            T initial)
        {
            _selector = selector;
            _callback = callback;
            _equality = equality;
            _lastValue = initial;
        }

        public void NotifyIfChanged(TState nextState)
        {
            var nextValue = _selector(nextState);
            if (!_equality(_lastValue, nextValue))
            {
                _lastValue = nextValue;
                _callback(nextValue);
            }
        }
    }

    private sealed class Unsubscribe : IDisposable
    {
        private Action? _action;
        public Unsubscribe(Action action) => _action = action;
        // Interlocked.Exchange makes Dispose idempotent and safe
        // under concurrent double-dispose: only the thread that wins
        // the swap sees a non-null action and runs it.
        public void Dispose() => Interlocked.Exchange(ref _action, null)?.Invoke();
    }
}
