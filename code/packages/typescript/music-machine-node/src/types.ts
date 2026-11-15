/**
 * Domain data types for text scores and portable multi-track scores (MUS01 / MUS03 / MUS04 / MUS07).
 * Hardened for CWE-20, CWE-190, CWE-400, CWE-674, CWE-1321, and CWE-1333.
 */

import {
  AdsrEnvelope,
  HarmonicPartial,
  InstrumentNoteRender,
  InstrumentProfile,
  parseNote,
  VariationProfile,
} from "@coding-adventures/musical-instruments";
import type { PCMBuffer } from "@coding-adventures/pcm-audio";
import {
  ALL_STANDARD_HIT_IDS,
  type PercussionHitEvent,
  type PercussionHitRender,
} from "@coding-adventures/percussion-kit";

import {
  DEFAULT_INSTRUMENT_ID,
  DEFAULT_METER,
  DEFAULT_TEMPO_BPM,
  MAX_EVENTS,
  MAX_INSTRUMENTS,
  MAX_PARTIALS,
  MAX_PPQ,
  MAX_SAMPLE_RATE_HZ,
  MAX_TRACKS,
  MIN_SAMPLE_RATE_HZ,
  PORTABLE_FORMAT_VERSION,
} from "./constants.js";
import {
  assertSafeProperty,
  beatsForDurationSymbol,
  resolveInstrumentId,
  validateFinite,
  validateIdentifier,
  validateMeter,
  validateNonNegativeInt,
  validatePositive,
  validatePositiveInt,
  validateSafeInteger,
  validateUnitFloat,
} from "./validation.js";

export type ScoreEventKind = "note" | "rest" | "hit";

export class ScoreEvent {
  readonly kind: ScoreEventKind;
  readonly note: string | null;
  readonly hitId: string | null;
  readonly durationSymbol: string;
  readonly beatCount: number;
  readonly durationSeconds: number;
  readonly sourceToken: string;

  constructor(
    kind: ScoreEventKind,
    note: string | null,
    hitId: string | null,
    durationSymbol: string,
    beatCount: number,
    durationSeconds: number,
    sourceToken: string,
  ) {
    if (kind !== "note" && kind !== "rest" && kind !== "hit") {
      throw new RangeError(
        `event kind must be 'note', 'rest', or 'hit', got '${String(kind)}'`,
      );
    }
    this.kind = kind;
    this.beatCount = validatePositive("beatCount", beatCount);
    this.durationSeconds = validatePositive("durationSeconds", durationSeconds);
    this.durationSymbol = durationSymbol;
    this.sourceToken = sourceToken;

    if (kind === "note") {
      if (typeof note !== "string" || note.length === 0) {
        throw new RangeError("note events must include a note");
      }
      parseNote(note); // validate note spelling
      this.note = note;
      this.hitId = null;
    } else if (kind === "hit") {
      if (typeof hitId !== "string" || hitId.length === 0) {
        throw new RangeError("hit events must include a hitId");
      }
      const lower = hitId.toLowerCase();
      if (!ALL_STANDARD_HIT_IDS.includes(lower)) {
        throw new RangeError(`unknown hit_id '${lower}'`);
      }
      this.note = null;
      this.hitId = lower;
    } else {
      this.note = null;
      this.hitId = null;
    }
  }
}

export class TextScore {
  readonly title: string;
  readonly tempoBpm: number;
  readonly meter: string;
  readonly amplitude: number;
  readonly sampleRateHz: number;
  readonly instrumentId: string;
  readonly gmProgram: number | null;
  readonly events: readonly ScoreEvent[];
  readonly drumkit: string | null;

  constructor(options: {
    readonly title: string;
    readonly tempoBpm: number;
    readonly meter: string;
    readonly amplitude: number;
    readonly sampleRateHz: number;
    readonly instrumentId: string;
    readonly gmProgram?: number | null;
    readonly events: readonly ScoreEvent[];
    readonly drumkit?: string | null;
  }) {
    this.title = typeof options.title === "string" ? options.title : "Untitled";
    this.tempoBpm = validatePositive("tempoBpm", options.tempoBpm);
    this.meter = validateMeter(options.meter);
    this.amplitude = validateUnitFloat("amplitude", options.amplitude);
    validateSafeInteger("sampleRateHz", options.sampleRateHz);
    if (
      options.sampleRateHz < MIN_SAMPLE_RATE_HZ ||
      options.sampleRateHz > MAX_SAMPLE_RATE_HZ
    ) {
      throw new RangeError(
        `sampleRateHz must be in [${String(MIN_SAMPLE_RATE_HZ)}, ${String(MAX_SAMPLE_RATE_HZ)}]`,
      );
    }
    this.sampleRateHz = options.sampleRateHz;

    if (options.gmProgram !== undefined && options.gmProgram !== null) {
      validatePositiveInt("gmProgram", options.gmProgram);
      if (options.gmProgram < 1 || options.gmProgram > 128) {
        throw new RangeError("gmProgram must be in [1, 128]");
      }
      this.gmProgram = options.gmProgram;
    } else {
      this.gmProgram = null;
    }

    this.instrumentId = options.instrumentId;
    this.drumkit = options.drumkit ?? null;

    if (!Array.isArray(options.events) || options.events.length === 0) {
      throw new RangeError("score must contain at least one event");
    }
    if (options.events.length > MAX_EVENTS) {
      throw new RangeError(
        `event count ${String(options.events.length)} exceeds MAX_EVENTS=${String(MAX_EVENTS)}`,
      );
    }
    this.events = Object.freeze([...options.events]);
  }

  secondsPerBeat(): number {
    return 60.0 / this.tempoBpm;
  }

  totalDurationBeats(): number {
    let sum = 0.0;
    for (let i = 0; i < this.events.length; i += 1) {
      sum += this.events[i].beatCount;
    }
    return sum;
  }

  totalDurationSeconds(): number {
    let sum = 0.0;
    for (let i = 0; i < this.events.length; i += 1) {
      sum += this.events[i].durationSeconds;
    }
    return sum;
  }
}

export class TempoEvent {
  readonly startTick: number;
  readonly bpm: number;

  constructor(startTick: number, bpm: number) {
    this.startTick = validateNonNegativeInt("startTick", startTick);
    this.bpm = validatePositive("bpm", bpm);
  }
}

export class MeterEvent {
  readonly startTick: number;
  readonly meter: string;

  constructor(startTick: number, meter: string) {
    this.startTick = validateNonNegativeInt("startTick", startTick);
    this.meter = validateMeter(meter);
  }
}

export class InstrumentDeclaration {
  readonly id: string;
  readonly profileId: string;
  readonly gain: number;
  readonly kind: string;
  readonly kit: string | null;
  readonly harmonics: readonly HarmonicPartial[] | null;
  readonly envelope: AdsrEnvelope | null;
  readonly customProfile: InstrumentProfile | null;

  constructor(
    id: string,
    profileId: string,
    gain: number,
    kind: string,
    kit: string | null = null,
    harmonics: readonly HarmonicPartial[] | null = null,
    envelope: AdsrEnvelope | null = null,
  ) {
    validateIdentifier("instrument id", id);
    validateUnitFloat("gain", gain);

    let actualKind = kind;
    let actualProfileId = profileId;
    let actualKit = kit;
    let customProfile: InstrumentProfile | null = null;

    if (kind === "drumkit" || profileId.startsWith("drumkit") || kit !== null) {
      const kitName = kit ?? "standard_kit";
      actualKit = kitName === "standard" ? "standard_kit" : kitName;
      actualKind = "drumkit";
      actualProfileId = "drumkit";
    } else if (kind === "additive" || harmonics !== null) {
      if (!harmonics || harmonics.length === 0) {
        throw new RangeError(
          "kind=additive must provide at least one harmonic partial",
        );
      }
      if (harmonics.length > MAX_PARTIALS) {
        throw new RangeError(
          `harmonics count ${String(harmonics.length)} exceeds MAX_PARTIALS=${String(MAX_PARTIALS)}`,
        );
      }
      const env =
        envelope ??
        new AdsrEnvelope(0.005, 0.010, 0.90, 0.030);
      customProfile = new InstrumentProfile(
        id,
        id,
        "additive",
        1.0,
        harmonics,
        env,
        new VariationProfile(),
        null,
        "custom",
      );
      actualKind = "additive";
      actualProfileId = id;
    } else if (kind === "sine" || kind === "silence") {
      actualKind = kind;
      actualProfileId = kind;
    } else {
      actualKind = "preset";
      actualProfileId = resolveInstrumentId(profileId);
    }

    this.id = id;
    this.profileId = actualProfileId;
    this.gain = gain;
    this.kind = actualKind;
    this.kit = actualKit;
    this.harmonics = harmonics ? Object.freeze([...harmonics]) : null;
    this.envelope = envelope ?? null;
    this.customProfile = customProfile;
  }
}

export class TrackDeclaration {
  readonly id: string;
  readonly instrumentId: string;

  constructor(id: string, instrumentId: string) {
    this.id = validateIdentifier("track id", id);
    this.instrumentId = validateIdentifier("instrumentId", instrumentId);
  }
}

export class PortableScoreEvent {
  readonly trackId: string;
  readonly startTick: number;
  readonly durationTick: number;
  readonly kind: ScoreEventKind;
  readonly notes: readonly string[];
  readonly velocity: number;
  readonly sourceOrder: number;
  readonly hitId: string | null;

  constructor(
    trackId: string,
    startTick: number,
    durationTick: number,
    kind: ScoreEventKind,
    notes: readonly string[],
    velocity: number = 1.0,
    sourceOrder: number = 0,
    hitId: string | null = null,
  ) {
    this.trackId = validateIdentifier("trackId", trackId);
    this.startTick = validateNonNegativeInt("startTick", startTick);
    this.durationTick = validatePositiveInt("durationTick", durationTick);

    // Overflow guard (CWE-190)
    const endTick = this.startTick + this.durationTick;
    if (!Number.isSafeInteger(endTick)) {
      throw new RangeError(
        `event startTick ${String(startTick)} + durationTick ${String(durationTick)} exceeds safe integer range`,
      );
    }

    if (kind !== "note" && kind !== "rest" && kind !== "hit") {
      throw new RangeError(
        `kind must be 'note', 'rest', or 'hit', got '${String(kind)}'`,
      );
    }
    this.kind = kind;
    this.velocity = validateUnitFloat("velocity", velocity);
    this.sourceOrder = validateNonNegativeInt("sourceOrder", sourceOrder);

    if (kind === "note") {
      if (!Array.isArray(notes) || notes.length === 0) {
        throw new RangeError("note events must include at least one note");
      }
      for (let i = 0; i < notes.length; i += 1) {
        parseNote(notes[i]); // validate spelling
      }
      this.notes = Object.freeze([...notes]);
      this.hitId = null;
    } else if (kind === "hit") {
      if (notes.length > 0) {
        throw new RangeError("hit events cannot include notes");
      }
      if (typeof hitId !== "string" || hitId.length === 0) {
        throw new RangeError("hit events must include a hitId");
      }
      const lower = hitId.toLowerCase();
      if (!ALL_STANDARD_HIT_IDS.includes(lower)) {
        throw new RangeError(`unknown hit_id '${lower}'`);
      }
      this.notes = Object.freeze([]);
      this.hitId = lower;
    } else {
      if (notes.length > 0) {
        throw new RangeError("rest events cannot include notes");
      }
      this.notes = Object.freeze([]);
      this.hitId = null;
    }
  }
}

export class PortableScore {
  readonly formatVersion: string;
  readonly title: string;
  readonly ppq: number;
  readonly sampleRateHz: number;
  readonly tempoEvents: readonly TempoEvent[];
  readonly meterEvents: readonly MeterEvent[];
  readonly instruments: readonly InstrumentDeclaration[];
  readonly tracks: readonly TrackDeclaration[];
  readonly events: readonly PortableScoreEvent[];

  constructor(options: {
    readonly formatVersion: string;
    readonly title: string;
    readonly ppq: number;
    readonly sampleRateHz: number;
    readonly tempoEvents: readonly TempoEvent[];
    readonly meterEvents: readonly MeterEvent[];
    readonly instruments: readonly InstrumentDeclaration[];
    readonly tracks: readonly TrackDeclaration[];
    readonly events: readonly PortableScoreEvent[];
  }) {
    if (options.formatVersion !== PORTABLE_FORMAT_VERSION) {
      throw new RangeError(
        `format_version must be '${PORTABLE_FORMAT_VERSION}', got '${String(options.formatVersion)}'`,
      );
    }
    this.formatVersion = options.formatVersion;
    this.title = typeof options.title === "string" ? options.title : "Untitled";

    validatePositiveInt("ppq", options.ppq);
    if (options.ppq > MAX_PPQ) {
      throw new RangeError(`ppq must be <= ${String(MAX_PPQ)}, got ${String(options.ppq)}`);
    }
    this.ppq = options.ppq;

    validateSafeInteger("sampleRateHz", options.sampleRateHz);
    if (
      options.sampleRateHz < MIN_SAMPLE_RATE_HZ ||
      options.sampleRateHz > MAX_SAMPLE_RATE_HZ
    ) {
      throw new RangeError(
        `sampleRateHz must be in [${String(MIN_SAMPLE_RATE_HZ)}, ${String(MAX_SAMPLE_RATE_HZ)}], got ${String(options.sampleRateHz)}`,
      );
    }
    this.sampleRateHz = options.sampleRateHz;

    // Budgets (CWE-400)
    if (options.tempoEvents.length > MAX_EVENTS) {
      throw new RangeError(
        `tempoEvents count ${String(options.tempoEvents.length)} exceeds MAX_EVENTS=${String(MAX_EVENTS)}`,
      );
    }
    if (options.meterEvents.length > MAX_EVENTS) {
      throw new RangeError(
        `meterEvents count ${String(options.meterEvents.length)} exceeds MAX_EVENTS=${String(MAX_EVENTS)}`,
      );
    }
    if (options.instruments.length > MAX_INSTRUMENTS) {
      throw new RangeError(
        `instruments count ${String(options.instruments.length)} exceeds MAX_INSTRUMENTS=${String(MAX_INSTRUMENTS)}`,
      );
    }
    if (options.tracks.length > MAX_TRACKS) {
      throw new RangeError(
        `tracks count ${String(options.tracks.length)} exceeds MAX_TRACKS=${String(MAX_TRACKS)}`,
      );
    }
    if (options.events.length > MAX_EVENTS) {
      throw new RangeError(
        `events count ${String(options.events.length)} exceeds MAX_EVENTS=${String(MAX_EVENTS)}`,
      );
    }

    // Validate unique instrument IDs
    const instrumentIds = new Set<string>();
    for (let i = 0; i < options.instruments.length; i += 1) {
      const inst = options.instruments[i];
      if (instrumentIds.has(inst.id)) {
        throw new RangeError(`duplicate instrument id '${inst.id}'`);
      }
      instrumentIds.add(inst.id);
    }

    // Validate unique track IDs and track instrument references
    const trackIds = new Set<string>();
    for (let i = 0; i < options.tracks.length; i += 1) {
      const tr = options.tracks[i];
      if (trackIds.has(tr.id)) {
        throw new RangeError(`duplicate track id '${tr.id}'`);
      }
      if (!instrumentIds.has(tr.instrumentId)) {
        throw new RangeError(
          `track '${tr.id}' references unknown instrument '${tr.instrumentId}'`,
        );
      }
      trackIds.add(tr.id);
    }

    // Validate event track references and tick ranges
    for (let i = 0; i < options.events.length; i += 1) {
      const ev = options.events[i];
      if (!trackIds.has(ev.trackId)) {
        throw new RangeError(
          `event references unknown track '${ev.trackId}'`,
        );
      }
      const endTick = ev.startTick + ev.durationTick;
      if (!Number.isSafeInteger(endTick)) {
        throw new RangeError(
          `event '${ev.trackId}' startTick + durationTick exceeds safe integer`,
        );
      }
    }

    // Sort tempo events and ensure tick 0 event exists
    const sortedTempos = [...options.tempoEvents].sort(
      (a, b) => a.startTick - b.startTick,
    );
    if (sortedTempos.length === 0 || sortedTempos[0].startTick !== 0) {
      sortedTempos.unshift(new TempoEvent(0, DEFAULT_TEMPO_BPM));
    }
    this.tempoEvents = Object.freeze(sortedTempos);

    // Sort meter events and ensure tick 0 event exists
    const sortedMeters = [...options.meterEvents].sort(
      (a, b) => a.startTick - b.startTick,
    );
    if (sortedMeters.length === 0 || sortedMeters[0].startTick !== 0) {
      sortedMeters.unshift(new MeterEvent(0, DEFAULT_METER));
    }
    this.meterEvents = Object.freeze(sortedMeters);

    this.instruments = Object.freeze([...options.instruments]);
    this.tracks = Object.freeze([...options.tracks]);

    // Sort events by (startTick, sourceOrder)
    const sortedEvents = [...options.events].sort((a, b) => {
      if (a.startTick !== b.startTick) {
        return a.startTick - b.startTick;
      }
      return a.sourceOrder - b.sourceOrder;
    });
    this.events = Object.freeze(sortedEvents);
  }

  totalDurationTicks(): number {
    let maxTick = 0;
    for (let i = 0; i < this.events.length; i += 1) {
      const end = this.events[i].startTick + this.events[i].durationTick;
      if (end > maxTick) {
        maxTick = end;
      }
    }
    return maxTick;
  }

  toText(): string {
    throw new Error("toText not initialized");
  }

  toJson(): string {
    throw new Error("toJson not initialized");
  }

  static fromJson?(jsonString: string): PortableScore;
  static fromText?(text: string): PortableScore;
}

export class RenderedScore {
  readonly score: TextScore;
  readonly pcmBuffer: PCMBuffer;
  readonly renderedNotes: readonly InstrumentNoteRender[];
  readonly renderedHits: readonly (PercussionHitRender | PercussionHitEvent)[];
  readonly clippedSampleCount: number;

  constructor(
    score: TextScore,
    pcmBuffer: PCMBuffer,
    renderedNotes: readonly InstrumentNoteRender[],
    renderedHits: readonly (PercussionHitRender | PercussionHitEvent)[],
    clippedSampleCount: number,
  ) {
    this.score = score;
    this.pcmBuffer = pcmBuffer;
    this.renderedNotes = Object.freeze([...renderedNotes]);
    this.renderedHits = Object.freeze([...renderedHits]);
    this.clippedSampleCount = validateNonNegativeInt(
      "clippedSampleCount",
      clippedSampleCount,
    );
  }
}

export class RenderedPortableScore {
  readonly score: PortableScore;
  readonly pcmBuffer: PCMBuffer;
  readonly renderedNotes: readonly InstrumentNoteRender[];
  readonly renderedHits: readonly (PercussionHitRender | PercussionHitEvent)[];
  readonly clippedSampleCount: number;

  constructor(
    score: PortableScore,
    pcmBuffer: PCMBuffer,
    renderedNotes: readonly InstrumentNoteRender[],
    renderedHits: readonly (PercussionHitRender | PercussionHitEvent)[],
    clippedSampleCount: number,
  ) {
    this.score = score;
    this.pcmBuffer = pcmBuffer;
    this.renderedNotes = Object.freeze([...renderedNotes]);
    this.renderedHits = Object.freeze([...renderedHits]);
    this.clippedSampleCount = validateNonNegativeInt(
      "clippedSampleCount",
      clippedSampleCount,
    );
  }
}
