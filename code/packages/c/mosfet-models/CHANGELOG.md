# Changelog

All notable changes to the `mosfet-models` (C) package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Added

- **Initial package — SPICE Level-1 (Shockley) MOSFET I-V model** (CCPP02 port
  campaign, bucket A / pure-ISO). The C port of the Rust `mosfet-models` crate:
  the classical square-law MOSFET model with body effect, channel-length
  modulation, an optional subthreshold branch, small-signal conductances, and the
  Meyer capacitance model. A pure-ISO crate (no OS), so it rides the `iso-harness`
  (links nothing, strict-conformance flags on).
  - **API.** `mos_level1_params_default`; `mos_evaluate_level1` (bias point →
    `MosResult` in NMOS convention); `mosfet_dc` (NMOS/PMOS, handling PMOS sign
    flips — subsumes the Rust `Mosfet::dc` and `Level1Model::dc`); `mos_region_str`.
    `MosLevel1Params` (15 parameters + a subthreshold-enable flag), `MosRegion`
    (cutoff / subthreshold / triode / saturation), `MosResult` (Id + gm/gds/gmb +
    caps + region) — all by value, no allocation.
  - **Composes `c/device-physics` + `c/float-math`.** The thermal voltage kT/q
    comes from `dp_thermal_voltage`; the `sqrt`/`exp` from the from-scratch
    `fm_sqrt`/`fm_exp` — no libm. `run.sh` compiles both packages' sources in;
    nothing is linked. `BUILD` declares
    `deps=c/iso-harness c/device-physics c/float-math`.
  - **Faithfulness.** Direct transcription of the Rust equations: threshold with
    body effect (clamped under forward bias), V_OV region split (cutoff /
    subthreshold / triode / saturation), CLM via λ, `gmb = −gm·dV_t/dV_BS`, and
    the piecewise Meyer + W/L-scaled overlap capacitances. PMOS negates inputs and
    Id.
  - **Test (`tests/mosfet_models_test.c`).** The Rust integration tests: parameter
    defaults, region detection, sign of gm/gds/gmb, `gds ≈ 0` without CLM, the
    body effect, PMOS negative-Id and NMOS/PMOS magnitude match, `Level1Model.dc`,
    `Region::as_str`, non-negative capacitances, overlap-cap W-scaling, and the
    saturation Id formula cross-check. 45 checks, verified under gcc + clang with
    `-pedantic-errors`, clean under ASan+UBSan, 0 leaks.
