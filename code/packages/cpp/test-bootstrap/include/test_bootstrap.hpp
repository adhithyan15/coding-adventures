// test_bootstrap.hpp -- the C++ property-based testing binding (VER05j).
// ============================================================================
//
// WHAT THIS IS
// ------------
// A property-based test checks a CLAIM about every input a generator can
// produce, not about one example:
//
//     for every list xs of small ints:   sum(xs) <= 20
//
// The engine that generates candidates, shrinks a failing one to a witness a
// human can read, and judges honestly lives in C (code/packages/c/test-bootstrap,
// spec VER05i).  THIS header is only a *binding*: idiomatic C++ types over that
// engine.  It owns ownership, lifetimes and the exception boundary.  It owns NO
// engine decision: not one draw, one shrink move, one step count, one failure
// id, one budget or one cap.  A second engine would be a duplicate witness (the
// TypeScript WD-901 problem), not an independent one.
//
// A TASTE OF THE API
// ------------------
//     namespace pbt = ca::pbt;
//
//     auto xs = pbt::gen::list(pbt::gen::integer(0, 10), 0, 6);
//     pbt::Options opts;  opts.ci = pbt::CiMode::Off;   // unredacted witness
//     pbt::Verdict v = pbt::for_all(xs, [](pbt::ValueView list) {
//         std::int64_t sum = 0;
//         for (std::size_t i = 0; i < list.size(); ++i) sum += list.at(i).as_i64();
//         return sum <= 20;                 // true means "the property holds"
//     }, /*seed=*/7, opts);
//     if (!v.passed()) { /* v.minimal() is the shrunk counterexample */ }
//
// THE FOUR OWNERS (RAII)
// ----------------------
//   Value      owns one pbt_val.  Copy = deep clone, move = steal.
//   ValueView  a non-owning window onto a pbt_val (what lambdas receive).
//   Gen        owns one reference on a pbt_gen, plus "anchors": the user
//              closures the C combinators point at but never own.
//   Verdict    copies everything out of the C verdict and frees it at once.
// Nothing is leaked, including when a user lambda throws.
//
// THE EXCEPTION BOUNDARY (spec VER05j section 4)
// ----------------------------------------------
// A C++ exception that propagates through C frames is undefined behaviour: the
// C code never runs its cleanup and leaks, and on some targets the unwinder
// cannot even cross it.  So EVERY callback the binding hands to C is a
// trampoline `try { user code } catch (...) { return <status>; }`:
//
//   callback   | any exception           | std::bad_alloc
//   -----------+-------------------------+-------------------------------
//   property   | -1  (property-panic)    | -1  (no OOM path exists)
//   filter     | -1  (engine: PBT_E_GEN) | -1
//   map / bind | PBT_E_GEN               | PBT_E_NOMEM (positive statuses
//   sampler    | PBT_E_GEN               |   pass straight through the C map)
//   shrinker   | PBT_E_GEN               | PBT_E_NOMEM
//
// A StatusError carries its own PBT_E_* code (that is how Run::sample reports a
// failing inner generator).  The trampolines are `noexcept`, so a bug in the
// trampoline itself would std::terminate loudly instead of reaching C.
//
// The engine already turns those statuses into verdicts: a failing generator is
// `property-exhausted`, a negative property is `property-panic`, a failing
// shrinker keeps the original witness.  So a throwing lambda yields a VERDICT.

#ifndef CA_PBT_TEST_BOOTSTRAP_HPP
#define CA_PBT_TEST_BOOTSTRAP_HPP

#include <cstddef>
#include <cstdint>
#include <exception>
#include <functional>
#include <memory>
#include <new>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>
#include <vector>

// The C engine's headers carry no `extern "C"` guards, so we supply them.
extern "C" {
#include "pbt.h"
#include "pbt_tfv.h"
}

namespace ca::pbt {

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

// An API-level failure.  `status()` is the C engine's PBT_E_* code (0 for errors
// that did not come from the engine).
class Error : public std::runtime_error {
public:
    Error(int status, const std::string& what) : std::runtime_error(what), status_(status) {}
    int status() const noexcept { return status_; }

private:
    int status_;
};

// Thrown by user code (or by Run::sample) to report a specific engine status.
class StatusError : public Error {
public:
    explicit StatusError(int status, const std::string& what = "engine status") : Error(status, what) {}
};

// A shrinker was asked about a value that does not fail (PBT_E_NOT_FALSIFYING).
class NotFalsifying : public Error {
public:
    NotFalsifying() : Error(PBT_E_NOT_FALSIFYING, "the initial value satisfies the property: nothing to shrink") {}
};

// Thrown by Verdict::assert_passed().
class PropertyFailed : public Error {
public:
    explicit PropertyFailed(const std::string& what) : Error(0, what) {}
};

// ---------------------------------------------------------------------------
// Test seams of the C++ layer (spec VER05j section 6).  NOT part of the API.
// ---------------------------------------------------------------------------
// A conformance run that cannot fail proves nothing.  These seams let a test
// make THIS layer wrong (shift a seed, drop a step, swap bounds) and watch the
// corpus run notice.  Production code only READS them; tests call reset().
//
// They exist ONLY when the translation unit is compiled with -DCA_PBT_TEST_SEAMS
// (the corpus test does).  Without it there is no global at all: seeded() and
// stepped() are the identity and bounds are never swapped, so a production
// build cannot be mis-steered by a stray write.
namespace detail {

#ifdef CA_PBT_TEST_SEAMS
struct Seams {
    std::uint64_t seed_offset = 0;  // every seed handed to the engine becomes seed + offset
    int shrink_step_delta = 0;      // every step count reported to the caller is steps + delta
    bool swap_bounds = false;       // gen::integer(min, max) swaps its bounds
    void reset() noexcept { *this = Seams(); }
};
inline Seams seams;

inline std::uint64_t seeded(std::uint64_t seed) noexcept { return seed + seams.seed_offset; }
inline int stepped(int steps) noexcept { return steps + seams.shrink_step_delta; }
inline bool bounds_swapped() noexcept { return seams.swap_bounds; }
#else
inline std::uint64_t seeded(std::uint64_t seed) noexcept { return seed; }
inline int stepped(int steps) noexcept { return steps; }
inline bool bounds_swapped() noexcept { return false; }
#endif

// ---------------------------------------------------------------------------
// Mapping the exception in flight to a status (used inside catch blocks only).
// ---------------------------------------------------------------------------
inline int classify() noexcept {
    try {
        throw;
    } catch (const StatusError& e) {
        return e.status() > 0 ? e.status() : PBT_E_GEN;
    } catch (const std::bad_alloc&) {
        return PBT_E_NOMEM;
    } catch (...) {
        return PBT_E_GEN;
    }
}

}  // namespace detail

// ---------------------------------------------------------------------------
// Values
// ---------------------------------------------------------------------------

class Value;
class ValueView;

enum class Kind { Nil, Bool, Int, Bytes, Str, List, Tuple, Ptr };

namespace detail {
inline Kind kind_of(pbt_kind k) noexcept {
    switch (k) {
        case PBT_V_NIL: return Kind::Nil;
        case PBT_V_BOOL: return Kind::Bool;
        case PBT_V_INT: return Kind::Int;
        case PBT_V_BYTES: return Kind::Bytes;
        case PBT_V_STR: return Kind::Str;
        case PBT_V_LIST: return Kind::List;
        case PBT_V_TUPLE: return Kind::Tuple;
        default: return Kind::Ptr;
    }
}
}  // namespace detail

// The read-only accessors, shared by the owner (Value) and the window
// (ValueView) through CRTP so they cannot drift apart.
template <class Derived>
class Readable {
public:
    Kind kind() const noexcept { return detail::kind_of(r().kind); }

    bool as_bool() const {
        require(PBT_V_BOOL, "bool");
        return r().u.b != 0;
    }
    // The integer, if it fits.  Wider integers stay available through decimal().
    std::int64_t as_i64() const {
        require(PBT_V_INT, "int");
        if (!pbt_big_fits_i64(&r().u.i)) throw Error(PBT_E_LIMIT, "the integer does not fit in int64_t");
        return pbt_big_to_i64(&r().u.i);
    }
    std::uint64_t as_u64() const {
        require(PBT_V_INT, "int");
        if (pbt_big_sign(&r().u.i) < 0 || !pbt_big_fits_u64(&r().u.i))
            throw Error(PBT_E_LIMIT, "the integer does not fit in uint64_t");
        return pbt_big_low_u64(&r().u.i);
    }
    // The integer reduced modulo 2^64 (two's complement for negatives).
    std::uint64_t to_u64_wrapping() const {
        require(PBT_V_INT, "int");
        return pbt_big_mod_u64(&r().u.i);
    }
    // -1, 0 or +1: how this integer compares with v.
    int cmp_i64(std::int64_t v) const {
        require(PBT_V_INT, "int");
        pbt_big b;
        pbt_big_set_i64(&b, v);
        int c = pbt_big_cmp(&r().u.i, &b);
        return c < 0 ? -1 : (c > 0 ? 1 : 0);
    }
    // Canonical decimal text of the integer (any width up to the engine's cap).
    std::string decimal() const {
        require(PBT_V_INT, "int");
        char buf[PBT_BIG_DEC_BUF];
        if (pbt_big_to_dec(&r().u.i, buf, sizeof buf) == 0) throw Error(PBT_E_LIMIT, "decimal rendering failed");
        return std::string(buf);
    }
    std::vector<std::uint8_t> as_bytes() const {
        require(PBT_V_BYTES, "bytes");
        const pbt_val& v = r();
        if (v.u.bytes.n == 0) return {};
        return std::vector<std::uint8_t>(v.u.bytes.p, v.u.bytes.p + v.u.bytes.n);
    }
    std::string as_string() const {
        require(PBT_V_STR, "string");
        const pbt_val& v = r();
        if (v.u.bytes.n == 0) return {};
        return std::string(reinterpret_cast<const char*>(v.u.bytes.p), v.u.bytes.n);
    }
    // Element count of bytes / string / list / tuple.
    std::size_t size() const {
        const pbt_val& v = r();
        switch (v.kind) {
            case PBT_V_BYTES:
            case PBT_V_STR: return v.u.bytes.n;
            case PBT_V_LIST:
            case PBT_V_TUPLE: return v.u.list.n;
            default: throw Error(PBT_E_ARG, "size() needs bytes, a string, a list or a tuple");
        }
    }
    // The view points INTO this value: it must not outlive it.  (Value deletes at() on a temporary.)
    ValueView at(std::size_t i) const;
    std::vector<Value> items() const;
    // The engine's canonical rendering (VER05 section 7.2).
    std::string canonical() const {
        pbt_str s;
        pbt_str_init(&s);
        if (pbt_canon(&r(), &s) != 0) {
            pbt_str_free(&s);
            throw Error(PBT_E_NOMEM, "canonical rendering failed");
        }
        std::string out = s.s != nullptr ? std::string(s.s, s.n) : std::string();
        pbt_str_free(&s);
        return out;
    }
    template <class Other>
    bool equals(const Readable<Other>& o) const noexcept {
        return pbt_val_equal(&r(), &o.r()) != 0;
    }
    const pbt_val& r() const noexcept { return *static_cast<const Derived*>(this)->raw(); }

private:
    void require(pbt_kind k, const char* what) const {
        if (r().kind != k) throw Error(PBT_E_ARG, std::string("the value is not a ") + what);
    }
};

// A non-owning window onto a pbt_val: valid for the call that handed it out.  Never store one
// (or a Scope, or a Falsifies, which wrap pointers into the engine's live call) beyond the
// callback it was given to.
class ValueView : public Readable<ValueView> {
public:
    explicit ValueView(const pbt_val* p) noexcept : p_(p) {}
    const pbt_val* raw() const noexcept { return p_; }

private:
    const pbt_val* p_;
};

// An owned value.  Copy deep-clones, move steals (the source becomes nil).
class Value : public Readable<Value> {
public:
    Value() noexcept { pbt_val_nil(&v_); }
    ~Value() { pbt_val_free(&v_); }
    Value(const Value& o) {
        pbt_val_nil(&v_);
        if (pbt_val_clone(&v_, &o.v_) != 0) throw Error(PBT_E_NOMEM, "value clone failed (out of memory or too deep)");
    }
    Value(Value&& o) noexcept : v_(o.v_) { pbt_val_nil(&o.v_); }
    Value& operator=(const Value& o) {
        Value copy(o);
        swap(copy);
        return *this;
    }
    Value& operator=(Value&& o) noexcept {
        Value moved(std::move(o));
        swap(moved);
        return *this;
    }
    void swap(Value& o) noexcept {
        pbt_val t = v_;
        v_ = o.v_;
        o.v_ = t;
    }

    const pbt_val* raw() const noexcept { return &v_; }
    pbt_val* raw_mut() noexcept { return &v_; }
    // A view is call-scoped: converting a temporary Value would dangle, so it is deleted.
    operator ValueView() const& noexcept { return ValueView(&v_); }
    operator ValueView() const&& = delete;
    ValueView at(std::size_t i) const& { return Readable<Value>::at(i); }
    ValueView at(std::size_t) const&& = delete;

    // ---- factories ----
    static Value nil() { return Value(); }
    static Value boolean(bool b) {
        Value v;
        pbt_val_set_bool(&v.v_, b ? 1 : 0);
        return v;
    }
    static Value integer(std::int64_t x) {
        Value v;
        pbt_val_set_i64(&v.v_, x);
        return v;
    }
    static Value integer_u64(std::uint64_t x) {
        Value v;
        pbt_val_set_u64(&v.v_, x);
        return v;
    }
    // "[-]digits"; throws Error(PBT_E_ARG) for text the engine refuses.
    static Value integer_dec(std::string_view text) {
        pbt_big b;
        if (pbt_big_parse_dec(&b, text.data(), text.size()) != 0)
            throw Error(PBT_E_ARG, "not a decimal integer within the engine's caps");
        return from_big(b);
    }
    static Value from_big(const pbt_big& b) {
        Value v;
        pbt_val_set_int(&v.v_, &b);
        return v;
    }
    static Value bytes(const std::uint8_t* data, std::size_t n) {
        Value v;
        if (pbt_val_set_bytes(&v.v_, data, n) != 0) throw Error(PBT_E_NOMEM, "out of memory building bytes");
        return v;
    }
    static Value bytes(const std::vector<std::uint8_t>& data) { return bytes(data.data(), data.size()); }
    static Value string(std::string_view s) {
        Value v;
        if (pbt_val_set_str(&v.v_, s.data(), s.size()) != 0) throw Error(PBT_E_NOMEM, "out of memory building a string");
        return v;
    }
    static Value list(std::vector<Value> items) { return container(PBT_V_LIST, std::move(items)); }
    static Value tuple(std::vector<Value> items) { return container(PBT_V_TUPLE, std::move(items)); }

    // ---- interop with raw C values ----
    // Deep copy of a C value.
    static Value from_c(const pbt_val& src) {
        Value v;
        if (pbt_val_clone(&v.v_, &src) != 0) throw Error(PBT_E_NOMEM, "value clone failed (out of memory or too deep)");
        return v;
    }
    // Take ownership of a C value, leaving `src` nil.
    static Value adopt(pbt_val& src) noexcept {
        Value v;
        v.v_ = src;
        pbt_val_nil(&src);
        return v;
    }
    // Give up ownership: the caller now owns the returned C value.
    pbt_val release() noexcept {
        pbt_val t = v_;
        pbt_val_nil(&v_);
        return t;
    }

    friend bool operator==(const Value& a, const Value& b) noexcept { return a.equals(b); }
    friend bool operator!=(const Value& a, const Value& b) noexcept { return !a.equals(b); }

private:
    static Value container(pbt_kind kind, std::vector<Value> items) {
        Value v;
        if (pbt_val_set_list(&v.v_, kind, items.size()) != 0)
            throw Error(PBT_E_LIMIT, "list too long or out of memory");
        for (std::size_t i = 0; i < items.size(); ++i) v.v_.u.list.items[i] = items[i].release();
        return v;
    }
    pbt_val v_;
};

template <class D>
ValueView Readable<D>::at(std::size_t i) const {
    const pbt_val& v = r();
    if (v.kind != PBT_V_LIST && v.kind != PBT_V_TUPLE) throw Error(PBT_E_ARG, "at() needs a list or a tuple");
    if (i >= v.u.list.n) throw Error(PBT_E_ARG, "index out of range");
    return ValueView(&v.u.list.items[i]);
}

template <class D>
std::vector<Value> Readable<D>::items() const {
    const pbt_val& v = r();
    if (v.kind != PBT_V_LIST && v.kind != PBT_V_TUPLE) throw Error(PBT_E_ARG, "items() needs a list or a tuple");
    std::vector<Value> out;
    out.reserve(v.u.list.n);
    for (std::size_t i = 0; i < v.u.list.n; ++i) out.push_back(Value::from_c(v.u.list.items[i]));
    return out;
}

// ---------------------------------------------------------------------------
// Rng, Scope, Falsifies, Run
// ---------------------------------------------------------------------------

// A SplitMix64 stream (seeded exactly as the engine seeds its own).
class Rng {
public:
    explicit Rng(std::uint64_t seed) noexcept { pbt_rng_init(&r_, detail::seeded(seed)); }
    std::uint64_t next() noexcept { return pbt_rng_next(&r_); }
    // Uniform on [0, n) by rejection sampling.
    std::uint64_t bounded(std::uint64_t n) noexcept { return pbt_rng_bounded(&r_, n); }
    pbt_rng& raw() noexcept { return r_; }

private:
    pbt_rng r_;
};

// The shrink-budget scope a property or filter runs in.  Pass it to a nested
// check_property so the nested check draws from the enclosing budget.
class Scope {
public:
    Scope() noexcept : b_(nullptr) {}
    explicit Scope(pbt_budget* b) noexcept : b_(b) {}
    pbt_budget* raw() const noexcept { return b_; }

private:
    pbt_budget* b_;
};

// "Does this candidate STILL fail?", handed to a custom shrinker.
class Falsifies {
public:
    Falsifies(pbt_pred p, pbt_budget* b) noexcept : p_(p), b_(b) {}
    bool operator()(ValueView v) const { return p_.fn(p_.self, b_, v.raw()) != 0; }
    Scope scope() const noexcept { return Scope(b_); }

private:
    pbt_pred p_;
    pbt_budget* b_;
};

class Gen;

// What a custom sampler receives: the live run (RNG stream plus budget).
class Run {
public:
    explicit Run(pbt_run* r) noexcept : r_(r) {}
    std::uint64_t next() noexcept { return pbt_rng_next(&r_->rng); }
    std::uint64_t bounded(std::uint64_t n) noexcept { return pbt_rng_bounded(&r_->rng, n); }
    // Draw from another generator.  Throws StatusError(status) if it fails, so
    // the failure propagates to the engine with its own code.
    inline Value sample(const Gen& g);
    pbt_run* raw() const noexcept { return r_; }

private:
    pbt_run* r_;
};

using Sampler = std::function<Value(Run&)>;
// Shrink `value` in place while `still_fails` says the shrunk value fails; return
// the number of ACCEPTED moves.  Throwing yields the engine's "keep the original".
using ShrinkFn = std::function<int(Value& value, const Falsifies& still_fails, int max_steps)>;

namespace detail {

// Work (filter retries) one direct Gen::sample call may spend, across all stacked filters.
constexpr std::uint64_t kDirectSampleWork = std::uint64_t(1) << 22;

// A property/predicate normalised to `bool(ValueView, Scope)`.
using PredFn = std::function<bool(ValueView, Scope)>;
using MapFn = std::function<Value(ValueView)>;

template <class F, class... A>
bool call_bool(F& f, A&&... a) {
    if constexpr (std::is_void_v<std::invoke_result_t<F&, A...>>) {
        f(std::forward<A>(a)...);
        return true;  // a void property holds unless it throws
    } else {
        return static_cast<bool>(f(std::forward<A>(a)...));
    }
}

template <class F>
bool invoke_pred(F& f, ValueView v, Scope s) {
    if constexpr (std::is_invocable_v<F&, ValueView, Scope>) {
        return call_bool(f, v, s);
    } else {
        return call_bool(f, v);
    }
}

// Wrap a callable BY REFERENCE (the caller keeps it alive for the call).
template <class F>
PredFn pred_ref(F& f) {
    return [fp = &f](ValueView v, Scope s) { return invoke_pred(*fp, v, s); };
}
// Wrap a callable BY VALUE (it must outlive the call: stored in an anchor).
template <class F>
PredFn pred_owned(F f) {
    return [fn = std::move(f)](ValueView v, Scope s) mutable { return invoke_pred(fn, v, s); };
}

// ---- trampolines: C function pointers.  Nothing may escape them. ----

inline int prop_tramp(void* user, pbt_budget* scope, const pbt_val* v) noexcept {
    try {
        return (*static_cast<PredFn*>(user))(ValueView(v), Scope(scope)) ? 1 : 0;
    } catch (...) {
        return -1;  // property-panic (a std::bad_alloc has no OOM path here)
    }
}

inline int filter_tramp(void* self, pbt_budget* b, const pbt_val* v) noexcept {
    try {
        return (*static_cast<PredFn*>(self))(ValueView(v), Scope(b)) ? 1 : 0;
    } catch (...) {
        return -1;  // the engine reports PBT_E_GEN
    }
}

inline int map_tramp(void* self, const pbt_val* in, pbt_val* out) noexcept {
    try {
        Value r = (*static_cast<MapFn*>(self))(ValueView(in));
        *out = r.release();
        return 0;
    } catch (...) {
        return classify();
    }
}

// A "predicate for the shrinker" that remembers an exception instead of letting
// it cross C, and rethrows it once the C call has returned.
struct StickyPred {
    std::function<bool(ValueView)> f;
    std::exception_ptr err;
};
inline int sticky_tramp(void* self, pbt_budget*, const pbt_val* v) noexcept {
    StickyPred* p = static_cast<StickyPred*>(self);
    if (p->err) return 0;
    try {
        return p->f(ValueView(v)) ? 1 : 0;
    } catch (...) {
        p->err = std::current_exception();
        return 0;
    }
}

inline int shrink_tramp(void* self, pbt_budget* b, pbt_val* value, pbt_pred falsifies, int max_steps, int* steps) noexcept {
    try {
        ShrinkFn& fn = *static_cast<ShrinkFn*>(self);
        // `work` takes the engine's value out of its slot (leaving it nil); the
        // guard puts whatever `work` holds back, on return AND on unwind, so the
        // slot is always valid and freeable.
        Value work = Value::adopt(*value);
        struct Guard {
            pbt_val* slot;
            Value* work;
            ~Guard() {
                pbt_val_free(slot);
                *slot = work->release();
            }
        } guard{value, &work};
        Falsifies f(falsifies, b);
        int n = fn(work, f, max_steps);
        // The count is untrusted user output: reject a negative one or one that would take the
        // running total past max_steps.  After this check the addition cannot overflow.
        long long remaining = static_cast<long long>(max_steps) - static_cast<long long>(*steps);
        if (n < 0 || static_cast<long long>(n) > remaining) return PBT_E_ARG;
        *steps += n;
        return 0;
    } catch (...) {
        return classify();
    }
}

struct CustomCtx {
    Sampler sample;
    ShrinkFn shrink;
};
inline int custom_sample_tramp(void* self, pbt_run* run, pbt_val* out) noexcept {
    try {
        Run r(run);
        Value v = static_cast<CustomCtx*>(self)->sample(r);
        *out = v.release();
        return 0;
    } catch (...) {
        return classify();
    }
}
inline int custom_shrink_tramp(void* self, pbt_budget* b, pbt_val* value, pbt_pred falsifies, int max_steps, int* steps) noexcept {
    return shrink_tramp(&static_cast<CustomCtx*>(self)->shrink, b, value, falsifies, max_steps, steps);
}

}  // namespace detail

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

// A generator: one reference on a pbt_gen plus the closures it points at.
//
// THREAD SAFETY: none.  The C engine's reference count is not atomic and the closures are
// shared by copies, so a Gen (and its copies) must stay on one thread, as the engine does.
class Gen {
public:
    Gen(const Gen& o) : g_(o.g_), anchors_(o.anchors_) { pbt_gen_retain(g_); }
    Gen(Gen&& o) noexcept : g_(o.g_), anchors_(std::move(o.anchors_)) { o.g_ = nullptr; }
    Gen& operator=(const Gen& o) {
        Gen copy(o);
        swap(copy);
        return *this;
    }
    Gen& operator=(Gen&& o) noexcept {
        Gen moved(std::move(o));
        swap(moved);
        return *this;
    }
    ~Gen() { pbt_gen_release(g_); }  // the C generator goes first, then the anchors
    void swap(Gen& o) noexcept {
        std::swap(g_, o.g_);
        anchors_.swap(o.anchors_);
    }

    const pbt_gen* raw() const noexcept { return g_; }
    const std::vector<std::shared_ptr<void>>& anchors() const noexcept { return anchors_; }

    // Take over one reference the caller holds (a constructor's result).
    static Gen adopt(pbt_gen* g, std::vector<std::shared_ptr<void>> anchors = {}) {
        if (g == nullptr) throw Error(PBT_E_ARG, "the engine refused to build the generator (invalid arguments or out of memory)");
        return Gen(g, std::move(anchors));
    }

    // Draw one value from `rng`.  Throws StatusError if the generator fails.
    //
    // The draw runs in a fresh root budget capped at detail::kDirectSampleWork
    // filter retries.  Without one, stacked rare filters (each retrying up to
    // PBT_MAX_FILTER_RETRIES) would multiply uncharged and could run for hours;
    // with it the draw ends as a StatusError(PBT_E_EXHAUSTED).
    Value sample(Rng& rng) const {
        pbt_budget storage;
        pbt_budget* scope = pbt_budget_open_child_cap(&storage, nullptr, 0, detail::kDirectSampleWork);
        pbt_run run;
        run.rng = rng.raw();
        run.budget = scope;
        run.depth = 0;
        pbt_val out;
        int rc = pbt_gen_sample(&run, g_, &out);
        pbt_budget_close(&storage);
        rng.raw() = run.rng;
        if (rc != 0) throw StatusError(rc, "generation failed");
        return Value::adopt(out);
    }

    // Shrink `value` while `still_fails` holds; returns the accepted-move count.
    // An exception from `still_fails` is held until the C call returns, then rethrown.
    template <class F>
    int shrink(Value& value, F&& still_fails, int max_steps = 1000) const {
        detail::StickyPred sp;
        sp.f = [fp = &still_fails](ValueView v) { return detail::call_bool(*fp, v); };
        pbt_pred pred;
        pred.fn = detail::sticky_tramp;
        pred.self = &sp;
        int steps = 0;
        int rc = pbt_gen_shrink(nullptr, g_, value.raw_mut(), pred, max_steps, &steps);
        if (sp.err) std::rethrow_exception(sp.err);
        if (rc != 0) throw StatusError(rc, "shrinking failed");
        return detail::stepped(steps);
    }

    // ---- combinators ----

    // map and bind drop shrinking (there is no inverse), as in C.
    template <class F>
    Gen map(F&& f) const {
        auto ctx = std::make_shared<detail::MapFn>(std::forward<F>(f));
        auto anchors = with(ctx);  // allocate the anchors BEFORE the C gen exists: a bad_alloc cannot leak it
        return adopt(pbt_gen_map(g_, detail::map_tramp, ctx.get()), std::move(anchors));
    }

    // Keep values for which `pred` is true; `pred` may take (ValueView) or (ValueView, Scope).
    template <class F>
    Gen filter(F&& pred, int max_retries = PBT_DEFAULT_MAX_RETRIES) const {
        auto ctx = std::make_shared<detail::PredFn>(detail::pred_owned(std::decay_t<F>(std::forward<F>(pred))));
        auto anchors = with(ctx);
        return adopt(pbt_gen_filter(g_, detail::filter_tramp, ctx.get(), max_retries), std::move(anchors));
    }

    // Flat map: `f(ValueView) -> Gen` picks the generator for the second stage.
    template <class F>
    Gen bind(F&& f) const {
        auto ctx = std::make_shared<std::function<Gen(ValueView)>>(std::forward<F>(f));
        auto anchors = with(ctx);
        return adopt(pbt_gen_flat_map(g_, &bind_entry, ctx.get()), std::move(anchors));
    }

    // A generator from a sampler and an optional custom shrinker.
    static Gen custom(Sampler sampler, ShrinkFn shrinker = nullptr) {
        auto ctx = std::make_shared<detail::CustomCtx>();
        ctx->sample = std::move(sampler);
        ctx->shrink = std::move(shrinker);
        std::vector<std::shared_ptr<void>> anchors;
        anchors.push_back(ctx);
        return adopt(pbt_gen_custom(detail::custom_sample_tramp, ctx->shrink ? detail::custom_shrink_tramp : nullptr,
                                    ctx.get(), nullptr),
                     std::move(anchors));
    }

private:
    Gen(pbt_gen* g, std::vector<std::shared_ptr<void>> a) noexcept : g_(g), anchors_(std::move(a)) {}

    std::vector<std::shared_ptr<void>> with(std::shared_ptr<void> extra) const {
        std::vector<std::shared_ptr<void>> a = anchors_;
        a.push_back(std::move(extra));
        return a;
    }
    static int bind_entry(void* self, const pbt_val* in, pbt_gen** out) noexcept;

    pbt_gen* g_;
    std::vector<std::shared_ptr<void>> anchors_;
};

inline Value Run::sample(const Gen& g) {
    pbt_val out;
    int rc = pbt_gen_sample(r_, g.raw(), &out);
    if (rc != 0) throw StatusError(rc, "nested generation failed");
    return Value::adopt(out);
}

// ---- flat-map trampoline (needs the complete Gen) ----
//
// The engine releases the generator a flat-map callback returns right after it
// samples it, so the produced Gen (a temporary here) must be handed over inside
// a forwarding custom generator whose destructor deletes it.  See spec section 2.
namespace detail {
struct Forward {
    explicit Forward(Gen g) : inner(std::move(g)) {}
    Gen inner;
};
inline int forward_sample(void* self, pbt_run* run, pbt_val* out) {
    return pbt_gen_sample(run, static_cast<Forward*>(self)->inner.raw(), out);
}
inline void forward_destroy(void* self) { delete static_cast<Forward*>(self); }
}  // namespace detail

inline int Gen::bind_entry(void* self, const pbt_val* in, pbt_gen** out) noexcept {
    try {
        Gen produced = (*static_cast<std::function<Gen(ValueView)>*>(self))(ValueView(in));
        auto* fwd = new detail::Forward(std::move(produced));
        pbt_gen* g = pbt_gen_custom(detail::forward_sample, nullptr, fwd, detail::forward_destroy);
        if (g == nullptr) {
            delete fwd;
            return PBT_E_NOMEM;
        }
        *out = g;
        return 0;
    } catch (...) {
        return detail::classify();
    }
}

namespace gen {

// ---- primitives ----
inline Gen boolean() { return Gen::adopt(pbt_gen_bool()); }
inline Gen u8() { return Gen::adopt(pbt_gen_u8()); }
inline Gen u64() { return Gen::adopt(pbt_gen_u64()); }
inline Gen i128() { return Gen::adopt(pbt_gen_i128()); }
// Uniform over [min, max].
inline Gen integer(std::int64_t min, std::int64_t max) {
    if (detail::bounds_swapped()) std::swap(min, max);
    return Gen::adopt(pbt_gen_int_i64(min, max));
}
// The same, for integers wider than 64 bits (decimal text).
inline Gen integer(std::string_view min_dec, std::string_view max_dec) {
    Value lo = Value::integer_dec(min_dec);
    Value hi = Value::integer_dec(max_dec);
    if (detail::bounds_swapped()) std::swap(lo, hi);
    return Gen::adopt(pbt_gen_int(&lo.raw()->u.i, &hi.raw()->u.i));
}
inline Gen bytes(std::size_t min_len, std::size_t max_len) { return Gen::adopt(pbt_gen_bytes(min_len, max_len)); }
inline Gen constant(const Value& v) { return Gen::adopt(pbt_gen_constant(v.raw())); }

// ---- combinators over several generators ----
namespace detail_ {
inline std::vector<std::shared_ptr<void>> merge(const std::vector<Gen>& gens) {
    std::vector<std::shared_ptr<void>> a;
    for (const Gen& g : gens) a.insert(a.end(), g.anchors().begin(), g.anchors().end());
    return a;
}
inline std::vector<pbt_gen*> raws(const std::vector<Gen>& gens) {
    std::vector<pbt_gen*> v;
    v.reserve(gens.size());
    for (const Gen& g : gens) v.push_back(const_cast<pbt_gen*>(g.raw()));
    return v;
}
}  // namespace detail_

inline Gen one_of(const std::vector<Gen>& gens) {
    std::vector<pbt_gen*> raw = detail_::raws(gens);
    auto anchors = detail_::merge(gens);  // anchors first (see Gen::map)
    return Gen::adopt(pbt_gen_one_of(raw.data(), raw.size()), std::move(anchors));
}
inline Gen frequency(const std::vector<std::pair<std::uint64_t, Gen>>& arms) {
    std::vector<pbt_arm> raw;
    std::vector<Gen> gens;
    raw.reserve(arms.size());
    for (const auto& a : arms) {
        pbt_arm arm;
        arm.weight = a.first;
        arm.gen = const_cast<pbt_gen*>(a.second.raw());
        raw.push_back(arm);
        gens.push_back(a.second);
    }
    auto anchors = detail_::merge(gens);
    return Gen::adopt(pbt_gen_frequency(raw.data(), raw.size()), std::move(anchors));
}
inline Gen list(const Gen& elem, std::size_t min_len, std::size_t max_len) {
    auto anchors = elem.anchors();  // copy first
    return Gen::adopt(pbt_gen_list(const_cast<pbt_gen*>(elem.raw()), min_len, max_len), std::move(anchors));
}
inline Gen tuple(const std::vector<Gen>& gens) {
    std::vector<pbt_gen*> raw = detail_::raws(gens);
    auto anchors = detail_::merge(gens);
    return Gen::adopt(pbt_gen_tuple(raw.data(), raw.size()), std::move(anchors));
}

}  // namespace gen

// ---------------------------------------------------------------------------
// Options and the verdict
// ---------------------------------------------------------------------------

enum class CiMode { Unset, On, Off };

// The default REDACTS (fail closed): no seed and no witness in the verdict.
struct Options {
    int num_tests = 100;
    int max_shrink_steps = 1000;
    CiMode ci = CiMode::Unset;
};

class Verdict {
public:
    bool passed() const noexcept { return passed_; }
    explicit operator bool() const noexcept { return passed_; }
    // One of the four registered failure ids; empty for a pass.
    const std::string& failure_id() const noexcept { return failure_id_; }
    const std::string& message() const noexcept { return message_; }
    int cases_run() const noexcept { return cases_run_; }
    int iteration() const noexcept { return iteration_; }
    int shrink_steps() const noexcept { return detail::stepped(shrink_steps_); }
    // pbt_check's own return: 0, or PBT_E_ARG / PBT_E_NOMEM (the verdict is still a failure).
    int status() const noexcept { return status_; }
    const std::string& digest() const noexcept { return digest_; }
    // Absent when the verdict is redacted (and for failures that carry none).
    const std::optional<std::uint64_t>& seed() const noexcept { return seed_; }
    // Present exactly for a NON-redacted property-counterexample.
    const std::optional<Value>& counterexample() const noexcept { return counterexample_; }
    const std::optional<Value>& minimal() const noexcept { return minimal_; }

    void assert_passed() const {
        if (!passed_) throw PropertyFailed(failure_id_ + ": " + message_);
    }

    // Copy everything out of a C verdict.  The caller still frees `v`.
    static Verdict from_c(const pbt_verdict& v, int status) {
        Verdict r;
        r.passed_ = v.passed != 0;
        r.failure_id_ = v.failure_id != nullptr ? v.failure_id : "";
        r.message_ = v.message != nullptr ? v.message : "";
        r.cases_run_ = v.cases_run;
        r.iteration_ = v.iteration;
        r.shrink_steps_ = v.shrink_steps;
        r.status_ = status;
        r.digest_ = v.digest;
        if (v.has_seed != 0) r.seed_ = v.seed;
        if (v.has_seed != 0 && r.failure_id_ == PBT_FAIL_COUNTEREXAMPLE) {
            r.counterexample_ = Value::from_c(v.counterexample);
            r.minimal_ = Value::from_c(v.minimal);
        }
        return r;
    }

private:
    Verdict() = default;
    bool passed_ = false;
    std::string failure_id_, message_, digest_;
    int cases_run_ = 0, iteration_ = 0, shrink_steps_ = 0, status_ = 0;
    std::optional<std::uint64_t> seed_;
    std::optional<Value> counterexample_, minimal_;
};

namespace detail {
// Frees the C verdict on every path out (including a throwing copy).
struct CVerdict {
    pbt_verdict v;
    CVerdict() { pbt_verdict_init(&v); }
    ~CVerdict() { pbt_verdict_free(&v); }
    CVerdict(const CVerdict&) = delete;
    CVerdict& operator=(const CVerdict&) = delete;
};

inline pbt_options to_c(const Options& o) noexcept {
    pbt_options c;
    c.num_tests = o.num_tests;
    c.max_shrink_steps = o.max_shrink_steps;
    c.ci = o.ci == CiMode::On ? PBT_CI_ON : (o.ci == CiMode::Off ? PBT_CI_OFF : PBT_CI_UNSET);
    return c;
}

inline Verdict check_core(pbt_budget* parent, std::uint64_t seed, const Gen& gen, PredFn& pf, const Options& opts,
                          ShrinkFn* shrinker) {
    CVerdict cv;
    pbt_prop prop;
    prop.fn = prop_tramp;
    prop.user = &pf;
    pbt_options o = to_c(opts);
    pbt_shrinker sh;
    sh.fn = shrink_tramp;
    sh.self = shrinker;
    int rc = pbt_check(parent, seeded(seed), gen.raw(), prop, &o, shrinker != nullptr ? &sh : nullptr, &cv.v);
    return Verdict::from_c(cv.v, rc);
}
}  // namespace detail

// ---------------------------------------------------------------------------
// Running properties
// ---------------------------------------------------------------------------
// `prop` returns bool (true = the property holds) or void (holds unless it
// throws), and takes (ValueView) or (ValueView, Scope).  Throwing is a
// `property-panic` verdict, never a crash and never a pass.

template <class P>
Verdict check_property(std::uint64_t seed, const Gen& gen, P&& prop, const Options& opts = Options()) {
    detail::PredFn pf = detail::pred_ref(prop);
    return detail::check_core(nullptr, seed, gen, pf, opts, nullptr);
}
template <class P>
Verdict check_property(std::uint64_t seed, const Gen& gen, P&& prop, const Options& opts, ShrinkFn shrinker) {
    detail::PredFn pf = detail::pred_ref(prop);
    return detail::check_core(nullptr, seed, gen, pf, opts, &shrinker);
}
// The nested form: a check inside a property, drawing from the enclosing budget.
template <class P>
Verdict check_property(const Scope& parent, std::uint64_t seed, const Gen& gen, P&& prop, const Options& opts = Options()) {
    detail::PredFn pf = detail::pred_ref(prop);
    return detail::check_core(parent.raw(), seed, gen, pf, opts, nullptr);
}
template <class P>
Verdict check_property(const Scope& parent, std::uint64_t seed, const Gen& gen, P&& prop, const Options& opts,
                       ShrinkFn shrinker) {
    detail::PredFn pf = detail::pred_ref(prop);
    return detail::check_core(parent.raw(), seed, gen, pf, opts, &shrinker);
}
template <class P>
Verdict for_all(const Gen& gen, P&& prop, std::uint64_t seed, const Options& opts = Options()) {
    return check_property(seed, gen, std::forward<P>(prop), opts);
}

// ---------------------------------------------------------------------------
// Standalone shrinkers and replay (the corpus entry points).  `prop` is true
// when the property HOLDS.
// ---------------------------------------------------------------------------

struct Shrunk {
    Value minimal;
    int steps;
};
struct Replay {
    Value initial_fail;
    Value minimal;
};

namespace detail {
inline void throw_status(int rc, const char* what) {
    if (rc == PBT_E_NOT_FALSIFYING) throw NotFalsifying();
    throw Error(rc, what);
}
inline const pbt_big& big_of(const Value& v) {
    if (v.raw()->kind != PBT_V_INT) throw Error(PBT_E_ARG, "an integer value is required");
    return v.raw()->u.i;
}
}  // namespace detail

template <class P>
Shrunk shrink_int(const Value& initial, const Value& target, const Value& lo, const Value& hi, P&& prop,
                  int max_steps = 1000) {
    detail::PredFn pf = detail::pred_ref(prop);
    pbt_prop cp;
    cp.fn = detail::prop_tramp;
    cp.user = &pf;
    pbt_big minimal;
    int steps = 0;
    int rc = pbt_shrink_int(nullptr, &detail::big_of(initial), &detail::big_of(target), &detail::big_of(lo),
                            &detail::big_of(hi), cp, max_steps, &minimal, &steps);
    if (rc != 0) detail::throw_status(rc, "shrink_int failed");
    return Shrunk{Value::from_big(minimal), detail::stepped(steps)};
}

template <class P>
Shrunk shrink_list_ints(Value list, const Value& elem_target, P&& prop, int max_steps = 1000) {
    if (list.raw()->kind != PBT_V_LIST) throw Error(PBT_E_ARG, "a list is required");
    for (std::size_t i = 0; i < list.size(); ++i)
        if (list.at(i).kind() != Kind::Int) throw Error(PBT_E_ARG, "a list of ints is required");
    detail::PredFn pf = detail::pred_ref(prop);
    pbt_prop cp;
    cp.fn = detail::prop_tramp;
    cp.user = &pf;
    int steps = 0;
    int rc = pbt_shrink_list_ints(nullptr, list.raw_mut(), &detail::big_of(elem_target), cp, max_steps, &steps);
    if (rc != 0) detail::throw_status(rc, "shrink_list_ints failed");
    return Shrunk{std::move(list), detail::stepped(steps)};
}

template <class P>
Shrunk shrink_bytes(Value bytes, P&& prop, int max_steps = 1000) {
    if (bytes.raw()->kind != PBT_V_BYTES) throw Error(PBT_E_ARG, "bytes are required");
    detail::PredFn pf = detail::pred_ref(prop);
    pbt_prop cp;
    cp.fn = detail::prop_tramp;
    cp.user = &pf;
    int steps = 0;
    int rc = pbt_shrink_bytes(nullptr, bytes.raw_mut(), cp, max_steps, &steps);
    if (rc != 0) detail::throw_status(rc, "shrink_bytes failed");
    return Shrunk{std::move(bytes), detail::stepped(steps)};
}

// Draw up to 1000 values in [0, 1000]; the first failing one is shrunk toward 0.
template <class P>
Replay simulate_replay(std::uint64_t seed, P&& prop) {
    detail::PredFn pf = detail::pred_ref(prop);
    pbt_prop cp;
    cp.fn = detail::prop_tramp;
    cp.user = &pf;
    pbt_big initial, minimal;
    int rc = pbt_simulate_replay(nullptr, detail::seeded(seed), cp, &initial, &minimal);
    if (rc != 0) detail::throw_status(rc, "simulate_replay failed");
    return Replay{Value::from_big(initial), Value::from_big(minimal)};
}

// ---------------------------------------------------------------------------
// Small pure helpers
// ---------------------------------------------------------------------------

// True only for the four registered failure ids (an embedded NUL is never valid).
inline bool is_failure_id(std::string_view candidate) {
    if (candidate.find('\0') != std::string_view::npos) return false;
    return pbt_is_failure_id(std::string(candidate).c_str()) != 0;
}

// Strict TFV v1 validation; `why` (optional) receives the engine's short reason.
inline bool tfv_valid(std::string_view text, std::string* why = nullptr) {
    char err[200];
    err[0] = '\0';
    bool ok = pbt_tfv_validate(text.data(), text.size(), err, sizeof err) == 0;
    if (!ok && why != nullptr) *why = err;
    return ok;
}

// Pure CI detection: pass the value of the CI variable (nullptr when unset).
inline bool ci_from_value(const char* value) { return pbt_ci_from_value(value) != 0; }

// Throws Error(PBT_E_ARG) for an id with an embedded NUL: the C engine would silently hash only
// the text before it, so two different ids could collide.
inline std::uint64_t sub_seed(std::uint64_t run_seed, const std::string& property_id) {
    if (property_id.find('\0') != std::string::npos) throw Error(PBT_E_ARG, "a property id must not contain a NUL");
    return pbt_sub_seed(run_seed, property_id.c_str());
}
inline std::uint64_t fnv1a64(std::string_view data) { return pbt_fnv1a64(data.data(), data.size()); }

}  // namespace ca::pbt

#endif  // CA_PBT_TEST_BOOTSTRAP_HPP
