// cpu_cache.hpp — Configurable CPU cache hierarchy simulator, header-only C++17.
// ============================================================================
//
// A faithful port of the Rust `cpu-cache` crate, in namespace `ca::cpu_cache`:
// a multi-level cache hierarchy (L1I / L1D / L2 / L3 / main memory) like those
// in modern CPUs. The same `Cache` serves as any level — only its configuration
// (size, associativity, latency) differs.
//
// Address decomposition is pure bit-slicing (sizes are powers of two):
//   offset     = address & (line_size - 1)
//   set_index  = (address >> offset_bits) & (num_sets - 1)
//   tag        = address >> (offset_bits + set_bits)
// `offset_bits` / `set_bits` are computed as an exact integer log2, so this
// header needs no <cmath>.
//
// Each set is N-way set-associative with true LRU. Where the Rust
// `CacheConfig::new` panics on an invalid configuration, this port throws
// `std::invalid_argument`. Pure ISO C++17.

#ifndef CPU_CACHE_HPP
#define CPU_CACHE_HPP

#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace cpu_cache {

// Write policy: defer writes (write-back) or propagate immediately
// (write-through).
enum class WritePolicy { WriteBack, WriteThrough };

namespace detail {
// Exact integer log2 of a power-of-two n (n >= 1).
inline std::uint32_t ilog2(std::size_t n) {
    std::uint32_t b = 0;
    while (n > 1) {
        n >>= 1;
        ++b;
    }
    return b;
}
}  // namespace detail

// ── Cache line — the smallest unit of cached data ────────────────────────────
struct CacheLine {
    bool valid = false;
    bool dirty = false;
    std::uint64_t tag = 0;
    std::vector<std::uint8_t> data;
    std::uint64_t last_access = 0;

    explicit CacheLine(std::size_t line_size) : data(line_size, 0) {}

    // Load data into the line, marking it valid and clean.
    void fill(std::uint64_t new_tag, const std::vector<std::uint8_t>& src,
              std::uint64_t cycle) {
        valid = true;
        dirty = false;  // freshly loaded data is clean
        tag = new_tag;
        data.assign(src.begin(), src.end());  // defensive copy
        last_access = cycle;
    }
    void touch(std::uint64_t cycle) { last_access = cycle; }
    void invalidate() {
        valid = false;
        dirty = false;
    }
    std::size_t line_size() const { return data.size(); }
};

// ── Cache configuration ──────────────────────────────────────────────────────
struct CacheConfig {
    std::string name;
    std::size_t total_size = 0;
    std::size_t line_size = 0;
    std::size_t associativity = 0;
    std::uint64_t access_latency = 0;
    WritePolicy write_policy = WritePolicy::WriteBack;

    // Validate and build a config. Throws std::invalid_argument where the Rust
    // `CacheConfig::new` would panic.
    static CacheConfig create(const std::string& name, std::size_t total_size,
                              std::size_t line_size, std::size_t associativity,
                              std::uint64_t access_latency) {
        if (total_size == 0) {
            throw std::invalid_argument("total_size must be positive");
        }
        if (line_size == 0 || (line_size & (line_size - 1)) != 0) {
            throw std::invalid_argument(
                "line_size must be a positive power of 2");
        }
        if (associativity == 0) {
            throw std::invalid_argument("associativity must be positive");
        }
        if (total_size % (line_size * associativity) != 0) {
            throw std::invalid_argument("total_size must be divisible");
        }
        CacheConfig c;
        c.name = name;
        c.total_size = total_size;
        c.line_size = line_size;
        c.associativity = associativity;
        c.access_latency = access_latency;
        c.write_policy = WritePolicy::WriteBack;
        return c;
    }

    CacheConfig with_write_policy(WritePolicy policy) const {
        CacheConfig c = *this;
        c.write_policy = policy;
        return c;
    }
    std::size_t num_lines() const { return total_size / line_size; }
    std::size_t num_sets() const { return num_lines() / associativity; }
};

// ── Cache set — a group of `associativity` ways ──────────────────────────────
class CacheSet {
  public:
    std::vector<CacheLine> lines;

    CacheSet(std::size_t associativity, std::size_t line_size) {
        lines.reserve(associativity);
        for (std::size_t i = 0; i < associativity; ++i) {
            lines.emplace_back(line_size);
        }
    }

    // Search for a valid line with `tag`.
    std::optional<std::size_t> lookup(std::uint64_t tag) const {
        for (std::size_t i = 0; i < lines.size(); ++i) {
            if (lines[i].valid && lines[i].tag == tag) {
                return i;
            }
        }
        return std::nullopt;
    }

    // On hit: touch and return {true, way}. On miss: return {false, LRU victim}.
    std::pair<bool, std::size_t> access(std::uint64_t tag, std::uint64_t cycle) {
        if (auto way = lookup(tag)) {
            lines[*way].touch(cycle);
            return {true, *way};
        }
        return {false, find_lru()};
    }

    // Bring data into the set (fill an invalid way, else evict LRU). Returns the
    // evicted line iff it was dirty (needs writeback), matching the Rust.
    std::optional<CacheLine> allocate(std::uint64_t tag,
                                      const std::vector<std::uint8_t>& data,
                                      std::uint64_t cycle) {
        for (auto& line : lines) {
            if (!line.valid) {
                line.fill(tag, data, cycle);
                return std::nullopt;
            }
        }
        std::size_t lru = find_lru();
        std::optional<CacheLine> evicted;
        if (lines[lru].dirty) {
            evicted = lines[lru];  // clone before overwrite
        }
        lines[lru].fill(tag, data, cycle);
        return evicted;
    }

  private:
    std::size_t find_lru() const {
        std::size_t best_index = 0;
        std::uint64_t best_time = UINT64_MAX;
        for (std::size_t i = 0; i < lines.size(); ++i) {
            if (!lines[i].valid) {
                return i;
            }
            if (lines[i].last_access < best_time) {
                best_time = lines[i].last_access;
                best_index = i;
            }
        }
        return best_index;
    }
};

// ── Statistics ───────────────────────────────────────────────────────────────
struct CacheStats {
    std::uint64_t reads = 0, writes = 0, hits = 0, misses = 0, evictions = 0,
                  writebacks = 0;

    std::uint64_t total_accesses() const { return reads + writes; }
    double hit_rate() const {
        std::uint64_t total = total_accesses();
        return total == 0 ? 0.0
                          : static_cast<double>(hits) /
                                static_cast<double>(total);
    }
    double miss_rate() const {
        std::uint64_t total = total_accesses();
        return total == 0 ? 0.0
                          : static_cast<double>(misses) /
                                static_cast<double>(total);
    }
    void record_read(bool hit) {
        ++reads;
        if (hit) {
            ++hits;
        } else {
            ++misses;
        }
    }
    void record_write(bool hit) {
        ++writes;
        if (hit) {
            ++hits;
        } else {
            ++misses;
        }
    }
    void record_eviction(bool dirty) {
        ++evictions;
        if (dirty) {
            ++writebacks;
        }
    }
    void reset() { *this = CacheStats{}; }
};

// ── Single-access record ─────────────────────────────────────────────────────
struct CacheAccess {
    std::uint64_t address = 0;
    bool hit = false;
    std::uint64_t tag = 0;
    std::size_t set_index = 0;
    std::size_t offset = 0;
    std::uint64_t cycles = 0;
    std::optional<CacheLine> evicted;
};

// ── A single configurable cache level ────────────────────────────────────────
class Cache {
  public:
    CacheConfig config;
    CacheStats stats;

    explicit Cache(CacheConfig cfg) : config(std::move(cfg)) {
        std::size_t num_sets = config.num_sets();
        sets_.reserve(num_sets);
        for (std::size_t i = 0; i < num_sets; ++i) {
            sets_.emplace_back(config.associativity, config.line_size);
        }
        offset_bits_ = detail::ilog2(config.line_size);
        set_bits_ = num_sets > 1 ? detail::ilog2(num_sets) : 0;
        set_mask_ = num_sets > 0 ? static_cast<std::uint64_t>(num_sets - 1) : 0;
    }

    // (tag, set_index, offset)
    std::tuple<std::uint64_t, std::size_t, std::size_t> decompose(
        std::uint64_t address) const {
        std::size_t offset = static_cast<std::size_t>(
            address & ((static_cast<std::uint64_t>(1) << offset_bits_) - 1));
        std::size_t set_index = static_cast<std::size_t>(
            (address >> offset_bits_) & set_mask_);
        std::uint64_t tag = address >> (offset_bits_ + set_bits_);
        return {tag, set_index, offset};
    }

    CacheAccess read(std::uint64_t address, std::uint64_t cycle) {
        auto [tag, set_index, offset] = decompose(address);
        CacheSet& set = sets_[set_index];
        auto [hit, idx] = set.access(tag, cycle);
        (void)idx;

        CacheAccess acc;
        acc.address = address;
        acc.tag = tag;
        acc.set_index = set_index;
        acc.offset = offset;
        acc.cycles = config.access_latency;

        if (hit) {
            stats.record_read(true);
            acc.hit = true;
            return acc;
        }
        stats.record_read(false);
        acc.hit = false;
        std::vector<std::uint8_t> dummy(config.line_size, 0);
        auto evicted = set.allocate(tag, dummy, cycle);
        if (evicted) {
            stats.record_eviction(true);
            acc.evicted = std::move(evicted);
        } else if (all_valid(set)) {
            stats.record_eviction(false);
        }
        return acc;
    }

    CacheAccess write(std::uint64_t address,
                      const std::vector<std::uint8_t>& data,
                      std::uint64_t cycle) {
        auto [tag, set_index, offset] = decompose(address);
        CacheSet& set = sets_[set_index];
        auto [hit, idx] = set.access(tag, cycle);

        CacheAccess acc;
        acc.address = address;
        acc.tag = tag;
        acc.set_index = set_index;
        acc.offset = offset;
        acc.cycles = config.access_latency;

        if (hit) {
            stats.record_write(true);
            CacheLine& line = set.lines[idx];
            for (std::size_t i = 0; i < data.size(); ++i) {
                if (offset + i < line.data.size()) {
                    line.data[offset + i] = data[i];
                }
            }
            if (config.write_policy == WritePolicy::WriteBack) {
                line.dirty = true;
            }
            acc.hit = true;
            return acc;
        }

        stats.record_write(false);
        acc.hit = false;
        std::vector<std::uint8_t> fill(config.line_size, 0);
        for (std::size_t i = 0; i < data.size(); ++i) {
            if (offset + i < fill.size()) {
                fill[offset + i] = data[i];
            }
        }
        auto evicted = set.allocate(tag, fill, cycle);
        if (evicted) {
            stats.record_eviction(true);
            acc.evicted = std::move(evicted);
        } else if (all_valid(set)) {
            stats.record_eviction(false);
        }
        if (config.write_policy == WritePolicy::WriteBack) {
            auto [new_hit, new_idx] = set.access(tag, cycle);
            if (new_hit) {
                set.lines[new_idx].dirty = true;
            }
        }
        return acc;
    }

    void invalidate() {
        for (auto& set : sets_) {
            for (auto& line : set.lines) {
                line.invalidate();
            }
        }
    }

    std::optional<CacheLine> fill_line(std::uint64_t address,
                                       const std::vector<std::uint8_t>& data,
                                       std::uint64_t cycle) {
        auto [tag, set_index, offset] = decompose(address);
        (void)offset;
        return sets_[set_index].allocate(tag, data, cycle);
    }

  private:
    static bool all_valid(const CacheSet& set) {
        for (const auto& line : set.lines) {
            if (!line.valid) {
                return false;
            }
        }
        return true;
    }

    std::vector<CacheSet> sets_;
    std::uint32_t offset_bits_ = 0;
    std::uint32_t set_bits_ = 0;
    std::uint64_t set_mask_ = 0;
};

// ── Hierarchy access record ──────────────────────────────────────────────────
struct HierarchyAccess {
    std::uint64_t address = 0;
    std::string served_by;
    std::uint64_t total_cycles = 0;
    std::size_t hit_at_level = 0;
    std::vector<CacheAccess> level_accesses;
};

// ── Multi-level hierarchy ────────────────────────────────────────────────────
class CacheHierarchy {
  public:
    std::optional<Cache> l1i, l1d, l2, l3;
    std::uint64_t main_memory_latency;

    CacheHierarchy(std::optional<Cache> l1i_, std::optional<Cache> l1d_,
                   std::optional<Cache> l2_, std::optional<Cache> l3_,
                   std::uint64_t main_memory_latency_)
        : l1i(std::move(l1i_)),
          l1d(std::move(l1d_)),
          l2(std::move(l2_)),
          l3(std::move(l3_)),
          main_memory_latency(main_memory_latency_) {}

    HierarchyAccess read(std::uint64_t address, bool is_instruction,
                         std::uint64_t cycle) {
        auto order = build_level_order(is_instruction);
        HierarchyAccess r;
        r.address = address;

        if (order.empty()) {
            r.served_by = "memory";
            r.total_cycles = main_memory_latency;
            r.hit_at_level = 0;
            return r;
        }

        std::uint64_t total = 0;
        r.served_by = "memory";
        std::size_t hit_level = order.size();
        for (std::size_t i = 0; i < order.size(); ++i) {
            CacheAccess a = order[i].first->read(address, cycle);
            total += order[i].first->config.access_latency;
            bool hit = a.hit;
            r.level_accesses.push_back(std::move(a));
            if (hit) {
                r.served_by = order[i].second;
                hit_level = i;
                break;
            }
        }
        if (r.served_by == "memory") {
            total += main_memory_latency;
        }

        // Inclusive fill: install the line in every level above the hit.
        std::size_t line_size = order.front().first->config.line_size;
        std::vector<std::uint8_t> dummy(line_size, 0);
        for (std::size_t fi = hit_level; fi-- > 0;) {
            order[fi].first->fill_line(address, dummy, cycle);
        }

        r.total_cycles = total;
        r.hit_at_level = hit_level;
        return r;
    }

    HierarchyAccess write(std::uint64_t address,
                          const std::vector<std::uint8_t>& data,
                          std::uint64_t cycle) {
        auto order = build_level_order(false);
        HierarchyAccess r;
        r.address = address;

        if (order.empty()) {
            r.served_by = "memory";
            r.total_cycles = main_memory_latency;
            r.hit_at_level = 0;
            return r;
        }

        CacheAccess first = order[0].first->write(address, data, cycle);
        std::uint64_t total = order[0].first->config.access_latency;
        bool first_hit = first.hit;
        r.level_accesses.push_back(std::move(first));

        if (first_hit) {
            r.served_by = order[0].second;
            r.total_cycles = total;
            r.hit_at_level = 0;
            return r;
        }

        r.served_by = "memory";
        std::size_t hit_level = order.size();
        for (std::size_t i = 1; i < order.size(); ++i) {
            CacheAccess a = order[i].first->read(address, cycle);
            total += order[i].first->config.access_latency;
            bool hit = a.hit;
            r.level_accesses.push_back(std::move(a));
            if (hit) {
                r.served_by = order[i].second;
                hit_level = i;
                break;
            }
        }
        if (r.served_by == "memory") {
            total += main_memory_latency;
        }
        r.total_cycles = total;
        r.hit_at_level = hit_level;
        return r;
    }

    void invalidate_all() {
        if (l1i) l1i->invalidate();
        if (l1d) l1d->invalidate();
        if (l2) l2->invalidate();
        if (l3) l3->invalidate();
    }

    void reset_stats() {
        if (l1i) l1i->stats.reset();
        if (l1d) l1d->stats.reset();
        if (l2) l2->stats.reset();
        if (l3) l3->stats.reset();
    }

  private:
    std::vector<std::pair<Cache*, std::string>> build_level_order(
        bool is_instruction) {
        std::vector<std::pair<Cache*, std::string>> order;
        if (is_instruction) {
            if (l1i) order.emplace_back(&*l1i, "L1I");
        } else if (l1d) {
            order.emplace_back(&*l1d, "L1D");
        }
        if (l2) order.emplace_back(&*l2, "L2");
        if (l3) order.emplace_back(&*l3, "L3");
        return order;
    }
};

}  // namespace cpu_cache
}  // namespace ca

#endif  // CPU_CACHE_HPP
