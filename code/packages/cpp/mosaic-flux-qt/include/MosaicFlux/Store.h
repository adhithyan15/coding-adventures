// Store.h — MosaicStore: state container + dispatcher (Qt-friendly).
//
// Templates and Q_OBJECT don't mix cleanly (moc can't process
// template classes), so v0.1.0 ships the store as a plain template
// without Q_OBJECT inheritance.  A v0.2.0 thin shim will adapt it
// to QObject so it can be exposed to QML — but the core stays here,
// header-only, dependency-free.
//
// Concurrency design (after security review):
//
//   * `_dispatchMu` serializes whole dispatches so two concurrent
//     `dispatch()` calls can never interleave apply/swap/notify
//     phases.  This is the Flux-correct semantic anyway: actions
//     must apply atomically.
//
//   * `_registry` is a `shared_ptr<Registry>`; subscriptions hold a
//     `weak_ptr` to it.  If the store is destroyed before an
//     outstanding `Subscription`, `~Subscription` finds the
//     `weak_ptr` expired and skips the erase, so we can't
//     use-after-free across the store/subscription lifetime gap.
//
//   * `action.apply(prev)` runs UNLOCKED so a misbehaving action
//     that reads state via `store.state()` cannot deadlock.
//
//   * Subscriber callbacks and middleware run while `_dispatchMu`
//     is held — Flux semantics demand it.  To prevent the classic
//     deadlock "callback calls dispatch", a `thread_local` reentry
//     guard throws `std::logic_error` on nested dispatch from the
//     same thread.  This matches Redux's "you may not dispatch
//     from a reducer" rule, surfaced as an exception instead of a
//     hang.  Follow-up actions should be queued by the caller
//     (e.g. dispatched on the next tick), not nested inline.

#ifndef MOSAIC_FLUX_STORE_H
#define MOSAIC_FLUX_STORE_H

#include <atomic>
#include <cstdint>
#include <functional>
#include <memory>
#include <mutex>
#include <stdexcept>
#include <type_traits>
#include <unordered_map>
#include <utility>
#include <vector>

#include "MosaicFlux/Action.h"
#include "MosaicFlux/Middleware.h"

namespace MosaicFlux {

template <typename State>
class MosaicStore {
private:
    struct ISubscription {
        virtual ~ISubscription() = default;
        virtual void notifyIfChanged(const State& next) = 0;
    };

public:
    // 64-bit ID space: at 1 GHz subscribe rate it takes >500 years
    // to wrap.  Use `uint64_t` explicitly so this guarantee survives
    // 32-bit targets where `size_t` would only buy us ~4 seconds at
    // the same rate.
    using SubscriptionId = std::uint64_t;

    // Registry lives in a shared_ptr; Subscriptions hold weak_ptr
    // to it so they survive store destruction without UAF.
    // Declared public-but-nested so Subscription can hold a
    // weak_ptr<Registry> without needing the full MosaicStore type.
    struct Registry {
        std::mutex mu;
        std::unordered_map<SubscriptionId, std::shared_ptr<ISubscription>> subs;
        std::atomic<SubscriptionId> nextId{0};
    };

    explicit MosaicStore(State initial,
                         std::vector<Middleware<State>> middleware = {})
        : _state(std::move(initial)),
          _middleware(composeMiddleware<State>(std::move(middleware))),
          _registry(std::make_shared<Registry>()) {}

    // The store is non-copyable and non-movable so the registry's
    // shared_ptr identity (and thus weak_ptr observers in live
    // Subscriptions) remains stable.
    MosaicStore(const MosaicStore&) = delete;
    MosaicStore& operator=(const MosaicStore&) = delete;
    MosaicStore(MosaicStore&&) = delete;
    MosaicStore& operator=(MosaicStore&&) = delete;

    // Read-only access to current state.  Returns by value to
    // avoid handing out a reference that would race with a
    // concurrent dispatch.
    State state() const {
        std::lock_guard<std::mutex> lk(_stateMu);
        return _state;
    }

    // Dispatch an action.  Apply runs UNLOCKED so a misbehaving
    // action that reads state cannot deadlock.  Subscribers and
    // middleware run under the dispatch lock; a nested dispatch
    // from the same thread throws `std::logic_error` rather than
    // deadlocking (matches Redux's reducer-purity contract).
    template <typename Action>
    void dispatch(const Action& action) {
        static_assert(std::is_base_of_v<MosaicAction<State>, Action>,
                      "dispatch requires a MosaicAction<State> subclass");

        if (inDispatch()) {
            throw std::logic_error(
                "MosaicFlux: nested dispatch is forbidden — "
                "a subscriber or middleware called dispatch() "
                "while another dispatch was in progress on the "
                "same thread.  Queue the follow-up action instead "
                "(e.g. dispatch on the next event-loop tick).");
        }
        ReentryGuard guard;  // sets inDispatch() = true for this thread
        std::lock_guard<std::mutex> dispLk(_dispatchMu);

        State prev;
        {
            std::lock_guard<std::mutex> lk(_stateMu);
            prev = _state;
        }
        State next = action.apply(prev);  // unlocked

        bool changed = !(prev == next);
        std::vector<std::shared_ptr<ISubscription>> snapshot;
        if (changed) {
            {
                std::lock_guard<std::mutex> lk(_stateMu);
                _state = next;
            }
            {
                std::lock_guard<std::mutex> lk(_registry->mu);
                snapshot.reserve(_registry->subs.size());
                for (auto& kv : _registry->subs) snapshot.push_back(kv.second);
            }
            for (auto& sub : snapshot) sub->notifyIfChanged(next);
        }
        _middleware(action, prev, next);  // always runs; unlocked
    }

    // One-shot projection without subscription.
    template <typename Sel>
    auto select(Sel sel) const -> decltype(sel(std::declval<State>())) {
        std::lock_guard<std::mutex> lk(_stateMu);
        return sel(_state);
    }

    // RAII subscription handle.  Destructor unsubscribes; explicit
    // unsubscribe() is idempotent and survives store destruction
    // (weak_ptr-based).
    class Subscription {
    public:
        Subscription() = default;
        Subscription(std::weak_ptr<Registry> registry, SubscriptionId id)
            : _registry(std::move(registry)), _id(id), _live(true) {}
        Subscription(const Subscription&) = delete;
        Subscription& operator=(const Subscription&) = delete;
        Subscription(Subscription&& o) noexcept
            : _registry(std::move(o._registry)),
              _id(o._id),
              _live(o._live.exchange(false)) {}
        Subscription& operator=(Subscription&& o) noexcept {
            if (this != &o) {
                unsubscribe();
                _registry = std::move(o._registry);
                _id = o._id;
                _live = o._live.exchange(false);
            }
            return *this;
        }
        ~Subscription() { unsubscribe(); }

        void unsubscribe() {
            // Atomic exchange makes this safe under concurrent
            // double-unsubscribe: only one thread wins.  weak_ptr
            // lock() returns null if the store was destroyed first,
            // so we never deref a dangling pointer.
            if (_live.exchange(false)) {
                if (auto r = _registry.lock()) {
                    std::lock_guard<std::mutex> lk(r->mu);
                    r->subs.erase(_id);
                }
            }
        }

    private:
        std::weak_ptr<Registry> _registry;
        SubscriptionId _id = 0;
        std::atomic<bool> _live{false};
    };

    // Subscribe to a slice of state.  `sel` projects state to T;
    // `cb` fires when the projected value changes (operator==).
    template <typename T, typename Sel, typename Cb>
    Subscription subscribe(Sel sel, Cb cb) {
        return subscribeImpl<T>(std::move(sel), std::move(cb),
                                [](const T& a, const T& b) { return a == b; });
    }

    // Subscribe with a custom equality predicate.
    template <typename T, typename Sel, typename Cb, typename Eq>
    Subscription subscribeWithEquality(Sel sel, Cb cb, Eq eq) {
        return subscribeImpl<T>(std::move(sel), std::move(cb), std::move(eq));
    }

private:
    // Per-thread reentry flag.  Function-local `thread_local` so it
    // works in a header-only template without ODR issues.
    static bool& inDispatch() {
        thread_local bool flag = false;
        return flag;
    }
    struct ReentryGuard {
        ReentryGuard() { inDispatch() = true; }
        ~ReentryGuard() { inDispatch() = false; }
        ReentryGuard(const ReentryGuard&) = delete;
        ReentryGuard& operator=(const ReentryGuard&) = delete;
    };

    template <typename T>
    struct ConcreteSub : ISubscription {
        std::function<T(const State&)> selector;
        std::function<void(const T&)> callback;
        std::function<bool(const T&, const T&)> equality;
        T last;

        void notifyIfChanged(const State& next) override {
            T nextValue = selector(next);
            if (!equality(last, nextValue)) {
                last = nextValue;
                callback(nextValue);
            }
        }
    };

    template <typename T, typename Sel, typename Cb, typename Eq>
    Subscription subscribeImpl(Sel sel, Cb cb, Eq eq) {
        auto sub = std::make_shared<ConcreteSub<T>>();
        sub->selector = std::move(sel);
        sub->callback = std::move(cb);
        sub->equality = std::move(eq);
        {
            std::lock_guard<std::mutex> lk(_stateMu);
            sub->last = sub->selector(_state);
        }

        SubscriptionId id = ++_registry->nextId;
        {
            std::lock_guard<std::mutex> lk(_registry->mu);
            _registry->subs[id] = sub;
        }
        return Subscription(_registry, id);
    }

    mutable std::mutex _stateMu;
    std::mutex _dispatchMu;
    State _state;
    Middleware<State> _middleware;
    std::shared_ptr<Registry> _registry;
};

}  // namespace MosaicFlux

#endif  // MOSAIC_FLUX_STORE_H
