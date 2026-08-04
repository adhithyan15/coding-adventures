// hash_map.hpp — a hash map built from scratch, in pure ISO C++17 (header-only).
// A faithful port of the Rust `hash-map` crate (DT18).
// ===========================================================================
//
// A generic `ca::hash_map<K, V>` with the two classic collision-resolution
// strategies and four selectable hash functions, matching the Rust crate:
//
//   • Chaining        — each bucket is a list of entries; resizes above load 1.0
//   • Open addressing — one slot array with linear probing and tombstones;
//                       resizes above load 0.75
//   • Hashes          — SipHash-2-4 (default), FNV-1a-32, MurmurHash3-32, djb2
//
// Keys are hashed by serialising them to bytes: `std::string` keys hash their
// characters, other (trivially-copyable) keys hash their object representation.
// This mirrors Rust's "serialise the key, then hash" approach; the map is
// self-consistent, so the exact bytes hashed are an implementation detail.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. No extensions, no libraries beyond the
// standard library.
#ifndef HASH_MAP_HPP
#define HASH_MAP_HPP

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <optional>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>

namespace ca {

enum class collision_strategy { chaining, open_addressing };
enum class hash_algorithm { siphash24, fnv1a32, murmur3_32, djb2 };

namespace detail {

// Serialise a key to bytes: std::string → its characters; any other
// trivially-copyable key → its object representation.
template <class K>
inline std::vector<std::uint8_t> key_bytes(const K &k) {
    if constexpr (std::is_same_v<std::decay_t<K>, std::string>) {
        return std::vector<std::uint8_t>(k.begin(), k.end());
    } else {
        static_assert(std::is_trivially_copyable_v<K>,
                      "hash_map keys must be std::string or trivially copyable");
        std::vector<std::uint8_t> b(sizeof(K));
        std::memcpy(b.data(), &k, sizeof(K));
        return b;
    }
}

inline std::uint32_t load32le(const std::uint8_t *p) {
    return static_cast<std::uint32_t>(p[0]) |
           (static_cast<std::uint32_t>(p[1]) << 8) |
           (static_cast<std::uint32_t>(p[2]) << 16) |
           (static_cast<std::uint32_t>(p[3]) << 24);
}
inline std::uint64_t load64le(const std::uint8_t *p) {
    return static_cast<std::uint64_t>(p[0]) |
           (static_cast<std::uint64_t>(p[1]) << 8) |
           (static_cast<std::uint64_t>(p[2]) << 16) |
           (static_cast<std::uint64_t>(p[3]) << 24) |
           (static_cast<std::uint64_t>(p[4]) << 32) |
           (static_cast<std::uint64_t>(p[5]) << 40) |
           (static_cast<std::uint64_t>(p[6]) << 48) |
           (static_cast<std::uint64_t>(p[7]) << 56);
}
inline std::uint32_t rotl32(std::uint32_t x, unsigned n) {
    return (x << n) | (x >> (32 - n));
}
inline std::uint64_t rotl64(std::uint64_t x, unsigned n) {
    return (x << n) | (x >> (64 - n));
}
inline std::uint32_t fmix32(std::uint32_t h) {
    h ^= h >> 16;
    h *= 0x85ebca6bu;
    h ^= h >> 13;
    h *= 0xc2b2ae35u;
    h ^= h >> 16;
    return h;
}
inline std::uint64_t fnv1a_32(const std::uint8_t *d, std::size_t n) {
    std::uint32_t h = 0x811c9dc5u;
    for (std::size_t i = 0; i < n; i++) {
        h ^= d[i];
        h *= 0x01000193u;
    }
    return h;
}
inline std::uint64_t djb2(const std::uint8_t *d, std::size_t n) {
    std::uint64_t h = 5381u;
    for (std::size_t i = 0; i < n; i++) {
        h = (h << 5) + h + d[i];
    }
    return h;
}
inline std::uint64_t murmur3_32(const std::uint8_t *d, std::size_t n) {
    std::uint32_t hash = 0;
    std::size_t blocks = n / 4;
    for (std::size_t i = 0; i < blocks; i++) {
        std::uint32_t k = load32le(d + i * 4);
        k *= 0xcc9e2d51u;
        k = rotl32(k, 15);
        k *= 0x1b873593u;
        hash ^= k;
        hash = rotl32(hash, 13);
        hash = hash * 5u + 0xe6546b64u;
    }
    std::uint32_t k = 0;
    std::size_t rem = n & 3u;
    std::size_t base = blocks * 4;
    for (std::size_t j = 0; j < rem; j++) {
        k ^= static_cast<std::uint32_t>(d[base + j]) << (j * 8);
    }
    if (rem != 0) {
        k *= 0xcc9e2d51u;
        k = rotl32(k, 15);
        k *= 0x1b873593u;
        hash ^= k;
    }
    hash ^= static_cast<std::uint32_t>(n);
    return fmix32(hash);
}
inline void sipround(std::uint64_t &v0, std::uint64_t &v1, std::uint64_t &v2,
                     std::uint64_t &v3) {
    v0 += v1;
    v1 = rotl64(v1, 13);
    v1 ^= v0;
    v0 = rotl64(v0, 32);
    v2 += v3;
    v3 = rotl64(v3, 16);
    v3 ^= v2;
    v0 += v3;
    v3 = rotl64(v3, 21);
    v3 ^= v0;
    v2 += v1;
    v1 = rotl64(v1, 17);
    v1 ^= v2;
    v2 = rotl64(v2, 32);
}
inline std::uint64_t siphash24(const std::uint8_t *d, std::size_t n) {
    static const std::uint8_t key[16] = {'c', 'o', 'd', 'e', 'x', '-', 'd', 't',
                                         '1', '8', '-', 'k', 'e', 'y', '!', '!'};
    std::uint64_t k0 = load64le(key);
    std::uint64_t k1 = load64le(key + 8);
    std::uint64_t v0 = 0x736f6d6570736575u ^ k0;
    std::uint64_t v1 = 0x646f72616e646f6du ^ k1;
    std::uint64_t v2 = 0x6c7967656e657261u ^ k0;
    std::uint64_t v3 = 0x7465646279746573u ^ k1;
    std::size_t blocks = n / 8;
    for (std::size_t i = 0; i < blocks; i++) {
        std::uint64_t m = load64le(d + i * 8);
        v3 ^= m;
        sipround(v0, v1, v2, v3);
        sipround(v0, v1, v2, v3);
        v0 ^= m;
    }
    std::uint64_t last = (static_cast<std::uint64_t>(n) & 0xffu) << 56;
    std::size_t rem = n & 7u;
    std::size_t base = blocks * 8;
    for (std::size_t j = 0; j < rem; j++) {
        last |= static_cast<std::uint64_t>(d[base + j]) << (j * 8);
    }
    v3 ^= last;
    sipround(v0, v1, v2, v3);
    sipround(v0, v1, v2, v3);
    v0 ^= last;
    v2 ^= 0xffu;
    sipround(v0, v1, v2, v3);
    sipround(v0, v1, v2, v3);
    sipround(v0, v1, v2, v3);
    sipround(v0, v1, v2, v3);
    return v0 ^ v1 ^ v2 ^ v3;
}

} // namespace detail

template <class K, class V>
class hash_map {
public:
    struct entry {
        K key;
        V value;
    };

    explicit hash_map(std::size_t capacity = 16,
                      collision_strategy strategy = collision_strategy::chaining,
                      hash_algorithm hash = hash_algorithm::siphash24)
        : strategy_(strategy), hash_(hash), size_(0),
          capacity_(capacity < 1 ? 1 : capacity) {
        if (strategy_ == collision_strategy::chaining) {
            buckets_.resize(capacity_);
        } else {
            slots_.resize(capacity_);
        }
    }

    // Insert or overwrite the value for `key`.
    void set(const K &key, const V &value) {
        insert_no_resize(key, value);
        maybe_resize();
    }

    // Look up `key`; returns the value by copy, or std::nullopt if absent.
    std::optional<V> get(const K &key) const {
        std::size_t idx = bucket_index(key);
        if (strategy_ == collision_strategy::chaining) {
            for (const entry &e : buckets_[idx]) {
                if (e.key == key) {
                    return e.value;
                }
            }
            return std::nullopt;
        }
        for (std::size_t probe = 0; probe < capacity_; probe++) {
            std::size_t i = (idx + probe) % capacity_;
            const slot &s = slots_[i];
            if (s.state == slot_empty) {
                return std::nullopt;
            }
            if (s.state == slot_occupied && s.data->key == key) {
                return s.data->value;
            }
        }
        return std::nullopt;
    }

    bool has(const K &key) const { return get(key).has_value(); }

    // Remove `key`; returns true if it was present.
    bool remove(const K &key) {
        std::size_t idx = bucket_index(key);
        if (strategy_ == collision_strategy::chaining) {
            std::vector<entry> &bucket = buckets_[idx];
            for (std::size_t i = 0; i < bucket.size(); i++) {
                if (bucket[i].key == key) {
                    bucket.erase(bucket.begin() +
                                 static_cast<std::ptrdiff_t>(i));
                    size_--;
                    return true;
                }
            }
            return false;
        }
        for (std::size_t probe = 0; probe < capacity_; probe++) {
            std::size_t i = (idx + probe) % capacity_;
            slot &s = slots_[i];
            if (s.state == slot_empty) {
                return false;
            }
            if (s.state == slot_occupied && s.data->key == key) {
                s.data.reset();
                s.state = slot_tombstone;
                size_--;
                return true;
            }
        }
        return false;
    }

    std::size_t size() const { return size_; }
    std::size_t capacity() const { return capacity_; }
    bool empty() const { return size_ == 0; }
    double load_factor() const {
        return static_cast<double>(size_) / static_cast<double>(capacity_);
    }
    collision_strategy strategy() const { return strategy_; }
    hash_algorithm hash_function() const { return hash_; }

    // All key/value pairs, in unspecified order.
    std::vector<entry> entries() const {
        std::vector<entry> out;
        out.reserve(size_);
        if (strategy_ == collision_strategy::chaining) {
            for (const std::vector<entry> &bucket : buckets_) {
                for (const entry &e : bucket) {
                    out.push_back(e);
                }
            }
        } else {
            for (const slot &s : slots_) {
                if (s.state == slot_occupied) {
                    out.push_back(*s.data);
                }
            }
        }
        return out;
    }
    std::vector<K> keys() const {
        std::vector<K> out;
        out.reserve(size_);
        for (const entry &e : entries()) {
            out.push_back(e.key);
        }
        return out;
    }

private:
    enum : int { slot_empty = 0, slot_tombstone = 1, slot_occupied = 2 };
    struct slot {
        int state = slot_empty;
        std::optional<entry> data;
    };

    static constexpr double chaining_threshold = 1.0;
    static constexpr double open_threshold = 0.75;

    std::uint64_t hash_bytes(const std::vector<std::uint8_t> &b) const {
        switch (hash_) {
        case hash_algorithm::siphash24:
            return detail::siphash24(b.data(), b.size());
        case hash_algorithm::fnv1a32:
            return detail::fnv1a_32(b.data(), b.size());
        case hash_algorithm::murmur3_32:
            return detail::murmur3_32(b.data(), b.size());
        case hash_algorithm::djb2:
            return detail::djb2(b.data(), b.size());
        }
        return 0; // unreachable
    }
    std::size_t bucket_index(const K &key) const {
        return static_cast<std::size_t>(hash_bytes(detail::key_bytes(key)) %
                                        static_cast<std::uint64_t>(capacity_));
    }

    void insert_no_resize(const K &key, const V &value) {
        std::size_t idx = bucket_index(key);
        if (strategy_ == collision_strategy::chaining) {
            for (entry &e : buckets_[idx]) {
                if (e.key == key) {
                    e.value = value;
                    return;
                }
            }
            buckets_[idx].push_back(entry{key, value});
            size_++;
            return;
        }
        std::size_t first_tomb = capacity_; // sentinel: "none"
        for (std::size_t probe = 0; probe < capacity_; probe++) {
            std::size_t i = (idx + probe) % capacity_;
            slot &s = slots_[i];
            if (s.state == slot_empty) {
                std::size_t at = (first_tomb != capacity_) ? first_tomb : i;
                slots_[at].data = entry{key, value};
                slots_[at].state = slot_occupied;
                size_++;
                return;
            }
            if (s.state == slot_tombstone) {
                if (first_tomb == capacity_) {
                    first_tomb = i;
                }
            } else if (s.data->key == key) {
                s.data->value = value;
                return;
            }
        }
        if (first_tomb != capacity_) {
            slots_[first_tomb].data = entry{key, value};
            slots_[first_tomb].state = slot_occupied;
            size_++;
        }
    }

    bool needs_resize() const {
        double load = load_factor();
        return strategy_ == collision_strategy::chaining
                   ? load > chaining_threshold
                   : load > open_threshold;
    }

    void maybe_resize() {
        if (!needs_resize()) {
            return;
        }
        std::vector<entry> all = entries();
        std::size_t new_cap = capacity_ * 2;
        capacity_ = new_cap;
        size_ = 0;
        if (strategy_ == collision_strategy::chaining) {
            buckets_.clear();
            buckets_.resize(new_cap);
        } else {
            slots_.clear();
            slots_.resize(new_cap);
        }
        for (entry &e : all) {
            insert_no_resize(e.key, e.value);
        }
    }

    collision_strategy strategy_;
    hash_algorithm hash_;
    std::size_t size_;
    std::size_t capacity_;
    std::vector<std::vector<entry>> buckets_; // chaining
    std::vector<slot> slots_;                 // open addressing
};

} // namespace ca

#endif // HASH_MAP_HPP
