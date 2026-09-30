import { randomBytes } from "node:crypto";
import { constants as fsConstants } from "node:fs";
import {
  chmod,
  lstat,
  open,
  realpath,
  rename,
  unlink,
} from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { tryParseCapability, type Capability } from "@coding-adventures/forme-capability";
import { PluginHostError } from "./errors.js";

const MAX_AUTHORITY_FILE_BYTES = 1024 * 1024;
const MAX_AUTHORITY_ROWS = 4_096;
const MANIFEST_HASH = /^blake2b:[0-9a-f]{64}$/;
const RFC3339_UTC = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.\d+)?Z$/;

export interface TrustedPluginKey {
  readonly algorithm: "ed25519";
  /** Canonical Base64 for the raw 32-byte Ed25519 public key. */
  readonly publicKey: string;
  readonly addedAt: string;
  readonly note?: string;
}

export interface PluginTrustStore {
  readonly trustedKeys: readonly TrustedPluginKey[];
}

export interface PluginGrantDecision {
  readonly capability: Capability;
  readonly grantedAt: string;
  readonly note?: string;
}

export interface PluginGrantsFile {
  readonly manifestHash: string;
  readonly granted: readonly PluginGrantDecision[];
}

export interface LoadedPluginGrants {
  readonly status: "missing" | "current" | "stale";
  readonly capabilities: readonly Capability[];
  readonly file: PluginGrantsFile | null;
}

export function parseTrustStore(text: string): PluginTrustStore {
  const document = parseAuthorityDocument(text, "trustedKeys");
  if (Object.keys(document.topLevel).length !== 0) {
    invalid("trust store has an unknown top-level field");
  }
  const trustedKeys = document.rows.map((row, index) => validateTrustedKey(row, index));
  const seen = new Set<string>();
  for (const key of trustedKeys) {
    if (seen.has(key.publicKey)) invalid("trust store contains a duplicate public key");
    seen.add(key.publicKey);
  }
  return Object.freeze({ trustedKeys: Object.freeze([...trustedKeys].sort(comparePublicKeys)) });
}

export function formatTrustStore(store: PluginTrustStore): string {
  const root = record(store);
  exactFields(root, ["trustedKeys"], "trust store");
  const trustedKeys = root.trustedKeys;
  if (!Array.isArray(trustedKeys)) invalid("trust store must contain trustedKeys");
  if (trustedKeys.length > MAX_AUTHORITY_ROWS) rowLimit();
  const normalized = trustedKeys.map((entry, index) => validateTrustedKey(record(entry), index));
  const seen = new Set<string>();
  for (const key of normalized) {
    if (seen.has(key.publicKey)) invalid("trust store contains a duplicate public key");
    seen.add(key.publicKey);
  }
  const output = new BoundedAuthorityOutput();
  for (const key of normalized.sort(comparePublicKeys)) {
    output.append("[[trustedKeys]]\n");
    output.appendField("algorithm", key.algorithm);
    output.appendField("publicKey", key.publicKey);
    output.appendField("addedAt", key.addedAt);
    if (key.note !== undefined) output.appendField("note", key.note);
    output.append("\n");
  }
  return output.finish();
}

export function parseGrantsFile(text: string): PluginGrantsFile {
  const document = parseAuthorityDocument(text, "granted");
  exactFields(document.topLevel, ["manifestHash"], "grants file");
  const manifestHash = document.topLevel.manifestHash;
  validateManifestHash(manifestHash);
  const granted = document.rows.map((row, index) => validateGrant(row, index));
  const seen = new Set<string>();
  for (const grant of granted) {
    if (seen.has(grant.capability)) invalid("grants file contains a duplicate capability");
    seen.add(grant.capability);
  }
  return Object.freeze({
    manifestHash,
    granted: Object.freeze([...granted].sort(compareCapabilities)),
  });
}

export function formatGrantsFile(file: PluginGrantsFile): string {
  const root = record(file);
  exactFields(root, ["manifestHash", "granted"], "grants file");
  validateManifestHash(root.manifestHash);
  const granted = root.granted;
  if (!Array.isArray(granted)) invalid("grants file must contain granted decisions");
  if (granted.length > MAX_AUTHORITY_ROWS) rowLimit();
  const normalized = granted.map((entry, index) => validateGrant(record(entry), index));
  const seen = new Set<string>();
  for (const grant of normalized) {
    if (seen.has(grant.capability)) invalid("grants file contains a duplicate capability");
    seen.add(grant.capability);
  }
  const output = new BoundedAuthorityOutput();
  output.appendField("manifestHash", root.manifestHash);
  if (normalized.length > 0) output.append("\n");
  for (const grant of normalized.sort(compareCapabilities)) {
    output.append("[[granted]]\n");
    output.appendField("capability", grant.capability);
    output.appendField("grantedAt", grant.grantedAt);
    if (grant.note !== undefined) output.appendField("note", grant.note);
    output.append("\n");
  }
  return output.finish();
}

export async function readTrustStore(path: string): Promise<PluginTrustStore> {
  const text = await readAuthorityFile(path, "trust store");
  return text === null ? Object.freeze({ trustedKeys: Object.freeze([]) }) : parseTrustStore(text);
}

export async function readGrantsFile(
  path: string,
  expectedManifestHash: string,
): Promise<LoadedPluginGrants> {
  validateManifestHash(expectedManifestHash);
  const text = await readAuthorityFile(path, "grants file");
  if (text === null) {
    return Object.freeze({ status: "missing", capabilities: Object.freeze([]), file: null });
  }
  const file = parseGrantsFile(text);
  if (file.manifestHash !== expectedManifestHash) {
    return Object.freeze({ status: "stale", capabilities: Object.freeze([]), file });
  }
  return Object.freeze({
    status: "current",
    capabilities: Object.freeze(file.granted.map(entry => entry.capability)),
    file,
  });
}

export async function writeTrustStore(path: string, store: PluginTrustStore): Promise<void> {
  await writeAuthorityFile(path, formatTrustStore(store), "trust store");
}

export async function writeGrantsFile(path: string, file: PluginGrantsFile): Promise<void> {
  await writeAuthorityFile(path, formatGrantsFile(file), "grants file");
}

interface AuthorityDocument {
  readonly topLevel: Record<string, string>;
  readonly rows: readonly Record<string, string>[];
}

function parseAuthorityDocument(text: string, tableName: "trustedKeys" | "granted"): AuthorityDocument {
  if (typeof text !== "string") invalid("authority file input must be a string");
  if (Buffer.byteLength(text, "utf8") > MAX_AUTHORITY_FILE_BYTES) byteLimit();
  if (text.startsWith("\uFEFF")) invalid("authority files must not contain a byte-order mark");
  const topLevel: Record<string, string> = Object.create(null) as Record<string, string>;
  const rows: Record<string, string>[] = [];
  let current: Record<string, string> | null = null;
  for (const [offset, sourceLine] of text.split(/\r?\n/u).entries()) {
    const line = stripComment(sourceLine).trim();
    if (line.length === 0) continue;
    if (line.startsWith("[[") || line.startsWith("[")) {
      if (line !== `[[${tableName}]]`) invalid(`unknown authority table on line ${offset + 1}`);
      if (rows.length >= MAX_AUTHORITY_ROWS) rowLimit();
      current = Object.create(null) as Record<string, string>;
      rows.push(current);
      continue;
    }
    const match = /^([A-Za-z][A-Za-z0-9]*)\s*=\s*(.+)$/u.exec(line);
    if (!match) invalid(`malformed authority assignment on line ${offset + 1}`);
    const target = current ?? topLevel;
    const key = match[1]!;
    if (Object.hasOwn(target, key)) invalid(`duplicate authority field ${key}`);
    target[key] = parseString(match[2]!, offset + 1);
  }
  return { topLevel, rows };
}

function stripComment(line: string): string {
  let quoted = false;
  let escaped = false;
  for (let index = 0; index < line.length; index += 1) {
    const char = line[index]!;
    if (escaped) {
      escaped = false;
      continue;
    }
    if (quoted && char === "\\") {
      escaped = true;
      continue;
    }
    if (char === '"') quoted = !quoted;
    else if (char === "#" && !quoted) return line.slice(0, index);
  }
  return line;
}

function parseString(value: string, line: number): string {
  if (!value.startsWith('"') || !value.endsWith('"')) {
    invalid(`authority value on line ${line} must be a basic string`);
  }
  let result = "";
  for (let index = 1; index < value.length - 1; index += 1) {
    const codeUnit = value.charCodeAt(index);
    if (codeUnit === 0x22) invalid(`authority value on line ${line} contains an unescaped quote`);
    if (codeUnit === 0x5c) {
      if (index + 1 >= value.length - 1) {
        invalid(`authority value on line ${line} has a dangling escape`);
      }
      const escape = value[index + 1];
      const simple = TOML_ESCAPES[escape!];
      if (simple !== undefined) {
        result += simple;
        index += 1;
        continue;
      }
      if (escape === "u" || escape === "U") {
        const digits = escape === "u" ? 4 : 8;
        const hex = value.slice(index + 2, index + 2 + digits);
        if (hex.length !== digits || !/^[0-9A-Fa-f]+$/u.test(hex)) {
          invalid(`authority value on line ${line} has an invalid Unicode escape`);
        }
        const codePoint = Number.parseInt(hex, 16);
        if (!isUnicodeScalar(codePoint)) {
          invalid(`authority value on line ${line} has a non-scalar Unicode escape`);
        }
        result += String.fromCodePoint(codePoint);
        index += 1 + digits;
        continue;
      }
      invalid(`authority value on line ${line} has an invalid TOML escape`);
    }
    const codePoint = value.codePointAt(index)!;
    if (!isUnicodeScalar(codePoint) || (codePoint < 0x20 && codePoint !== 0x09) || codePoint === 0x7f) {
      invalid(`authority value on line ${line} contains an invalid character`);
    }
    result += String.fromCodePoint(codePoint);
    if (codePoint > 0xffff) index += 1;
  }
  return result;
}

const TOML_ESCAPES: Readonly<Record<string, string>> = Object.freeze({
  b: "\b",
  t: "\t",
  n: "\n",
  f: "\f",
  r: "\r",
  '"': '"',
  "\\": "\\",
});

function validateTrustedKey(row: Record<string, unknown>, index: number): TrustedPluginKey {
  exactFields(row, ["algorithm", "publicKey", "addedAt"], `trusted key ${index}`, ["note"]);
  if (row.algorithm !== "ed25519") invalid(`trusted key ${index} must use ed25519`);
  if (typeof row.publicKey !== "string" || !isCanonicalPublicKey(row.publicKey)) {
    invalid(`trusted key ${index} publicKey must be canonical Base64 for 32 raw bytes`);
  }
  if (typeof row.addedAt !== "string") invalid(`trusted key ${index} addedAt must be a string`);
  validateTimestamp(row.addedAt, `trusted key ${index} addedAt`);
  const note = optionalString(row.note, `trusted key ${index} note`);
  return Object.freeze({
    algorithm: "ed25519",
    publicKey: row.publicKey,
    addedAt: row.addedAt,
    ...(note === undefined ? {} : { note }),
  });
}

function validateGrant(row: Record<string, unknown>, index: number): PluginGrantDecision {
  exactFields(row, ["capability", "grantedAt"], `grant ${index}`, ["note"]);
  if (typeof row.capability !== "string" || tryParseCapability(row.capability) === null) {
    invalid(`grant ${index} capability is not canonical`);
  }
  if (typeof row.grantedAt !== "string") invalid(`grant ${index} grantedAt must be a string`);
  validateTimestamp(row.grantedAt, `grant ${index} grantedAt`);
  const note = optionalString(row.note, `grant ${index} note`);
  return Object.freeze({
    capability: row.capability,
    grantedAt: row.grantedAt,
    ...(note === undefined ? {} : { note }),
  });
}

function exactFields(
  value: Record<string, unknown>,
  required: readonly string[],
  label: string,
  optional: readonly string[] = [],
): void {
  const allowed = new Set([...required, ...optional]);
  for (const key of Object.keys(value)) {
    if (!allowed.has(key)) invalid(`${label} has unknown field ${key}`);
  }
  for (const key of required) {
    if (!Object.hasOwn(value, key)) invalid(`${label} is missing ${key}`);
  }
}

function validateManifestHash(value: unknown): asserts value is string {
  if (typeof value !== "string" || !MANIFEST_HASH.test(value)) {
    invalid("manifestHash must be a lowercase blake2b identity");
  }
}

function validateTimestamp(value: string, label: string): void {
  const match = RFC3339_UTC.exec(value);
  if (!match) invalid(`${label} must be an RFC 3339 UTC timestamp`);
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const hour = Number(match[4]);
  const minute = Number(match[5]);
  const second = Number(match[6]);
  const monthLengths = [31, isLeapYear(year) ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  if (month < 1 || month > 12 || day < 1 || day > monthLengths[month - 1]!
      || hour > 23 || minute > 59 || second > 59) {
    invalid(`${label} must be an RFC 3339 UTC timestamp`);
  }
}

function isLeapYear(year: number): boolean {
  return year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
}

function isUnicodeScalar(codePoint: number): boolean {
  return codePoint >= 0 && codePoint <= 0x10ffff && !(codePoint >= 0xd800 && codePoint <= 0xdfff);
}

function isCanonicalPublicKey(value: string): boolean {
  if (!/^[A-Za-z0-9+/]+={0,2}$/u.test(value)) return false;
  const decoded = Buffer.from(value, "base64");
  return decoded.length === 32 && decoded.toString("base64") === value;
}

function optionalString(value: unknown, label: string): string | undefined {
  if (value === undefined) return undefined;
  if (typeof value !== "string") invalid(`${label} must be a string`);
  return value;
}

class BoundedAuthorityOutput {
  readonly #chunks: string[] = [];
  #bytes = 0;

  append(value: string): void {
    const bytes = Buffer.byteLength(value, "utf8");
    if (bytes > MAX_AUTHORITY_FILE_BYTES - this.#bytes) byteLimit();
    this.#chunks.push(value);
    this.#bytes += bytes;
  }

  appendField(name: string, value: string): void {
    if (!isScalarText(value)) invalid(`${name} must contain only Unicode scalar values`);
    if (Buffer.byteLength(value, "utf8") > MAX_AUTHORITY_FILE_BYTES - this.#bytes) byteLimit();
    this.append(`${name} = ${JSON.stringify(value)}\n`);
  }

  finish(): string {
    return this.#chunks.join("");
  }
}

function isScalarText(value: string): boolean {
  for (let index = 0; index < value.length; index += 1) {
    const codePoint = value.codePointAt(index)!;
    if (!isUnicodeScalar(codePoint)) return false;
    if (codePoint > 0xffff) index += 1;
  }
  return true;
}

function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) invalid("authority row must be an object");
  return value as Record<string, unknown>;
}

function comparePublicKeys(a: TrustedPluginKey, b: TrustedPluginKey): number {
  return compareCodePoints(a.publicKey, b.publicKey);
}

function compareCapabilities(a: PluginGrantDecision, b: PluginGrantDecision): number {
  return compareCodePoints(a.capability, b.capability);
}

function compareCodePoints(a: string, b: string): number {
  let aOffset = 0;
  let bOffset = 0;
  while (aOffset < a.length && bOffset < b.length) {
    const aPoint = a.codePointAt(aOffset)!;
    const bPoint = b.codePointAt(bOffset)!;
    if (aPoint !== bPoint) return aPoint < bPoint ? -1 : 1;
    aOffset += aPoint > 0xffff ? 2 : 1;
    bOffset += bPoint > 0xffff ? 2 : 1;
  }
  return a.length - b.length;
}

async function readAuthorityFile(path: string, label: string): Promise<string | null> {
  /* v8 ignore next -- the zero fallback is exercised only on platforms without O_NOFOLLOW */
  const noFollow = "O_NOFOLLOW" in fsConstants ? fsConstants.O_NOFOLLOW : 0;
  /* v8 ignore next -- the zero fallback is exercised only on platforms without O_NONBLOCK */
  const nonBlock = "O_NONBLOCK" in fsConstants ? fsConstants.O_NONBLOCK : 0;
  let handle;
  try {
    handle = await open(path, fsConstants.O_RDONLY | noFollow | nonBlock);
  } catch (error) {
    if (isErrno(error, "ENOENT")) return null;
    throw unsafe(`${label} is not a safe regular file`, error);
  }
  try {
    const opened = await handle.stat();
    if (!opened.isFile() || opened.nlink !== 1 || opened.size > MAX_AUTHORITY_FILE_BYTES) {
      throw unsafe(`${label} must be a bounded regular file with one link`);
    }
    const named = await lstat(path);
    if (!named.isFile() || named.isSymbolicLink() || named.nlink !== 1
        || named.dev !== opened.dev || named.ino !== opened.ino) {
      throw unsafe(`${label} changed or is linked`);
    }
    const bytes = Buffer.allocUnsafe(Math.min(opened.size, MAX_AUTHORITY_FILE_BYTES) + 1);
    let offset = 0;
    while (offset < bytes.length) {
      const result = await handle.read(bytes, offset, bytes.length - offset, offset);
      if (result.bytesRead === 0) break;
      offset += result.bytesRead;
    }
    if (offset > MAX_AUTHORITY_FILE_BYTES || offset > opened.size) byteLimit();
    try {
      return new TextDecoder("utf-8", { fatal: true }).decode(bytes.subarray(0, offset));
    } catch {
      invalid(`${label} must contain valid UTF-8`);
    }
  } finally {
    await handle.close();
  }
}

async function writeAuthorityFile(path: string, text: string, label: string): Promise<void> {
  /* v8 ignore next -- exported formatters enforce this invariant before calling the writer */
  if (Buffer.byteLength(text, "utf8") > MAX_AUTHORITY_FILE_BYTES) byteLimit();
  const requestedParent = resolve(dirname(path));
  let realParent: string;
  try {
    realParent = await realpath(requestedParent);
  } catch (error) {
    throw unsafe(`${label} parent must be an existing real directory`, error);
  }
  const requestedParentStat = await lstat(requestedParent);
  if (!requestedParentStat.isDirectory() || requestedParentStat.isSymbolicLink()) {
    throw unsafe(`${label} parent must be a real directory`);
  }
  const target = join(realParent, basename(path));
  try {
    const existing = await lstat(target);
    if (!existing.isFile() || existing.isSymbolicLink() || existing.nlink !== 1) {
      throw unsafe(`${label} target must be a singly-linked regular file`);
    }
  } catch (error) {
    if (!isErrno(error, "ENOENT")) throw error;
  }
  const temporary = join(realParent, `.${basename(path)}.${randomBytes(16).toString("hex")}.tmp`);
  let handle;
  try {
    handle = await open(
      temporary,
      fsConstants.O_WRONLY | fsConstants.O_CREAT | fsConstants.O_EXCL,
      0o600,
    );
    await handle.writeFile(text, "utf8");
    await handle.sync();
    await handle.close();
    handle = undefined;
    /* v8 ignore else -- Windows does not expose POSIX file modes */
    if (process.platform !== "win32") await chmod(temporary, 0o600);
    await rename(temporary, target);
    /* v8 ignore else -- Windows cannot open directories for fsync */
    if (process.platform !== "win32") await syncDirectory(realParent);
  } catch (error) {
    if (handle) await handle.close().catch(() => {});
    await unlink(temporary).catch(() => {});
    throw error;
  }
}

async function syncDirectory(path: string): Promise<void> {
  let handle;
  try {
    handle = await open(path, fsConstants.O_RDONLY);
    await handle.sync();
  } catch (error) {
    if (!isErrno(error, "EINVAL") && !isErrno(error, "ENOTSUP") && !isErrno(error, "EBADF")) throw error;
  } finally {
    await handle?.close();
  }
}

function invalid(message: string): never {
  throw new PluginHostError("AUTHORITY_FILE_INVALID", message);
}

function unsafe(message: string, cause?: unknown): PluginHostError {
  return new PluginHostError("AUTHORITY_FILE_UNSAFE", message, {}, cause === undefined ? undefined : { cause });
}

function byteLimit(): never {
  throw new PluginHostError("RESOURCE_LIMIT_EXCEEDED", `authority file exceeds the ${MAX_AUTHORITY_FILE_BYTES} byte limit`);
}

function rowLimit(): never {
  throw new PluginHostError("RESOURCE_LIMIT_EXCEEDED", `authority file exceeds the ${MAX_AUTHORITY_ROWS} row limit`);
}

function isErrno(error: unknown, code: string): error is NodeJS.ErrnoException {
  return error instanceof Error && "code" in error && error.code === code;
}
