import assert from "node:assert/strict";
import test from "node:test";
import { sampleCountForDuration } from "@coding-adventures/oscillator";
import { assertProperty, Gen } from "@coding-adventures/test-bootstrap";

import {
  applyHihatChoke,
  CHOKE_RELEASE_RAMP_SECONDS,
  getPercussionVoice,
  PercussionEnvelope,
  PercussionMode,
  renderPercussionHit,
  SplitMix64,
} from "../src/index.js";

test("PBT: PercussionEnvelope valueAt is bounded in [0.0, 1.0]", () => {
  const envGen = new Gen((rng) => {
    const u1 = Number(rng.nextU64() & 0xffffn) / 65535.0;
    const u2 = Number(rng.nextU64() & 0xffffn) / 65535.0;
    const u3 = Number(rng.nextU64() & 0xffffn) / 65535.0;
    const uTime = Number(rng.nextU64() & 0xffffn) / 65535.0;

    const atk = 0.001 + u1 * 0.05;
    const hold = 0.001 + u2 * 0.05;
    const dec = 0.01 + u3 * 0.5;

    const env = new PercussionEnvelope(atk, hold, dec);
    const hitDur = 0.05;
    const time = uTime * (atk + hold + dec * 3.0);
    return { env, time, hitDur };
  });

  assertProperty(
    envGen,
    ({ env, time, hitDur }) => {
      const val = env.valueAt(time, hitDur);
      assert.ok(
        val >= -1e-9 && val <= 1.0 + 1e-9,
        `Envelope value ${val} out of [0.0, 1.0] at time ${time}`,
      );
      return true;
    },
    { numTests: 60 },
  );
});

test("PBT: PercussionMode envelope amplitude decays monotonically over time", () => {
  const modeGen = new Gen((rng) => {
    const uFreq = Number(rng.nextU64() & 0xffffn) / 65535.0;
    const uAmp = Number(rng.nextU64() & 0xffffn) / 65535.0;
    const uDec = Number(rng.nextU64() & 0xffffn) / 65535.0;

    const freq = 50.0 + uFreq * 500.0;
    const amp = 0.1 + uAmp * 0.9;
    const dec = 0.01 + uDec * 0.2;

    const mode = new PercussionMode(freq, amp, dec);
    return mode;
  });

  assertProperty(
    modeGen,
    (mode) => {
      // Test exponential envelope decay at t1 and t2 (t2 > t1)
      const t1 = 0.05;
      const t2 = 0.15;
      const env1 = Math.exp(-t1 / mode.decaySeconds);
      const env2 = Math.exp(-t2 / mode.decaySeconds);
      assert.ok(env2 <= env1 + 1e-9, `Decay envelope at ${t2} (${env2}) must be <= at ${t1} (${env1})`);
      return true;
    },
    { numTests: 40 },
  );
});

test("PBT: SplitMix64 determinism invariant: identical seed produces identical sequence", () => {
  const seedGen = new Gen((rng) => rng.nextU64());

  assertProperty(
    seedGen,
    (seed) => {
      const prng1 = new SplitMix64(seed);
      const prng2 = new SplitMix64(seed);

      for (let i = 0; i < 20; i += 1) {
        assert.equal(prng1.nextU64(), prng2.nextU64());
        assert.equal(prng1.nextFloatSigned(), prng2.nextFloatSigned());
        assert.equal(prng1.nextFloatUnit(), prng2.nextFloatUnit());
      }
      return true;
    },
    { numTests: 40 },
  );
});

test("PBT: applyHihatChoke produces non-increasing ramp during release window", () => {
  const chokeGen = new Gen((rng) => {
    const uTime = Number(rng.nextU64() & 0xffffn) / 65535.0;
    const chokeTime = 0.01 + uTime * 0.05; // [10ms, 60ms]
    return chokeTime;
  });

  assertProperty(
    chokeGen,
    (chokeTime) => {
      const sampleRate = 20_000.0;
      const len = 2000;
      const samples = new Float64Array(len).fill(1.0);

      const choked = applyHihatChoke(samples, 0.0, chokeTime, sampleRate, CHOKE_RELEASE_RAMP_SECONDS);

      const startIdx = Math.max(0, Math.floor(chokeTime * sampleRate));
      const endIdx = Math.min(len, Math.ceil((chokeTime + CHOKE_RELEASE_RAMP_SECONDS) * sampleRate));

      for (let i = startIdx; i < endIdx - 1; i += 1) {
        assert.ok(
          choked[i] >= choked[i + 1] - 1e-9,
          `Choke ramp not monotonic at index ${i}: ${choked[i]} < ${choked[i + 1]}`,
        );
      }
      return true;
    },
    { numTests: 30 },
  );
});

test("PBT: Rendered percussion hit sample count matches sampleCountForDuration", () => {
  const hitGen = new Gen((rng) => {
    const uDur = Number(rng.nextU64() & 0xffffn) / 65535.0;
    const dur = 0.02 + uDur * 0.1; // [0.02, 0.12]
    return dur;
  });

  const kick = getPercussionVoice("kick");

  assertProperty(
    hitGen,
    (dur) => {
      const sampleRate = 44_100.0;
      const rendered = renderPercussionHit("kick", dur, {
        sampleRateHz: sampleRate,
      });
      const totalDur = kick.envelope.renderedDurationSeconds(dur);
      const expectedCount = sampleCountForDuration(totalDur, sampleRate);
      assert.equal(rendered.floatingSamples.sampleCount(), expectedCount);
      return true;
    },
    { numTests: 30 },
  );
});
