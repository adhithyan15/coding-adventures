/*
 * Tests for the C fab-process-simulation library, using the header-only
 * iso_test.h harness (pure ISO). Reference values were captured from the real
 * Rust crate (an oracle run) so the analytical models match exactly.
 */
#include "iso_test.h"

#include "fab_process_simulation.h"

/* A fresh single-layer substrate. */
static void bare_si(FabCrossSection *cs, double thickness) {
    fab_cross_section_init(cs);
    fab_cross_section_add_layer(cs, "Si", thickness);
}

int main(void) {
    /* ── Deal-Grove oxidation (sqrt model) ───────────────────────────────── */
    {
        FabCrossSection si, ox;
        bare_si(&si, 500.0);
        ISO_CHECK(fab_deal_grove_oxidation(&si, 5.0, 0, 0, 0, 0, &ox) == FAB_OK);
        ISO_CHECK_EQ_UINT(fab_layer_count(&ox), 2u);
        ISO_CHECK_STR_EQ(fab_layer_at(&ox, 0)->material, "SiO2");
        ISO_CHECK_EQ_DBL(fab_layer_at(&ox, 0)->thickness_nm, 5.7113938219, 1e-6);
        ISO_CHECK_STR_EQ(fab_layer_at(&ox, 1)->material, "Si");
        ISO_CHECK_EQ_DBL(fab_layer_at(&ox, 1)->thickness_nm, 500.0, 1e-9);

        /* A second oxidation grows the existing oxide (tau accounting). */
        FabCrossSection ox2;
        ISO_CHECK(fab_deal_grove_oxidation(&ox, 10.0, 0, 0, 0, 0, &ox2) ==
                  FAB_OK);
        ISO_CHECK_EQ_UINT(fab_layer_count(&ox2), 2u); /* oxide replaced */
        ISO_CHECK_EQ_DBL(fab_layer_at(&ox2, 0)->thickness_nm, 16.1470982847,
                         1e-6);

        /* Non-positive time is an error. */
        FabCrossSection bad;
        ISO_CHECK(fab_deal_grove_oxidation(&si, 0.0, 0, 0, 0, 0, &bad) ==
                  FAB_ERR_INVALID);

        fab_cross_section_free(&ox2);
        fab_cross_section_free(&ox);
        fab_cross_section_free(&si);
    }

    /* ── Deposition ──────────────────────────────────────────────────────── */
    {
        FabCrossSection si, dep;
        bare_si(&si, 500.0);
        ISO_CHECK(fab_deposit(&si, "Poly", 100.0, &dep) == FAB_OK);
        ISO_CHECK_EQ_UINT(fab_layer_count(&dep), 2u);
        ISO_CHECK_STR_EQ(fab_layer_at(&dep, 0)->material, "Poly");
        ISO_CHECK_EQ_DBL(fab_layer_at(&dep, 0)->thickness_nm, 100.0, 1e-9);
        ISO_CHECK_STR_EQ(fab_layer_at(&dep, 1)->material, "Si");

        FabCrossSection bad;
        ISO_CHECK(fab_deposit(&si, "Poly", -1.0, &bad) == FAB_ERR_INVALID);
        fab_cross_section_free(&dep);
        fab_cross_section_free(&si);
    }

    /* ── Etching (material-selective, top-down) ──────────────────────────── */
    {
        FabCrossSection si, dep, etched;
        bare_si(&si, 500.0);
        ISO_CHECK(fab_deposit(&si, "Poly", 100.0, &dep) == FAB_OK);

        /* Etch 60 nm of Poly -> 40 nm Poly remains on top. */
        ISO_CHECK(fab_etch(&dep, "Poly", 60.0, &etched) == FAB_OK);
        ISO_CHECK_EQ_UINT(fab_layer_count(&etched), 2u);
        ISO_CHECK_STR_EQ(fab_layer_at(&etched, 0)->material, "Poly");
        ISO_CHECK_EQ_DBL(fab_layer_at(&etched, 0)->thickness_nm, 40.0, 1e-9);
        fab_cross_section_free(&etched);

        /* Etch 150 nm of Poly -> Poly fully removed, stops at Si. */
        ISO_CHECK(fab_etch(&dep, "Poly", 150.0, &etched) == FAB_OK);
        ISO_CHECK_EQ_UINT(fab_layer_count(&etched), 1u);
        ISO_CHECK_STR_EQ(fab_layer_at(&etched, 0)->material, "Si");
        fab_cross_section_free(&etched);

        /* Zero depth is unchanged. */
        ISO_CHECK(fab_etch(&dep, "Poly", 0.0, &etched) == FAB_OK);
        ISO_CHECK_EQ_UINT(fab_layer_count(&etched), 2u);
        fab_cross_section_free(&etched);

        fab_cross_section_free(&dep);
        fab_cross_section_free(&si);
    }

    /* ── Implant-range lookup + interpolation (oracle values) ────────────── */
    {
        double rp, sd;
        ISO_CHECK(fab_implant_range("B", 30.0, &rp, &sd) == FAB_OK);
        ISO_CHECK_EQ_DBL(rp, 92.0, 1e-9);
        ISO_CHECK_EQ_DBL(sd, 38.0, 1e-9);
        ISO_CHECK(fab_implant_range("B", 20.0, &rp, &sd) == FAB_OK); /* interp */
        ISO_CHECK_EQ_DBL(rp, 62.5, 1e-9);
        ISO_CHECK_EQ_DBL(sd, 28.0, 1e-9);
        ISO_CHECK(fab_implant_range("B", 5.0, &rp, &sd) == FAB_OK); /* below */
        ISO_CHECK_EQ_DBL(rp, 16.5, 1e-9);
        ISO_CHECK_EQ_DBL(sd, 9.0, 1e-9);
        ISO_CHECK(fab_implant_range("B", 200.0, &rp, &sd) == FAB_OK); /* above */
        ISO_CHECK_EQ_DBL(rp, 520.0, 1e-9);
        ISO_CHECK_EQ_DBL(sd, 160.0, 1e-9);
        ISO_CHECK(fab_implant_range("P", 100.0, &rp, &sd) == FAB_OK);
        ISO_CHECK_EQ_DBL(rp, 130.0, 1e-9);
        ISO_CHECK(fab_implant_range("As", 30.0, &rp, &sd) == FAB_OK);
        ISO_CHECK_EQ_DBL(rp, 22.0, 1e-9);
        ISO_CHECK(fab_implant_range("Xe", 30.0, &rp, &sd) ==
                  FAB_ERR_UNKNOWN_SPECIES);
    }

    /* ── Ion implantation (Gaussian profile, exp model) ──────────────────── */
    {
        FabCrossSection si, imp;
        bare_si(&si, 500.0);
        ISO_CHECK(fab_implant(&si, "B", 30.0, 1e15, &imp) == FAB_OK);
        const FabDoping *prof = fab_layer_doping(fab_layer_at(&imp, 0), "B");
        ISO_CHECK(prof != NULL);
        if (prof) {
            ISO_CHECK_EQ_UINT(prof->n_samples, 48u);
            ISO_CHECK_EQ_DBL(prof->samples[0].depth_nm, 2.541667, 1e-5);
            /* Oracle first-sample concentration (relative check for the
             * 6-significant-figure oracle value). */
            ISO_CHECK_EQ_DBL(prof->samples[0].conc_per_cm3 / 6.571653e18, 1.0,
                             1e-5);
        }
        fab_cross_section_free(&imp);

        /* Implant errors. */
        FabCrossSection bad;
        ISO_CHECK(fab_implant(&si, "B", 30.0, 0.0, &bad) == FAB_ERR_INVALID);
        ISO_CHECK(fab_implant(&si, "Xe", 30.0, 1e15, &bad) ==
                  FAB_ERR_UNKNOWN_SPECIES);
        fab_cross_section_free(&si);

        /* No Si layer -> FAB_ERR_NO_SI. */
        FabCrossSection oxide_only;
        fab_cross_section_init(&oxide_only);
        fab_cross_section_add_layer(&oxide_only, "SiO2", 100.0);
        ISO_CHECK(fab_implant(&oxide_only, "B", 30.0, 1e15, &bad) ==
                  FAB_ERR_NO_SI);
        fab_cross_section_free(&oxide_only);
    }

    /* ── Diffusion (v0.1.0 preserves samples) + diffusivity ──────────────── */
    {
        FabCrossSection si, imp, diff;
        bare_si(&si, 500.0);
        ISO_CHECK(fab_implant(&si, "B", 30.0, 1e15, &imp) == FAB_OK);
        ISO_CHECK(fab_diffuse(&imp, 30.0, 0, 0.0, &diff) == FAB_OK);
        const FabDoping *p = fab_layer_doping(fab_layer_at(&diff, 0), "B");
        ISO_CHECK(p != NULL);
        if (p) {
            ISO_CHECK_EQ_UINT(p->n_samples, 48u); /* preserved */
        }
        fab_cross_section_free(&diff);
        fab_cross_section_free(&imp);
        fab_cross_section_free(&si);

        /* Arrhenius (T^2) diffusivity. */
        ISO_CHECK_EQ_DBL(fab_diffusivity_1000c("B"), 1e-14, 1e-28);
        ISO_CHECK_EQ_DBL(fab_diffusivity_cm2_per_s("B", 1000.0), 1e-14, 1e-20);
        ISO_CHECK_EQ_DBL(fab_diffusivity_cm2_per_s("B", 1100.0) / 1.163260e-14,
                         1.0, 1e-5);
    }

    return ISO_TEST_RESULT();
}
