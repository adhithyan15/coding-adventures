// Selector.h — memoised projections.
//
// Selectors do for state-reads what `useMemo` does for React: cache
// the last input(s) and recompute only when they change.  Mosaic
// emitters generate selectors above the IR layer; this header gives
// the runtime a place to point.
//
// `createSelector1` / `2` / `3` are the canonical shapes — mirroring
// Reselect / @reduxjs/toolkit.  The C++ flavour uses generic
// std::function-backed closures, so consumers can use lambdas freely.

#ifndef MOSAIC_FLUX_SELECTOR_H
#define MOSAIC_FLUX_SELECTOR_H

#include <functional>
#include <memory>
#include <mutex>
#include <optional>

namespace MosaicFlux {

namespace detail {
// Equality default: prefers operator== if defined; otherwise will
// fall back to copy + assign + compare via the lambda.
template <typename A>
inline bool defaultEquals(const A& a, const A& b) {
    return a == b;
}
}  // namespace detail

// One-input memoised selector.  Repeated calls with an `S` whose
// projected `A` is equal to the cached `A` return the cached `R`
// without recomputing.  The returned closure is safe to call from
// multiple threads concurrently — an internal mutex guards the
// memo slots so a parallel read can't observe a torn cache.
template <typename S, typename A, typename R>
std::function<R(const S&)> createSelector1(
    std::function<A(const S&)> sliceA,
    std::function<R(const A&)> compute) {
    struct Cache {
        std::mutex mu;
        std::optional<A> lastA;
        std::optional<R> lastR;
    };
    auto cache = std::make_shared<Cache>();
    return [sliceA = std::move(sliceA),
            compute = std::move(compute),
            cache](const S& state) -> R {
        A a = sliceA(state);
        std::lock_guard<std::mutex> lk(cache->mu);
        if (cache->lastA.has_value() &&
            detail::defaultEquals(*cache->lastA, a)) {
            return *cache->lastR;
        }
        R r = compute(a);
        cache->lastA = std::move(a);
        cache->lastR = r;
        return r;
    };
}

// Two-input memoised selector.  Thread-safe; see createSelector1.
template <typename S, typename A, typename B, typename R>
std::function<R(const S&)> createSelector2(
    std::function<A(const S&)> sliceA,
    std::function<B(const S&)> sliceB,
    std::function<R(const A&, const B&)> compute) {
    struct Cache {
        std::mutex mu;
        std::optional<A> lastA;
        std::optional<B> lastB;
        std::optional<R> lastR;
    };
    auto cache = std::make_shared<Cache>();
    return [sliceA = std::move(sliceA),
            sliceB = std::move(sliceB),
            compute = std::move(compute),
            cache](const S& state) -> R {
        A a = sliceA(state);
        B b = sliceB(state);
        std::lock_guard<std::mutex> lk(cache->mu);
        if (cache->lastA.has_value() && cache->lastB.has_value() &&
            detail::defaultEquals(*cache->lastA, a) &&
            detail::defaultEquals(*cache->lastB, b)) {
            return *cache->lastR;
        }
        R r = compute(a, b);
        cache->lastA = std::move(a);
        cache->lastB = std::move(b);
        cache->lastR = r;
        return r;
    };
}

// Three-input memoised selector.  Thread-safe; see createSelector1.
template <typename S, typename A, typename B, typename C, typename R>
std::function<R(const S&)> createSelector3(
    std::function<A(const S&)> sliceA,
    std::function<B(const S&)> sliceB,
    std::function<C(const S&)> sliceC,
    std::function<R(const A&, const B&, const C&)> compute) {
    struct Cache {
        std::mutex mu;
        std::optional<A> lastA;
        std::optional<B> lastB;
        std::optional<C> lastC;
        std::optional<R> lastR;
    };
    auto cache = std::make_shared<Cache>();
    return [sliceA = std::move(sliceA),
            sliceB = std::move(sliceB),
            sliceC = std::move(sliceC),
            compute = std::move(compute),
            cache](const S& state) -> R {
        A a = sliceA(state);
        B b = sliceB(state);
        C c = sliceC(state);
        std::lock_guard<std::mutex> lk(cache->mu);
        if (cache->lastA.has_value() && cache->lastB.has_value() &&
            cache->lastC.has_value() &&
            detail::defaultEquals(*cache->lastA, a) &&
            detail::defaultEquals(*cache->lastB, b) &&
            detail::defaultEquals(*cache->lastC, c)) {
            return *cache->lastR;
        }
        R r = compute(a, b, c);
        cache->lastA = std::move(a);
        cache->lastB = std::move(b);
        cache->lastC = std::move(c);
        cache->lastR = r;
        return r;
    };
}

}  // namespace MosaicFlux

#endif  // MOSAIC_FLUX_SELECTOR_H
