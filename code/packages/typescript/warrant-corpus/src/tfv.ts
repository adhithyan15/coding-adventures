/**
 * # A minimal TFV v1 reader (TF01 section 2) for `cases.tfv`
 *
 * TFV is the repository's language-neutral line format. This reader is written
 * from the format's rules, not copied from another lane's reader, so a bug in
 * one cannot hide a bug in the other. It accepts only the value kinds the
 * Warrant corpus uses and rejects the rest.
 *
 * A record is one line:
 *
 *     <id> R<count> K<name> <value> K<name> <value> ...
 *
 * Values are space-separated tokens:
 *
 *     N            null            T / F           booleans
 *     I<decimal>   integer         X<hex>          bytes ("X" alone is empty)
 *     S<hex>       UTF-8 text      L<n> v1 .. vn   list
 *     M<n> k1 v1 .. kn vn          map
 *
 * Because lists and maps announce their length, the parser is a small recursive
 * descent; it never has to guess where a value ends. Integers are `bigint`, so
 * a `U64` survives intact.
 */

import { CorpusError, need } from "./errors.ts";
import { decodeUtf8 } from "./jsonio.ts";
import { fromHex, type TfvValue } from "./values.ts";

export const HEADER = "tfv 1";
export const MAX_DEPTH = 64;
const ID = /^[A-Za-z0-9._/@#-]+$/;
const FIELD = /^[a-z][a-z0-9_]*$/;
const INT = /^-?(?:0|[1-9][0-9]*)$/;
const HEX = /^(?:[0-9a-f]{2})*$/;
const COUNT = /^(?:0|[1-9][0-9]*)$/;

/** One record: its id and its fields. */
export type TfvRecord = readonly [string, ReadonlyMap<string, TfvValue>];

function parseValue(tokens: readonly string[], start: number, depth: number): [TfvValue, number] {
  need(start < tokens.length, "tfv", "record ends inside a value");
  need(depth <= MAX_DEPTH, "tfv", "value nests too deeply");
  const token = tokens[start] as string;
  const tag = token.charAt(0);
  const body = token.slice(1);
  if (tag === "N" && body === "") return [["null"], start + 1];
  if (tag === "T" && body === "") return [["bool", true], start + 1];
  if (tag === "F" && body === "") return [["bool", false], start + 1];
  if (tag === "I") {
    need(INT.test(body) && body !== "-0", "tfv", "bad integer");
    return [["int", BigInt(body)], start + 1];
  }
  if (tag === "X") {
    need(HEX.test(body), "tfv", "bad bytes");
    return [["bytes", fromHex(body)], start + 1];
  }
  if (tag === "S") {
    need(HEX.test(body), "tfv", "bad text");
    return [["text", decodeUtf8(fromHex(body), "tfv")], start + 1];
  }
  if (tag === "L" || tag === "M") {
    need(COUNT.test(body), "tfv", "bad container length");
    const count = Number(body);
    need(count <= tokens.length, "tfv", "container length exceeds the record");
    let at = start + 1;
    if (tag === "L") {
      const items: TfvValue[] = [];
      for (let index = 0; index < count; index += 1) {
        const [item, next] = parseValue(tokens, at, depth + 1);
        items.push(item);
        at = next;
      }
      return [["list", items], at];
    }
    const pairs: (readonly [TfvValue, TfvValue])[] = [];
    for (let index = 0; index < count; index += 1) {
      const [key, afterKey] = parseValue(tokens, at, depth + 1);
      const [item, afterItem] = parseValue(tokens, afterKey, depth + 1);
      pairs.push([key, item]);
      at = afterItem;
    }
    return [["map", pairs], at];
  }
  throw new CorpusError("tfv", "unsupported TFV value kind");
}

/** Parse a whole `cases.tfv` into `[record id, {field: value}]` pairs. */
export function parseTfv(text: string): TfvRecord[] {
  const lines = text.split("\n");
  need(lines[0] === HEADER, "tfv", "missing `tfv 1` header");
  need(lines[lines.length - 1] === "", "tfv", "TFV must end with a newline");
  const records: TfvRecord[] = [];
  const seen = new Set<string>();
  for (const line of lines.slice(1, -1)) {
    need(line !== "" && !line.includes("\r"), "tfv", "blank or CR line in TFV");
    const tokens = line.split(" ");
    need(tokens.length >= 2, "tfv", "record is too short");
    const recordId = tokens[0] as string;
    need(ID.test(recordId), "tfv", "bad record id");
    need(!seen.has(recordId), "tfv", "duplicate record id");
    seen.add(recordId);
    const countToken = tokens[1] as string;
    need(countToken.startsWith("R") && COUNT.test(countToken.slice(1)), "tfv", "bad field count");
    const count = Number(countToken.slice(1));
    const fields = new Map<string, TfvValue>();
    let at = 2;
    for (let index = 0; index < count; index += 1) {
      need(at < tokens.length && (tokens[at] as string).startsWith("K"), "tfv", "expected a field key");
      const name = (tokens[at] as string).slice(1);
      need(FIELD.test(name), "tfv", "bad field name");
      need(!fields.has(name), "tfv", "duplicate field");
      const [value, next] = parseValue(tokens, at + 1, 0);
      fields.set(name, value);
      at = next;
    }
    need(at === tokens.length, "tfv", "trailing tokens after the last field");
    records.push([recordId, fields]);
  }
  return records;
}
