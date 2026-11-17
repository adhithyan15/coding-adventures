import assert from "node:assert/strict";
import test from "node:test";
import {
  getInstrument,
  renderInstrumentNote,
} from "@coding-adventures/musical-instruments";

import {
  extractHarmonicProfile,
  extractHarmonicProfileSimple,
  extract_harmonic_profile,
  extract_harmonic_profile_simple,
  InvalidInputError,
} from "../src/index.js";

function synthesizeKnownInstrument(
  f0: number,
  partialAmps: number[],
  sr: number,
  durationSec: number,
): Float64Array {
  const n = Math.floor(sr * durationSec);
  const out = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    const t = i / sr;
    // Attack - decay - sustain - release envelope
    const env =
      t < 0.05
        ? t / 0.05
        : t < 0.15
          ? 1.0 - 0.3 * ((t - 0.05) / 0.10)
          : t < 0.35
            ? 0.70
            : Math.max(0.0, 0.70 * (1.0 - (t - 0.35) / 0.05));

    let sample = 0.0;
    for (let hIdx = 0; hIdx < partialAmps.length; hIdx++) {
      const h = hIdx + 1;
      sample += partialAmps[hIdx] * Math.sin(2.0 * Math.PI * h * f0 * t);
    }
    out[i] = env * sample;
  }
  return out;
}

test("extractHarmonicProfile: synthetic instrument extraction", () => {
  const sr = 44100.0;
  const f0 = 440.0;
  // Fundamental (1.0), 2nd harmonic (0.5), 3rd harmonic (0.25)
  const knownPartials = [1.0, 0.5, 0.25];
  const sig = synthesizeKnownInstrument(f0, knownPartials, sr, 0.4);

  const profile = extractHarmonicProfile(
    sig,
    sr,
    f0,
    4,
    "test_synth",
    "Test Synth",
  );

  assert.equal(profile.id, "test_synth");
  assert.equal(profile.synthesisKind, "additive");
  assert.ok(profile.harmonicProfile.length >= 3);

  // Fundamental should have multiplier 1.0 and amplitude near 1.0
  const h1 = profile.harmonicProfile[0];
  assert.equal(h1.frequencyMultiplier, 1.0);
  assert.ok(Math.abs(h1.amplitude - 1.0) < 0.05);

  // 2nd harmonic should be near 0.5
  const h2 = profile.harmonicProfile[1];
  assert.equal(h2.frequencyMultiplier, 2.0);
  assert.ok(Math.abs(h2.amplitude - 0.5) < 0.1);

  // 3rd harmonic should be near 0.25
  const h3 = profile.harmonicProfile[2];
  assert.equal(h3.frequencyMultiplier, 3.0);
  assert.ok(Math.abs(h3.amplitude - 0.25) < 0.1);

  // ADSR envelope validation
  const env = profile.envelopeProfile;
  assert.ok(env.attackSeconds > 0.0);
  assert.ok(env.sustainLevel > 0.0 && env.sustainLevel <= 1.0);
});

test("extractHarmonicProfile: re-synthesis parity with musical-instruments preset", () => {
  const sr = 44100.0;
  // Render an original note from flute preset in musical-instruments
  const preset = getInstrument("flute_naive");
  const originalRender = renderInstrumentNote("A4", 0.3, {
    instrument: preset,
    sampleRateHz: sr,
  });

  const originalSamples = originalRender.floatingSamples.samples;

  // Extract profile from rendered samples
  const extractedProfile = extractHarmonicProfile(
    originalSamples,
    sr,
    440.0,
    6,
    "extracted_flute",
    "Extracted Flute",
  );

  assert.equal(extractedProfile.synthesisKind, "additive");
  assert.ok(extractedProfile.harmonicProfile.length > 0);
  assert.equal(extractedProfile.harmonicProfile[0].amplitude, 1.0);

  // Re-synthesize using the extracted profile
  const resynthRender = renderInstrumentNote("A4", 0.3, {
    instrument: extractedProfile,
    sampleRateHz: sr,
  });

  const resynthSamples = resynthRender.floatingSamples.samples;
  assert.ok(resynthSamples.length > 0);
  let resynthEnergy = 0.0;
  for (const s of resynthSamples) {
    resynthEnergy += s * s;
  }
  assert.ok(resynthEnergy > 0.0);
});

test("extractHarmonicProfile: long buffer with auto f0 detection", () => {
  const sr = 44100.0;
  const n = 6000;
  const phaseInc = (2.0 * Math.PI * 440.0) / sr;
  const sig = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    sig[i] = 0.8 * Math.sin(i * phaseInc);
  }

  const profile = extractHarmonicProfile(
    sig,
    sr,
    null, // auto-detect via yin_pitch
    30,
    "extracted_auto",
    "Extracted Auto",
  );
  assert.equal(profile.synthesisKind, "additive");
  assert.ok(profile.harmonicProfile.length > 0);
});

test("harmonic: aliases match camelCase", () => {
  const sr = 44100.0;
  const sig = synthesizeKnownInstrument(440.0, [1.0, 0.5], sr, 0.2);
  const p1 = extractHarmonicProfileSimple(sig, sr);
  const p2 = extract_harmonic_profile_simple(sig, sr);
  assert.equal(p1.id, p2.id);
  assert.equal(p1.harmonicProfile.length, p2.harmonicProfile.length);
  const p3 = extract_harmonic_profile(sig, sr, 440.0, 4);
  assert.equal(p3.harmonicProfile.length, 4);
});

test("extractHarmonicProfile: errors on silence and empty buffer", () => {
  const sr = 44100.0;
  assert.throws(
    () => extractHarmonicProfile(new Float64Array(2000), sr),
    InvalidInputError,
  );
  assert.throws(
    () => extractHarmonicProfile([], sr),
    InvalidInputError,
  );
});
