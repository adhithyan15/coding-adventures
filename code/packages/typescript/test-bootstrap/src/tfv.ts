import { Value, Record } from "./types.js";
import { valuesEqual } from "./deep_equal.js";

export const HEADER = "tfv 1";
export const MAX_DEPTH = 64;
export const MAX_INT_DIGITS = 1000;

export class TfvError extends Error {
  readonly line: number;

  constructor(line: number, message: string) {
    super(`line ${line}: ${message}`);
    this.name = "TfvError";
    this.line = line;
  }
}

function reject(line: number, message: string): never {
  throw new TfvError(line, message);
}

function isRecordId(s: string): boolean {
  if (s.length === 0) return false;
  return /^[A-Za-z0-9._/-]+$/.test(s);
}

function isFieldName(s: string): boolean {
  return /^[a-z][a-z0-9_]*$/.test(s);
}

function isCount(s: string): boolean {
  return s === "0" || /^[1-9][0-9]*$/.test(s);
}

function isCanonicalInt(s: string): boolean {
  if (s === "-0") return false;
  const digits = s.startsWith("-") ? s.slice(1) : s;
  return digits.length <= MAX_INT_DIGITS && isCount(digits);
}

function isLowerHex(s: string): boolean {
  return /^[0-9a-f]*$/.test(s);
}

function isEvenLowerHex(s: string): boolean {
  return s.length % 2 === 0 && isLowerHex(s);
}

function decodeHex(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.slice(i, i + 2), 16);
  }
  return bytes;
}

function parseCount(body: string, line: number): number {
  if (!isCount(body)) {
    reject(line, `malformed count ${JSON.stringify(body)}`);
  }
  const n = Number(body);
  if (!Number.isSafeInteger(n) || n < 0) {
    reject(line, "count too large");
  }
  return n;
}

function parseValue(
  tokens: string[],
  pos: number,
  line: number,
  depth: number
): [Value, number] {
  if (pos >= tokens.length) {
    reject(line, "value missing");
  }
  const token = tokens[pos];
  if (token.length === 0) {
    reject(line, "empty token");
  }
  const tag = token[0];
  const body = token.slice(1);
  const next = pos + 1;

  switch (tag) {
    case "N":
      if (body.length !== 0) reject(line, `malformed token ${JSON.stringify(token)}`);
      return [{ kind: "null" }, next];

    case "T":
      if (body.length !== 0) reject(line, `malformed token ${JSON.stringify(token)}`);
      return [{ kind: "bool", value: true }, next];

    case "F":
      if (body.length !== 0) reject(line, `malformed token ${JSON.stringify(token)}`);
      return [{ kind: "bool", value: false }, next];

    case "I":
      if (!isCanonicalInt(body)) reject(line, `malformed token ${JSON.stringify(token)}`);
      return [{ kind: "int", text: body }, next];

    case "D": {
      if (body.length !== 16 || !isLowerHex(body)) {
        reject(line, `malformed token ${JSON.stringify(token)}`);
      }
      const bits = BigInt(`0x${body}`);
      return [{ kind: "f64", bits }, next];
    }

    case "X": {
      if (!isEvenLowerHex(body)) {
        reject(line, `malformed token ${JSON.stringify(token)}`);
      }
      return [{ kind: "bytes", value: decodeHex(body) }, next];
    }

    case "S": {
      if (!isEvenLowerHex(body)) {
        reject(line, `malformed token ${JSON.stringify(token)}`);
      }
      const bytes = decodeHex(body);
      const decoder = new TextDecoder("utf-8", { fatal: true });
      try {
        const text = decoder.decode(bytes);
        return [{ kind: "text", value: text }, next];
      } catch {
        reject(line, "text is not valid UTF-8");
      }
      break;
    }

    case "L": {
      if (!isCount(body)) reject(line, `malformed token ${JSON.stringify(token)}`);
      if (depth >= MAX_DEPTH) {
        reject(line, `lists and maps nest at most ${MAX_DEPTH} deep`);
      }
      const count = parseCount(body, line);
      const items: Value[] = [];
      let currentPos = next;
      for (let i = 0; i < count; i++) {
        const [item, after] = parseValue(tokens, currentPos, line, depth + 1);
        items.push(item);
        currentPos = after;
      }
      return [{ kind: "list", items }, currentPos];
    }

    case "M": {
      if (!isCount(body)) reject(line, `malformed token ${JSON.stringify(token)}`);
      if (depth >= MAX_DEPTH) {
        reject(line, `lists and maps nest at most ${MAX_DEPTH} deep`);
      }
      const count = parseCount(body, line);
      const entries: [Value, Value][] = [];
      let currentPos = next;
      for (let i = 0; i < count; i++) {
        const [key, afterKey] = parseValue(tokens, currentPos, line, depth + 1);
        const [val, afterVal] = parseValue(tokens, afterKey, line, depth + 1);
        for (const [seenKey] of entries) {
          if (valuesEqual(seenKey, key)) {
            reject(line, "map has two deep-equal keys");
          }
        }
        entries.push([key, val]);
        currentPos = afterVal;
      }
      return [{ kind: "map", entries }, currentPos];
    }

    default:
      reject(line, `malformed token ${JSON.stringify(token)}`);
  }
}

function parseRecord(line: string, lineNumber: number): Record {
  if (line.includes("\r")) {
    reject(lineNumber, "carriage return");
  }
  const tokens = line.split(" ");
  if (tokens.some((t) => t.length === 0)) {
    reject(lineNumber, "empty token (double, leading or trailing space)");
  }
  if (tokens.length < 2 || !isRecordId(tokens[0])) {
    reject(lineNumber, "bad record id");
  }
  if (!tokens[1].startsWith("R")) {
    reject(lineNumber, "expected R<count>");
  }
  const count = parseCount(tokens[1].slice(1), lineNumber);

  let position = 2;
  const fields = new Map<string, Value>();
  for (let i = 0; i < count; i++) {
    if (position >= tokens.length) {
      reject(lineNumber, "record ends early");
    }
    const token = tokens[position];
    if (!token.startsWith("K") || !isFieldName(token.slice(1))) {
      reject(lineNumber, `expected K<field>, got ${JSON.stringify(token)}`);
    }
    const name = token.slice(1);
    if (fields.has(name)) {
      reject(lineNumber, `duplicate field ${JSON.stringify(name)}`);
    }
    const [value, nextPos] = parseValue(tokens, position + 1, lineNumber, 0);
    fields.set(name, value);
    position = nextPos;
  }

  if (position !== tokens.length) {
    reject(lineNumber, "trailing tokens");
  }

  return { id: tokens[0], fields };
}

/**
 * Parses an entire TFV file into Records (TF01 §2).
 */
export function parseFile(text: string): Record[] {
  const lines = text.split("\n");
  if (lines.length > 0 && lines[lines.length - 1] === "") {
    lines.pop();
  }
  if (lines.length === 0 || lines[0] !== HEADER) {
    reject(1, "first line must be exactly 'tfv 1'");
  }

  const records: Record[] = [];
  const seenIds = new Set<string>();

  for (let i = 1; i < lines.length; i++) {
    const lineNumber = i + 1;
    const line = lines[i];
    if (line.length === 0 || line.startsWith("#")) {
      continue;
    }
    const record = parseRecord(line, lineNumber);
    if (seenIds.has(record.id)) {
      reject(lineNumber, `duplicate record id ${JSON.stringify(record.id)}`);
    }
    seenIds.add(record.id);
    records.push(record);
  }

  return records;
}
