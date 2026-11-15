/**
 * Iterative parser for beginner-friendly sheet music (MUS01 / MUS03).
 * Strictly iterative, zero regex (CWE-1333), protected against recursion (CWE-674) and prototype pollution (CWE-1321).
 */

import { parseNote } from "@coding-adventures/musical-instruments";
import { ALL_STANDARD_HIT_IDS } from "@coding-adventures/percussion-kit";

import {
  DEFAULT_AMPLITUDE,
  DEFAULT_INSTRUMENT_ID,
  DEFAULT_MAX_EVENT_COUNT,
  DEFAULT_MAX_LINE_LENGTH,
  DEFAULT_MAX_SCORE_LENGTH,
  DEFAULT_METER,
  DEFAULT_SAMPLE_RATE_HZ,
  DEFAULT_TEMPO_BPM,
  DIRECTIVE_NAMES,
  MAX_EVENTS,
  MAX_LINES,
  MAX_SAMPLE_RATE_HZ,
  MAX_SCORE_BYTES,
  MIN_SAMPLE_RATE_HZ,
} from "./constants.js";
import { ScoreEvent, TextScore } from "./types.js";
import {
  assertSafeProperty,
  beatsForDurationSymbol,
  isIdentifier,
  resolveInstrumentId,
  validateMeter,
  validatePositive,
  validatePositiveInt,
  validateUnitFloat,
} from "./validation.js";

/**
 * Split text by whitespace without regular expressions (CWE-1333).
 */
export function splitWhitespace(text: string): string[] {
  const result: string[] = [];
  let inWord = false;
  let wordStart = 0;

  for (let i = 0; i < text.length; i += 1) {
    const ch = text.charCodeAt(i);
    // Space, tab, newline, carriage return, form feed
    const isWs = ch === 32 || ch === 9 || ch === 10 || ch === 13 || ch === 12;

    if (isWs) {
      if (inWord) {
        result.push(text.slice(wordStart, i));
        inWord = false;
      }
    } else {
      if (!inWord) {
        wordStart = i;
        inWord = true;
      }
    }
  }

  if (inWord) {
    result.push(text.slice(wordStart));
  }

  return result;
}

function isValidDirectiveName(name: string): boolean {
  if (name.length === 0) {
    return false;
  }
  const first = name.charCodeAt(0);
  const isAlpha =
    (first >= 65 && first <= 90) ||
    (first >= 97 && first <= 122) ||
    first === 95;
  if (!isAlpha) {
    return false;
  }
  for (let i = 1; i < name.length; i += 1) {
    const ch = name.charCodeAt(i);
    const valid =
      (ch >= 65 && ch <= 90) ||
      (ch >= 97 && ch <= 122) ||
      (ch >= 48 && ch <= 57) ||
      ch === 95;
    if (!valid) {
      return false;
    }
  }
  return true;
}

/**
 * Linear-time directive scanner matching `name: value` (zero regex, CWE-1333).
 */
function parseDirectiveLine(line: string): [string, string] | null {
  const colonIdx = line.indexOf(":");
  if (colonIdx <= 0) {
    return null;
  }
  const name = line.slice(0, colonIdx).trim().toLowerCase();
  if (!isValidDirectiveName(name)) {
    return null;
  }
  const value = line.slice(colonIdx + 1).trim();
  return [name, value];
}

/**
 * Parse a single music token (e.g. "G4/e", "R/q", "kick/s") into a ScoreEvent.
 */
function parseScoreToken(
  token: string,
  tempoBpm: number,
  isDrumScore: boolean,
): ScoreEvent {
  const slashIdx = token.indexOf("/");
  if (slashIdx <= 0 || slashIdx !== token.lastIndexOf("/")) {
    throw new RangeError(
      `music token '${token}' must look like pitch/duration`,
    );
  }
  const pitchText = token.slice(0, slashIdx);
  const durationSymbol = token.slice(slashIdx + 1);
  if (pitchText.length === 0 || durationSymbol.length === 0) {
    throw new RangeError(
      `music token '${token}' must look like pitch/duration`,
    );
  }

  const beatCount = beatsForDurationSymbol(durationSymbol);
  const secondsPerBeat = 60.0 / tempoBpm;
  const durationSeconds = beatCount * secondsPerBeat;

  const lowerPitch = pitchText.toLowerCase();
  if (lowerPitch === "r" || lowerPitch === "rest") {
    return new ScoreEvent(
      "rest",
      null,
      null,
      durationSymbol,
      beatCount,
      durationSeconds,
      token,
    );
  }

  if (isDrumScore && ALL_STANDARD_HIT_IDS.includes(lowerPitch)) {
    return new ScoreEvent(
      "hit",
      null,
      lowerPitch,
      durationSymbol,
      beatCount,
      durationSeconds,
      token,
    );
  }

  // Parse as a note
  const note = parseNote(pitchText);
  return new ScoreEvent(
    "note",
    note.toString(),
    null,
    durationSymbol,
    beatCount,
    durationSeconds,
    token,
  );
}

/**
 * Parse beginner-friendly text sheet music into a TextScore model.
 */
export function parseScore(
  text: string,
  options: {
    readonly maxScoreLength?: number;
    readonly maxLineLength?: number;
    readonly maxEventCount?: number;
  } = {},
): TextScore {
  return parseScoreWithLimits(
    text,
    options.maxScoreLength ?? DEFAULT_MAX_SCORE_LENGTH,
    options.maxLineLength ?? DEFAULT_MAX_LINE_LENGTH,
    options.maxEventCount ?? DEFAULT_MAX_EVENT_COUNT,
  );
}

/**
 * Parse beginner-friendly text sheet music with explicit resource bounds (CWE-400 / CWE-674).
 */
export function parseScoreWithLimits(
  text: string,
  maxScoreLength: number = DEFAULT_MAX_SCORE_LENGTH,
  maxLineLength: number = DEFAULT_MAX_LINE_LENGTH,
  maxEventCount: number = DEFAULT_MAX_EVENT_COUNT,
): TextScore {
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

  let title = "Untitled";
  let tempoBpm = DEFAULT_TEMPO_BPM;
  let meter = DEFAULT_METER;
  let amplitude = DEFAULT_AMPLITUDE;
  let sampleRateHz = DEFAULT_SAMPLE_RATE_HZ;
  let instrumentId = DEFAULT_INSTRUMENT_ID;
  let instrumentDirectiveSeen = false;
  let gmProgram: number | null = null;
  let drumkit: string | null = null;
  const events: ScoreEvent[] = [];

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

    const directive = parseDirectiveLine(line);
    if (directive !== null) {
      const [name, value] = directive;
      assertSafeProperty(name);

      if (events.length > 0) {
        throw new RangeError(
          `line ${String(lineNumber)}: directive '${name}' appears after music`,
        );
      }
      if (!DIRECTIVE_NAMES.has(name)) {
        throw new RangeError(
          `line ${String(lineNumber)}: unknown directive '${name}'`,
        );
      }
      if (value.length === 0) {
        throw new RangeError(
          `line ${String(lineNumber)}: directive '${name}' is empty`,
        );
      }

      switch (name) {
        case "title": {
          title = value;
          break;
        }
        case "tempo": {
          const val = Number.parseFloat(value);
          if (Number.isNaN(val)) {
            throw new RangeError(
              `line ${String(lineNumber)}: tempo must be a number, got '${value}'`,
            );
          }
          if (!Number.isFinite(val)) {
            throw new RangeError(
              `line ${String(lineNumber)}: tempo must be a finite number, got '${value}'`,
            );
          }
          validatePositive("tempo", val);
          tempoBpm = val;
          break;
        }
        case "meter": {
          validateMeter(value);
          meter = value;
          break;
        }
        case "amplitude": {
          const val = Number.parseFloat(value);
          if (!Number.isFinite(val)) {
            throw new RangeError(
              `line ${String(lineNumber)}: amplitude must be a number, got '${value}'`,
            );
          }
          validateUnitFloat("amplitude", val);
          amplitude = val;
          break;
        }
        case "sample_rate": {
          const val = Number.parseFloat(value);
          if (
            !Number.isFinite(val) ||
            !Number.isInteger(val) ||
            val < MIN_SAMPLE_RATE_HZ ||
            val > MAX_SAMPLE_RATE_HZ
          ) {
            throw new RangeError(
              `line ${String(lineNumber)}: sample_rate must be an integer in [${String(MIN_SAMPLE_RATE_HZ)}, ${String(MAX_SAMPLE_RATE_HZ)}], got '${value}'`,
            );
          }
          sampleRateHz = val;
          break;
        }
        case "instrument": {
          if (gmProgram !== null) {
            throw new RangeError(
              `line ${String(lineNumber)}: score may specify instrument or program, not both`,
            );
          }
          if (
            value === "drumkit" ||
            value === "standard_kit" ||
            value === "standard"
          ) {
            instrumentId = "drumkit";
            drumkit = "standard_kit";
          } else {
            instrumentId = resolveInstrumentId(value);
          }
          instrumentDirectiveSeen = true;
          break;
        }
        case "program": {
          if (instrumentDirectiveSeen) {
            throw new RangeError(
              `line ${String(lineNumber)}: score may specify instrument or program, not both`,
            );
          }
          const prog = Number.parseInt(value, 10);
          if (!Number.isInteger(prog) || prog < 1 || prog > 128) {
            throw new RangeError(
              `line ${String(lineNumber)}: program must be an integer in [1, 128], got '${value}'`,
            );
          }
          gmProgram = prog;
          instrumentId = resolveInstrumentId(undefined, prog);
          break;
        }
        case "drumkit": {
          drumkit = value;
          instrumentId = "drumkit";
          break;
        }
        default:
          throw new RangeError(
            `line ${String(lineNumber)}: unknown directive '${name}'`,
          );
      }
      continue;
    }

    // Music token line
    const isDrums = instrumentId === "drumkit" || drumkit !== null;
    const tokens = splitWhitespace(line);

    for (let tIdx = 0; tIdx < tokens.length; tIdx += 1) {
      const token = tokens[tIdx];
      if (token === "|") {
        continue;
      }
      if (events.length >= validMaxEventCount) {
        throw new RangeError(
          `line ${String(lineNumber)}: event count exceeds maxEventCount=${String(validMaxEventCount)}`,
        );
      }
      const event = parseScoreToken(token, tempoBpm, isDrums);
      events.push(event);
    }
  }

  if (events.length === 0) {
    throw new RangeError("score must contain at least one event");
  }

  return new TextScore({
    title,
    tempoBpm,
    meter,
    amplitude,
    sampleRateHz,
    instrumentId,
    gmProgram,
    events,
    drumkit,
  });
}
