/**
 * Deterministic JSON serialization and deserialization for PortableScore (CWE-674 / CWE-400 / CWE-1321 / CWE-20).
 * Bounded recursion depth, prototype-pollution free, strict key validation.
 */

import {
  AdsrEnvelope,
  HarmonicPartial,
} from "@coding-adventures/musical-instruments";

import {
  BANNED_PROPERTIES,
  MAX_EVENTS,
  MAX_INSTRUMENTS,
  MAX_JSON_DEPTH,
  MAX_PARTIALS,
  MAX_SCORE_BYTES,
  MAX_TRACKS,
} from "./constants.js";
import { formatFloat } from "./portable_parser.js";
import {
  InstrumentDeclaration,
  MeterEvent,
  PortableScore,
  PortableScoreEvent,
  ScoreEventKind,
  TempoEvent,
  TrackDeclaration,
} from "./types.js";
import { assertSafeProperty, validatePositiveInt } from "./validation.js";

/**
 * Validate object keys recursively against prototype pollution and max depth.
 */
function validateJsonStructure(value: unknown, depth: number, maxDepth: number): void {
  if (depth > maxDepth) {
    throw new RangeError(
      `JSON nesting depth exceeds maxDepth=${String(maxDepth)}`,
    );
  }
  if (value === null || typeof value !== "object") {
    return;
  }
  if (Array.isArray(value)) {
    for (let i = 0; i < value.length; i += 1) {
      validateJsonStructure(value[i], depth + 1, maxDepth);
    }
    return;
  }
  for (const key of Object.keys(value)) {
    assertSafeProperty(key);
    validateJsonStructure(
      (value as Record<string, unknown>)[key],
      depth + 1,
      maxDepth,
    );
  }
}

/**
 * Parse canonical JSON into a PortableScore model with explicit security bounds.
 */
export function parsePortableScoreJson(
  jsonText: string,
  options: {
    readonly maxBytes?: number;
    readonly maxDepth?: number;
  } = {},
): PortableScore {
  if (typeof jsonText !== "string") {
    throw new TypeError("jsonText must be a string");
  }
  const maxBytes = validatePositiveInt("maxBytes", options.maxBytes ?? MAX_SCORE_BYTES);
  if (maxBytes > MAX_SCORE_BYTES) {
    throw new RangeError(`maxBytes exceeds MAX_SCORE_BYTES=${String(MAX_SCORE_BYTES)}`);
  }
  const maxDepth = validatePositiveInt("maxDepth", options.maxDepth ?? MAX_JSON_DEPTH);
  if (maxDepth > MAX_JSON_DEPTH) {
    throw new RangeError(`maxDepth exceeds MAX_JSON_DEPTH=${String(MAX_JSON_DEPTH)}`);
  }

  // Pre-flight length check before buffer allocation
  if (jsonText.length > maxBytes) {
    throw new RangeError(
      `JSON byte length exceeds maxBytes=${String(maxBytes)}`,
    );
  }

  // Exact UTF-8 byte length check
  const byteLen = new TextEncoder().encode(jsonText).length;
  if (byteLen > maxBytes) {
    throw new RangeError(
      `JSON byte length ${String(byteLen)} exceeds maxBytes=${String(maxBytes)}`,
    );
  }

  let raw: unknown;
  try {
    raw = JSON.parse(jsonText);
  } catch (err: unknown) {
    throw new RangeError(`JSON parse error: ${String(err)}`);
  }

  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) {
    throw new RangeError("JSON score root must be an object");
  }

  validateJsonStructure(raw, 0, maxDepth);
  const map = raw as Record<string, unknown>;

  // Check expected keys
  const expectedKeys = new Set([
    "events",
    "format_version",
    "instruments",
    "meter_events",
    "ppq",
    "sample_rate_hz",
    "tempo_events",
    "title",
    "tracks",
  ]);

  const unknownKeys: string[] = [];
  for (const k of Object.keys(map)) {
    if (!expectedKeys.has(k)) {
      unknownKeys.push(k);
    }
  }
  if (unknownKeys.length > 0) {
    unknownKeys.sort();
    throw new RangeError(`unknown score keys: ${unknownKeys.join(", ")}`);
  }

  const formatVersion = map.format_version;
  if (typeof formatVersion !== "string") {
    throw new RangeError("missing 'format_version'");
  }

  const title = typeof map.title === "string" ? map.title : "Untitled";

  const ppq = map.ppq;
  if (typeof ppq !== "number" || !Number.isInteger(ppq) || ppq <= 0) {
    throw new RangeError("missing or invalid 'ppq'");
  }

  const sampleRateHz = map.sample_rate_hz;
  if (
    typeof sampleRateHz !== "number" ||
    !Number.isInteger(sampleRateHz) ||
    sampleRateHz <= 0
  ) {
    throw new RangeError("missing or invalid 'sample_rate_hz'");
  }

  // Parse tempo_events
  const tempoRaw = map.tempo_events;
  if (!Array.isArray(tempoRaw)) {
    throw new RangeError("tempo_events must be a list");
  }
  if (tempoRaw.length > MAX_EVENTS) {
    throw new RangeError(
      `tempo_events count ${String(tempoRaw.length)} exceeds MAX_EVENTS=${String(MAX_EVENTS)}`,
    );
  }
  const tempoEvents: TempoEvent[] = [];
  for (let idx = 0; idx < tempoRaw.length; idx += 1) {
    const item = tempoRaw[idx];
    if (item === null || typeof item !== "object" || Array.isArray(item)) {
      throw new RangeError(`tempo_events[${String(idx)}] must be a mapping`);
    }
    const o = item as Record<string, unknown>;
    const startTick = o.start_tick;
    const bpm = o.bpm;
    if (typeof startTick !== "number" || !Number.isInteger(startTick) || startTick < 0) {
      throw new RangeError(`tempo_events[${String(idx)}] missing 'start_tick'`);
    }
    if (typeof bpm !== "number" || !Number.isFinite(bpm) || bpm <= 0.0) {
      throw new RangeError(`tempo_events[${String(idx)}] missing 'bpm'`);
    }
    tempoEvents.push(new TempoEvent(startTick, bpm));
  }

  // Parse meter_events
  const meterRaw = map.meter_events;
  if (!Array.isArray(meterRaw)) {
    throw new RangeError("meter_events must be a list");
  }
  if (meterRaw.length > MAX_EVENTS) {
    throw new RangeError(
      `meter_events count ${String(meterRaw.length)} exceeds MAX_EVENTS=${String(MAX_EVENTS)}`,
    );
  }
  const meterEvents: MeterEvent[] = [];
  for (let idx = 0; idx < meterRaw.length; idx += 1) {
    const item = meterRaw[idx];
    if (item === null || typeof item !== "object" || Array.isArray(item)) {
      throw new RangeError(`meter_events[${String(idx)}] must be a mapping`);
    }
    const o = item as Record<string, unknown>;
    const startTick = o.start_tick;
    const meter = o.meter;
    if (typeof startTick !== "number" || !Number.isInteger(startTick) || startTick < 0) {
      throw new RangeError(`meter_events[${String(idx)}] missing 'start_tick'`);
    }
    if (typeof meter !== "string") {
      throw new RangeError(`meter_events[${String(idx)}] missing 'meter'`);
    }
    meterEvents.push(new MeterEvent(startTick, meter));
  }

  // Parse instruments
  const instRaw = map.instruments;
  if (!Array.isArray(instRaw)) {
    throw new RangeError("instruments must be a list");
  }
  if (instRaw.length > MAX_INSTRUMENTS) {
    throw new RangeError(
      `instruments count ${String(instRaw.length)} exceeds MAX_INSTRUMENTS=${String(MAX_INSTRUMENTS)}`,
    );
  }
  const instruments: InstrumentDeclaration[] = [];
  for (let idx = 0; idx < instRaw.length; idx += 1) {
    const item = instRaw[idx];
    if (item === null || typeof item !== "object" || Array.isArray(item)) {
      throw new RangeError(`instruments[${String(idx)}] must be a mapping`);
    }
    const o = item as Record<string, unknown>;
    const id = o.id;
    if (typeof id !== "string") {
      throw new RangeError(`instruments[${String(idx)}] missing 'id'`);
    }
    const profileId = typeof o.profile_id === "string" ? o.profile_id : id;
    const gain = typeof o.gain === "number" ? o.gain : 1.0;
    const kind = typeof o.kind === "string" ? o.kind : "preset";
    const kit = typeof o.kit === "string" ? o.kit : null;

    let harmonics: HarmonicPartial[] | null = null;
    if (o.harmonics !== undefined && o.harmonics !== null) {
      if (!Array.isArray(o.harmonics)) {
        throw new RangeError("harmonics must be a list of partials");
      }
      if (o.harmonics.length > MAX_PARTIALS) {
        throw new RangeError(
          `harmonic partial count ${String(o.harmonics.length)} exceeds MAX_PARTIALS=${String(MAX_PARTIALS)}`,
        );
      }
      harmonics = [];
      for (let pi = 0; pi < o.harmonics.length; pi += 1) {
        const pVal = o.harmonics[pi];
        if (!Array.isArray(pVal) || (pVal.length !== 2 && pVal.length !== 3)) {
          throw new RangeError(`malformed partial at ${String(pi)}`);
        }
        const mult = pVal[0];
        const amp = pVal[1];
        const phase = pVal.length === 3 ? pVal[2] : 0.0;
        if (typeof mult !== "number" || !Number.isFinite(mult) || mult <= 0.0) {
          throw new RangeError(`malformed partial multiplier at ${String(pi)}`);
        }
        if (typeof amp !== "number" || !Number.isFinite(amp) || amp < 0.0) {
          throw new RangeError(`malformed partial amplitude at ${String(pi)}`);
        }
        if (typeof phase !== "number" || !Number.isFinite(phase)) {
          throw new RangeError(`malformed partial phase at ${String(pi)}`);
        }
        harmonics.push(new HarmonicPartial(mult, amp, phase));
      }
    }

    let envelope: AdsrEnvelope | null = null;
    if (o.envelope !== undefined && o.envelope !== null) {
      if (!Array.isArray(o.envelope) || o.envelope.length !== 4) {
        throw new RangeError("envelope must be [atk, dec, sus, rel]");
      }
      const atk = o.envelope[0];
      const dec = o.envelope[1];
      const sus = o.envelope[2];
      const rel = o.envelope[3];
      if (
        typeof atk !== "number" ||
        typeof dec !== "number" ||
        typeof sus !== "number" ||
        typeof rel !== "number"
      ) {
        throw new RangeError("envelope parameters must be numbers");
      }
      envelope = new AdsrEnvelope(atk, dec, sus, rel);
    }

    instruments.push(
      new InstrumentDeclaration(
        id,
        profileId,
        gain,
        kind,
        kit,
        harmonics,
        envelope,
      ),
    );
  }

  // Parse tracks
  const trackRaw = map.tracks;
  if (!Array.isArray(trackRaw)) {
    throw new RangeError("tracks must be a list");
  }
  if (trackRaw.length > MAX_TRACKS) {
    throw new RangeError(
      `tracks count ${String(trackRaw.length)} exceeds MAX_TRACKS=${String(MAX_TRACKS)}`,
    );
  }
  const tracks: TrackDeclaration[] = [];
  for (let idx = 0; idx < trackRaw.length; idx += 1) {
    const item = trackRaw[idx];
    if (item === null || typeof item !== "object" || Array.isArray(item)) {
      throw new RangeError(`tracks[${String(idx)}] must be a mapping`);
    }
    const o = item as Record<string, unknown>;
    const id = o.id;
    const instrumentId = o.instrument_id;
    if (typeof id !== "string") {
      throw new RangeError(`tracks[${String(idx)}] missing 'id'`);
    }
    if (typeof instrumentId !== "string") {
      throw new RangeError(`tracks[${String(idx)}] missing 'instrument_id'`);
    }
    tracks.push(new TrackDeclaration(id, instrumentId));
  }

  // Parse events
  const eventRaw = map.events;
  if (!Array.isArray(eventRaw)) {
    throw new RangeError("events must be a list");
  }
  if (eventRaw.length > MAX_EVENTS) {
    throw new RangeError(
      `events count ${String(eventRaw.length)} exceeds MAX_EVENTS=${String(MAX_EVENTS)}`,
    );
  }
  const events: PortableScoreEvent[] = [];
  for (let idx = 0; idx < eventRaw.length; idx += 1) {
    const item = eventRaw[idx];
    if (item === null || typeof item !== "object" || Array.isArray(item)) {
      throw new RangeError(`events[${String(idx)}] must be a mapping`);
    }
    const o = item as Record<string, unknown>;
    const trackId = o.track_id;
    const startTick = o.start_tick;
    const durationTick = o.duration_tick;
    const kindStr = o.kind;

    if (typeof trackId !== "string") {
      throw new RangeError(`events[${String(idx)}] missing 'track_id'`);
    }
    if (typeof startTick !== "number" || !Number.isInteger(startTick) || startTick < 0) {
      throw new RangeError(`events[${String(idx)}] missing 'start_tick'`);
    }
    if (typeof durationTick !== "number" || !Number.isInteger(durationTick) || durationTick <= 0) {
      throw new RangeError(`events[${String(idx)}] missing 'duration_tick'`);
    }
    if (kindStr !== "note" && kindStr !== "rest" && kindStr !== "hit") {
      throw new RangeError(
        `events[${String(idx)}] invalid kind '${String(kindStr)}'`,
      );
    }
    const kind = kindStr as ScoreEventKind;
    const velocity = typeof o.velocity === "number" ? o.velocity : 1.0;
    const sourceOrder =
      typeof o.source_order === "number" && Number.isInteger(o.source_order)
        ? o.source_order
        : idx;
    const hitId = typeof o.hit_id === "string" ? o.hit_id : null;

    const notes: string[] = [];
    if (Array.isArray(o.notes)) {
      for (let ni = 0; ni < o.notes.length; ni += 1) {
        const noteStr = o.notes[ni];
        if (typeof noteStr !== "string") {
          throw new RangeError(`notes[${String(ni)}] must be a string`);
        }
        notes.push(noteStr);
      }
    }

    events.push(
      new PortableScoreEvent(
        trackId,
        startTick,
        durationTick,
        kind,
        notes,
        velocity,
        sourceOrder,
        hitId,
      ),
    );
  }

  return new PortableScore({
    formatVersion,
    title,
    ppq,
    sampleRateHz,
    tempoEvents,
    meterEvents,
    instruments,
    tracks,
    events,
  });
}

/**
 * Format a PortableScore to canonical JSON text with deterministic key ordering.
 */
export function formatPortableScoreJson(score: PortableScore): string {
  const obj: Record<string, unknown> = {
    events: score.events.map((e) => {
      const evObj: Record<string, unknown> = {
        duration_tick: e.durationTick,
        kind: e.kind,
        source_order: e.sourceOrder,
        start_tick: e.startTick,
        track_id: e.trackId,
        velocity: Number.parseFloat(formatFloat(e.velocity)),
      };
      if (e.kind === "note") {
        evObj.notes = [...e.notes];
      }
      if (e.kind === "hit" && e.hitId !== null) {
        evObj.hit_id = e.hitId;
      }
      return evObj;
    }),
    format_version: score.formatVersion,
    instruments: score.instruments.map((inst) => {
      const instObj: Record<string, unknown> = {
        gain: Number.parseFloat(formatFloat(inst.gain)),
        id: inst.id,
        kind: inst.kind,
        profile_id: inst.profileId,
      };
      if (inst.kind === "drumkit" && inst.kit !== null) {
        instObj.kit = inst.kit;
      }
      if (inst.kind === "additive" && inst.harmonics) {
        instObj.harmonics = inst.harmonics.map((h) => {
          if (h.phaseCycles !== 0.0) {
            return [
              Number.parseFloat(formatFloat(h.frequencyMultiplier)),
              Number.parseFloat(formatFloat(h.amplitude)),
              Number.parseFloat(formatFloat(h.phaseCycles)),
            ];
          }
          return [
            Number.parseFloat(formatFloat(h.frequencyMultiplier)),
            Number.parseFloat(formatFloat(h.amplitude)),
          ];
        });
      }
      if (inst.kind === "additive" && inst.envelope) {
        instObj.envelope = [
          Number.parseFloat(formatFloat(inst.envelope.attackSeconds)),
          Number.parseFloat(formatFloat(inst.envelope.decaySeconds)),
          Number.parseFloat(formatFloat(inst.envelope.sustainLevel)),
          Number.parseFloat(formatFloat(inst.envelope.releaseSeconds)),
        ];
      }
      return instObj;
    }),
    meter_events: score.meterEvents.map((m) => ({
      meter: m.meter,
      start_tick: m.startTick,
    })),
    ppq: score.ppq,
    sample_rate_hz: score.sampleRateHz,
    tempo_events: score.tempoEvents.map((t) => ({
      bpm: Number.parseFloat(formatFloat(t.bpm)),
      start_tick: t.startTick,
    })),
    title: score.title,
    tracks: score.tracks.map((tr) => ({
      id: tr.id,
      instrument_id: tr.instrumentId,
    })),
  };

  return JSON.stringify(obj, null, 2);
}

// Attach toJson to PortableScore prototype
PortableScore.prototype.toJson = function (): string {
  return formatPortableScoreJson(this);
};

(PortableScore as unknown as { fromJson: typeof parsePortableScoreJson }).fromJson =
  parsePortableScoreJson;
