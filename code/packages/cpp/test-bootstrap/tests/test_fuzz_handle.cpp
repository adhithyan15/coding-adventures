// test_fuzz_handle.cpp -- the RAII Handle wrapper (VER06k section 4), run under ASan in run.sh.
//
// A handle is a window onto ONE execution.  The wrapper is move-only and INERT when default made,
// moved from, retired (the execution returned), reissued (the C ring slot got a new epoch) or
// used from another thread.  "Inert" is observable: edge() records nothing, tick() and reserve()
// report exhaustion and consume nothing.  Memory safety is by construction (the C slots are
// static); ASan confirms nothing is touched after free.
#include <cstdio>
#include <optional>
#include <thread>
#include <type_traits>
#include <vector>

#include "fuzz_bootstrap.hpp"
#include "iso_test.h"

namespace {

namespace fz = ca::fuzz;

static_assert(!std::is_copy_constructible<fz::Handle>::value, "a Handle must not be copyable");
static_assert(!std::is_copy_assignable<fz::Handle>::value, "a Handle must not be copy-assignable");
static_assert(std::is_nothrow_move_constructible<fz::Handle>::value, "Handle moves are noexcept");
static_assert(std::is_nothrow_move_assignable<fz::Handle>::value, "Handle moves are noexcept");
static_assert(std::is_nothrow_default_constructible<fz::Handle>::value, "a default Handle is inert and free");

void expect_inert(fz::Handle& h) {
    ISO_CHECK(!h.alive());
    ISO_CHECK(h.tick());
    ISO_CHECK(h.reserve(1));
    h.edge(5);  // must not crash or record
    h.edge("label");
}

void test_default_and_moved_from_are_inert() {
    fz::Handle d;
    expect_inert(d);
    fz::Options o;
    o.max_iterations = 0;
    fz::Report r = fz::run({fz::Bytes{1}}, o, [](fz::ByteView, fz::Handle& h) {
        ISO_CHECK(h.alive());
        ISO_CHECK(!h.tick());
        fz::Handle moved(std::move(h));
        expect_inert(h);  // NOLINT: a moved-from handle is specified to be inert
        ISO_CHECK(moved.alive());
        ISO_CHECK(!moved.tick());
        fz::Handle assigned;
        assigned = std::move(moved);
        expect_inert(moved);  // NOLINT
        ISO_CHECK(assigned.alive());
        fz::Handle& alias = assigned;
        assigned = std::move(alias);  // self-move leaves it alive
        ISO_CHECK(assigned.alive());
    });
    ISO_CHECK(r.is_clean() || r.iterations() == 0);
}

// A handle that escaped its execution.
std::optional<fz::Handle> g_kept;

void test_retained_wrapper_is_inert_after_the_execution() {
    fz::Options o;
    o.max_iterations = 0;
    o.step_budget = 1;
    fz::Report first = fz::run({fz::Bytes{1}}, o, [](fz::ByteView, fz::Handle& h) {
        h.edge(1);
        g_kept.emplace(std::move(h));  // keep it past the execution
        ISO_CHECK(g_kept->alive());
    });
    ISO_CHECK_EQ_UINT(first.total_edges(), 1);
    ISO_CHECK(g_kept.has_value());
    if (!g_kept.has_value()) return;
    expect_inert(*g_kept);
    ISO_CHECK(g_kept->native() != nullptr);  // the pointer is still there: inertness is by checking, not by nulling
    // a later execution in the same slot region is not touched by the stale wrapper
    fz::Report second = fz::run({fz::Bytes{1}}, o, [](fz::ByteView, fz::Handle& h) {
        h.edge(2);
        g_kept->edge(77);                    // would add an edge to ANY live handle it aliased
        ISO_CHECK(g_kept->tick());           // reports exhaustion ...
        ISO_CHECK(!h.tick());                // ... and consumed none of this execution's single step
        ISO_CHECK(g_kept->reserve(1));
    });
    ISO_CHECK_EQ_UINT(second.total_edges(), 1);
    ISO_CHECK(second.is_clean() || second.iterations() == 0);
}

// The C ring has 4,096 slots: after enough executions a retained pointer aliases a LATER execution's
// handle.  The C engine documents that (VER06j section 5.3); the wrapper's epoch check removes it.
void test_ring_reuse_cannot_alias() {
    g_kept.reset();
    fz::Options o;
    o.max_iterations = 5000;  // more than the ring
    o.step_budget = 1;
    bool captured = false;
    bool aliased = false;
    std::size_t aliased_checks = 0;
    fz::Report r = fz::run({fz::Bytes{1}}, o, [&](fz::ByteView, fz::Handle& h) {
        if (!captured) {
            captured = true;
            g_kept.emplace();
            // a SECOND wrapper over the same execution: made by moving, so the epoch is this execution's
            *g_kept = std::move(h);
            return;
        }
        if (g_kept->native() == h.native()) {  // the ring reissued the slot: same pointer, new epoch
            aliased = true;
            ++aliased_checks;
            ISO_CHECK(!g_kept->alive());
            ISO_CHECK(g_kept->tick());   // inert: reports exhaustion
            g_kept->edge(123);           // inert: records nothing
            ISO_CHECK(g_kept->reserve(1));
            ISO_CHECK(h.alive());
            ISO_CHECK(!h.tick());        // and the LIVE handle's single step was not consumed by the stale one
        }
    });
    ISO_CHECK(aliased);
    ISO_CHECK(aliased_checks >= 1);
    std::printf("  ring reuse observed %lu time(s); the stale wrapper stayed inert every time\n",
                static_cast<unsigned long>(aliased_checks));
    ISO_CHECK(r.iterations() == 5000);
    g_kept.reset();
}

void test_other_threads_are_inert() {
    fz::Options o;
    o.max_iterations = 0;
    o.step_budget = 1;
    bool thread_ran = false;
    fz::Report r = fz::run({fz::Bytes{1}}, o, [&](fz::ByteView, fz::Handle& h) {
        bool thread_alive = true;
        bool thread_tick = false;
        bool thread_reserve = false;
        // the wrapper is thread affine: the helper gets a move of it ON ANOTHER THREAD
        std::thread t([&] {
            thread_alive = h.alive();
            thread_tick = h.tick();
            thread_reserve = h.reserve(1);
            h.edge(9);
            thread_ran = true;
        });
        t.join();  // a target must join its threads before returning
        ISO_CHECK(thread_ran);
        ISO_CHECK(!thread_alive);
        ISO_CHECK(thread_tick);
        ISO_CHECK(thread_reserve);
        ISO_CHECK(h.alive());   // the owning thread still has its handle ...
        ISO_CHECK(!h.tick());   // ... and the single step: the helper consumed nothing
    });
    ISO_CHECK_EQ_UINT(r.total_edges(), 0);  // the helper's edge(9) was dropped
}

void test_labels_and_edges_record_coverage() {
    fz::Options o;
    o.max_iterations = 0;
    fz::Report r = fz::run({fz::Bytes{1}}, o, [](fz::ByteView, fz::Handle& h) {
        h.edge("parse");
        h.edge("parse");  // the same label: one edge
        h.edge(fz::edge_id("emit"));
        h.edge(70000);
        h.edge(70000 - 65536);  // the same edge modulo 65,536
    });
    ISO_CHECK_EQ_UINT(r.total_edges(), 3);
    ISO_CHECK(r.tier() == FZ_TIER_CALLER);
}

}  // namespace

int main() {
    test_default_and_moved_from_are_inert();
    test_retained_wrapper_is_inert_after_the_execution();
    test_ring_reuse_cannot_alias();
    test_other_threads_are_inert();
    test_labels_and_edges_record_coverage();
    return ISO_TEST_RESULT();
}
