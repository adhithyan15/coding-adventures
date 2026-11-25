// fuzz_bootstrap.hpp -- the C++ coverage-guided fuzz binding (VER06k).
// ============================================================================
//
// WHAT A FUZZER DOES
// ------------------
// A fuzzer feeds mutated inputs to a function (the "target") and watches for
// trouble: a CRASH, a TIMEOUT (too many steps) or an OOM (too much memory).
// "Coverage-guided" means it keeps the inputs that reached NEW program points
// (edges) and mutates those further, so it climbs through nested checks that
// blind mutation would never get past:
//
//     if (d[0] == 'F') if (d[1] == 'U') if (d[2] == 'Z') throw Crash("fuzz");
//
// THIS HEADER IS A BINDING, NOT AN ENGINE
// ---------------------------------------
// The engine lives in C (code/packages/c/test-bootstrap, spec VER06j).  Every
// mutator, the signature hash, the minimiser, the power schedule, the budgets and
// the TAP grammar come from there, unchanged.  Here we only ADAPT TYPES:
//
//     C                          C++
//     const uint8_t*, size_t  -> ByteView (a window) / Bytes (an owner)
//     fz_bytes                -> Bytes, freed by RAII even if copying throws
//     pbt_big                 -> BigInt
//     fz_handle*              -> Handle (move-only, inert once retired)
//     fz_report               -> Report (move-only owner)
//     int status              -> ca::fuzz::Error (a C++ exception)
//     target returns nonzero  -> the target THROWS
//
// A second engine in C++ would be a duplicate witness, not an independent one
// (policy WD-901: Kotlin binds over Java, F# over C#, C++ over C).
//
// THE EXCEPTION BOUNDARY (the heart of the binding)
// -------------------------------------------------
// A C++ exception must never unwind through a C frame: that is undefined
// behaviour.  Every callback handed to C is a `noexcept` trampoline
//
//     int trampoline(...) noexcept { try { user code } catch (...) { classify } }
//
// and a throw is classified by its TYPE, never by what() (a message can hold
// input bytes; it is also the part that differs between two hits of one bug):
//
//     ca::fuzz::Crash(kind, frames)  -> crash with exactly that kind and frames
//     std::bad_alloc                 -> OOM (counted, never a crash signature)
//     any other std::exception       -> crash, kind = typeid(e).name()
//     anything else                  -> crash, kind = "non-std-exception"
//
// A TASTE OF THE API
// ------------------
//     namespace fz = ca::fuzz;
//     fz::Options opts;  opts.max_iterations = 500;
//     fz::Report r = fz::run({fz::Bytes{'A', 'B'}}, opts,
//         [](fz::ByteView d, fz::Handle& h) {
//             h.edge(1);
//             if (d.size() > 0 && d[0] == 0xFF) throw fz::Crash("ff", {"parse"});
//         });
//     for (const std::string& line : r.tap_failures("my_target")) { /* not ok ... */ }

#ifndef CA_FUZZ_BOOTSTRAP_HPP
#define CA_FUZZ_BOOTSTRAP_HPP

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <exception>
#include <functional>
#include <new>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <thread>
#include <typeinfo>
#include <utility>
#include <vector>

#if defined(__has_include)
#if __has_include(<span>) && defined(__cplusplus) && __cplusplus >= 202002L
#include <span>
#define CA_FUZZ_HAS_SPAN 1
#endif
#endif

// The C engine's headers carry no `extern "C"` guards, so we supply them.
extern "C" {
#include "fuzz.h"
#include "pbt_tfv.h"
}

namespace ca::fuzz {

using Bytes = std::vector<std::uint8_t>;
using Frames = std::vector<std::string>;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

// An engine status turned into an exception.  `status()` is the FZ_E_* code.
class Error : public std::runtime_error {
public:
    Error(int status, const std::string& what) : std::runtime_error(what), status_(status) {}
    int status() const noexcept { return status_; }

private:
    int status_;
};

// What a target throws to declare a crash with an explicit kind and frames (the C++
// analogue of fz_crash_set / fz_crash_push).  what() is a fixed string: it never
// carries the kind, so nothing a log prints can leak data.
class Crash : public std::exception {
public:
    explicit Crash(std::string kind, Frames frames = {}) : kind_(std::move(kind)), frames_(std::move(frames)) {}
    const std::string& kind() const noexcept { return kind_; }
    const Frames& frames() const noexcept { return frames_; }
    const char* what() const noexcept override { return "ca::fuzz::Crash"; }

private:
    std::string kind_;
    Frames frames_;
};

// ---------------------------------------------------------------------------
// Views and big integers
// ---------------------------------------------------------------------------

// A non-owning window onto bytes (std::span is C++20; the repo pins C++17).  Like std::span it binds
// implicitly to temporaries, so it is safe as a PARAMETER, but never store a ByteView past the
// full-expression that made it.
class ByteView {
public:
    ByteView() noexcept = default;
    ByteView(const std::uint8_t* p, std::size_t n) noexcept : p_(p), n_(n) {}
    ByteView(const Bytes& v) noexcept : p_(v.data()), n_(v.size()) {}  // NOLINT: implicit by design
    ByteView(std::string_view s) noexcept : p_(reinterpret_cast<const std::uint8_t*>(s.data())), n_(s.size()) {}
    template <std::size_t N>
    ByteView(const std::uint8_t (&a)[N]) noexcept : p_(a), n_(N) {}
#ifdef CA_FUZZ_HAS_SPAN
    ByteView(std::span<const std::uint8_t> s) noexcept : p_(s.data()), n_(s.size()) {}
#endif
    const std::uint8_t* data() const noexcept { return p_; }
    std::size_t size() const noexcept { return n_; }
    bool empty() const noexcept { return n_ == 0; }
    std::uint8_t operator[](std::size_t i) const noexcept { return p_[i]; }
    Bytes to_bytes() const { return n_ == 0 ? Bytes() : Bytes(p_, p_ + n_); }

private:
    const std::uint8_t* p_ = nullptr;
    std::size_t n_ = 0;
};

// An integer of up to 1,000 bits (the corpus carries integers wider than 64 bits).
class BigInt {
public:
    BigInt() noexcept { pbt_big_zero(&v_); }
    explicit BigInt(std::int64_t v) noexcept { pbt_big_set_i64(&v_, v); }
    static BigInt from_u64(std::uint64_t v) noexcept {
        BigInt b;
        pbt_big_set_u64(&b.v_, v);
        return b;
    }
    // "[-]digits"; throws Error(FZ_E_LIMIT) for empty text, a non-digit or a value over the cap.
    static BigInt parse(std::string_view dec) {
        BigInt b;
        if (pbt_big_parse_dec(&b.v_, dec.data(), dec.size()) != 0)
            throw Error(FZ_E_LIMIT, "the integer is malformed or too wide for the engine");
        return b;
    }
    bool fits_i64() const noexcept { return pbt_big_fits_i64(&v_) != 0; }
    std::int64_t to_i64() const {
        if (!fits_i64()) throw Error(FZ_E_LIMIT, "the integer does not fit in int64_t");
        return pbt_big_to_i64(&v_);
    }
    bool is_negative() const noexcept { return pbt_big_sign(&v_) < 0; }
    bool fits_u64() const noexcept { return pbt_big_sign(&v_) >= 0 && pbt_big_fits_u64(&v_) != 0; }
    std::uint64_t low_u64() const noexcept { return pbt_big_low_u64(&v_); }
    std::uint64_t mod_u64() const noexcept { return pbt_big_mod_u64(&v_); }
    const pbt_big& raw() const noexcept { return v_; }

private:
    pbt_big v_;
};

enum class Endian { Little = FZ_LITTLE, Big = FZ_BIG };

// ---------------------------------------------------------------------------
// Test seams of the C++ layer (spec VER06k section 7).  NOT part of the API.
// ---------------------------------------------------------------------------
// A conformance run that cannot fail proves nothing.  These seams let a test make
// THIS layer wrong and watch the corpus run notice.  They exist only when the
// translation unit defines CA_FUZZ_TEST_SEAMS; otherwise every hook is the
// identity and there is no global at all.
namespace detail {

#ifdef CA_FUZZ_TEST_SEAMS
struct Seams {
    std::int64_t offset_delta = 0;     // added to the offset every mutator passes on
    bool swap_endian = false;          // multi-byte mutators flip the byte order
    std::uint64_t seed_offset = 0;     // mutate_sequence seeds
    bool invert_dedup = false;         // is_duplicate_crash answers the opposite
    bool leak_tap_target = false;      // format_tap_failure appends the target again
    bool admit_everything = false;     // corpus_admit always reports novelty
    bool accept_any_failure_id = false;  // is_failure_id accepts everything
    void reset() noexcept { *this = Seams(); }
};
inline Seams seams;

inline std::int64_t skew(std::int64_t o) noexcept {
    return static_cast<std::int64_t>(static_cast<std::uint64_t>(o) + static_cast<std::uint64_t>(seams.offset_delta));
}
inline BigInt skew(const BigInt& o) {
    if (seams.offset_delta == 0) return o;
    BigInt d(seams.offset_delta);
    pbt_big r;
    if (pbt_big_add(&r, &o.raw(), &d.raw()) != 0) return o;
    // BigInt has no raw constructor: round-trip through decimal text.
    char buf[PBT_BIG_DEC_BUF];
    if (pbt_big_to_dec(&r, buf, sizeof buf) == 0) return o;
    return BigInt::parse(buf);
}
inline Endian endian(Endian e) noexcept {
    return seams.swap_endian ? (e == Endian::Little ? Endian::Big : Endian::Little) : e;
}
inline std::uint64_t seeded(std::uint64_t s) noexcept { return s + seams.seed_offset; }
inline bool dedup(bool b) noexcept { return seams.invert_dedup ? !b : b; }
inline bool leak_tap() noexcept { return seams.leak_tap_target; }
inline bool admit(bool b) noexcept { return seams.admit_everything ? true : b; }
inline bool any_id() noexcept { return seams.accept_any_failure_id; }
#else
inline std::int64_t skew(std::int64_t o) noexcept { return o; }
inline const BigInt& skew(const BigInt& o) noexcept { return o; }
inline Endian endian(Endian e) noexcept { return e; }
inline std::uint64_t seeded(std::uint64_t s) noexcept { return s; }
inline bool dedup(bool b) noexcept { return b; }
inline bool leak_tap() noexcept { return false; }
inline bool admit(bool b) noexcept { return b; }
inline bool any_id() noexcept { return false; }
#endif

// RAII for an fz_bytes: freed even when copying it into a vector throws bad_alloc.
class OwnedBytes {
public:
    OwnedBytes() noexcept : b_{nullptr, 0} {}
    OwnedBytes(const OwnedBytes&) = delete;
    OwnedBytes& operator=(const OwnedBytes&) = delete;
    ~OwnedBytes() { fz_bytes_free(&b_); }
    fz_bytes* get() noexcept { return &b_; }
    Bytes to_bytes() const { return b_.n == 0 ? Bytes() : Bytes(b_.p, b_.p + b_.n); }

private:
    fz_bytes b_;
};

inline void check(int rc, const char* what) {
    if (rc != FZ_OK) throw Error(rc, std::string(what) + " failed with engine status " + std::to_string(rc));
}

// The common shape of every mutator: call C, check the status, copy out, free.
template <class Fn, class... A>
Bytes mutate(const char* what, Fn fn, ByteView d, A... a) {
    OwnedBytes o;
    check(fn(o.get(), d.data(), d.size(), a...), what);
    return o.to_bytes();
}

inline std::vector<fz_str> to_fz(const Frames& f) {
    std::vector<fz_str> v;
    v.reserve(f.size());
    for (const std::string& s : f) v.push_back(fz_str{s.data(), s.size()});
    return v;
}

}  // namespace detail

// ---------------------------------------------------------------------------
// Mutators (VER06 section 3).  int64 forms and BigInt forms; the BigInt form is
// the one the corpus uses.  Every output is a fresh Bytes.
// ---------------------------------------------------------------------------
inline Bytes bit_flip(ByteView d, std::int64_t bit_offset, std::int64_t count) {
    return detail::mutate("bit_flip", fz_bit_flip, d, detail::skew(bit_offset), count);
}
inline Bytes bit_flip(ByteView d, const BigInt& bit_offset, const BigInt& count) {
    return detail::mutate("bit_flip", fz_bit_flip_big, d, &detail::skew(bit_offset).raw(), &count.raw());
}
inline Bytes byte_flip(ByteView d, std::int64_t byte_offset, std::int64_t count) {
    return detail::mutate("byte_flip", fz_byte_flip, d, detail::skew(byte_offset), count);
}
inline Bytes byte_flip(ByteView d, const BigInt& byte_offset, const BigInt& count) {
    return detail::mutate("byte_flip", fz_byte_flip_big, d, &detail::skew(byte_offset).raw(), &count.raw());
}
inline Bytes byte_arithmetic(ByteView d, std::int64_t off, std::int64_t delta, int width, Endian e) {
    return detail::mutate("byte_arithmetic", fz_byte_arithmetic, d, detail::skew(off), delta, width,
                          static_cast<fz_endian>(detail::endian(e)));
}
inline Bytes byte_arithmetic(ByteView d, const BigInt& off, const BigInt& delta, int width, Endian e) {
    return detail::mutate("byte_arithmetic", fz_byte_arithmetic_big, d, &detail::skew(off).raw(), &delta.raw(), width,
                          static_cast<fz_endian>(detail::endian(e)));
}
inline Bytes boundary_insert(ByteView d, std::int64_t off, int width, std::uint64_t val, Endian e) {
    return detail::mutate("boundary_insert", fz_boundary_insert, d, detail::skew(off), width, val,
                          static_cast<fz_endian>(detail::endian(e)));
}
inline Bytes boundary_insert(ByteView d, const BigInt& off, int width, const BigInt& val, Endian e) {
    return detail::mutate("boundary_insert", fz_boundary_insert_big, d, &detail::skew(off).raw(), width, &val.raw(),
                          static_cast<fz_endian>(detail::endian(e)));
}
inline Bytes block_delete(ByteView d, std::int64_t offset, std::int64_t length) {
    return detail::mutate("block_delete", fz_block_delete, d, detail::skew(offset), length);
}
inline Bytes block_delete(ByteView d, const BigInt& offset, const BigInt& length) {
    return detail::mutate("block_delete", fz_block_delete_big, d, &detail::skew(offset).raw(), &length.raw());
}
inline Bytes block_insert(ByteView d, std::int64_t offset, ByteView ins) {
    return detail::mutate("block_insert", fz_block_insert, d, detail::skew(offset), ins.data(), ins.size());
}
inline Bytes block_insert(ByteView d, const BigInt& offset, ByteView ins) {
    return detail::mutate("block_insert", fz_block_insert_big, d, &detail::skew(offset).raw(), ins.data(), ins.size());
}
inline Bytes block_duplicate(ByteView d, std::int64_t src, std::int64_t length, std::int64_t dst) {
    return detail::mutate("block_duplicate", fz_block_duplicate, d, detail::skew(src), length, dst);
}
inline Bytes block_duplicate(ByteView d, const BigInt& src, const BigInt& length, const BigInt& dst) {
    return detail::mutate("block_duplicate", fz_block_duplicate_big, d, &detail::skew(src).raw(), &length.raw(),
                          &dst.raw());
}
inline Bytes splice(ByteView target, ByteView donor, std::int64_t target_split, std::int64_t donor_split) {
    return detail::mutate("splice", fz_splice, target, donor.data(), donor.size(), detail::skew(target_split),
                          donor_split);
}
inline Bytes splice(ByteView target, ByteView donor, const BigInt& target_split, const BigInt& donor_split) {
    return detail::mutate("splice", fz_splice_big, target, donor.data(), donor.size(),
                          &detail::skew(target_split).raw(), &donor_split.raw());
}

// `steps` seeded mutations (the draw order is the cross-lane contract).
inline Bytes mutate_sequence(ByteView d, std::uint64_t seed, std::size_t steps, std::size_t max_len) {
    return detail::mutate("mutate_sequence", fz_mutate_sequence, d, detail::seeded(seed), steps, max_len);
}

// The same, charging bytes copied against work_cap.  work_done is reported on EVERY
// return, FZ_E_WORK included, so this call reports through the value, not an exception.
struct SequenceResult {
    int status = FZ_OK;
    Bytes bytes;
    std::uint64_t work_done = 0;
};
inline SequenceResult mutate_sequence_budget(ByteView d, std::uint64_t seed, std::size_t steps, std::size_t max_len,
                                             std::uint64_t work_cap) {
    SequenceResult r;
    detail::OwnedBytes o;
    r.status = fz_mutate_sequence_budget(o.get(), d.data(), d.size(), detail::seeded(seed), steps, max_len, work_cap,
                                         &r.work_done);
    if (r.status == FZ_OK) r.bytes = o.to_bytes();
    return r;
}

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------
inline std::vector<std::uint64_t> interest_values(int width) {
    std::uint64_t out[7] = {0, 0, 0, 0, 0, 0, 0};
    std::size_t n = fz_interest_values(width, out);
    return std::vector<std::uint64_t>(out, out + n);
}
inline std::vector<std::uint8_t> interest8() { return std::vector<std::uint8_t>(fz_interest8, fz_interest8 + 5); }
inline std::vector<int> deltas8() { return std::vector<int>(fz_deltas8, fz_deltas8 + 6); }

inline std::size_t calculate_energy(std::size_t base, std::size_t seed_len, std::size_t new_edges,
                                    std::size_t total_edges) {
    return fz_calculate_energy(base, seed_len, new_edges, total_edges);
}
// FNV-1a of the label, up to its first NUL.
inline std::uint64_t edge_id(std::string_view label) {
    std::string s(label);
    return fz_edge_id(s.c_str());
}

struct Novelty {
    bool admitted = false;
    std::size_t new_edges = 0;
};
inline Novelty corpus_admit(const std::vector<std::uint64_t>& existing, const std::vector<std::uint64_t>& trace) {
    int admitted = 0;
    std::size_t fresh = 0;
    detail::check(fz_corpus_admit(existing.data(), existing.size(), trace.data(), trace.size(), &admitted, &fresh),
                  "corpus_admit");
    Novelty n;
    n.admitted = detail::admit(admitted != 0);
    n.new_edges = fresh;
    return n;
}

// lower32(fnv1a64(kind ":" frames joined by ":")) as 8 lowercase hex digits.
inline std::string crash_signature(std::string_view kind, const Frames& frames) {
    std::vector<fz_str> f = detail::to_fz(frames);
    char out[9];
    fz_crash_signature_n(out, fz_str{kind.data(), kind.size()}, f.data(), f.size());
    return std::string(out, 8);
}
inline bool is_duplicate_crash(std::string_view kind_a, const Frames& frames_a, std::string_view kind_b,
                               const Frames& frames_b) {
    std::vector<fz_str> fa = detail::to_fz(frames_a);
    std::vector<fz_str> fb = detail::to_fz(frames_b);
    int dup = fz_is_duplicate_crash(fz_str{kind_a.data(), kind_a.size()}, fa.data(), fa.size(),
                                    fz_str{kind_b.data(), kind_b.size()}, fb.data(), fb.size());
    return detail::dedup(dup != 0);
}

inline bool is_failure_id(std::string_view id) {
    return detail::any_id() || fz_is_failure_id(id.data(), id.size()) != 0;
}

// `not ok N - <target> [<failure-id>:<hex8>]`.  Throws Error(FZ_E_ARG) for an unregistered id or a
// signature that is not 8 lowercase hex digits.
inline std::string format_tap_failure(int test_num, std::string_view target, std::string_view failure_id,
                                      std::string_view sig_hash) {
    char buf[512];
    detail::check(fz_format_tap_failure(buf, sizeof buf, test_num, target.data(), target.size(), failure_id.data(),
                                        failure_id.size(), sig_hash.data(), sig_hash.size()),
                  "format_tap_failure");
    std::string line(buf);
    if (detail::leak_tap()) line += std::string(target);
    return line;
}

// Does the strict TFV reader accept this text?
inline bool tfv_valid(std::string_view text) {
    char err[200];
    return pbt_tfv_validate(text.data(), text.size(), err, sizeof err) == 0;
}

// ---------------------------------------------------------------------------
// Handle: what a target sees
// ---------------------------------------------------------------------------
class Handle;
namespace detail {
struct Access;
}

// A window onto ONE execution.  The C engine issues a handle per execution from a static
// ring and retires it when the target returns; this wrapper is move-only, remembers the
// handle's epoch and the creating thread, and is INERT when moved from, retired, reissued
// or used off-thread.  tick() / reserve() return true when the budget is EXHAUSTED.
class Handle {
public:
    Handle() noexcept = default;
    Handle(const Handle&) = delete;
    Handle& operator=(const Handle&) = delete;
    Handle(Handle&& o) noexcept : h_(o.h_), epoch_(o.epoch_), owner_(o.owner_) { o.h_ = nullptr; }
    Handle& operator=(Handle&& o) noexcept {
        if (this != &o) {
            h_ = o.h_;
            epoch_ = o.epoch_;
            owner_ = o.owner_;
            o.h_ = nullptr;
        }
        return *this;
    }

    // Still the live handle of the execution it was made for, on its own thread.
    bool alive() const noexcept {
        return h_ != nullptr && owner_ == std::this_thread::get_id() && fz_handle_user(h_) != nullptr &&
               fz_handle_epoch(h_) == epoch_;
    }
    // The target executed program point `id` (reduced modulo 65,536).
    void edge(std::uint64_t id) noexcept {
        if (alive()) fz_edge(h_, id);
    }
    void edge(std::string_view label) { edge(edge_id(label)); }
    // One unit of work; true when the step budget is exhausted (sticky, shared with reserve).
    bool tick() noexcept { return !alive() || fz_tick(h_) != 0; }
    // An allocation of n bytes is about to be made; true when the memory budget is exhausted.
    bool reserve(std::size_t n) noexcept { return !alive() || fz_reserve(h_, n) != 0; }
    // The raw C handle (tests); never dereference it yourself.
    const fz_handle* native() const noexcept { return h_; }

private:
    friend struct detail::Access;
    explicit Handle(fz_handle* h) noexcept
        : h_(h), epoch_(fz_handle_epoch(h)), owner_(std::this_thread::get_id()) {}
    fz_handle* h_ = nullptr;
    std::uint64_t epoch_ = 0;
    std::thread::id owner_;
};

namespace detail {
struct Access {
    static Handle make(fz_handle* h) noexcept { return Handle(h); }
};
}  // namespace detail

// ---------------------------------------------------------------------------
// Classification of a thrown exception (used inside catch blocks only).
// ---------------------------------------------------------------------------
namespace detail {

enum class Caught { Crash, Oom };

// Fill `c` for the exception in flight.  noexcept and allocation free: it only copies
// into the engine-owned fz_crash.  NEVER reads what().
inline Caught classify_current(fz_crash* c) noexcept {
    try {
        throw;
    } catch (const Crash& e) {
        fz_crash_set(c, e.kind().c_str());
        for (const std::string& f : e.frames()) fz_crash_push(c, f.c_str());
        return Caught::Crash;
    } catch (const std::bad_alloc&) {
        return Caught::Oom;
    } catch (const std::exception& e) {
        fz_crash_set(c, typeid(e).name());
        return Caught::Crash;
    } catch (...) {
        fz_crash_set(c, "non-std-exception");
        return Caught::Crash;
    }
}

}  // namespace detail

// ---------------------------------------------------------------------------
// Campaign
// ---------------------------------------------------------------------------
struct Options {
    std::uint64_t seed;
    std::size_t max_iterations;
    std::size_t max_input_len;
    std::size_t base_energy;
    std::size_t minimize_budget;
    std::uint64_t step_budget;
    std::uint64_t mem_budget;

    // The defaults are the engine's (fz_options_default), not retyped here.
    Options() {
        fz_options d = fz_options_default();
        seed = d.seed;
        max_iterations = d.max_iterations;
        max_input_len = d.max_input_len;
        base_energy = d.base_energy;
        minimize_budget = d.minimize_budget;
        step_budget = d.step_budget;
        mem_budget = d.mem_budget;
    }
    fz_options to_c() const {
        fz_options o = fz_options_default();
        o.seed = seed;
        o.max_iterations = max_iterations;
        o.max_input_len = max_input_len;
        o.base_energy = base_energy;
        o.minimize_budget = minimize_budget;
        o.step_budget = step_budget;
        o.mem_budget = mem_budget;
        return o;
    }
};

// The code under test.  Returning normally is a pass; throwing is classified (see the header).
using Target = std::function<void(ByteView, Handle&)>;

// One recorded crash signature.  A copy: it stays valid after the Report is gone.
class CrashRecord {
public:
    const std::string& signature() const noexcept { return sig_; }
    const std::string& kind() const noexcept { return kind_; }
    const Frames& frames() const noexcept { return frames_; }
    std::size_t hits() const noexcept { return hits_; }
    bool minimized() const noexcept { return minimized_; }
    bool input_dropped() const noexcept { return dropped_; }
    // PRIVATE: the minimal crashing input.  Never log it.  Empty when the engine kept none.
    const std::optional<Bytes>& minimal_input() const noexcept { return minimal_; }

private:
    friend class Report;
    std::string sig_;
    std::string kind_;
    Frames frames_;
    std::size_t hits_ = 0;
    bool minimized_ = false;
    bool dropped_ = false;
    std::optional<Bytes> minimal_;
};

// Owns one fz_report.  Move-only; a moved-from Report is the empty report.
class Report {
public:
    Report() noexcept { fz_report_init(&r_); }
    Report(const Report&) = delete;
    Report& operator=(const Report&) = delete;
    Report(Report&& o) noexcept : r_(o.r_) { fz_report_init(&o.r_); }
    Report& operator=(Report&& o) noexcept {
        if (this != &o) {
            fz_report_free(&r_);
            r_ = o.r_;
            fz_report_init(&o.r_);
        }
        return *this;
    }
    ~Report() { fz_report_free(&r_); }

    std::size_t iterations() const noexcept { return r_.iterations; }
    std::size_t admitted_seeds() const noexcept { return r_.admitted_seeds; }
    std::size_t total_edges() const noexcept { return r_.total_edges; }
    std::size_t timeouts() const noexcept { return r_.timeouts; }
    std::size_t ooms() const noexcept { return r_.ooms; }
    std::size_t crashes_dropped() const noexcept { return r_.crashes_dropped; }
    std::size_t admissions_dropped() const noexcept { return r_.admissions_dropped; }
    std::string_view tier() const noexcept { return std::string_view(r_.tier); }
    bool is_clean() const noexcept { return fz_report_is_clean(&r_) != 0; }
    std::size_t unique_crashes() const noexcept { return fz_report_unique_crashes(&r_); }

    // Counts, tier and cleanliness only: never an input byte or a seed.
    std::string summary() const {
        std::size_t need = fz_report_summary(&r_, nullptr, 0);
        std::string s(need + 1, '\0');
        fz_report_summary(&r_, &s[0], s.size());
        s.resize(need);
        return s;
    }
    // One TAP line per crash signature, then timeouts, then ooms (the engine's text).
    std::vector<std::string> tap_failures(std::string_view target, int first_test = 1) const {
        std::string t(target);  // the C side measures with strlen: cut at the first NUL
        std::vector<std::string> lines;
        std::size_t n = fz_tap_failure_count(&r_);
        for (std::size_t i = 0; i < n; ++i) {
            char buf[512];
            detail::check(fz_tap_failure_line(&r_, i, t.c_str(), first_test, buf, sizeof buf), "tap_failure_line");
            lines.emplace_back(buf);
        }
        return lines;
    }
    std::vector<CrashRecord> crashes() const {
        std::vector<CrashRecord> v;
        for (std::size_t i = 0; i < r_.ncrashes; ++i) {
            const fz_crash_record& c = r_.crashes[i];
            CrashRecord rec;
            rec.sig_ = std::string(c.sig, 8);
            rec.kind_ = c.crash.kind;
            for (std::size_t k = 0; k < c.crash.nframes; ++k) rec.frames_.emplace_back(c.crash.frames[k]);
            rec.hits_ = c.hits;
            rec.minimized_ = c.minimized != 0;
            rec.dropped_ = c.input_dropped != 0;
            if (c.minimal != nullptr) rec.minimal_ = Bytes(c.minimal, c.minimal + c.minimal_len);
            v.push_back(std::move(rec));
        }
        return v;
    }
    // PRIVATE: the retained corpus.  Never log it.
    std::vector<Bytes> corpus() const {
        std::vector<Bytes> v;
        for (std::size_t i = 0; i < r_.ncorpus; ++i) v.emplace_back(r_.corpus[i].p, r_.corpus[i].p + r_.corpus[i].n);
        return v;
    }
    fz_report* mutable_c() noexcept { return &r_; }

private:
    fz_report r_;
};

namespace detail {

// What the trampoline needs: the user's target.  It rides in fz_options.user.
struct CampaignCtx {
    const Target* target;
};

// noexcept: nothing the target throws may cross this C frame (UB).
inline int target_trampoline(const std::uint8_t* d, std::size_t n, fz_handle* h, fz_crash* out) noexcept {
    CampaignCtx* ctx = static_cast<CampaignCtx*>(fz_handle_user(h));
    if (ctx == nullptr) {
        fz_crash_set(out, "binding-misuse");
        return 1;
    }
    try {
        Handle handle = Access::make(h);
        (*ctx->target)(ByteView(d, n), handle);
        return 0;
    } catch (...) {  // also swallows foreign / forced-unwind exceptions (e.g. glibc abi::__forced_unwind from
                     // pthread_cancel): cancelling a target thread is out of scope (cooperative trust model)
        if (classify_current(out) == Caught::Oom) (void)fz_reserve(h, SIZE_MAX);  // sticky: the engine says OOM
        return 1;
    }
}

}  // namespace detail

// Run a bounded coverage-guided campaign.  Throws Error for an invalid option (FZ_E_ARG) or an
// engine allocation failure (FZ_E_NOMEM); findings are in the Report.
inline Report run(const std::vector<Bytes>& seeds, const Options& options, Target target) {
    static const std::uint8_t kEmpty = 0;
    std::vector<fz_input> in;
    in.reserve(seeds.size());
    for (const Bytes& s : seeds) in.push_back(fz_input{s.empty() ? &kEmpty : s.data(), s.size()});
    detail::CampaignCtx ctx{&target};
    fz_options o = options.to_c();
    o.user = &ctx;
    Report rep;
    detail::check(fz_run(in.data(), in.size(), &o, detail::target_trampoline, rep.mutable_c()), "run");
    return rep;
}

// ---------------------------------------------------------------------------
// Minimisation
// ---------------------------------------------------------------------------
struct Outcome {
    enum class Kind { Pass, Crash, Timeout, Oom };
    Kind kind = Kind::Pass;
    std::string crash_kind;
    Frames frames;
    static Outcome pass() { return Outcome(); }
    static Outcome crash(std::string kind, Frames frames = {}) {
        Outcome o;
        o.kind = Kind::Crash;
        o.crash_kind = std::move(kind);
        o.frames = std::move(frames);
        return o;
    }
    static Outcome timeout() {
        Outcome o;
        o.kind = Kind::Timeout;
        return o;
    }
    static Outcome oom() {
        Outcome o;
        o.kind = Kind::Oom;
        return o;
    }
};

// Run one candidate.  A throw is classified like a target's (bad_alloc is the oom outcome).
using MinimizeTest = std::function<Outcome(ByteView)>;

namespace detail {
struct MinCtx {
    const MinimizeTest* test;
};
inline int test_trampoline(void* user, const std::uint8_t* d, std::size_t n, fz_outcome* out) noexcept {
    const MinCtx* ctx = static_cast<const MinCtx*>(user);
    std::memset(&out->crash, 0, sizeof out->crash);
    out->kind = FZ_PASS;
    try {
        Outcome o = (*ctx->test)(ByteView(d, n));
        switch (o.kind) {
            case Outcome::Kind::Pass: out->kind = FZ_PASS; break;
            case Outcome::Kind::Timeout: out->kind = FZ_TIMEOUT; break;
            case Outcome::Kind::Oom: out->kind = FZ_OOM; break;
            case Outcome::Kind::Crash:
                out->kind = FZ_CRASH;
                fz_crash_set(&out->crash, o.crash_kind.c_str());
                for (const std::string& f : o.frames) fz_crash_push(&out->crash, f.c_str());
                break;
        }
    } catch (...) {  // also swallows foreign / forced-unwind exceptions (e.g. glibc abi::__forced_unwind from
                     // pthread_cancel): cancelling a target thread is out of scope (cooperative trust model)
        out->kind = classify_current(&out->crash) == Caught::Oom ? FZ_OOM : FZ_CRASH;
    }
    return FZ_OK;
}
}  // namespace detail

// Shrink `data` while keeping the EXACT crash signature `sig` (8 lowercase hex digits).
inline Bytes minimize(ByteView data, std::string_view sig, const MinimizeTest& test, std::size_t budget) {
    if (sig.size() != 8) throw Error(FZ_E_ARG, "a signature is exactly 8 hex digits");
    std::string s(sig);
    detail::MinCtx ctx{&test};
    detail::OwnedBytes out;
    detail::check(fz_minimize_crashing_input(out.get(), data.data(), data.size(), s.c_str(), detail::test_trampoline,
                                             &ctx, budget),
                  "minimize");
    return out.to_bytes();
}

}  // namespace ca::fuzz

#endif  // CA_FUZZ_BOOTSTRAP_HPP
