// Tests for cpu-cache, mirroring the Rust crate's unit tests across all five
// modules, using the header-only iso_test.h harness (pure ISO C++17).
#include "iso_test.h"

#include <cstdint>
#include <optional>
#include <stdexcept>
#include <vector>

#include "cpu_cache.hpp"

namespace cc = ca::cpu_cache;
using Bytes = std::vector<std::uint8_t>;

static constexpr double EPS = 1e-9;

static cc::Cache make_l1d() {
    return cc::Cache(cc::CacheConfig::create("L1D", 1024, 64, 4, 1));
}
static cc::Cache make_l2() {
    return cc::Cache(cc::CacheConfig::create("L2", 4096, 64, 8, 10));
}
static cc::Cache make_l3() {
    return cc::Cache(cc::CacheConfig::create("L3", 16384, 64, 16, 30));
}

int main() {
    // ══ CacheLine ══════════════════════════════════════════════════════════
    {
        cc::CacheLine line(64);
        ISO_CHECK(!line.valid);
        ISO_CHECK(!line.dirty);
        ISO_CHECK_EQ_UINT(line.tag, 0u);
        ISO_CHECK_EQ_UINT(line.last_access, 0u);
        ISO_CHECK_EQ_UINT(line.data.size(), 64u);
        ISO_CHECK_EQ_UINT(line.line_size(), 64u);
    }
    {
        cc::CacheLine line(64);
        Bytes data(64, 0xAB);
        line.fill(42, data, 100);
        ISO_CHECK(line.valid);
        ISO_CHECK(!line.dirty);
        ISO_CHECK_EQ_UINT(line.tag, 42u);
        ISO_CHECK_EQ_UINT(line.last_access, 100u);
        ISO_CHECK(line.data == data);
    }
    {
        cc::CacheLine line(4);
        Bytes data{1, 2, 3, 4};
        line.fill(1, data, 0);
        data[0] = 99;
        ISO_CHECK_EQ_INT(line.data[0], 1);  // defensive copy
    }
    {
        cc::CacheLine line(64);
        line.fill(1, Bytes(64, 0), 10);
        ISO_CHECK_EQ_UINT(line.last_access, 10u);
        line.touch(50);
        ISO_CHECK_EQ_UINT(line.last_access, 50u);
    }
    {
        cc::CacheLine line(64);
        line.fill(1, Bytes(64, 0), 10);
        line.dirty = true;
        line.invalidate();
        ISO_CHECK(!line.valid);
        ISO_CHECK(!line.dirty);
    }
    {
        cc::CacheLine l32(32), l128(128);
        ISO_CHECK_EQ_UINT(l32.line_size(), 32u);
        ISO_CHECK_EQ_UINT(l128.line_size(), 128u);
    }

    // ══ CacheConfig ════════════════════════════════════════════════════════
    {
        auto cfg = cc::CacheConfig::create("L1D", 1024, 64, 4, 1);
        ISO_CHECK(cfg.name == "L1D");
        ISO_CHECK_EQ_UINT(cfg.total_size, 1024u);
        ISO_CHECK_EQ_UINT(cfg.line_size, 64u);
        ISO_CHECK_EQ_UINT(cfg.associativity, 4u);
        ISO_CHECK_EQ_UINT(cfg.access_latency, 1u);
        ISO_CHECK_EQ_UINT(cfg.num_lines(), 16u);
        ISO_CHECK_EQ_UINT(cfg.num_sets(), 4u);
    }
    {
        // Invalid configs throw (the Rust panics).
        auto throws = [](std::size_t ts, std::size_t ls, std::size_t assoc) {
            try {
                cc::CacheConfig::create("bad", ts, ls, assoc, 1);
                return false;
            } catch (const std::invalid_argument&) {
                return true;
            }
        };
        ISO_CHECK(throws(0, 64, 4));
        ISO_CHECK(throws(1024, 48, 4));
        ISO_CHECK(throws(1024, 64, 0));
        ISO_CHECK(throws(1000, 64, 4));
    }
    {
        auto cfg = cc::CacheConfig::create("L1D", 1024, 64, 4, 1)
                       .with_write_policy(cc::WritePolicy::WriteThrough);
        ISO_CHECK(cfg.write_policy == cc::WritePolicy::WriteThrough);
    }
    {
        auto cfg = cc::CacheConfig::create("DM", 256, 64, 1, 1);
        ISO_CHECK_EQ_UINT(cfg.num_lines(), 4u);
        ISO_CHECK_EQ_UINT(cfg.num_sets(), 4u);
    }

    // ══ CacheSet ═══════════════════════════════════════════════════════════
    {
        cc::CacheSet set(4, 64);
        ISO_CHECK_EQ_UINT(set.lines.size(), 4u);
        for (const auto& l : set.lines) {
            ISO_CHECK(!l.valid);
        }
        ISO_CHECK(!set.lookup(42).has_value());
    }
    {
        cc::CacheSet set(4, 64);
        auto evicted = set.allocate(42, Bytes(64, 0xAA), 100);
        ISO_CHECK(!evicted.has_value());
        auto way = set.lookup(42);
        ISO_CHECK(way.has_value());
        ISO_CHECK_EQ_UINT(*way, 0u);
        ISO_CHECK_EQ_UINT(set.lines[0].tag, 42u);
    }
    {
        cc::CacheSet set(4, 64);
        for (std::uint64_t tag = 0; tag < 4; ++tag) {
            ISO_CHECK(!set.allocate(tag, Bytes(64, 0), tag).has_value());
        }
        for (std::size_t i = 0; i < 4; ++i) {
            ISO_CHECK(set.lines[i].valid);
            ISO_CHECK_EQ_UINT(set.lines[i].tag, i);
        }
    }
    {
        cc::CacheSet set(2, 64);
        set.allocate(10, Bytes(64, 0), 1);
        set.allocate(20, Bytes(64, 0), 2);
        auto evicted = set.allocate(30, Bytes(64, 0), 3);
        ISO_CHECK(!evicted.has_value());  // clean victim
        ISO_CHECK(!set.lookup(10).has_value());
        ISO_CHECK(set.lookup(30).has_value());
    }
    {
        cc::CacheSet set(2, 64);
        set.allocate(10, Bytes(64, 0), 1);
        set.allocate(20, Bytes(64, 0), 2);
        set.lines[0].dirty = true;
        auto evicted = set.allocate(30, Bytes(64, 0), 3);
        ISO_CHECK(evicted.has_value());
        ISO_CHECK(evicted->dirty);
        ISO_CHECK_EQ_UINT(evicted->tag, 10u);
    }
    {
        cc::CacheSet set(4, 64);
        set.allocate(10, Bytes(64, 0), 1);
        set.allocate(20, Bytes(64, 0), 2);
        auto [hit, idx] = set.access(10, 50);
        ISO_CHECK(hit);
        ISO_CHECK_EQ_UINT(set.lines[idx].last_access, 50u);
    }
    {
        cc::CacheSet set(2, 64);
        set.allocate(10, Bytes(64, 0), 1);
        set.allocate(20, Bytes(64, 0), 2);
        auto [hit, idx] = set.access(99, 3);
        ISO_CHECK(!hit);
        ISO_CHECK_EQ_UINT(idx, 0u);
    }

    // ══ Cache ══════════════════════════════════════════════════════════════
    {
        auto c = make_l1d();
        auto [tag, si, off] = c.decompose(0x100);
        ISO_CHECK_EQ_UINT(off, 0u);
        ISO_CHECK_EQ_UINT(si, 0u);
        ISO_CHECK_EQ_UINT(tag, 0x100u >> 8);
    }
    {
        auto c = make_l1d();
        auto a = c.read(0x100, 0);
        ISO_CHECK(!a.hit);
        ISO_CHECK_EQ_UINT(a.cycles, 1u);
        ISO_CHECK_EQ_UINT(c.stats.reads, 1u);
        ISO_CHECK_EQ_UINT(c.stats.misses, 1u);
        a = c.read(0x100, 1);
        ISO_CHECK(a.hit);
        ISO_CHECK_EQ_UINT(c.stats.hits, 1u);
    }
    {
        auto c = make_l1d();
        c.read(0x000, 0);
        c.read(0x100, 1);
        c.read(0x200, 2);
        c.read(0x300, 3);
        ISO_CHECK_EQ_UINT(c.stats.misses, 4u);
        c.read(0x400, 4);
        ISO_CHECK_EQ_UINT(c.stats.misses, 5u);
        ISO_CHECK(!c.read(0x000, 5).hit);
    }
    {
        auto c = make_l1d();
        c.read(0x100, 0);
        auto a = c.write(0x100, Bytes{0xAB}, 1);
        ISO_CHECK(a.hit);
        ISO_CHECK_EQ_UINT(c.stats.writes, 1u);
        ISO_CHECK_EQ_UINT(c.stats.hits, 1u);
    }
    {
        auto c = make_l1d();
        ISO_CHECK(!c.write(0x100, Bytes{0xAB}, 0).hit);
        ISO_CHECK(c.read(0x100, 1).hit);
    }
    {
        auto c = make_l1d();
        c.read(0x100, 0);
        c.write(0x100, Bytes{0xAB}, 1);
        c.read(0x000, 2);
        c.read(0x200, 3);
        c.read(0x300, 4);
        auto a = c.read(0x400, 5);
        ISO_CHECK(!a.hit);
        if (a.evicted) {
            ISO_CHECK(a.evicted->dirty);
        }
    }
    {
        auto c = cc::Cache(cc::CacheConfig::create("L1D", 1024, 64, 4, 1)
                               .with_write_policy(cc::WritePolicy::WriteThrough));
        c.read(0x100, 0);
        c.write(0x100, Bytes{0xAB}, 1);
        auto [tag, si, off] = c.decompose(0x100);
        (void)off;
        // Re-read to locate the line; write-through must keep it clean. We
        // inspect via a fresh read hit and the public evicted path is n/a here,
        // so verify indirectly: a subsequent set of conflict misses evicts it
        // as a CLEAN line (no writeback recorded).
        c.read(0x000, 2);
        c.read(0x200, 3);
        c.read(0x300, 4);
        c.read(0x400, 5);  // evicts 0x100
        ISO_CHECK_EQ_UINT(c.stats.writebacks, 0u);
        (void)tag;
        (void)si;
    }
    {
        auto c = make_l1d();
        c.read(0x100, 0);
        c.read(0x200, 1);
        c.invalidate();
        ISO_CHECK(!c.read(0x100, 2).hit);
    }
    {
        auto c = make_l1d();
        auto evicted = c.fill_line(0x100, Bytes(64, 0xCD), 0);
        ISO_CHECK(!evicted.has_value());
        ISO_CHECK(c.read(0x100, 1).hit);
    }
    {
        auto c = make_l1d();
        for (std::uint64_t i = 0; i < 64; ++i) {
            auto a = c.read(0x100 + i, i);
            if (i == 0) {
                ISO_CHECK(!a.hit);
            } else {
                ISO_CHECK(a.hit);
            }
        }
        ISO_CHECK_EQ_UINT(c.stats.hits, 63u);
        ISO_CHECK_EQ_UINT(c.stats.misses, 1u);
    }
    {
        auto c = make_l1d();
        for (std::uint64_t i = 0; i < 4; ++i) {
            ISO_CHECK(!c.read(i * 64, i).hit);
        }
        ISO_CHECK_EQ_UINT(c.stats.misses, 4u);
        for (std::uint64_t i = 0; i < 4; ++i) {
            ISO_CHECK(c.read(i * 64, i + 4).hit);
        }
        ISO_CHECK_EQ_UINT(c.stats.hits, 4u);
    }

    // ══ CacheStats ═════════════════════════════════════════════════════════
    {
        cc::CacheStats s;
        ISO_CHECK_EQ_UINT(s.reads, 0u);
        ISO_CHECK_EQ_UINT(s.total_accesses(), 0u);
        ISO_CHECK_EQ_DBL(s.hit_rate(), 0.0, EPS);
        ISO_CHECK_EQ_DBL(s.miss_rate(), 0.0, EPS);
    }
    {
        cc::CacheStats s;
        s.record_read(true);
        ISO_CHECK_EQ_UINT(s.reads, 1u);
        ISO_CHECK_EQ_UINT(s.hits, 1u);
        ISO_CHECK_EQ_DBL(s.hit_rate(), 1.0, EPS);
        s.record_read(false);
        ISO_CHECK_EQ_UINT(s.misses, 1u);
        ISO_CHECK_EQ_DBL(s.miss_rate(), 0.5, EPS);
    }
    {
        cc::CacheStats s;
        s.record_read(true);
        s.record_read(true);
        s.record_write(false);
        s.record_write(true);
        ISO_CHECK_EQ_UINT(s.total_accesses(), 4u);
        ISO_CHECK_EQ_UINT(s.hits, 3u);
        ISO_CHECK_EQ_UINT(s.misses, 1u);
        ISO_CHECK_EQ_DBL(s.hit_rate(), 0.75, EPS);
        ISO_CHECK_EQ_DBL(s.miss_rate(), 0.25, EPS);
    }
    {
        cc::CacheStats s;
        s.record_eviction(false);
        s.record_eviction(true);
        s.record_eviction(true);
        ISO_CHECK_EQ_UINT(s.evictions, 3u);
        ISO_CHECK_EQ_UINT(s.writebacks, 2u);
        s.reset();
        ISO_CHECK_EQ_UINT(s.total_accesses(), 0u);
        ISO_CHECK_EQ_UINT(s.evictions, 0u);
        ISO_CHECK_EQ_UINT(s.writebacks, 0u);
    }

    // ══ CacheHierarchy ═════════════════════════════════════════════════════
    {
        cc::CacheHierarchy h(std::nullopt, std::nullopt, std::nullopt,
                             std::nullopt, 100);
        auto r = h.read(0x1000, false, 0);
        ISO_CHECK(r.served_by == "memory");
        ISO_CHECK_EQ_UINT(r.total_cycles, 100u);
    }
    {
        cc::CacheHierarchy h(std::nullopt, make_l1d(), std::nullopt,
                             std::nullopt, 100);
        auto r = h.read(0x1000, false, 0);
        ISO_CHECK(r.served_by == "memory");
        ISO_CHECK_EQ_UINT(r.total_cycles, 1u + 100u);
        r = h.read(0x1000, false, 1);
        ISO_CHECK(r.served_by == "L1D");
        ISO_CHECK_EQ_UINT(r.total_cycles, 1u);
    }
    {
        cc::CacheHierarchy h(std::nullopt, make_l1d(), make_l2(), std::nullopt,
                             100);
        auto r = h.read(0x1000, false, 0);
        ISO_CHECK(r.served_by == "memory");
        ISO_CHECK_EQ_UINT(r.total_cycles, 1u + 10u + 100u);
        r = h.read(0x1000, false, 1);
        ISO_CHECK(r.served_by == "L1D");
        ISO_CHECK_EQ_UINT(r.total_cycles, 1u);
    }
    {
        cc::CacheHierarchy h(std::nullopt, make_l1d(), make_l2(), make_l3(),
                             100);
        auto r = h.read(0x1000, false, 0);
        ISO_CHECK(r.served_by == "memory");
        ISO_CHECK_EQ_UINT(r.total_cycles, 1u + 10u + 30u + 100u);
    }
    {
        cc::CacheHierarchy h(std::nullopt, make_l1d(), make_l2(), std::nullopt,
                             100);
        auto w = h.write(0x2000, Bytes{0xAB}, 0);
        ISO_CHECK(w.served_by == "memory");
        auto r = h.read(0x2000, false, 1);
        ISO_CHECK(r.served_by == "L1D");
    }
    {
        cc::Cache l1i(cc::CacheConfig::create("L1I", 1024, 64, 4, 1));
        cc::CacheHierarchy h(std::move(l1i), make_l1d(), std::nullopt,
                             std::nullopt, 100);
        auto r = h.read(0x1000, true, 0);
        ISO_CHECK(r.served_by == "memory");
        r = h.read(0x1000, false, 1);
        ISO_CHECK(r.served_by == "memory");
        r = h.read(0x1000, true, 2);
        ISO_CHECK(r.served_by == "L1I");
    }
    {
        cc::CacheHierarchy h(std::nullopt, make_l1d(), make_l2(), std::nullopt,
                             100);
        h.read(0x1000, false, 0);
        h.invalidate_all();
        auto r = h.read(0x1000, false, 1);
        ISO_CHECK(r.served_by == "memory");
    }
    {
        cc::CacheHierarchy h(std::nullopt, make_l1d(), make_l2(), std::nullopt,
                             100);
        h.read(0x1000, false, 0);
        h.reset_stats();
        ISO_CHECK_EQ_UINT(h.l1d->stats.total_accesses(), 0u);
    }
    {
        cc::CacheHierarchy h(std::nullopt, make_l1d(), make_l2(), std::nullopt,
                             100);
        h.l2->read(0x3000, 0);       // pre-fill L2
        h.read(0x3000, false, 1);    // L1 miss, L2 hit -> fills L1
        auto r = h.read(0x3000, false, 2);
        ISO_CHECK(r.served_by == "L1D");
    }

    return ISO_TEST_RESULT();
}
