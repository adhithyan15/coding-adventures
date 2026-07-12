// zeroize.hpp — secure in-memory wiping for secrets, header-only in pure ISO
// C++17 (namespace ca::zeroize). A faithful port of the Rust `zeroize` crate.
// ===========================================================================
//
// A compiler is allowed to DELETE a "clear the secret" store when it proves no
// later read observes the zero — leaving the secret in RAM (exposed to swap,
// core dumps, use-after-free elsewhere). To make the clear observably happen,
// each store must be VOLATILE; a trailing compiler fence keeps the stores from
// being reordered past the buffer's end of life.
//
// This port mirrors the Rust construction exactly:
//   * volatile byte stores (the compiler may not elide observable accesses)
//   * std::atomic_signal_fence(seq_cst) — the C++ equivalent of Rust's
//     compiler_fence(SeqCst): a compile-only optimizer barrier, no CPU
//     instruction, no cross-core semantics.
//
// The `Zeroize` trait becomes overloaded `zeroize(T&)` functions (found by
// argument-dependent lookup, so user types can add their own), and `Zeroizing`
// is an RAII wrapper whose destructor wipes the inner value — the C++ analogue
// of Rust's `Drop`, so every exit path (including an exception unwind) scrubs
// the secret.
//
// PORTABILITY. Pure ISO C++17 — standard library only, no compiler extensions.
#ifndef CA_ZEROIZE_HPP
#define CA_ZEROIZE_HPP

#include <array>
#include <atomic>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

namespace ca {
namespace zeroize {

inline constexpr const char* VERSION = "0.1.0";

// The primitive: overwrite `len` bytes at `ptr` with 0 via volatile stores,
// then a compiler-only fence. NULL is allowed only when len == 0.
inline void zeroize_bytes(void* ptr, std::size_t len) {
    if (len == 0) return;
    volatile unsigned char* p = static_cast<volatile unsigned char*>(ptr);
    for (std::size_t i = 0; i < len; i++) p[i] = 0u;
    std::atomic_signal_fence(std::memory_order_seq_cst);
}

// ── zeroize(T&) overloads (extend by adding your own, found via ADL) ─────────

// Any fixed-width integer (and bool / char): a single volatile-zero store.
template <class T,
          std::enable_if_t<std::is_integral_v<T>, int> = 0>
inline void zeroize(T& value) {
    zeroize_bytes(&value, sizeof(T));
}

// A fixed-size byte array.
template <std::size_t N>
inline void zeroize(std::array<std::uint8_t, N>& arr) {
    zeroize_bytes(arr.data(), N);
}

// A byte vector — scrub the live bytes, then clear the length.
//
// DIVERGENCE FROM RUST. The Rust impl scrubs the whole allocated *capacity*
// (reaching stale secret bytes in the unused tail) via raw pointers. In C++,
// the bytes between size() and capacity() are not live objects, and touching
// them is undefined (and flagged by sanitizers' container-overflow checks), so
// we scrub only the live size() bytes. If you need capacity scrubbing, hold the
// secret in a fixed-size std::array or use the C `ZrBytes` buffer.
inline void zeroize(std::vector<std::uint8_t>& vec) {
    if (!vec.empty()) zeroize_bytes(vec.data(), vec.size());
    vec.clear();
}

// A string — same reasoning as the byte vector (scrub the live length).
inline void zeroize(std::string& s) {
    if (!s.empty()) zeroize_bytes(&s[0], s.size());
    s.clear();
}

// An optional — wipe the payload (through its own zeroize) if present, then
// reset to std::nullopt.
template <class T>
inline void zeroize(std::optional<T>& opt) {
    if (opt.has_value()) {
        using ca::zeroize::zeroize;
        zeroize(*opt);
    }
    opt.reset();
}

// ── Zeroizing<T> — an owning wrapper whose destructor wipes the value ────────

template <class T>
class Zeroizing {
public:
    explicit Zeroizing(T value) : inner_(std::move(value)) {}

    Zeroizing(const Zeroizing&) = delete;             // don't duplicate secrets
    Zeroizing& operator=(const Zeroizing&) = delete;
    Zeroizing(Zeroizing&& other) noexcept
        : inner_(std::move(other.inner_)), active_(other.active_) {
        other.active_ = false;
    }
    Zeroizing& operator=(Zeroizing&& other) noexcept {
        if (this != &other) {
            wipe();
            inner_ = std::move(other.inner_);
            active_ = other.active_;
            other.active_ = false;
        }
        return *this;
    }

    ~Zeroizing() { wipe(); }

    // Deref access to the protected value.
    T& operator*() { return inner_; }
    const T& operator*() const { return inner_; }
    T* operator->() { return &inner_; }
    const T* operator->() const { return &inner_; }

    // Move the inner value out WITHOUT wiping (the caller opts out and takes
    // over the wipe-on-drop responsibility).
    T into_inner() {
        active_ = false;
        return std::move(inner_);
    }

private:
    void wipe() {
        if (active_) {
            using ca::zeroize::zeroize;
            zeroize(inner_);
            active_ = false;
        }
    }

    T inner_;
    bool active_ = true;
};

}  // namespace zeroize
}  // namespace ca

#endif  // CA_ZEROIZE_HPP
