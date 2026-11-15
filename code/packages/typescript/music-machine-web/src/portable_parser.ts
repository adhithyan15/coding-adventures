/**
 * Iterative parser and formatter for canonical `music-machine-score/v2` portable scores (MUS04).
 * Strictly iterative, zero regex (CWE-1333), protected against recursion (CWE-674) and prototype pollution (CWE-1321).
 */

import {
  AdsrEnvelope,
  HarmonicPartial,
  instrumentForGmProgram,
  MAX_NOTE_DURATION_SECONDS,
} from "@coding-adventures/musical-instruments";

import {
  DEFAULT_MAX_EVENT_COUNT,
  DEFAULT_MAX_LINE_LENGTH,
  DEFAULT_MAX_SCORE_LENGTH,
  DEFAULT_PPQ,
  DEFAULT_SAMPLE_RATE_HZ,
  MAX_EVENTS,
  MAX_LINES,
  MAX_PARTIALS,
  MAX_PPQ,
  MAX_SAMPLE_RATE_HZ,
  MAX_SCORE_BYTES,
  MIN_SAMPLE_RATE_HZ,
  PORTABLE_FORMAT_VERSION,
} from "./constants.js";
import { splitWhitespace } from "./score_parser.js";
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
  isIdentifier,
  resolveInstrumentId,
  validateIdentifier,
  validateMeter,
  validateNonNegativeInt,
  validatePositive,
  validatePositiveInt,
  validateSafeInteger,
  validateUnitFloat,
} from "./validation.js";

/**
 * Parse property tokens matching `key=value` (linear-time, zero regex).
 */
function parseProperties(tokens: string[]): Map<string, string> {
  const props = new Map<string, string>();
  for (let i = 0; i < tokens.length; i += 1) {
    const tok = tokens[i];
    const eqIdx = tok.indexOf("=");
    if (eqIdx <= 0 || eqIdx !== tok.lastIndexOf("=")) {
      throw new RangeError(`property '${tok}' must be key=value`);
    }
    const key = tok.slice(0, eqIdx);
    const val = tok.slice(eqIdx + 1);
    assertSafeProperty(key);
    if (props.has(key)) {
      throw new RangeError(`duplicate property '${key}'`);
    }
    props.set(key, val);
  }
  return props;
}

/**
 * Parse comma-separated harmonic partials e.g. "1:1.0,2:0.5:0.25".
 */
function parseHarmonics(text: string): HarmonicPartial[] {
  const rawParts: string[] = [];
  let start = 0;
  for (let i = 0; i < text.length; i += 1) {
    if (text[i] === ",") {
      const part = text.slice(start, i).trim();
      if (part.length > 0) {
        rawParts.push(part);
      }
      start = i + 1;
    }
  }
  const lastPart = text.slice(start).trim();
  if (lastPart.length > 0) {
    rawParts.push(lastPart);
  }

  if (rawParts.length === 0) {
    throw new RangeError("harmonics must contain at least one partial");
  }
  if (rawParts.length > MAX_PARTIALS) {
    throw new RangeError(
      `harmonics count ${String(rawParts.length)} exceeds MAX_PARTIALS=${String(MAX_PARTIALS)}`,
    );
  }

  const partials: HarmonicPartial[] = [];
  for (let i = 0; i < rawParts.length; i += 1) {
    const item = rawParts[i];
    const colonParts = item.split(":");
    if (colonParts.length !== 2 && colonParts.length !== 3) {
      throw new RangeError(
        `harmonic partial '${item}' must be multiplier:amplitude[:phase]`,
      );
    }
    const mult = Number.parseFloat(colonParts[0]);
    const amp = Number.parseFloat(colonParts[1]);
    const phase = colonParts.length === 3 ? Number.parseFloat(colonParts[2]) : 0.0;

    if (!Number.isFinite(mult) || mult <= 0.0) {
      throw new RangeError(`invalid multiplier in partial '${item}'`);
    }
    if (!Number.isFinite(amp) || amp < 0.0) {
      throw new RangeError(`invalid amplitude in partial '${item}'`);
    }
    if (!Number.isFinite(phase)) {
      throw new RangeError(`invalid phase in partial '${item}'`);
    }

    partials.push(new HarmonicPartial(mult, amp, phase));
  }
  return partials;
}

/**
 * Parse comma-separated ADSR envelope "attack,decay,sustain,release".
 */
function parseEnvelope(text: string): AdsrEnvelope {
  const parts = text.split(",").map((s) => s.trim());
  if (parts.length !== 4) {
    throw new RangeError(
      `envelope must be attack,decay,sustain,release, got '${text}'`,
    );
  }
  const atk = Number.parseFloat(parts[0]);
  const dec = Number.parseFloat(parts[1]);
  const sus = Number.parseFloat(parts[2]);
  const rel = Number.parseFloat(parts[3]);

  if (!Number.isFinite(atk) || !Number.isFinite(dec) || !Number.isFinite(sus) || !Number.isFinite(rel)) {
    throw new RangeError(`non-finite value in envelope '${text}'`);
  }
  if (atk > MAX_NOTE_DURATION_SECONDS || dec > MAX_NOTE_DURATION_SECONDS || rel > MAX_NOTE_DURATION_SECONDS) {
    throw new RangeError(
      `envelope times must be <= ${String(MAX_NOTE_DURATION_SECONDS)}`,
    );
  }

  return new AdsrEnvelope(atk, dec, sus, rel);
}

/**
 * Format float canonically for deterministic V2 text and JSON output.
 */
export function formatFloat(val: number): string {
  if (!Number.isFinite(val)) {
    throw new RangeError(`cannot format non-finite float ${String(val)}`);
  }
  const s = String(val);
  if (!s.includes(".") && !s.includes("e") && !s.includes("E")) {
    return `${s}.0`;
  }
  return s;
}

/**
 * Parse canonical `music-machine-score/v2` multi-track score text.
 */
export function parsePortableScore(
  text: string,
  options: {
    readonly maxScoreLength?: number;
    readonly maxLineLength?: number;
    readonly maxEventCount?: number;
  } = {},
): PortableScore {
  return parsePortableScoreWithLimits(
    text,
    options.maxScoreLength ?? DEFAULT_MAX_SCORE_LENGTH,
    options.maxLineLength ?? DEFAULT_MAX_LINE_LENGTH,
    options.maxEventCount ?? DEFAULT_MAX_EVENT_COUNT,
  );
}

/**
 * Parse canonical `music-machine-score/v2` multi-track score text with explicit bounds.
 */
export function parsePortableScoreWithLimits(
  text: string,
  maxScoreLength: number = DEFAULT_MAX_SCORE_LENGTH,
  maxLineLength: number = DEFAULT_MAX_LINE_LENGTH,
  maxEventCount: number = DEFAULT_MAX_EVENT_COUNT,
): PortableScore {
  if (typeof text !== "string") {
    throw new TypeError("score text must be a string");
  }
  const validMaxScoreLength = validatePositiveInt("maxScoreLength", maxScoreLength);
  if (validMaxScoreLength > MAX_SCORE_BYTES) {
    throw new RangeError(`maxScoreLength exceeds MAX_SCORE_BYTES=${String(MAX_SCORE_BYTES)}`);
  }
  const validMaxLineLength = validatePositiveInt("maxLineLength", maxLineLength);
  if (validMaxLineLength > DEFAULT_MAX_LINE_LENGTH) {
    throw new RangeError(`maxLineLength exceeds maxLineLength=${String(DEFAULT_MAX_LINE_LENGTH)}`);
  }
  const validMaxEventCount = validatePositiveInt("maxEventCount", maxEventCount);
  if (validMaxEventCount > MAX_EVENTS) {
    throw new RangeError(`maxEventCount exceeds MAX_EVENTS=${String(MAX_EVENTS)}`);
  }

  if (text.length > validMaxScoreLength) {
    throw new RangeError(
      `score text length ${String(text.length)} exceeds maxScoreLength=${String(validMaxScoreLength)}`,
    );
  }

  let sawFormat = false;
  let title = "Untitled";
  let ppq = DEFAULT_PPQ;
  let sampleRateHz = DEFAULT_SAMPLE_RATE_HZ;
  const tempoEvents: TempoEvent[] = [];
  const meterEvents: MeterEvent[] = [];
  const instruments: InstrumentDeclaration[] = [];
  const tracks: TrackDeclaration[] = [];
  const events: PortableScoreEvent[] = [];

  // Split lines without regex
  const lines: string[] = [];
  let lineStart = 0;
  for (let i = 0; i < text.length; i += 1) {
    if (text[i] === "\n") {
      lines.push(text.slice(lineStart, i));
      lineStart = i + 1;
    }
  }
  if (lineStart <= text.length) {
    lines.push(text.slice(lineStart));
  }

  if (lines.length > MAX_LINES) {
    throw new RangeError(
      `line count ${String(lines.length)} exceeds MAX_LINES=${String(MAX_LINES)}`,
    );
  }

  for (let lineIdx = 0; lineIdx < lines.length; lineIdx += 1) {
    const lineNumber = lineIdx + 1;
    const rawLine = lines[lineIdx];

    if (rawLine.length > validMaxLineLength) {
      throw new RangeError(
        `line ${String(lineNumber)}: line length ${String(rawLine.length)} exceeds maxLineLength=${String(validMaxLineLength)}`,
      );
    }

    const line = rawLine.trim();
    if (line.length === 0 || line.startsWith("#")) {
      continue;
    }

    if (!sawFormat) {
      if (line !== `format: ${PORTABLE_FORMAT_VERSION}`) {
        throw new RangeError(
          `line ${String(lineNumber)}: first directive must be 'format: ${PORTABLE_FORMAT_VERSION}'`,
        );
      }
      sawFormat = true;
      continue;
    }

    // Check for directives: title:, ppq:, sample_rate:
    const colonIdx = line.indexOf(":");
    if (colonIdx > 0) {
      const prefix = line.slice(0, colonIdx).trim().toLowerCase();
      if (prefix === "title" || prefix === "ppq" || prefix === "sample_rate") {
        const value = line.slice(colonIdx + 1).trim();
        if (prefix === "title") {
          title = value;
        } else if (prefix === "ppq") {
          const val = Number.parseInt(value, 10);
          if (!Number.isInteger(val) || val <= 0 || val > MAX_PPQ) {
            throw new RangeError(
              `line ${String(lineNumber)}: ppq must be in [1, ${String(MAX_PPQ)}], got ${value}`,
            );
          }
          ppq = val;
        } else if (prefix === "sample_rate") {
          const val = Number.parseInt(value, 10);
          if (
            !Number.isInteger(val) ||
            val < MIN_SAMPLE_RATE_HZ ||
            val > MAX_SAMPLE_RATE_HZ
          ) {
            throw new RangeError(
              `line ${String(lineNumber)}: sample_rate must be in [${String(MIN_SAMPLE_RATE_HZ)}, ${String(MAX_SAMPLE_RATE_HZ)}], got ${value}`,
            );
          }
          sampleRateHz = val;
        }
        continue;
      }
    }

    const tokens = splitWhitespace(line);
    if (tokens.length === 0) {
      continue;
    }
    const command = tokens[0];

    switch (command) {
      case "tempo": {
        if (tokens.length !== 3) {
          throw new RangeError(
            `line ${String(lineNumber)}: tempo lines must be: tempo <start_tick> <bpm>`,
          );
        }
        const startTick = Number.parseInt(tokens[1], 10);
        const bpm = Number.parseFloat(tokens[2]);
        if (!Number.isInteger(startTick) || startTick < 0) {
          throw new RangeError(
            `line ${String(lineNumber)}: invalid tempo start_tick '${tokens[1]}'`,
          );
        }
        if (!Number.isFinite(bpm) || bpm <= 0.0) {
          throw new RangeError(
            `line ${String(lineNumber)}: tempo bpm must be > 0.0, got ${String(bpm)}`,
          );
        }
        tempoEvents.push(new TempoEvent(startTick, bpm));
        break;
      }
      case "meter": {
        if (tokens.length !== 3) {
          throw new RangeError(
            `line ${String(lineNumber)}: meter lines must be: meter <start_tick> <meter>`,
          );
        }
        const startTick = Number.parseInt(tokens[1], 10);
        if (!Number.isInteger(startTick) || startTick < 0) {
          throw new RangeError(
            `line ${String(lineNumber)}: invalid meter start_tick '${tokens[1]}'`,
          );
        }
        validateMeter(tokens[2]);
        meterEvents.push(new MeterEvent(startTick, tokens[2]));
        break;
      }
      case "instrument": {
        if (tokens.length < 3) {
          throw new RangeError(
            `line ${String(lineNumber)}: instrument lines must include id and properties`,
          );
        }
        const instId = tokens[1];
        validateIdentifier("instrument id", instId);

        const props = parseProperties(tokens.slice(2));
        const allowed = new Set([
          "gain",
          "program",
          "profile",
          "kind",
          "kit",
          "harmonics",
          "envelope",
        ]);
        for (const k of props.keys()) {
          if (!allowed.has(k)) {
            throw new RangeError(
              `line ${String(lineNumber)}: unknown instrument property '${k}'`,
            );
          }
        }

        const gainStr = props.get("gain");
        const gain = gainStr !== undefined ? Number.parseFloat(gainStr) : 1.0;
        validateUnitFloat("gain", gain);

        const hasProgram = props.has("program");
        const hasProfile = props.has("profile");
        const hasKind = props.has("kind");
        const selectorCount =
          (hasProgram ? 1 : 0) + (hasProfile ? 1 : 0) + (hasKind ? 1 : 0);
        if (selectorCount !== 1) {
          throw new RangeError(
            `line ${String(lineNumber)}: instrument must specify exactly one of program, profile, or kind`,
          );
        }

        if (hasProgram) {
          const progStr = props.get("program")!;
          const prog = Number.parseInt(progStr, 10);
          if (!Number.isInteger(prog) || prog < 1 || prog > 128) {
            throw new RangeError(
              `line ${String(lineNumber)}: program must be in [1, 128], got ${progStr}`,
            );
          }
          const p = instrumentForGmProgram(prog);
          instruments.push(
            new InstrumentDeclaration(
              instId,
              p.id,
              gain,
              "preset",
              null,
              null,
              null,
            ),
          );
        } else if (hasProfile) {
          const profStr = props.get("profile")!;
          const resolved = resolveInstrumentId(profStr);
          instruments.push(
            new InstrumentDeclaration(
              instId,
              resolved,
              gain,
              "preset",
              null,
              null,
              null,
            ),
          );
        } else if (hasKind) {
          const kindStr = props.get("kind")!;
          switch (kindStr) {
            case "sine":
            case "silence": {
              instruments.push(
                new InstrumentDeclaration(
                  instId,
                  kindStr,
                  gain,
                  kindStr,
                  null,
                  null,
                  null,
                ),
              );
              break;
            }
            case "drumkit": {
              const kitName = props.get("kit") ?? "standard_kit";
              const normKit = kitName === "standard" ? "standard_kit" : kitName;
              instruments.push(
                new InstrumentDeclaration(
                  instId,
                  "drumkit",
                  gain,
                  "drumkit",
                  normKit,
                  null,
                  null,
                ),
              );
              break;
            }
            case "additive": {
              const harmStr = props.get("harmonics");
              if (!harmStr) {
                throw new RangeError(
                  `line ${String(lineNumber)}: kind=additive must provide harmonics`,
                );
              }
              const harmonics = parseHarmonics(harmStr);
              const envStr = props.get("envelope");
              const envelope = envStr ? parseEnvelope(envStr) : null;
              instruments.push(
                new InstrumentDeclaration(
                  instId,
                  instId,
                  gain,
                  "additive",
                  null,
                  harmonics,
                  envelope,
                ),
              );
              break;
            }
            default:
              throw new RangeError(
                `line ${String(lineNumber)}: unsupported instrument kind '${kindStr}'`,
              );
          }
        }
        break;
      }
      case "track": {
        if (tokens.length < 3) {
          throw new RangeError(
            `line ${String(lineNumber)}: track lines must include id and instrument=<id>`,
          );
        }
        const trackId = tokens[1];
        validateIdentifier("track id", trackId);

        const props = parseProperties(tokens.slice(2));
        const instId = props.get("instrument");
        if (!instId) {
          throw new RangeError(
            `line ${String(lineNumber)}: track must include instrument=<id>`,
          );
        }
        for (const k of props.keys()) {
          if (k !== "instrument") {
            throw new RangeError(
              `line ${String(lineNumber)}: unknown track property '${k}'`,
            );
          }
        }

        tracks.push(new TrackDeclaration(trackId, instId));
        break;
      }
      case "event": {
        if (events.length >= validMaxEventCount) {
          throw new RangeError(
            `line ${String(lineNumber)}: event count exceeds maxEventCount=${String(validMaxEventCount)}`,
          );
        }
        if (tokens.length < 5) {
          throw new RangeError(
            `line ${String(lineNumber)}: event lines must be: event <track> <start> <duration> note|rest|hit ...`,
          );
        }
        const trackId = tokens[1];
        const startTick = Number.parseInt(tokens[2], 10);
        const durationTick = Number.parseInt(tokens[3], 10);
        const kindStr = tokens[4];

        if (!Number.isInteger(startTick) || startTick < 0) {
          throw new RangeError(
            `line ${String(lineNumber)}: invalid event start_tick '${tokens[2]}'`,
          );
        }
        if (!Number.isInteger(durationTick) || durationTick <= 0) {
          throw new RangeError(
            `line ${String(lineNumber)}: invalid event duration_tick '${tokens[3]}'`,
          );
        }
        if (kindStr !== "note" && kindStr !== "rest" && kindStr !== "hit") {
          throw new RangeError(
            `line ${String(lineNumber)}: event kind must be 'note', 'rest', or 'hit', got '${kindStr}'`,
          );
        }

        const sourceOrder = events.length;

        if (kindStr === "rest") {
          if (tokens.length !== 5) {
            throw new RangeError(
              `line ${String(lineNumber)}: rest events must not include notes or properties`,
            );
          }
          events.push(
            new PortableScoreEvent(
              trackId,
              startTick,
              durationTick,
              "rest",
              [],
              1.0,
              sourceOrder,
              null,
            ),
          );
        } else if (kindStr === "hit") {
          if (tokens.length < 6) {
            throw new RangeError(
              `line ${String(lineNumber)}: hit events must include a hit_id`,
            );
          }
          const hitId = tokens[5].toLowerCase();
          const props = parseProperties(tokens.slice(6));
          for (const k of props.keys()) {
            if (k !== "velocity") {
              throw new RangeError(
                `line ${String(lineNumber)}: unknown event property '${k}'`,
              );
            }
          }
          const vStr = props.get("velocity");
          const velocity = vStr !== undefined ? Number.parseFloat(vStr) : 1.0;
          validateUnitFloat("velocity", velocity);

          events.push(
            new PortableScoreEvent(
              trackId,
              startTick,
              durationTick,
              "hit",
              [],
              velocity,
              sourceOrder,
              hitId,
            ),
          );
        } else {
          // note
          if (tokens.length < 6) {
            throw new RangeError(
              `line ${String(lineNumber)}: note events must include a pitch list`,
            );
          }
          const pitchList = tokens[5];
          const noteStrings = pitchList
            .split(",")
            .map((s) => s.trim())
            .filter((s) => s.length > 0);
          if (noteStrings.length === 0) {
            throw new RangeError(
              `line ${String(lineNumber)}: note events must include at least one note`,
            );
          }

          const props = parseProperties(tokens.slice(6));
          for (const k of props.keys()) {
            if (k !== "velocity") {
              throw new RangeError(
                `line ${String(lineNumber)}: unknown event property '${k}'`,
              );
            }
          }
          const vStr = props.get("velocity");
          const velocity = vStr !== undefined ? Number.parseFloat(vStr) : 1.0;
          validateUnitFloat("velocity", velocity);

          events.push(
            new PortableScoreEvent(
              trackId,
              startTick,
              durationTick,
              "note",
              noteStrings,
              velocity,
              sourceOrder,
              null,
            ),
          );
        }
        break;
      }
      default:
        throw new RangeError(
          `line ${String(lineNumber)}: unknown command '${command}'`,
        );
    }
  }

  if (!sawFormat) {
    throw new RangeError(
      `first directive must be 'format: ${PORTABLE_FORMAT_VERSION}'`,
    );
  }

  return new PortableScore({
    formatVersion: PORTABLE_FORMAT_VERSION,
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
 * Format a PortableScore back to canonical human-readable V2 text.
 */
export function toPortableScoreText(score: PortableScore): string {
  const lines: string[] = [];
  lines.push(`format: ${score.formatVersion}`);
  lines.push(`title: ${score.title}`);
  lines.push(`ppq: ${String(score.ppq)}`);
  lines.push(`sample_rate: ${String(score.sampleRateHz)}`);

  for (let i = 0; i < score.tempoEvents.length; i += 1) {
    const t = score.tempoEvents[i];
    lines.push(`tempo ${String(t.startTick)} ${formatFloat(t.bpm)}`);
  }
  for (let i = 0; i < score.meterEvents.length; i += 1) {
    const m = score.meterEvents[i];
    lines.push(`meter ${String(m.startTick)} ${m.meter}`);
  }

  if (score.instruments.length > 0) {
    lines.push("");
    for (let i = 0; i < score.instruments.length; i += 1) {
      const inst = score.instruments[i];
      switch (inst.kind) {
        case "drumkit": {
          const kit = inst.kit ?? "standard_kit";
          lines.push(
            `instrument ${inst.id} kind=drumkit kit=${kit} gain=${formatFloat(inst.gain)}`,
          );
          break;
        }
        case "additive": {
          const harmsStr = inst.harmonics
            ? inst.harmonics
                .map((h) => {
                  if (h.phaseCycles !== 0.0) {
                    return `${formatFloat(h.frequencyMultiplier)}:${formatFloat(h.amplitude)}:${formatFloat(h.phaseCycles)}`;
                  }
                  return `${formatFloat(h.frequencyMultiplier)}:${formatFloat(h.amplitude)}`;
                })
                .join(",")
            : "";
          const envStr = inst.envelope
            ? `${formatFloat(inst.envelope.attackSeconds)},${formatFloat(inst.envelope.decaySeconds)},${formatFloat(inst.envelope.sustainLevel)},${formatFloat(inst.envelope.releaseSeconds)}`
            : "";
          lines.push(
            `instrument ${inst.id} kind=additive harmonics=${harmsStr} envelope=${envStr} gain=${formatFloat(inst.gain)}`,
          );
          break;
        }
        case "sine":
        case "silence": {
          lines.push(
            `instrument ${inst.id} kind=${inst.kind} gain=${formatFloat(inst.gain)}`,
          );
          break;
        }
        default: {
          lines.push(
            `instrument ${inst.id} profile=${inst.profileId} gain=${formatFloat(inst.gain)}`,
          );
          break;
        }
      }
    }
  }

  if (score.tracks.length > 0) {
    lines.push("");
    for (let i = 0; i < score.tracks.length; i += 1) {
      const tr = score.tracks[i];
      lines.push(`track ${tr.id} instrument=${tr.instrumentId}`);
    }
  }

  if (score.events.length > 0) {
    lines.push("");
    for (let i = 0; i < score.events.length; i += 1) {
      const ev = score.events[i];
      switch (ev.kind) {
        case "note":
          lines.push(
            `event ${ev.trackId} ${String(ev.startTick)} ${String(ev.durationTick)} note ${ev.notes.join(",")} velocity=${formatFloat(ev.velocity)}`,
          );
          break;
        case "hit":
          lines.push(
            `event ${ev.trackId} ${String(ev.startTick)} ${String(ev.durationTick)} hit ${ev.hitId ?? "kick"} velocity=${formatFloat(ev.velocity)}`,
          );
          break;
        case "rest":
          lines.push(
            `event ${ev.trackId} ${String(ev.startTick)} ${String(ev.durationTick)} rest`,
          );
          break;
      }
    }
  }

  return lines.join("\n");
}

// Attach toText to PortableScore prototype
PortableScore.prototype.toText = function (): string {
  return toPortableScoreText(this);
};
