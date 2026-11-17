import assert from "node:assert/strict";
import test from "node:test";

import {
  ALL_STANDARD_HIT_IDS,
  allDrumKits,
  allStandardHitIds,
  applyHihatChoke,
  calcHitSampleCount,
  canonicalHitId,
  CHOKE_RELEASE_RAMP_SECONDS,
  DEFAULT_MAX_MODE_COUNT,
  DEFAULT_MAX_SAMPLE_COUNT,
  DEFAULT_SAMPLE_RATE_HZ,
  DrumKitProfile,
  getDrumKit,
  getPercussionVoice,
  isChokedBy,
  isModeCountSafe,
  MAX_HIT_DURATION_SECONDS,
  MAX_HITS,
  MAX_PERCUSSION_MODES,
  MAX_SAMPLE_COUNT,
  MAX_SAMPLE_RATE_HZ,
  MAX_SEQUENCE_EVENTS,
  PercussionEnvelope,
  PercussionHitEvent,
  PercussionHitRender,
  PercussionMode,
  PercussionNoiseProfile,
  PercussionSequenceRender,
  PercussionSignal,
  PercussionVoiceProfile,
  pseudoNoise,
  renderPercussionHit,
  renderPercussionHitPcm,
  renderPercussionSequence,
  SplitMix64,
  splitmix64Noise,
  standardKit,
  validateHitCountCeiling,
  validateModeCountCeiling,
  validateSequenceEventCountCeiling,
} from "../src/index.js";

const EPSILON = 1e-6;

function assertClose(actual: number, expected: number, eps = EPSILON, msg?: string): void {
  assert.ok(
    Math.abs(actual - expected) <= eps,
    `${msg ?? "assertClose"}: expected ${expected}, got ${actual} (diff ${Math.abs(actual - expected)})`,
  );
}

test("Constants match MUS07 specifications", () => {
  assert.equal(MAX_PERCUSSION_MODES, 32);
  assert.equal(MAX_HIT_DURATION_SECONDS, 30.0);
  assert.equal(MAX_SAMPLE_RATE_HZ, 192_000.0);
  assert.equal(MAX_SAMPLE_COUNT, 16_777_216);
  assert.equal(DEFAULT_MAX_MODE_COUNT, 32);
  assert.equal(DEFAULT_MAX_SAMPLE_COUNT, 10_000_000);
  assert.equal(DEFAULT_SAMPLE_RATE_HZ, 44_100.0);
  assert.equal(CHOKE_RELEASE_RAMP_SECONDS, 0.015);
  assert.equal(MAX_HITS, 65_536);
  assert.equal(MAX_SEQUENCE_EVENTS, 16_384);
  assert.ok(ALL_STANDARD_HIT_IDS.length >= 15);
});

test("Standard kit has 16 expected primary voices plus aliases", () => {
  const kit = standardKit();
  assert.equal(kit.id, "standard_kit");
  assert.equal(kit.displayName, "Standard Kit");
  assert.ok(kit.voiceCount >= 15);

  const expectedVoices = [
    "kick",
    "snare",
    "rimshot",
    "closed_hihat",
    "open_hihat",
    "pedal_hihat",
    "high_tom",
    "mid_tom",
    "low_tom",
    "crash",
    "ride",
    "clap",
    "cowbell",
    "tambourine",
    "shaker",
  ];

  for (const voiceId of expectedVoices) {
    assert.ok(kit.hasVoice(voiceId), `Standard kit must have voice ${voiceId}`);
    const voice = kit.getVoice(voiceId);
    assert.ok(voice instanceof PercussionVoiceProfile);
  }

  // Aliases resolve correctly
  assert.equal(kit.resolveHitId("bd"), "kick");
  assert.equal(kit.resolveHitId("bass_drum"), "kick");
  assert.equal(kit.resolveHitId("sd"), "snare");
  assert.equal(kit.resolveHitId("snare_acoustic"), "snare");
  assert.equal(kit.resolveHitId("ch"), "closed_hihat");
  assert.equal(kit.resolveHitId("oh"), "open_hihat");
  assert.equal(kit.resolveHitId("ph"), "pedal_hihat");
  assert.equal(kit.resolveHitId("rimshot"), "rimshot");
  assert.equal(canonicalHitId("side_stick"), "rimshot");
});

test("getDrumKit and getPercussionVoice lookup routines", () => {
  const std = getDrumKit("standard_kit");
  assert.equal(std.id, "standard_kit");
  assert.throws(() => getDrumKit("nonexistent"), /Unknown drum kit/);

  const kits = allDrumKits();
  assert.equal(kits.length, 1);
  assert.equal(kits[0].id, "standard_kit");

  const kick = getPercussionVoice("kick");
  assert.equal(kick.id, "kick_naive");
  assert.equal(kick.family, "kick");

  const snareFromAlias = getPercussionVoice("sd");
  assert.equal(snareFromAlias.family, "snare");

  assert.throws(() => getPercussionVoice("mystery_sound"), /Unknown percussion voice/);
});

test("PercussionMode validation and parameters", () => {
  const mode = new PercussionMode(200.0, 0.8, 0.05, 0.1);
  assert.equal(mode.frequencyHz, 200.0);
  assert.equal(mode.amplitude, 0.8);
  assert.equal(mode.decaySeconds, 0.05);
  assert.equal(mode.phaseCycles, 0.1);

  // Validation
  assert.throws(() => new PercussionMode(0, 0.8, 0.05), /frequencyHz/);
  assert.throws(() => new PercussionMode(200.0, -0.1, 0.05), /amplitude/);
  assert.throws(() => new PercussionMode(200.0, 0.8, -0.01), /decaySeconds/);
  assert.throws(() => new PercussionMode(200.0, 0.8, 0.0), /decaySeconds must be > 0.0/);
});

test("PercussionEnvelope covers attack, hold, and decay stages", () => {
  const env = new PercussionEnvelope(0.01, 0.02, 0.1);
  assert.equal(env.attackSeconds, 0.01);
  assert.equal(env.holdSeconds, 0.02);
  assert.equal(env.decaySeconds, 0.1);
  assert.equal(env.renderedDurationSeconds(0.5), 0.6);

  const hitDur = 0.02; // sustainEnd = 0.01 + 0.02 = 0.03
  // At t=0, attack starts at 0.0
  assertClose(env.valueAt(0.0, hitDur), 0.0);
  // Halfway through attack
  assertClose(env.valueAt(0.005, hitDur), 0.5);
  // End of attack / hold
  assertClose(env.valueAt(0.01, hitDur), 1.0);
  assertClose(env.valueAt(0.025, hitDur), 1.0);
  // During decay
  assert.ok(env.valueAt(0.05, hitDur) < 1.0 && env.valueAt(0.05, hitDur) > 0.0);
  // Far into decay
  assert.equal(env.valueAt(2.0, hitDur), 0.0);

  // Zero attack time returns 1.0 immediately
  const zeroAtk = new PercussionEnvelope(0.0, 0.01, 0.05);
  assertClose(zeroAtk.valueAt(0.0, 0.01), 1.0);

  // Validation
  assert.throws(() => new PercussionEnvelope(-1, 0.02, 0.1), /attackSeconds/);
  assert.throws(() => new PercussionEnvelope(0.01, -1, 0.1), /holdSeconds/);
  assert.throws(() => new PercussionEnvelope(0.01, 0.02, -1), /decaySeconds/);
  assert.throws(() => new PercussionEnvelope(0.01, 0.02, 0.0), /decaySeconds must be > 0.0/);
});

test("PercussionNoiseProfile and pseudoNoise evaluation", () => {
  const noise = new PercussionNoiseProfile(0.5, 0.1, 12345n, "splitmix64");
  assert.equal(noise.algorithm, "splitmix64");
  assert.equal(noise.decaySeconds, 0.1);
  assert.equal(noise.amplitude, 0.5);
  assert.equal(noise.seed, 12345n);

  const sample = noise.sampleNoise(10);
  assert.ok(Number.isFinite(sample));
  assert.ok(sample >= -1.0 && sample <= 1.0);

  // Pseudo-noise algorithm branch
  const pseudoProfile = new PercussionNoiseProfile(0.4, 0.05, 99, "pseudo_noise");
  assert.equal(pseudoProfile.algorithm, "pseudo_noise");
  const pseudoSample = pseudoProfile.sampleNoise(5);
  assert.ok(Number.isFinite(pseudoSample));

  // pseudoNoise helper directly
  const val1 = pseudoNoise(0, 1000);
  assert.ok(Number.isFinite(val1));
  assert.ok(val1 >= -1.0 && val1 <= 1.0);

  // Validation
  assert.throws(() => new PercussionNoiseProfile(-0.1, 0.1), /amplitude/);
  assert.throws(() => new PercussionNoiseProfile(0.5, -0.1), /decaySeconds/);
  assert.throws(() => new PercussionNoiseProfile(0.5, 0.0), /decaySeconds must be > 0.0/);
});

test("SplitMix64 and splitmix64Noise determinism", () => {
  const s1 = splitmix64Noise(0, 9999n);
  const s2 = splitmix64Noise(0, 9999n);
  assert.equal(s1, s2);
  assert.ok(s1 >= -1.0 && s1 <= 1.0);

  const prng1 = new SplitMix64(42n);
  const prng2 = new SplitMix64(42n);
  for (let i = 0; i < 20; i += 1) {
    assert.equal(prng1.nextU64(), prng2.nextU64());
    assert.equal(prng1.nextFloatSigned(), prng2.nextFloatSigned());
  }
});

test("Hi-hat choke logic and applyHihatChoke release ramp", () => {
  const openHihat = getPercussionVoice("open_hihat");
  const closedHihat = getPercussionVoice("closed_hihat");
  const pedalHihat = getPercussionVoice("pedal_hihat");
  const kick = getPercussionVoice("kick");
  const snare = getPercussionVoice("snare");

  // Choke pairings
  assert.equal(isChokedBy(openHihat, "open_hihat", closedHihat, "closed_hihat"), true);
  assert.equal(isChokedBy(openHihat, "open_hihat", pedalHihat, "pedal_hihat"), true);
  assert.equal(isChokedBy(closedHihat, "closed_hihat", openHihat, "open_hihat"), false);
  assert.equal(isChokedBy(snare, "snare", kick, "kick"), false);

  const rate = 10_000.0;
  const samples = new Float64Array(500).fill(1.0);

  // Choke at 20 ms (sample 200) with 15 ms ramp (150 samples -> reaches sample 350)
  const choked = applyHihatChoke(samples, 0.0, 0.02, rate, CHOKE_RELEASE_RAMP_SECONDS);
  assert.equal(choked.length, 500);

  // Before choke: unchanged (1.0)
  for (let i = 0; i < 200; i += 1) {
    assertClose(choked[i], 1.0, 1e-9, `sample ${i} before choke`);
  }

  // During 15 ms ramp: monotonic downward
  for (let i = 200; i < 349; i += 1) {
    assert.ok(choked[i] >= choked[i + 1] - 1e-9, `choke ramp at index ${i} must decrease`);
  }
  assertClose(choked[200], 1.0, 1e-6);
  assertClose(choked[275], 0.5, 0.02);

  // After 15 ms ramp: strictly zero
  for (let i = 350; i < 500; i += 1) {
    assert.equal(choked[i], 0.0, `sample ${i} after choke should be 0.0`);
  }

  // Edge cases
  // Choke after buffer end
  const chokedLate = applyHihatChoke(samples, 0.0, 1.0, rate, CHOKE_RELEASE_RAMP_SECONDS);
  assert.equal(chokedLate[499], 1.0);

  // Choke before hit start
  const chokedEarly = applyHihatChoke(samples, 0.05, 0.01, rate, CHOKE_RELEASE_RAMP_SECONDS);
  assertClose(chokedEarly[0], 1.0, 1e-6);
  assert.equal(chokedEarly[150], 0.0);

  // Empty buffer
  const emptyChoked = applyHihatChoke(new Float64Array(0), 0.0, 0.01, rate);
  assert.equal(emptyChoked.length, 0);
});

test("PercussionSignal continuous evaluation and amplitude decay", () => {
  const kick = getPercussionVoice("kick");
  const sig = new PercussionSignal(kick, 0.5, 1.0, 44_100.0);

  assert.equal(sig.voice.family, "kick");
  assert.equal(sig.durationSeconds(), 0.5);
  assert.equal(sig.sampleRateHz, 44_100.0);
  assert.equal(sig.amplitude, 1.0);

  // At t=0
  const v0 = sig.valueAt(0.0);
  assert.ok(Number.isFinite(v0));

  // Signal decays to near zero at the end of duration
  const vEnd = sig.valueAt(0.48);
  assert.ok(Math.abs(vEnd) < 0.1);

  // Past duration and decay
  assert.equal(sig.valueAt(2.0), 0.0);
  // Before t=0
  assert.throws(() => sig.valueAt(-0.01), /timeSeconds must be >= 0.0/);

  // Sampling helper
  const sampledBuf = sig.sample(0.05);
  assert.ok(sampledBuf.sampleCount() > 0);
});

test("renderPercussionHit produces valid float and PCM buffers", () => {
  const rendered = renderPercussionHit("snare", 0.15, {
    sampleRateHz: 44_100.0,
    amplitude: 0.9,
  });

  assert.ok(rendered instanceof PercussionHitRender);
  assert.equal(rendered.voice.family, "snare");
  assert.equal(rendered.floatingSamples.sampleRateHz, 44_100.0);
  assert.ok(rendered.floatingSamples.sampleCount() > 0);
  assert.ok(rendered.pcmBuffer.sampleCount() > 0);
  assert.equal(
    rendered.floatingSamples.sampleCount(),
    rendered.pcmBuffer.sampleCount(),
  );

  const pcmDirect = renderPercussionHitPcm("snare", 0.15, {
    sampleRateHz: 44_100.0,
    amplitude: 0.9,
  });
  assert.equal(pcmDirect.sampleCount(), rendered.pcmBuffer.sampleCount());
  assert.deepEqual(pcmDirect.samples, rendered.pcmBuffer.samples);
});

test("renderPercussionSequence with choke group cutting open hi-hat", () => {
  const events = [
    new PercussionHitEvent("kick", 0.0, 0.1, 1.0),
    new PercussionHitEvent("open_hihat", 0.05, 0.3, 0.9), // open hi-hat at 50 ms (rings 300 ms to 350 ms)
    new PercussionHitEvent("closed_hihat", 0.15, 0.05, 0.9), // closed hi-hat at 150 ms chokes open hi-hat
    new PercussionHitEvent("snare", 0.25, 0.1, 1.0),
  ];

  const seq = renderPercussionSequence(events, {
    sampleRateHz: 44_100.0,
  });

  assert.ok(seq instanceof PercussionSequenceRender);
  assert.equal(seq.events.length, 4);
  assert.ok(seq.durationSeconds >= 0.35);
  assert.ok(seq.floatingSamples.sampleCount() > 0);
  assert.ok(seq.pcmBuffer.sampleCount() > 0);

  // Empty sequence returns empty render
  const emptySeq = renderPercussionSequence([], { sampleRateHz: 44_100.0 });
  assert.equal(emptySeq.events.length, 0);
  assert.equal(emptySeq.floatingSamples.sampleCount(), 0);
});

test("DrumKitProfile construction and voice management", () => {
  const kick = getPercussionVoice("kick");
  const snare = getPercussionVoice("snare");
  const customKit = new DrumKitProfile(
    "custom_kit",
    "Custom Kit",
    new Map([["kick", kick], ["snare", snare]]),
  );

  assert.equal(customKit.id, "custom_kit");
  assert.equal(customKit.voiceCount, 2);
  assert.equal(customKit.resolveHitId("kick"), "kick");
  assert.equal(customKit.resolveHitId("snare"), "snare");
  assert.equal(customKit.hasVoice("kick"), true);
  assert.equal(customKit.hasVoice("snare"), true);
  assert.equal(customKit.hasVoice("cowbell"), false);

  assert.throws(() => new DrumKitProfile("", "Name", new Map([["kick", kick]])), /kit id must be a non-empty string/);
  assert.throws(() => new DrumKitProfile("id", "", new Map([["kick", kick]])), /kit displayName must be a non-empty string/);
  assert.throws(() => new DrumKitProfile("id", "Name", new Map()), /must contain at least one/);
});

test("Warrant operations conformance", () => {
  assert.equal(isModeCountSafe(0), true);
  assert.equal(isModeCountSafe(MAX_PERCUSSION_MODES), true);
  assert.equal(isModeCountSafe(MAX_PERCUSSION_MODES + 1), false);
  assert.equal(isModeCountSafe(-1), false);
  assert.equal(isModeCountSafe(1.5), false);

  assert.equal(calcHitSampleCount(100, 44_100), 4410);
  assert.equal(calcHitSampleCount(-1, 44_100), null);
  assert.equal(calcHitSampleCount(100, -1), null);
  assert.equal(calcHitSampleCount(100, MAX_SAMPLE_RATE_HZ + 1), null);
});

test("Additional coverage: aliases, factory functions, and edge cases", () => {
  // Test all standard hit IDs
  const allIds = [
    "kick", "snare", "closed_hihat", "open_hihat", "pedal_hihat",
    "low_tom", "mid_tom", "high_tom", "crash", "ride",
    "clap", "cowbell", "tambourine", "shaker", "woodblock",
  ];
  for (const id of allIds) {
    const v = getPercussionVoice(id);
    assert.ok(v instanceof PercussionVoiceProfile);
  }

  // Canonical alias branches
  assert.equal(canonicalHitId("floor_tom"), "low_tom");
  assert.equal(canonicalHitId("lt"), "low_tom");
  assert.equal(canonicalHitId("tom_mid"), "mid_tom");
  assert.equal(canonicalHitId("mt"), "mid_tom");
  assert.equal(canonicalHitId("tom_high"), "high_tom");
  assert.equal(canonicalHitId("ht"), "high_tom");
  assert.equal(canonicalHitId("crash_cymbal"), "crash");
  assert.equal(canonicalHitId("cc"), "crash");
  assert.equal(canonicalHitId("ride_cymbal"), "ride");
  assert.equal(canonicalHitId("rc"), "ride");
  assert.equal(canonicalHitId("rim"), "rimshot");
  assert.equal(canonicalHitId("rs"), "rimshot");
  assert.equal(canonicalHitId("hand_clap"), "clap");
  assert.equal(canonicalHitId("handclap"), "clap");
  assert.equal(canonicalHitId("cp"), "clap");
  assert.equal(canonicalHitId("cb"), "cowbell");
  assert.equal(canonicalHitId("tamb"), "tambourine");
  assert.equal(canonicalHitId("tb"), "tambourine");
  assert.equal(canonicalHitId("sh"), "shaker");
  assert.equal(canonicalHitId("wood_block"), "woodblock");
  assert.equal(canonicalHitId(123 as unknown as string), null);
  assert.equal(canonicalHitId("unknown_xyz"), null);

  // Choke ramp with rampSamples = 0
  const zeroRamp = applyHihatChoke(new Float64Array([1.0, 1.0]), 0.0, 0.0, 44_100, 0.0);
  assert.equal(zeroRamp[0], 0.0);

  const zeroRampLate = applyHihatChoke(new Float64Array([1.0, 1.0, 1.0]), 0.0, 1.0 / 44_100, 44_100, 0.0);
  assert.equal(zeroRampLate[0], 1.0);
  assert.equal(zeroRampLate[1], 0.0);
  assert.equal(zeroRampLate[2], 0.0);

  // isChokedBy non-hihat same chokeGroup
  const conga1 = new PercussionVoiceProfile("c1", "Conga 1", "conga", 0.5, [], null, new PercussionEnvelope(0, 0.01, 0.1), 0, "congas");
  const conga2 = new PercussionVoiceProfile("c2", "Conga 2", "conga", 0.5, [], null, new PercussionEnvelope(0, 0.01, 0.1), 0, "congas");
  const bongo = new PercussionVoiceProfile("b1", "Bongo", "bongo", 0.5, [], null, new PercussionEnvelope(0, 0.01, 0.1), 0, "bongos");
  assert.equal(isChokedBy(conga1, "c1", conga2, "c2"), true);
  assert.equal(isChokedBy(conga1, "c1", bongo, "b1"), false);

  // PercussionVoiceProfile validations
  assert.throws(() => new PercussionVoiceProfile("id", "name", "fam", 0.5, null as unknown as any, null, new PercussionEnvelope(0, 0.01, 0.1)), /modes must be an array/);
  assert.throws(() => new PercussionVoiceProfile("id", "name", "fam", 0.5, ["bad"] as unknown as any, null, new PercussionEnvelope(0, 0.01, 0.1)), /must be a PercussionMode/);
  assert.throws(() => new PercussionVoiceProfile("id", "name", "fam", 0.5, [], "bad" as unknown as any, new PercussionEnvelope(0, 0.01, 0.1)), /noiseProfile must be a PercussionNoiseProfile/);
  assert.throws(() => new PercussionVoiceProfile("id", "name", "fam", 0.5, [], null, null as unknown as any), /envelope must be a PercussionEnvelope/);

  // PercussionHitEvent static helpers and getters
  const evHit = PercussionHitEvent.hit("kick", 0.1);
  assert.equal(evHit.hitId, "kick");
  assert.equal(evHit.startTimeSeconds, 0.1);

  const evDur = PercussionHitEvent.withDuration("snare", 0.2, 0.08);
  assert.equal(evDur.durationSeconds, 0.08);

  // PercussionSequenceRender getters
  const seq = renderPercussionSequence([evHit]);
  assert.equal(seq.sampleRateHz, 44_100.0);
  assert.ok(seq.sampleCount() > 0);

  // SplitMix64 methods
  const prng = new SplitMix64(123);
  assert.ok(prng.nextFloatUnit() >= 0.0);
  assert.ok(prng.nextFloat() >= -1.0);
  assert.throws(() => new SplitMix64(NaN), /seed must be finite/);
  assert.throws(() => new SplitMix64("abc" as unknown as any), /seed must be a number or bigint/);

  // Signal click branch
  const clickVoice = new PercussionVoiceProfile("click_v", "Click", "misc", 0.8, [], null, new PercussionEnvelope(0, 0.01, 0.05), 0.5);
  const clickSig = new PercussionSignal(clickVoice, 0.05, 1.0, 44_100);
  assert.ok(clickSig.valueAt(0.001) > 0.0);

  // Signal sampling bounds check
  assert.throws(() => clickSig.sample(0.1, 10), /exceeds maxSampleCount/);

  // Envelope renderedDurationSeconds validation
  assert.throws(() => new PercussionEnvelope(0.01, 0.01, 0.05).renderedDurationSeconds(-1), /hitDurationSeconds/);

  // Warrant ops sample count ceiling exceeded
  assert.equal(calcHitSampleCount(100_000_000, 44_100), null);

  // Validation routines bounds
  assert.throws(() => validateModeCountCeiling("v", 0), /must be > 0/);
  assert.throws(() => validateHitCountCeiling("v", 0), /must be > 0/);
  assert.throws(() => validateSequenceEventCountCeiling("v", 0), /must be > 0/);

  // Profile parameter validation
  assert.throws(() => new PercussionVoiceProfile("id", "", "fam", 0.5, [], null, new PercussionEnvelope(0, 0.01, 0.1)), /displayName/);
  assert.throws(() => new PercussionVoiceProfile("id", "name", "", 0.5, [], null, new PercussionEnvelope(0, 0.01, 0.1)), /family/);
  assert.throws(() => new PercussionVoiceProfile("id", "name", "fam", 1.5, [], null, new PercussionEnvelope(0, 0.01, 0.1)), /gain/);
  const whitespaceChokeVoice = new PercussionVoiceProfile("id", "name", "fam", 0.5, [], null, new PercussionEnvelope(0, 0.01, 0.1), 0, "   ");
  assert.equal(whitespaceChokeVoice.chokeGroup, null);

  // Render budget and mode ceilings
  assert.throws(() => renderPercussionHit("kick", 1.0, { maxSampleCount: 10 }), /exceeding maxSampleCount=10/);
  assert.throws(() => renderPercussionHit("kick", 0.05, { maxModeCount: 1 }), /exceeding maxModeCount=1/);

  // Sequence render validation branches
  assert.throws(() => renderPercussionSequence(["not_an_event" as unknown as any]), /must be a PercussionHitEvent/);
  assert.throws(
    () => renderPercussionSequence([evHit], { maxSampleCount: 10 }),
    /exceeding maxSampleCount/i,
  );
  assert.throws(
    () => renderPercussionSequence([evHit], { maxModeCount: 1 }),
    /exceeding maxModeCount/i,
  );

  // DrumKitProfile object input and error branches
  const kick = getPercussionVoice("kick");
  const objKit = new DrumKitProfile("obj_kit", "Obj Kit", { kick });
  assert.equal(objKit.voiceCount, 1);
  assert.equal(objKit.hasVoice("kick"), true);
  assert.equal(objKit.resolveHitId(123 as unknown as string), null);
  assert.throws(() => objKit.getVoice("nonexistent"), /Unknown percussion voice/);
  assert.throws(() => new DrumKitProfile("bad", "Bad", 123 as unknown as any), /voicesByHitId must be a Map or object/);

  // allStandardHitIds
  const stdIds = allStandardHitIds();
  assert.equal(stdIds.length, 15);
  for (const id of stdIds) {
    assert.ok(standardKit().hasVoice(id));
  }
});
