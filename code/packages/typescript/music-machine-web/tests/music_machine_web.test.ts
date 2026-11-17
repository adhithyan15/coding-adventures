import assert from "node:assert/strict";
import test from "node:test";

import {
  type AudioBufferLike,
  type AudioBufferSourceNodeLike,
  type AudioContextLike,
  type GainNodeLike,
} from "@coding-adventures/web-audio-sink";

import {
  formatFloat,
  HAPPY_BIRTHDAY_TEXT,
  MINI_ORCHESTRA_TEXT,
  parsePortableScore,
  parsePortableScoreJson,
  parseScore,
  PITCHED_PERCUSSION_MIX_TEXT,
  playPortableScore,
  playPortableScoreText,
  playScore,
  playScoreText,
  PortableScore,
  PortableScoreBuilder,
  renderPortableScoreToPcm,
  renderPortableScoreToWav,
  renderScoreToPcm,
  renderScoreToWav,
  TempoMap,
  TextScore,
  toPortableScoreText,
  transposeNoteText,
  warrantParsePortableScore,
  warrantParseScore,
  warrantRenderPortableScoreToPcm,
  warrantRenderScoreToPcm,
} from "../src/index.js";

class MockAudioBuffer implements AudioBufferLike {
  readonly length: number;
  readonly sampleRate: number;
  readonly numberOfChannels: number;
  private readonly data: Float32Array;

  constructor(numberOfChannels: number, length: number, sampleRate: number) {
    this.numberOfChannels = numberOfChannels;
    this.length = length;
    this.sampleRate = sampleRate;
    this.data = new Float32Array(length);
  }

  getChannelData(_channel: number): Float32Array {
    return this.data;
  }
}

class MockAudioBufferSourceNode implements AudioBufferSourceNodeLike {
  buffer: AudioBufferLike | null = null;
  onended: ((this: unknown, event: Event) => void) | null = null;
  started = false;
  destination: unknown = null;

  connect(destination: unknown): unknown {
    this.destination = destination;
    return destination;
  }

  start(_when?: number): void {
    this.started = true;
    queueMicrotask(() => {
      if (this.onended) {
        this.onended.call(this, {} as Event);
      }
    });
  }
}

class MockGainNode implements GainNodeLike {
  readonly gain = { value: 1.0 };
  destination: unknown = null;

  connect(destination: unknown): unknown {
    this.destination = destination;
    return destination;
  }
}

class MockAudioContext implements AudioContextLike {
  readonly sampleRate = 44100;
  readonly currentTime = 0.0;
  readonly destination: unknown = {};
  state = "running";
  closed = false;

  createBuffer(
    numberOfChannels: number,
    length: number,
    sampleRate: number,
  ): AudioBufferLike {
    return new MockAudioBuffer(numberOfChannels, length, sampleRate);
  }

  createBufferSource(): AudioBufferSourceNodeLike {
    return new MockAudioBufferSourceNode();
  }

  createGain(): GainNodeLike {
    return new MockGainNode();
  }

  async close(): Promise<void> {
    this.closed = true;
  }
}

test("parseScore: parses Happy Birthday text score and calculates duration metrics", () => {
  const score = parseScore(HAPPY_BIRTHDAY_TEXT);
  assert.equal(score.title, "Happy Birthday");
  assert.equal(score.tempoBpm, 120);
  assert.equal(score.meter, "3/4");
  assert.equal(score.amplitude, 0.18);
  assert.equal(score.sampleRateHz, 44100);
  assert.equal(score.instrumentId, "sine");
  assert.ok(score.events.length > 10);

  const durationBeats = score.totalDurationBeats();
  assert.ok(durationBeats > 20.0);
  const durationSeconds = score.totalDurationSeconds();
  assert.ok(durationSeconds > 10.0);
  assert.equal(score.secondsPerBeat(), 0.5);
});

test("parseScore: supports custom directives, GM program, rests, and drum hits", () => {
  const scoreText = `title: Percussion Groove
tempo: 100
meter: 4/4
amplitude: 0.25
drumkit: standard_kit
sample_rate: 22050

kick/q snare/q | R/h | closed_hihat/q kick/q |
`;
  const score = parseScore(scoreText);
  assert.equal(score.title, "Percussion Groove");
  assert.equal(score.tempoBpm, 100);
  assert.equal(score.sampleRateHz, 22050);
  assert.equal(score.instrumentId, "drumkit");
  assert.equal(score.drumkit, "standard_kit");
  assert.equal(score.events.length, 5);
  assert.equal(score.events[0].kind, "hit");
  assert.equal(score.events[0].hitId, "kick");
  assert.equal(score.events[2].kind, "rest");

  const rendered = renderScoreToPcm(score, 1_000_000);
  assert.ok(rendered.pcmBuffer.sampleCount() > 0);
  assert.equal(rendered.pcmBuffer.pcmFormat.sampleRateHz, 22050);

  const wavBytes = renderScoreToWav(score);
  assert.ok(wavBytes.length > 44);
  assert.equal(wavBytes[0], 0x52); // 'R'
  assert.equal(wavBytes[1], 0x49); // 'I'
  assert.equal(wavBytes[2], 0x46); // 'F'
  assert.equal(wavBytes[3], 0x46); // 'F'
});

test("parsePortableScore: parses Mini Orchestra score and renders multi-track PCM", () => {
  const score = parsePortableScore(MINI_ORCHESTRA_TEXT);
  assert.equal(score.title, "Mini Orchestra");
  assert.equal(score.ppq, 120);
  assert.equal(score.sampleRateHz, 16000);
  assert.equal(score.instruments.length, 4);
  assert.equal(score.tracks.length, 4);
  assert.ok(score.events.length > 15);

  const rendered = renderPortableScoreToPcm(score, 1_000_000);
  assert.ok(rendered.pcmBuffer.sampleCount() > 0);
  assert.equal(score.events.length, 36);
  assert.equal(rendered.renderedNotes.length, 52);

  const wavBytes = renderPortableScoreToWav(score);
  assert.ok(wavBytes.length > 44);

  // Check V2 text serialization roundtrip
  const textOut = toPortableScoreText(score);
  const reParsed = parsePortableScore(textOut);
  assert.equal(reParsed.title, score.title);
  assert.equal(reParsed.events.length, score.events.length);
});

test("parsePortableScore: parses Pitched Percussion Mix with hi-hat choke sequence", () => {
  const score = parsePortableScore(PITCHED_PERCUSSION_MIX_TEXT);
  assert.equal(score.title, "Pitched Percussion Mix");
  assert.equal(score.sampleRateHz, 44100);
  const rendered = renderPortableScoreToPcm(score);
  assert.ok(rendered.pcmBuffer.sampleCount() > 0);
});

test("parsePortableScoreJson: deterministic JSON serialization and deserialization roundtrip", () => {
  const score = parsePortableScore(MINI_ORCHESTRA_TEXT);
  const jsonText = score.toJson();
  assert.ok(jsonText.includes('"format_version": "music-machine-score/v2"'));

  const fromJsonScore = parsePortableScoreJson(jsonText);
  assert.equal(fromJsonScore.title, score.title);
  assert.equal(fromJsonScore.ppq, score.ppq);
  assert.equal(fromJsonScore.sampleRateHz, score.sampleRateHz);
  assert.equal(fromJsonScore.events.length, score.events.length);

  // Key ordering check: JSON keys must be sorted deterministically
  const parsedJson = JSON.parse(jsonText);
  const topKeys = Object.keys(parsedJson);
  const sortedKeys = [...topKeys].sort();
  assert.deepEqual(topKeys, sortedKeys);
});

test("PortableScoreBuilder & PhraseBuilder: fluent sequencing and measure math", () => {
  const builder = new PortableScoreBuilder({
    title: "Composer Test",
    ppq: 480,
    sampleRateHz: 44100,
  });

  builder
    .addTempo(0, 120)
    .addMeter(0, "4/4")
    .addInstrument("lead", { profile: "flute_naive", gain: 0.5 })
    .addInstrument("drums", { kind: "drumkit", kit: "standard_kit", gain: 0.8 })
    .addTrack("melody", { instrumentId: "lead" })
    .addTrack("perc", { instrumentId: "drums" });

  assert.equal(builder.measureTicks("4/4"), 480 * 4);
  assert.equal(builder.measureStartTick(2), 480 * 4);
  assert.equal(builder.tickInMeasure(2, 1.5), 480 * 4 + 480 * 1.5);

  const phrase = builder.phrase("melody", { measureNumber: 1 });
  phrase
    .note("C4", 1.0, { velocity: 0.8 })
    .note("E4", 1.0, { velocity: 0.8 })
    .chord(["G4", "C5"], 2.0, { velocity: 0.9 });

  assert.equal(phrase.currentTick, 480 * 4);

  const motif = phrase.motif();
  assert.equal(motif.durationTick, 480 * 4);
  assert.equal(motif.events.length, 3);

  // Apply motif transposed up a whole step (2 semitones)
  builder.applyMotif(motif, "melody", 480 * 4, {
    transposeSemitones: 2,
    repeatCount: 1,
  });

  const drumPhrase = builder.phrase("perc", { measureNumber: 1 });
  drumPhrase
    .hit("kick", 1.0)
    .hit("snare", 1.0)
    .hit("kick", 1.0)
    .hit("snare", 1.0);

  const score = builder.build();
  assert.equal(score.title, "Composer Test");
  assert.equal(score.events.length, 10);

  const rendered = renderPortableScoreToPcm(score);
  assert.ok(rendered.pcmBuffer.sampleCount() > 0);
});

test("captureSection & applySection: arrange entire sections with track mapping", () => {
  const builder = new PortableScoreBuilder({ ppq: 120 });
  builder
    .addTempo(0, 120)
    .addMeter(0, "4/4")
    .addInstrument("synth", { kind: "sine", gain: 0.3 })
    .addTrack("t1", { instrumentId: "synth" })
    .addTrack("t2", { instrumentId: "synth" });

  builder.addNote("t1", 0, 120, "C4");
  builder.addNote("t2", 0, 120, "E4");
  builder.addNote("t1", 120, 120, "G4");
  builder.addNote("t2", 120, 120, "B4");

  const section = builder.captureSection({ startTick: 0, endTick: 240 });
  assert.equal(section.durationTick, 240);
  assert.equal(section.events.length, 4);

  // Apply section to measures 2 and 3
  builder.applySection(section, 240, {
    repeatCount: 2,
    transposeSemitones: 12, // an octave higher
  });

  const score = builder.build();
  assert.equal(score.events.length, 12);
  const rendered = renderPortableScoreToPcm(score);
  assert.ok(rendered.pcmBuffer.sampleCount() > 0);
});

test("Web Audio Sink: playScore and playPortableScore interact correctly with AudioContext", async () => {
  const score = parseScore(HAPPY_BIRTHDAY_TEXT);
  const mockCtx = new MockAudioContext();

  const report = await playScore(score, {
    audioContext: mockCtx,
    gain: 0.5,
    closeContextWhenDone: true,
  });

  assert.equal(report.backendName, "web-audio");
  assert.equal(report.channelCount, 1);
  assert.ok(report.framesPlayed > 0);
  assert.ok(report.durationSeconds > 0);
  assert.equal(mockCtx.closed, true);

  const pScore = parsePortableScore(MINI_ORCHESTRA_TEXT);
  const mockCtx2 = new MockAudioContext();
  const pReport = await playPortableScore(pScore, {
    audioContext: mockCtx2,
    gain: 0.8,
  });
  assert.equal(pReport.backendName, "web-audio");
  assert.ok(pReport.framesPlayed > 0);

  const pReportText = await playPortableScoreText(MINI_ORCHESTRA_TEXT, {
    audioContext: new MockAudioContext(),
  });
  assert.equal(pReportText.backendName, "web-audio");
});

test("TempoMap: binary search query accuracy", () => {
  const tempos = [
    { startTick: 0, bpm: 120 },
    { startTick: 480, bpm: 60 },
  ];
  const tm = new TempoMap(tempos, 480);
  assert.equal(tm.secondsAtTick(0), 0.0);
  assert.equal(tm.secondsAtTick(480), 0.5); // 1 beat at 120 bpm = 0.5s
  assert.equal(tm.secondsAtTick(960), 1.5); // 0.5 + 1 beat at 60 bpm (1.0s) = 1.5s
});

test("warrant operations: enforce formal budgets and parameter ranges", () => {
  assert.equal(warrantParseScore(100), 100);
  assert.throws(() => warrantParseScore(1_000_001), /score_bytes > MAX_SCORE_BYTES/);

  assert.equal(warrantParsePortableScore(500), 500);
  assert.throws(() => warrantParsePortableScore(1_000_001), /score_bytes > MAX_SCORE_BYTES/);

  assert.equal(warrantRenderScoreToPcm(50, 44100), 50);
  assert.throws(() => warrantRenderScoreToPcm(10_001, 44100), /event_count > MAX_EVENTS/);
  assert.throws(() => warrantRenderScoreToPcm(50, 0), /invalid sample_rate_hz/);
  assert.throws(() => warrantRenderScoreToPcm(50, 200_000), /invalid sample_rate_hz/);

  assert.equal(warrantRenderPortableScoreToPcm(100, 48000), 100);
  assert.throws(() => warrantRenderPortableScoreToPcm(10_001, 48000), /event_count > MAX_EVENTS/);
  assert.throws(() => warrantRenderPortableScoreToPcm(100, 0), /invalid sample_rate_hz/);
});

test("formatFloat: produces canonical float format matching Rust / Python JSON", () => {
  assert.equal(formatFloat(1), "1.0");
  assert.equal(formatFloat(1.5), "1.5");
  assert.equal(formatFloat(0), "0.0");
});
