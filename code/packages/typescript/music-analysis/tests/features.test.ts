import assert from "node:assert/strict";
import test from "node:test";

import {
  BudgetExceededError,
  InvalidInputError,
  rmsEnergy,
  rmsEnergyFramed,
  rms_energy,
  rms_energy_framed,
  spectralCentroid,
  spectralFlux,
  spectralFluxSequence,
  spectralRolloff,
  spectral_centroid,
  spectral_flux,
  spectral_flux_sequence,
  spectral_rolloff,
  zeroCrossingRate,
  zeroCrossingRateFramed,
  zero_crossing_rate,
  zero_crossing_rate_framed,
} from "../src/index.js";

test("spectralCentroid: single tone peak", () => {
  // 5 bins: [0, 2500, 5000, 7500, 10000] Hz for sr=20000
  // Peak at bin 2 (5000 Hz)
  const spectrum = [0.0, 0.0, 1.0, 0.0, 0.0];
  const c = spectralCentroid(spectrum, 20000.0);
  assert.ok(Math.abs(c - 5000.0) < 1e-3);
});

test("spectralCentroid: flat spectrum center of mass", () => {
  // Flat spectrum centroid should be exactly at Nyquist / 2
  const spectrum = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
  const c = spectralCentroid(spectrum, 16000.0);
  assert.ok(Math.abs(c - 4000.0) < 1e-3);
});

test("spectralCentroid: zero energy and single bin", () => {
  assert.equal(spectralCentroid([0.0, 0.0, 0.0, 0.0, 0.0], 44100.0), 0.0);
  assert.equal(spectralCentroid([1.0], 44100.0), 0.0);
});

test("spectralFlux: frame pairs with rectified flag", () => {
  const spec1 = [1.0, 2.0, 3.0, 4.0];
  const spec2 = [1.0, 2.0, 3.0, 4.0];
  // No change
  assert.equal(spectralFlux(spec1, spec2, true), 0.0);

  // Increase in upper bins
  const spec3 = [1.0, 2.0, 5.0, 7.0];
  const flux = spectralFlux(spec1, spec3, true);
  assert.ok(Math.abs(flux - 5.0) < 1e-5);

  // Decrease with rectified=true should be 0
  const spec4 = [0.5, 1.0, 1.5, 2.0];
  assert.equal(spectralFlux(spec1, spec4, true), 0.0);

  // Decrease with rectified=false should be sum of squared diffs
  const unrectified = spectralFlux(spec1, spec4, false);
  assert.ok(unrectified > 0.0);
});

test("spectralFluxSequence: successive frames", () => {
  const frames = [
    [1.0, 1.0],
    [2.0, 2.0],
    [1.0, 1.0],
  ];
  const fluxes = spectralFluxSequence(frames, true);
  assert.equal(fluxes.length, 2);
  assert.ok(Math.abs(fluxes[0] - 2.0) < 1e-5);
  assert.ok(Math.abs(fluxes[1] - 0.0) < 1e-5); // rectified decrease

  const singleFrame = [[1.0, 2.0]];
  assert.equal(spectralFluxSequence(singleFrame, true).length, 0);
});

test("spectralRolloff: frequency cutoff", () => {
  // 5 bins: [0, 2500, 5000, 7500, 10000] Hz for sr=20000
  const spectrum = [0.0, 0.0, 1.0, 0.0, 0.0];
  // All energy is at 5000 Hz
  const ro = spectralRolloff(spectrum, 20000.0, 0.85);
  assert.ok(Math.abs(ro - 5000.0) < 1e-3);

  // Flat spectrum
  const spectrumFlat = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
  const roFlat = spectralRolloff(spectrumFlat, 18000.0, 0.5);
  assert.ok(roFlat > 0.0 && roFlat < 9000.0);

  // Zero energy
  assert.equal(spectralRolloff([0.0, 0.0, 0.0, 0.0, 0.0], 20000.0, 0.85), 0.0);
  assert.equal(spectralRolloff([1.0], 20000.0, 0.85), 0.0);

  // 1.0 rolloff percentage
  const res = spectralRolloff([1.0, 2.0, 3.0], 44100.0, 1.0);
  assert.ok(Math.abs(res - 22050.0) < 1e-3);
});

test("zeroCrossingRate: DC, alternating, and framed", () => {
  // Constant positive -> 0
  assert.equal(zeroCrossingRate([1.0, 1.0, 1.0, 1.0]), 0.0);

  // Alternating -> 1.0
  assert.equal(zeroCrossingRate([1.0, -1.0, 1.0, -1.0]), 1.0);

  // Single sample
  assert.equal(zeroCrossingRate([1.0]), 0.0);

  // Sliding window framing
  const sig = [1.0, 1.0, -1.0, -1.0, 1.0, 1.0];
  const zcrFramed = zeroCrossingRateFramed(sig, 3, 1);
  assert.equal(zcrFramed.length, 4);

  // Short signal with framing
  const shortFramed = zeroCrossingRateFramed([1.0, -1.0], 10, 5);
  assert.equal(shortFramed.length, 1);
});

test("CWE-20: zeroCrossingRateFramed frameLength <= 1 prevention of 0/0=NaN", () => {
  const sig = [1.0, -1.0, 1.0, -1.0, 1.0];
  const zcr = zeroCrossingRateFramed(sig, 1, 1);
  assert.equal(zcr.length, 5);
  for (const val of zcr) {
    assert.ok(!Number.isNaN(val), "zcr value must not be NaN");
    assert.equal(val, 0.0, "frame of length 1 has 0 crossings, rate must be 0.0");
  }

  const zcrHop2 = zeroCrossingRateFramed(sig, 1, 2);
  assert.equal(zcrHop2.length, 3);
  for (const val of zcrHop2) {
    assert.ok(!Number.isNaN(val));
    assert.equal(val, 0.0);
  }
});

test("rmsEnergy: DC, silence, sine wave, and framed", () => {
  // DC signal
  assert.ok(Math.abs(rmsEnergy([2.0, 2.0, 2.0]) - 2.0) < 1e-5);

  // Zero signal
  assert.equal(rmsEnergy([0.0, 0.0]), 0.0);

  // Sine wave RMS should be A / sqrt(2)
  const n = 1000;
  const sine = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    sine[i] = Math.sin((2.0 * Math.PI * 5.0 * i) / n);
  }
  const rms = rmsEnergy(sine);
  assert.ok(Math.abs(rms - 1.0 / Math.SQRT2) < 1e-2);

  // Windowed framing
  const sig = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
  const rmsFramed = rmsEnergyFramed(sig, 4, 2);
  assert.equal(rmsFramed.length, 4);
  for (const val of rmsFramed) {
    assert.ok(Math.abs(val - 1.0) < 1e-5);
  }

  // Short signal with framing
  const shortFramed = rmsEnergyFramed([2.0, 2.0], 10, 5);
  assert.equal(shortFramed.length, 1);
  assert.ok(Math.abs(shortFramed[0] - 2.0) < 1e-5);
});

test("features: snake-case aliases match camelCase", () => {
  const spec = [0.1, 0.5, 0.8, 0.2];
  const sig = [0.1, -0.2, 0.3, -0.4];
  assert.equal(spectral_centroid(spec, 44100.0), spectralCentroid(spec, 44100.0));
  assert.equal(spectral_flux(spec, spec, true), spectralFlux(spec, spec, true));
  assert.deepEqual(spectral_flux_sequence([spec, spec], true), spectralFluxSequence([spec, spec], true));
  assert.equal(spectral_rolloff(spec, 44100.0, 0.85), spectralRolloff(spec, 44100.0, 0.85));
  assert.equal(zero_crossing_rate(sig), zeroCrossingRate(sig));
  assert.deepEqual(zero_crossing_rate_framed(sig, 2, 1), zeroCrossingRateFramed(sig, 2, 1));
  assert.equal(rms_energy(sig), rmsEnergy(sig));
  assert.deepEqual(rms_energy_framed(sig, 2, 1), rmsEnergyFramed(sig, 2, 1));
});

test("features: invalid and out-of-bounds inputs", () => {
  assert.throws(() => spectralCentroid([], 44100.0), InvalidInputError);
  assert.throws(() => spectralCentroid([1.0, 2.0], -100.0), InvalidInputError);
  assert.throws(() => spectralCentroid([1.0, 2.0], 200000.0), BudgetExceededError);
  assert.throws(() => spectralRolloff([1.0, 2.0], 44100.0, 0.0), InvalidInputError);
  assert.throws(() => spectralRolloff([1.0, 2.0], 44100.0, 1.5), BudgetExceededError);
  assert.throws(() => spectralFluxSequence([], true), InvalidInputError);
  assert.throws(() => rmsEnergy([]), InvalidInputError);
  assert.throws(() => zeroCrossingRate([]), InvalidInputError);
});
