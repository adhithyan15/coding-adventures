/**
 * High-level programmatic builder, phrase sequencer, motif capturer, and section arranger (MUS04).
 * Hardened for CWE-20, CWE-190, CWE-400, CWE-1321, and CWE-1333.
 */

import {
  AdsrEnvelope,
  HarmonicPartial,
  instrumentForGmProgram,
  parseNote,
} from "@coding-adventures/musical-instruments";
import { ALL_STANDARD_HIT_IDS } from "@coding-adventures/percussion-kit";

import {
  BEAT_TICK_TOLERANCE,
  DEFAULT_PPQ,
  DEFAULT_SAMPLE_RATE_HZ,
  MAX_PPQ,
  MAX_SAMPLE_RATE_HZ,
  PORTABLE_FORMAT_VERSION,
} from "./constants.js";
import {
  InstrumentDeclaration,
  MeterEvent,
  PortableScore,
  PortableScoreEvent,
  ScoreEventKind,
  TempoEvent,
  TrackDeclaration,
} from "./types.js";
import {
  assertSafeProperty,
  meterParts,
  resolveInstrumentId,
  transposeNoteText,
  validateFinite,
  validateIdentifier,
  validateMeter,
  validateNonNegative,
  validateNonNegativeInt,
  validatePositive,
  validatePositiveInt,
  validateSafeInteger,
  validateUnitFloat,
} from "./validation.js";

export class PhraseMotifEvent {
  readonly offsetTick: number;
  readonly durationTick: number;
  readonly kind: ScoreEventKind;
  readonly notes: readonly string[];
  readonly velocity: number;
  readonly hitId: string | null;

  constructor(
    offsetTick: number,
    durationTick: number,
    kind: ScoreEventKind,
    notes: readonly string[] = [],
    velocity: number = 1.0,
    hitId: string | null = null,
  ) {
    this.offsetTick = validateNonNegativeInt("offsetTick", offsetTick);
    this.durationTick = validatePositiveInt("durationTick", durationTick);
    if (kind !== "note" && kind !== "rest" && kind !== "hit") {
      throw new RangeError(
        `kind must be 'note', 'rest', or 'hit', got '${String(kind)}'`,
      );
    }
    this.kind = kind;
    this.velocity = validateUnitFloat("velocity", velocity);

    if (kind === "note") {
      if (!notes || notes.length === 0) {
        throw new RangeError("note motif events must include at least one note");
      }
      for (const n of notes) {
        parseNote(n);
      }
      this.notes = Object.freeze([...notes]);
      this.hitId = null;
    } else if (kind === "hit") {
      if (notes && notes.length > 0) {
        throw new RangeError("hit motif events cannot include notes");
      }
      if (!hitId) {
        throw new RangeError("hit motif events must include a hitId");
      }
      const lower = hitId.toLowerCase();
      if (!ALL_STANDARD_HIT_IDS.includes(lower)) {
        throw new RangeError(`unknown hit_id '${lower}'`);
      }
      this.notes = Object.freeze([]);
      this.hitId = lower;
    } else {
      if (notes && notes.length > 0) {
        throw new RangeError("rest motif events cannot include notes");
      }
      this.notes = Object.freeze([]);
      this.hitId = null;
    }
  }
}

export class PhraseMotif {
  readonly durationTick: number;
  readonly events: readonly PhraseMotifEvent[];

  constructor(durationTick: number, events: readonly PhraseMotifEvent[]) {
    this.durationTick = validateNonNegativeInt("durationTick", durationTick);
    for (const ev of events) {
      if (!(ev instanceof PhraseMotifEvent)) {
        throw new TypeError("events must be PhraseMotifEvent instances");
      }
      if (ev.offsetTick + ev.durationTick > this.durationTick) {
        throw new RangeError("event extends past motif duration_tick");
      }
    }
    this.events = Object.freeze([...events]);
  }
}

export class ArrangementSectionEvent {
  readonly trackId: string;
  readonly offsetTick: number;
  readonly durationTick: number;
  readonly kind: ScoreEventKind;
  readonly notes: readonly string[];
  readonly velocity: number;
  readonly hitId: string | null;

  constructor(
    trackId: string,
    offsetTick: number,
    durationTick: number,
    kind: ScoreEventKind,
    notes: readonly string[] = [],
    velocity: number = 1.0,
    hitId: string | null = null,
  ) {
    this.trackId = validateIdentifier("trackId", trackId);
    this.offsetTick = validateNonNegativeInt("offsetTick", offsetTick);
    this.durationTick = validatePositiveInt("durationTick", durationTick);
    if (kind !== "note" && kind !== "rest" && kind !== "hit") {
      throw new RangeError(
        `kind must be 'note', 'rest', or 'hit', got '${String(kind)}'`,
      );
    }
    this.kind = kind;
    this.velocity = validateUnitFloat("velocity", velocity);

    if (kind === "note") {
      if (!notes || notes.length === 0) {
        throw new RangeError("note section events must include at least one note");
      }
      for (const n of notes) {
        parseNote(n);
      }
      this.notes = Object.freeze([...notes]);
      this.hitId = null;
    } else if (kind === "hit") {
      if (notes && notes.length > 0) {
        throw new RangeError("hit section events cannot include notes");
      }
      if (!hitId) {
        throw new RangeError("hit section events must include a hitId");
      }
      const lower = hitId.toLowerCase();
      if (!ALL_STANDARD_HIT_IDS.includes(lower)) {
        throw new RangeError(`unknown hit_id '${lower}'`);
      }
      this.notes = Object.freeze([]);
      this.hitId = lower;
    } else {
      if (notes && notes.length > 0) {
        throw new RangeError("rest section events cannot include notes");
      }
      this.notes = Object.freeze([]);
      this.hitId = null;
    }
  }
}

export class ArrangementSection {
  readonly durationTick: number;
  readonly events: readonly ArrangementSectionEvent[];

  constructor(
    durationTick: number,
    events: readonly ArrangementSectionEvent[],
  ) {
    this.durationTick = validateNonNegativeInt("durationTick", durationTick);
    for (const ev of events) {
      if (!(ev instanceof ArrangementSectionEvent)) {
        throw new TypeError("events must be ArrangementSectionEvent instances");
      }
      if (ev.offsetTick + ev.durationTick > this.durationTick) {
        throw new RangeError("section event extends beyond end_tick");
      }
    }
    this.events = Object.freeze([...events]);
  }
}

export class PortableScoreBuilder {
  title: string;
  ppq: number;
  sampleRateHz: number;
  readonly formatVersion: string;

  private readonly tempoEvents: TempoEvent[] = [];
  private readonly meterEvents: MeterEvent[] = [];
  private readonly instruments: InstrumentDeclaration[] = [];
  private readonly tracks: TrackDeclaration[] = [];
  private readonly events: PortableScoreEvent[] = [];
  private sourceOrderCounter = 0;

  constructor(options: {
    readonly title?: string;
    readonly ppq?: number;
    readonly sampleRateHz?: number;
    readonly formatVersion?: string;
  } = {}) {
    this.title = options.title ?? "Untitled";
    this.ppq = validatePositiveInt("ppq", options.ppq ?? DEFAULT_PPQ);
    if (this.ppq > MAX_PPQ) {
      throw new RangeError(`ppq must be <= ${String(MAX_PPQ)}`);
    }
    this.sampleRateHz = validateSafeInteger(
      "sampleRateHz",
      options.sampleRateHz ?? DEFAULT_SAMPLE_RATE_HZ,
    );
    if (
      this.sampleRateHz <= 0 ||
      this.sampleRateHz > MAX_SAMPLE_RATE_HZ
    ) {
      throw new RangeError(
        `sampleRateHz must be in (0, ${String(MAX_SAMPLE_RATE_HZ)}]`,
      );
    }
    this.formatVersion = options.formatVersion ?? PORTABLE_FORMAT_VERSION;
    if (this.formatVersion !== PORTABLE_FORMAT_VERSION) {
      throw new RangeError(
        `formatVersion must be '${PORTABLE_FORMAT_VERSION}'`,
      );
    }
  }

  addTempo(startTick: number, bpm: number): this {
    this.tempoEvents.push(new TempoEvent(startTick, bpm));
    return this;
  }

  addMeter(startTick: number, meter: string): this {
    this.meterEvents.push(new MeterEvent(startTick, meter));
    return this;
  }

  addInstrument(
    instrumentId: string,
    options: {
      readonly profile?: string;
      readonly program?: number;
      readonly kind?: string;
      readonly kit?: string;
      readonly harmonics?: readonly HarmonicPartial[];
      readonly envelope?: AdsrEnvelope;
      readonly gain?: number;
    } = {},
  ): this {
    const { profile, program, kind, kit, harmonics, envelope, gain = 1.0 } = options;
    const selectorCount =
      (profile !== undefined ? 1 : 0) +
      (program !== undefined ? 1 : 0) +
      (kind !== undefined ? 1 : 0);
    if (selectorCount !== 1) {
      throw new RangeError(
        "instrument must specify exactly one of profile, program, or kind",
      );
    }

    let decl: InstrumentDeclaration;
    if (program !== undefined) {
      const p = instrumentForGmProgram(program);
      decl = new InstrumentDeclaration(
        instrumentId,
        p.id,
        gain,
        "preset",
        null,
        null,
        null,
      );
    } else if (profile !== undefined) {
      const pId = resolveInstrumentId(profile);
      decl = new InstrumentDeclaration(
        instrumentId,
        pId,
        gain,
        "preset",
        null,
        null,
        null,
      );
    } else {
      if (kind === "drumkit") {
        decl = new InstrumentDeclaration(
          instrumentId,
          "drumkit",
          gain,
          "drumkit",
          kit ?? "standard",
          null,
          null,
        );
      } else if (kind === "additive") {
        decl = new InstrumentDeclaration(
          instrumentId,
          instrumentId,
          gain,
          "additive",
          null,
          harmonics ?? null,
          envelope ?? null,
        );
      } else if (kind === "sine" || kind === "silence") {
        decl = new InstrumentDeclaration(
          instrumentId,
          kind,
          gain,
          kind,
          null,
          null,
          null,
        );
      } else {
        throw new RangeError(
          `kind must be sine, silence, drumkit, or additive, got '${String(kind)}'`,
        );
      }
    }

    this.instruments.push(decl);
    return this;
  }

  addTrack(trackId: string, options: { readonly instrumentId: string }): this {
    this.tracks.push(new TrackDeclaration(trackId, options.instrumentId));
    return this;
  }

  addNote(
    trackId: string,
    startTick: number,
    durationTick: number,
    note: string,
    options: { readonly velocity?: number } = {},
  ): this {
    this.events.push(
      new PortableScoreEvent(
        trackId,
        startTick,
        durationTick,
        "note",
        [note],
        options.velocity ?? 1.0,
        this.nextSourceOrder(),
        null,
      ),
    );
    return this;
  }

  addChord(
    trackId: string,
    startTick: number,
    durationTick: number,
    notes: readonly string[],
    options: { readonly velocity?: number } = {},
  ): this {
    if (!Array.isArray(notes) || notes.length === 0) {
      throw new RangeError("notes must be a non-empty array of note strings");
    }
    this.events.push(
      new PortableScoreEvent(
        trackId,
        startTick,
        durationTick,
        "note",
        notes,
        options.velocity ?? 1.0,
        this.nextSourceOrder(),
        null,
      ),
    );
    return this;
  }

  addHit(
    trackId: string,
    startTick: number,
    durationTick: number,
    hitId: string,
    options: { readonly velocity?: number } = {},
  ): this {
    this.events.push(
      new PortableScoreEvent(
        trackId,
        startTick,
        durationTick,
        "hit",
        [],
        options.velocity ?? 1.0,
        this.nextSourceOrder(),
        hitId,
      ),
    );
    return this;
  }

  addRest(trackId: string, startTick: number, durationTick: number): this {
    this.events.push(
      new PortableScoreEvent(
        trackId,
        startTick,
        durationTick,
        "rest",
        [],
        1.0,
        this.nextSourceOrder(),
        null,
      ),
    );
    return this;
  }

  beatsToTicks(beatCount: number): number {
    validateNonNegative("beatCount", beatCount);
    const rawTicks = beatCount * this.ppq;
    const rounded = Math.round(rawTicks);
    if (Math.abs(rawTicks - rounded) > BEAT_TICK_TOLERANCE) {
      throw new RangeError(
        `beatCount ${String(beatCount)} is not representable at ppq=${String(this.ppq)}`,
      );
    }
    return rounded;
  }

  measureTicks(meter?: string): number {
    const activeMeter = meter ?? this.activeMeter();
    const [num, den] = meterParts(activeMeter);
    const beats = num * (4.0 / den);
    return this.beatsToTicks(beats);
  }

  measureStartTick(measureNumber: number, options: { readonly meter?: string } = {}): number {
    validatePositiveInt("measureNumber", measureNumber);
    return (measureNumber - 1) * this.measureTicks(options.meter);
  }

  tickInMeasure(
    measureNumber: number,
    beatOffset: number = 0.0,
    options: { readonly meter?: string } = {},
  ): number {
    const startTick = this.measureStartTick(measureNumber, options);
    return startTick + this.beatsToTicks(beatOffset);
  }

  phrase(
    trackId: string,
    options: {
      readonly measureNumber?: number;
      readonly beatOffset?: number;
      readonly meter?: string;
    } = {},
  ): PhraseBuilder {
    const startTick = this.tickInMeasure(
      options.measureNumber ?? 1,
      options.beatOffset ?? 0.0,
      { meter: options.meter },
    );
    return new PhraseBuilder(this, trackId, startTick);
  }

  captureMotif(
    trackId: string,
    options: { readonly startTick: number; readonly endTick: number },
  ): PhraseMotif {
    const checkedTrack = validateIdentifier("track id", trackId);
    const start = validateNonNegativeInt("startTick", options.startTick);
    const end = validateNonNegativeInt("endTick", options.endTick);
    if (end < start) {
      throw new RangeError("endTick must be >= startTick");
    }

    const motifEvents: PhraseMotifEvent[] = [];
    for (const event of this.events) {
      if (
        event.trackId === checkedTrack &&
        event.startTick >= start &&
        event.startTick < end
      ) {
        motifEvents.push(
          new PhraseMotifEvent(
            event.startTick - start,
            event.durationTick,
            event.kind,
            event.notes,
            event.velocity,
            event.hitId,
          ),
        );
      }
    }
    return new PhraseMotif(end - start, motifEvents);
  }

  applyMotif(
    motif: PhraseMotif,
    trackId: string,
    startTick: number,
    options: {
      readonly transposeSemitones?: number;
      readonly velocityScale?: number;
      readonly repeatCount?: number;
      readonly repeatSpacingTick?: number;
    } = {},
  ): this {
    if (!(motif instanceof PhraseMotif)) {
      throw new TypeError("motif must be a PhraseMotif");
    }
    const checkedTrack = validateIdentifier("track id", trackId);
    const originTick = validateNonNegativeInt("startTick", startTick);
    const repeats = validatePositiveInt("repeatCount", options.repeatCount ?? 1);
    const spacing =
      options.repeatSpacingTick !== undefined
        ? validatePositiveInt("repeatSpacingTick", options.repeatSpacingTick)
        : motif.durationTick;
    const scale = validateUnitFloat(
      "velocityScale",
      options.velocityScale ?? 1.0,
    );
    const transpose = validateSafeInteger(
      "transposeSemitones",
      options.transposeSemitones ?? 0,
    );

    for (let rep = 0; rep < repeats; rep += 1) {
      const baseTick = originTick + rep * spacing;
      for (const event of motif.events) {
        const evStart = baseTick + event.offsetTick;
        const evVel = Math.min(1.0, event.velocity * scale);

        if (event.kind === "note") {
          const transposedNotes = event.notes.map((n) =>
            transposeNoteText(n, transpose),
          );
          this.events.push(
            new PortableScoreEvent(
              checkedTrack,
              evStart,
              event.durationTick,
              "note",
              transposedNotes,
              evVel,
              this.nextSourceOrder(),
              null,
            ),
          );
        } else if (event.kind === "hit") {
          this.events.push(
            new PortableScoreEvent(
              checkedTrack,
              evStart,
              event.durationTick,
              "hit",
              [],
              evVel,
              this.nextSourceOrder(),
              event.hitId,
            ),
          );
        } else {
          this.events.push(
            new PortableScoreEvent(
              checkedTrack,
              evStart,
              event.durationTick,
              "rest",
              [],
              evVel,
              this.nextSourceOrder(),
              null,
            ),
          );
        }
      }
    }
    return this;
  }

  captureSection(options: {
    readonly startTick: number;
    readonly endTick: number;
    readonly trackIds?: readonly string[];
  }): ArrangementSection {
    const start = validateNonNegativeInt("startTick", options.startTick);
    const end = validateNonNegativeInt("endTick", options.endTick);
    if (end < start) {
      throw new RangeError("endTick must be >= startTick");
    }

    let activeTracks: Set<string> | null = null;
    if (options.trackIds) {
      activeTracks = new Set(
        options.trackIds.map((t) => validateIdentifier("track id", t)),
      );
    }

    const sectionDuration = end - start;
    const sectionEvents: ArrangementSectionEvent[] = [];
    for (const event of this.events) {
      if (activeTracks && !activeTracks.has(event.trackId)) {
        continue;
      }
      if (event.startTick < start || event.startTick >= end) {
        continue;
      }
      const offsetTick = event.startTick - start;
      if (offsetTick + event.durationTick > sectionDuration) {
        throw new RangeError("captured section event extends beyond end_tick");
      }
      sectionEvents.push(
        new ArrangementSectionEvent(
          event.trackId,
          offsetTick,
          event.durationTick,
          event.kind,
          event.notes,
          event.velocity,
          event.hitId,
        ),
      );
    }

    return new ArrangementSection(sectionDuration, sectionEvents);
  }

  applySection(
    section: ArrangementSection,
    startTick: number,
    options: {
      readonly trackMap?: Record<string, string>;
      readonly transposeSemitones?: number | Record<string, number>;
      readonly velocityScale?: number | Record<string, number>;
      readonly repeatCount?: number;
      readonly repeatSpacingTick?: number;
    } = {},
  ): this {
    if (!(section instanceof ArrangementSection)) {
      throw new TypeError("section must be an ArrangementSection");
    }
    const originTick = validateNonNegativeInt("startTick", startTick);
    const repeats = validatePositiveInt("repeatCount", options.repeatCount ?? 1);
    const spacing =
      options.repeatSpacingTick !== undefined
        ? validatePositiveInt("repeatSpacingTick", options.repeatSpacingTick)
        : section.durationTick;

    const trackMap = options.trackMap ?? {};
    const transposeOpt = options.transposeSemitones ?? 0;
    const scaleOpt = options.velocityScale ?? 1.0;

    for (let rep = 0; rep < repeats; rep += 1) {
      const baseTick = originTick + rep * spacing;
      for (const event of section.events) {
        const targetTrack = Object.prototype.hasOwnProperty.call(trackMap, event.trackId)
          ? trackMap[event.trackId]
          : event.trackId;

        let transpose = 0;
        if (typeof transposeOpt === "number") {
          transpose = validateSafeInteger("transposeSemitones", transposeOpt);
        } else if (typeof transposeOpt === "object" && transposeOpt !== null) {
          if (Object.prototype.hasOwnProperty.call(transposeOpt, event.trackId)) {
            transpose = validateSafeInteger(
              `transposeSemitones[${event.trackId}]`,
              transposeOpt[event.trackId],
            );
          }
        }

        let scale = 1.0;
        if (typeof scaleOpt === "number") {
          scale = validateUnitFloat("velocityScale", scaleOpt);
        } else if (typeof scaleOpt === "object" && scaleOpt !== null) {
          if (Object.prototype.hasOwnProperty.call(scaleOpt, event.trackId)) {
            scale = validateUnitFloat(
              `velocityScale[${event.trackId}]`,
              scaleOpt[event.trackId],
            );
          }
        }

        const evStart = baseTick + event.offsetTick;
        const evVel = Math.min(1.0, event.velocity * scale);

        if (event.kind === "note") {
          const transposedNotes = event.notes.map((n) =>
            transposeNoteText(n, transpose),
          );
          this.events.push(
            new PortableScoreEvent(
              targetTrack,
              evStart,
              event.durationTick,
              "note",
              transposedNotes,
              evVel,
              this.nextSourceOrder(),
              null,
            ),
          );
        } else if (event.kind === "hit") {
          this.events.push(
            new PortableScoreEvent(
              targetTrack,
              evStart,
              event.durationTick,
              "hit",
              [],
              evVel,
              this.nextSourceOrder(),
              event.hitId,
            ),
          );
        } else {
          this.events.push(
            new PortableScoreEvent(
              targetTrack,
              evStart,
              event.durationTick,
              "rest",
              [],
              evVel,
              this.nextSourceOrder(),
              null,
            ),
          );
        }
      }
    }
    return this;
  }

  build(): PortableScore {
    return new PortableScore({
      formatVersion: this.formatVersion,
      title: this.title,
      ppq: this.ppq,
      sampleRateHz: this.sampleRateHz,
      tempoEvents: this.tempoEvents,
      meterEvents: this.meterEvents,
      instruments: this.instruments,
      tracks: this.tracks,
      events: this.events,
    });
  }

  toText(): string {
    return this.build().toText();
  }

  private nextSourceOrder(): number {
    const order = this.sourceOrderCounter;
    this.sourceOrderCounter += 1;
    return order;
  }

  private activeMeter(): string {
    if (this.meterEvents.length > 0) {
      return this.meterEvents[this.meterEvents.length - 1].meter;
    }
    return "4/4";
  }
}

export class PhraseBuilder {
  readonly scoreBuilder: PortableScoreBuilder;
  readonly trackId: string;
  startTick: number;
  readonly originTick: number;

  constructor(
    scoreBuilder: PortableScoreBuilder,
    trackId: string,
    startTick: number = 0,
    originTick?: number,
  ) {
    if (!(scoreBuilder instanceof PortableScoreBuilder)) {
      throw new TypeError("scoreBuilder must be a PortableScoreBuilder");
    }
    this.scoreBuilder = scoreBuilder;
    this.trackId = validateIdentifier("track id", trackId);
    this.startTick = validateNonNegativeInt("startTick", startTick);
    this.originTick =
      originTick !== undefined
        ? validateNonNegativeInt("originTick", originTick)
        : this.startTick;
  }

  get currentTick(): number {
    return this.startTick;
  }

  note(
    note: string,
    durationBeats: number,
    options: { readonly velocity?: number } = {},
  ): this {
    const durationTick = this.scoreBuilder.beatsToTicks(durationBeats);
    this.scoreBuilder.addNote(
      this.trackId,
      this.startTick,
      durationTick,
      note,
      options,
    );
    this.startTick += durationTick;
    return this;
  }

  chord(
    notes: readonly string[],
    durationBeats: number,
    options: { readonly velocity?: number } = {},
  ): this {
    const durationTick = this.scoreBuilder.beatsToTicks(durationBeats);
    this.scoreBuilder.addChord(
      this.trackId,
      this.startTick,
      durationTick,
      notes,
      options,
    );
    this.startTick += durationTick;
    return this;
  }

  hit(
    hitId: string,
    durationBeats: number,
    options: { readonly velocity?: number } = {},
  ): this {
    const durationTick = this.scoreBuilder.beatsToTicks(durationBeats);
    this.scoreBuilder.addHit(
      this.trackId,
      this.startTick,
      durationTick,
      hitId,
      options,
    );
    this.startTick += durationTick;
    return this;
  }

  rest(durationBeats: number): this {
    const durationTick = this.scoreBuilder.beatsToTicks(durationBeats);
    this.scoreBuilder.addRest(this.trackId, this.startTick, durationTick);
    this.startTick += durationTick;
    return this;
  }

  advanceBeats(beatCount: number): this {
    this.startTick += this.scoreBuilder.beatsToTicks(beatCount);
    return this;
  }

  jumpToMeasure(
    measureNumber: number,
    beatOffset: number = 0.0,
    options: { readonly meter?: string } = {},
  ): this {
    this.startTick = this.scoreBuilder.tickInMeasure(
      measureNumber,
      beatOffset,
      options,
    );
    return this;
  }

  motif(options: { readonly startTick?: number } = {}): PhraseMotif {
    const start = options.startTick !== undefined ? options.startTick : this.originTick;
    return this.scoreBuilder.captureMotif(this.trackId, {
      startTick: start,
      endTick: this.currentTick,
    });
  }

  applyMotif(
    motif: PhraseMotif,
    options: {
      readonly transposeSemitones?: number;
      readonly velocityScale?: number;
      readonly repeatCount?: number;
      readonly spacingBeats?: number;
      readonly advance?: boolean;
    } = {},
  ): this {
    const spacingTick =
      options.spacingBeats !== undefined
        ? this.scoreBuilder.beatsToTicks(options.spacingBeats)
        : motif.durationTick;
    const repeats = options.repeatCount ?? 1;

    this.scoreBuilder.applyMotif(motif, this.trackId, this.startTick, {
      transposeSemitones: options.transposeSemitones,
      velocityScale: options.velocityScale,
      repeatCount: repeats,
      repeatSpacingTick: spacingTick,
    });

    if (options.advance !== false) {
      this.startTick += spacingTick * repeats;
    }
    return this;
  }
}
