// test_fuzz_alloc.cpp -- allocation failure swept through the fuzz binding (VER06k sections 3 and 4).
//
// Two seams, both swept N = 1, 2, 3, ... until a run no longer reaches the N-th allocation:
//
//   1. the C engine's allocator seam (pbt_hooks.fail_after): the engine's N-th allocation returns
//      NULL.  Every binding call must then THROW ca::fuzz::Error (never report success on a run
//      that lost memory) and leak nothing (pbt_hooks.live_allocs back at baseline);
//   2. a throwing global operator new (replaced in this executable only, forwarding to malloc so
//      ASan still sees every block): the N-th C++ allocation throws std::bad_alloc.  The binding
//      must let bad_alloc out only from OUTSIDE a C frame, release every owner on the way (RAII:
//      OwnedBytes, Report, the campaign context) and leak nothing.  A throw INSIDE the target is
//      the oom outcome by design, so there it must not escape at all.
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <new>
#include <vector>

#include "fuzz_bootstrap.hpp"
#include "iso_test.h"

extern "C" {
#include "fuzz_internal.h"
#include "pbt_hooks.h"
}

namespace {
long g_fail_after = 0;  // 0 = never; N = the N-th operator new from now throws
long g_failed = 0;
}  // namespace

void* operator new(std::size_t n) {
    if (g_fail_after > 0 && --g_fail_after == 0) {
        ++g_failed;
        throw std::bad_alloc();
    }
    void* p = std::malloc(n == 0 ? 1 : n);
    if (p == nullptr) throw std::bad_alloc();
    return p;
}
void operator delete(void* p) noexcept { std::free(p); }
void operator delete(void* p, std::size_t) noexcept { std::free(p); }

namespace {

namespace fz = ca::fuzz;

using Scenario = void (*)();

// Mutators: a pure function of the engine's allocator, so a failed allocation MUST be an Error.
void sc_mutators() {
    fz::Bytes in{1, 2, 3, 4, 5, 6, 7, 8};
    (void)fz::bit_flip(in, 3, 5);
    (void)fz::byte_arithmetic(in, 1, 7, 4, fz::Endian::Big);
    (void)fz::block_insert(in, 2, fz::Bytes{9, 9, 9});
    (void)fz::splice(in, fz::Bytes{7, 7}, 3, 1);
    (void)fz::mutate_sequence(in, 42, 4, 100);
    (void)fz::corpus_admit({1, 2, 3}, {3, 4, 5});
}

void sc_campaign() {
    fz::Options o;
    o.max_iterations = 8;
    fz::Report r = fz::run({fz::Bytes{'A', 'B'}}, o, [](fz::ByteView d, fz::Handle& h) {
        h.edge(1);
        if (d.size() > 0 && d[0] == 'A') throw fz::Crash("k", {"f"});
    });
    (void)r.crashes();
    (void)r.tap_failures("t");
    (void)r.summary();
    (void)r.corpus();
}

void sc_minimize() {
    (void)fz::minimize(fz::Bytes{1, 2, 3, 4, 5, 6}, fz::crash_signature("k", {}), [](fz::ByteView d) {
        for (std::size_t i = 0; i < d.size(); ++i)
            if (d[i] == 3) return fz::Outcome::crash("k");
        return fz::Outcome::pass();
    }, 100);
}

struct Named {
    const char* name;
    Scenario fn;
};

// Sweep 1: the C allocator.  Every injected failure must surface as Error(FZ_E_NOMEM).
void sweep_c_allocator(const Named& s, bool strict) {
    long runs = 0;
    for (long n = 1; n < 5000; ++n) {
        long baseline = pbt_hooks.live_allocs;
        std::size_t handles = fz_handles_live();
        pbt_hooks_reset();
        long before = pbt_hooks.injected;
        pbt_hooks.fail_after = n;
        bool threw = false;
        try {
            s.fn();
        } catch (const fz::Error& e) {
            threw = true;
            ISO_CHECK_EQ_INT(e.status(), FZ_E_NOMEM);
        }
        long injected = pbt_hooks.injected - before;
        pbt_hooks_reset();
        ++runs;
        if (pbt_hooks.live_allocs != baseline) std::printf("  %s leaked at C allocation %ld\n", s.name, n);
        ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, baseline);
        ISO_CHECK_EQ_UINT(fz_handles_live(), handles);
        if (strict && injected > 0) {
            if (!threw) std::printf("  %s claimed success although C allocation %ld failed\n", s.name, n);
            ISO_CHECK(threw);  // never a false success
        }
        if (injected == 0) {
            ISO_CHECK_MSG(n > 1, "the scenario must allocate at least once");
            break;
        }
    }
    std::printf("  C allocator, %s: %ld runs\n", s.name, runs);
}

// Sweep 2: a throwing operator new.  bad_alloc may leave, nothing may leak, nothing else may leave.
void sweep_operator_new(const Named& s) {
    long runs = 0;
    for (long n = 1; n < 20000; ++n) {
        long baseline = pbt_hooks.live_allocs;
        std::size_t handles = fz_handles_live();
        long before = g_failed;
        g_fail_after = n;
        try {
            s.fn();
        } catch (const std::bad_alloc&) {
        } catch (const fz::Error&) {
        }
        g_fail_after = 0;
        ++runs;
        if (pbt_hooks.live_allocs != baseline) std::printf("  %s leaked when C++ allocation %ld threw\n", s.name, n);
        ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, baseline);
        ISO_CHECK_EQ_UINT(fz_handles_live(), handles);
        if (g_failed == before) {
            ISO_CHECK_MSG(n > 1, "the scenario must allocate at least once");
            break;
        }
    }
    std::printf("  operator new, %s: %ld runs\n", s.name, runs);
}

}  // namespace

int main() {
    pbt_hooks_reset();
    const Named all[] = {{"mutators", sc_mutators}, {"campaign", sc_campaign}, {"minimize", sc_minimize}};
    std::printf("== the C allocator seam\n");
    sweep_c_allocator(all[0], true);
    sweep_c_allocator(all[1], true);
    sweep_c_allocator(all[2], true);
    std::printf("== a throwing operator new\n");
    for (const Named& s : all) sweep_operator_new(s);
    return ISO_TEST_RESULT();
}
