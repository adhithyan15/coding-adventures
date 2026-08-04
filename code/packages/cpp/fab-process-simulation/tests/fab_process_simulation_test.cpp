// Tests for the C++ fab-process-simulation library, using the header-only
// iso_test.h harness (pure ISO). Reference values were captured from the real
// Rust crate (an oracle run).
#include "iso_test.h"

#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

#include "fab_process_simulation.hpp"

namespace fab = ca::fab_process_simulation;

static fab::CrossSection bare_si(double thickness) {
    fab::CrossSection cs;
    cs.layers.emplace_back("Si", thickness);
    return cs;
}

template <typename F>
static bool throws_invalid(F fn) {
    try {
        fn();
    } catch (const std::invalid_argument&) {
        return true;
    }
    return false;
}

int main() {
    // ── Deal-Grove oxidation ──────────────────────────────────────────────
    {
        fab::CrossSection si = bare_si(500.0);
        fab::CrossSection ox = fab::deal_grove_oxidation(si, 5.0);
        ISO_CHECK_EQ_UINT(ox.layers.size(), 2u);
        ISO_CHECK_STR_EQ(ox.layers[0].material.c_str(), "SiO2");
        ISO_CHECK_EQ_DBL(ox.layers[0].thickness_nm, 5.7113938219, 1e-6);
        ISO_CHECK_STR_EQ(ox.layers[1].material.c_str(), "Si");

        fab::CrossSection ox2 = fab::deal_grove_oxidation(ox, 10.0);
        ISO_CHECK_EQ_UINT(ox2.layers.size(), 2u);
        ISO_CHECK_EQ_DBL(ox2.layers[0].thickness_nm, 16.1470982847, 1e-6);

        ISO_CHECK(throws_invalid([&] { fab::deal_grove_oxidation(si, 0.0); }));
    }

    // ── Deposition ────────────────────────────────────────────────────────
    {
        fab::CrossSection si = bare_si(500.0);
        fab::CrossSection dep = fab::deposit(si, "Poly", 100.0);
        ISO_CHECK_EQ_UINT(dep.layers.size(), 2u);
        ISO_CHECK_STR_EQ(dep.layers[0].material.c_str(), "Poly");
        ISO_CHECK(throws_invalid([&] { fab::deposit(si, "Poly", -1.0); }));
    }

    // ── Etching ───────────────────────────────────────────────────────────
    {
        fab::CrossSection dep = fab::deposit(bare_si(500.0), "Poly", 100.0);
        fab::CrossSection e1 = fab::etch(dep, "Poly", 60.0);
        ISO_CHECK_EQ_UINT(e1.layers.size(), 2u);
        ISO_CHECK_EQ_DBL(e1.layers[0].thickness_nm, 40.0, 1e-9);

        fab::CrossSection e2 = fab::etch(dep, "Poly", 150.0);
        ISO_CHECK_EQ_UINT(e2.layers.size(), 1u);
        ISO_CHECK_STR_EQ(e2.layers[0].material.c_str(), "Si");

        ISO_CHECK_EQ_UINT(fab::etch(dep, "Poly", 0.0).layers.size(), 2u);
    }

    // ── Implant-range lookup (oracle values) ──────────────────────────────
    {
        auto b30 = fab::implant_range("B", 30.0);
        ISO_CHECK_EQ_DBL(b30.first, 92.0, 1e-9);
        ISO_CHECK_EQ_DBL(b30.second, 38.0, 1e-9);
        auto b20 = fab::implant_range("B", 20.0);  // interpolated
        ISO_CHECK_EQ_DBL(b20.first, 62.5, 1e-9);
        ISO_CHECK_EQ_DBL(b20.second, 28.0, 1e-9);
        auto b5 = fab::implant_range("B", 5.0);  // below min
        ISO_CHECK_EQ_DBL(b5.first, 16.5, 1e-9);
        auto b200 = fab::implant_range("B", 200.0);  // above max
        ISO_CHECK_EQ_DBL(b200.first, 520.0, 1e-9);
        ISO_CHECK(throws_invalid([] { fab::implant_range("Xe", 30.0); }));
    }

    // ── Ion implantation (Gaussian profile) ───────────────────────────────
    {
        fab::CrossSection imp = fab::implant(bare_si(500.0), "B", 30.0, 1e15);
        const auto& doping = imp.layers[0].doping;
        auto it = doping.find("B");
        ISO_CHECK(it != doping.end());
        if (it != doping.end()) {
            ISO_CHECK_EQ_UINT(it->second.size(), 48u);
            ISO_CHECK_EQ_DBL(it->second[0].first, 2.541667, 1e-5);
            ISO_CHECK_EQ_DBL(it->second[0].second / 6.571653e18, 1.0, 1e-5);
        }

        ISO_CHECK(throws_invalid(
            [] { fab::implant(bare_si(500.0), "B", 30.0, 0.0); }));
        ISO_CHECK(throws_invalid(
            [] { fab::implant(bare_si(500.0), "Xe", 30.0, 1e15); }));
        // no Si layer
        fab::CrossSection ox;
        ox.layers.emplace_back("SiO2", 100.0);
        ISO_CHECK(throws_invalid([&] { fab::implant(ox, "B", 30.0, 1e15); }));
    }

    // ── Diffusion (preserves samples) + diffusivity ───────────────────────
    {
        fab::CrossSection imp = fab::implant(bare_si(500.0), "B", 30.0, 1e15);
        fab::CrossSection diff = fab::diffuse(imp, 30.0);
        ISO_CHECK_EQ_UINT(diff.layers[0].doping.at("B").size(), 48u);

        ISO_CHECK_EQ_DBL(fab::diffusivity_1000c("B"), 1e-14, 1e-28);
        ISO_CHECK_EQ_DBL(fab::diffusivity_cm2_per_s("B", 1000.0), 1e-14, 1e-20);
        ISO_CHECK_EQ_DBL(fab::diffusivity_cm2_per_s("B", 1100.0) / 1.163260e-14,
                         1.0, 1e-5);
    }

    return ISO_TEST_RESULT();
}
