// test_alloc_throw.cpp -- a THROWING C++ allocator through the generator builders (VER05j section 7).
//
// The builders create the C generator and also allocate the "anchor" vector that keeps the user's
// closures alive.  If that second allocation throws std::bad_alloc AFTER the C generator exists,
// the generator leaks.  The builders therefore allocate the anchors first.  This test proves it:
// the global operator new is replaced by one that can be told "fail the N-th allocation from now",
// and for N = 1, 2, ... every builder is run and the C engine's live-allocation counter
// (pbt_hooks.live_allocs) must be back at its baseline, whichever allocation failed.
//
// The replacement lives in this executable only; it forwards to malloc/free so ASan still sees
// every block.
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <new>
#include <vector>

#include "iso_test.h"
#include "test_bootstrap.hpp"

extern "C" {
#include "pbt_hooks.h"
}

namespace pbt = ca::pbt;

namespace {
long g_fail_after = 0;  // 0 = never; N = the N-th operator new from now throws
long g_failed = 0;      // how many allocations have been failed
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

using Builder = void (*)();

void build_map() { (void)pbt::gen::u8().map([](pbt::ValueView v) { return pbt::Value::integer(v.as_i64()); }); }
void build_filter() { (void)pbt::gen::u8().filter([](pbt::ValueView) { return true; }); }
void build_bind() { (void)pbt::gen::u8().bind([](pbt::ValueView) { return pbt::gen::boolean(); }); }
void build_custom() {
    (void)pbt::Gen::custom([](pbt::Run&) { return pbt::Value::integer(1); },
                           [](pbt::Value&, const pbt::Falsifies&, int) { return 0; });
}
void build_one_of() { (void)pbt::gen::one_of({pbt::gen::u8(), pbt::gen::boolean()}); }
void build_frequency() { (void)pbt::gen::frequency({{1, pbt::gen::u8()}, {2, pbt::gen::boolean()}}); }
void build_list() {
    pbt::Gen m = pbt::gen::u8().map([](pbt::ValueView v) { return pbt::Value::integer(v.as_i64()); });
    (void)pbt::gen::list(m, 0, 3);
}
void build_tuple() { (void)pbt::gen::tuple({pbt::gen::u8(), pbt::gen::bytes(0, 2)}); }

struct Named {
    const char* name;
    Builder fn;
};

// Fail the N-th C++ allocation of `fn` for N = 1, 2, ... until one run no longer fails.
void sweep(const Named& b) {
    long runs = 0;
    for (long n = 1; n < 5000; ++n) {
        long baseline = pbt_hooks.live_allocs;
        long before = g_failed;
        g_fail_after = n;
        try {
            b.fn();
        } catch (const std::bad_alloc&) {
        } catch (const pbt::Error&) {
        }
        g_fail_after = 0;
        ++runs;
        if (pbt_hooks.live_allocs != baseline)
            std::printf("  %s: C generator leaked when C++ allocation %ld threw (%ld live, baseline %ld)\n", b.name, n,
                        pbt_hooks.live_allocs, baseline);
        ISO_CHECK_EQ_INT(pbt_hooks.live_allocs, baseline);
        if (g_failed == before) {
            ISO_CHECK_MSG(n > 1, "the builder must allocate at least once");
            break;  // n exceeded the builder's allocations: complete
        }
    }
    std::printf("  %s: %ld runs\n", b.name, runs);
}

}  // namespace

int main() {
    pbt_hooks_reset();
    const Named all[] = {{"map", build_map},         {"filter", build_filter},     {"bind", build_bind},
                         {"custom", build_custom},   {"one_of", build_one_of},     {"frequency", build_frequency},
                         {"list", build_list},       {"tuple", build_tuple}};
    for (const Named& b : all) sweep(b);
    return ISO_TEST_RESULT();
}
