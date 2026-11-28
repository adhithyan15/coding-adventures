/**
 * Two in-memory early-Web profiles: CERN's 1991 HTTP/0.9 wire and the W3C
 * archive's November 1992 HTML Tags vocabulary. This intentionally does not
 * delegate to a current HTTP stack or browser HTML parser.
 */

/** A small tagged result used at all expected validation boundaries. */
export type Result<T, E> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly error: E };

/** Request codec conditions; malformed wire does not throw. */
export type Http09RequestError =
  | { readonly kind: "invalid-address" }
  | { readonly kind: "invalid-request" };

/** Response decoder conditions; non-ASCII bytes are rejected. */
export type Http09ResponseError = { readonly kind: "non-ascii" };

const addressBrand = Symbol("Http09Address");
let encodeAddressRequest = (_address: Http09Address): Uint8Array => {
  throw new Error("HTTP/0.9 address encoder was not initialized");
};

/**
 * A validated request address. Its runtime private field keeps the admission
 * check meaningful after Node strips TypeScript's compile-time modifiers.
 */
export class Http09Address {
  readonly #value: string;

  /** Create a validated instance or return a tagged error. */
  static create(value: string): Result<Http09Address, Http09RequestError> {
    if (typeof value !== "string" || !isPrintableAddress(value)) {
      return { ok: false, error: { kind: "invalid-address" } };
    }
    return { ok: true, value: new Http09Address(value, addressBrand) };
  }

  /** The string is immutable; it is exposed for callers that need the path. */
  get value(): string {
    return this.#value;
  }

  /** Capture private-slot access in a module closure rather than exporting it. */
  static {
    encodeAddressRequest = (address: Http09Address): Uint8Array => {
      const value = address.#value;
      if (!isPrintableAddress(value)) {
        throw new TypeError("Http09Address invariant was violated");
      }
      const wire = new Uint8Array(4 + value.length + 2);
      wire[0] = 0x47; // G
      wire[1] = 0x45; // E
      wire[2] = 0x54; // T
      wire[3] = 0x20; // space
      for (let i = 0; i < value.length; i += 1) {
        wire[4 + i] = value.charCodeAt(i);
      }
      wire[wire.length - 2] = 0x0d;
      wire[wire.length - 1] = 0x0a;
      return wire;
    };
  }

  /** The private slot, token, and value are checked even for JavaScript calls. */
  private constructor(value: string, token: symbol) {
    if (token !== addressBrand || !isPrintableAddress(value)) {
      throw new TypeError("Http09Address must be created with Http09Address.create");
    }
    this.#value = value;
    Object.freeze(this);
  }

}

/** Addresses are nonempty and every UTF-16 code unit is printable ASCII. */
function isPrintableAddress(value: string): boolean {
  if (value.length === 0) return false;
  for (let i = 0; i < value.length; i += 1) {
    const code = value.charCodeAt(i);
    if (code < 0x21 || code > 0x7e) return false;
  }
  return true;
}

/** Format a validated address as the one request line used by HTTP/0.9. */
export function encodeHttp09Request(address: Http09Address): Uint8Array {
  // The closure reads #value, so a forged object or Proxy cannot emit
  // arbitrary bytes. Such a call is outside the TypeScript API and throws.
  return encodeAddressRequest(address);
}

/** Return a fixed copy of a same-realm Uint8Array without using overridden
 * `length`, `slice`, or iterator properties. A detached or proxied view fails
 * closed. The copy also fixes the extent of a resizable backing buffer. */
function snapshotBytes(input: unknown): Uint8Array | undefined {
  if (!(input instanceof Uint8Array)) return undefined;
  const typedArrayPrototype = Object.getPrototypeOf(Uint8Array.prototype) as object;
  const lengthGetter = Object.getOwnPropertyDescriptor(typedArrayPrototype, "length")?.get;
  if (typeof lengthGetter !== "function") return undefined;
  let length: number;
  try {
    length = Reflect.apply(lengthGetter, input, []) as number;
  } catch {
    return undefined;
  }
  if (!Number.isSafeInteger(length) || length < 0) return undefined;
  const result = new Uint8Array(length);
  try {
    for (let i = 0; i < length; i += 1) {
      const byte: unknown = input[i];
      if (typeof byte !== "number" || !Number.isInteger(byte) || byte < 0 || byte > 0xff) {
        return undefined;
      }
      result[i] = byte;
    }
  } catch {
    return undefined;
  }
  return result;
}

/** Read an ASCII address using bounded chunks rather than one giant spread. */
function asciiFromBytes(bytes: Uint8Array, start: number, end: number): string {
  const fragments: string[] = [];
  const chunkSize = 4096;
  for (let offset = start; offset < end; offset += chunkSize) {
    const chunk: number[] = [];
    const stop = Math.min(offset + chunkSize, end);
    for (let i = offset; i < stop; i += 1) chunk.push(bytes[i]);
    fragments.push(String.fromCharCode(...chunk));
  }
  return fragments.join("");
}

/** Parse exactly one `GET address` request terminated by CRLF or LF. */
export function parseHttp09Request(
  wire: Uint8Array,
): Result<Http09Address, Http09RequestError> {
  const bytes = snapshotBytes(wire);
  if (bytes === undefined || bytes.length < 6) {
    return { ok: false, error: { kind: "invalid-request" } };
  }
  if (bytes[0] !== 0x47 || bytes[1] !== 0x45 || bytes[2] !== 0x54 || bytes[3] !== 0x20) {
    return { ok: false, error: { kind: "invalid-request" } };
  }
  if (bytes[bytes.length - 1] !== 0x0a) {
    return { ok: false, error: { kind: "invalid-request" } };
  }
  let end = bytes.length - 1;
  if (end > 0 && bytes[end - 1] === 0x0d) end -= 1;
  if (end <= 4) return { ok: false, error: { kind: "invalid-request" } };
  for (let i = 4; i < end; i += 1) {
    if (bytes[i] < 0x21 || bytes[i] > 0x7e) {
      return { ok: false, error: { kind: "invalid-request" } };
    }
  }
  const address = Http09Address.create(asciiFromBytes(bytes, 4, end));
  return address.ok
    ? address
    : { ok: false, error: { kind: "invalid-request" } };
}

/** Decode a completed ASCII response body and preserve its line endings. */
export function decodeHttp09Response(
  body: Uint8Array,
): Result<string, Http09ResponseError> {
  const bytes = snapshotBytes(body);
  if (bytes === undefined) return { ok: false, error: { kind: "non-ascii" } };
  for (const byte of bytes) {
    if (byte > 0x7f) return { ok: false, error: { kind: "non-ascii" } };
  }
  return { ok: true, value: asciiFromBytes(bytes, 0, bytes.length) };
}

/** An index into the same flat arena held by an HtmlDocument. */
export type NodeId = number;

/** Plain text after CRLF normalization and a single archived entity pass. */
export interface HtmlTextNode {
  readonly kind: "text";
  readonly text: string;
}

/** A historical element with recognized attributes and ordered child indices. */
export interface HtmlElementNode {
  readonly kind: "element";
  readonly tag: string;
  readonly attributes: Readonly<Record<string, string | null>>;
  readonly children: readonly NodeId[];
}

/** One flat arena entry; it never recursively owns its children. */
export type HtmlNode = HtmlTextNode | HtmlElementNode;

/** Ordered roots and node arena for the historical markup document. */
export interface HtmlDocument {
  readonly nodes: readonly HtmlNode[];
  readonly roots: readonly NodeId[];
}

/** Failures reserved for invalid untyped input or impossible builder state. */
export type HtmlParseError =
  | { readonly kind: "invalid-source" }
  | { readonly kind: "invalid-tree-state" };

interface DraftTextNode {
  kind: "text";
  fragments: string[];
}

interface DraftElementNode {
  kind: "element";
  tag: string;
  attributes: Record<string, string | null>;
  children: NodeId[];
}

type DraftNode = DraftTextNode | DraftElementNode;

interface TagToken {
  readonly name: string;
  readonly attributes: string;
  readonly closing: boolean;
  readonly end: number;
}

const knownTags = new Set([
  "title", "address", "a", "dl", "li", "xmp", "listing", "plaintext", "ul", "menu", "dir",
  "dt", "dd", "p", "isindex", "h1", "h2", "h3", "h4", "h5", "h6",
]);

/** The archived tokenizer recognizes five ASCII whitespace code units. */
function isHtmlSpace(code: number): boolean {
  return code === 0x20 || code === 0x09 || code === 0x0a || code === 0x0d || code === 0x0c;
}

function isAsciiLetter(code: number): boolean {
  return (code >= 0x41 && code <= 0x5a) || (code >= 0x61 && code <= 0x7a);
}

function isNameCharacter(code: number): boolean {
  return isAsciiLetter(code) || (code >= 0x30 && code <= 0x39) || code === 0x2d;
}

/** Fold only ASCII uppercase letters. JavaScript's Unicode lowercasing is not used. */
function lowerAscii(value: string): string {
  let result = "";
  for (let i = 0; i < value.length; i += 1) {
    const code = value.charCodeAt(i);
    result += String.fromCharCode(code >= 0x41 && code <= 0x5a ? code + 0x20 : code);
  }
  return result;
}

/** Parse one profile token, permitting `>` inside quoted attribute values. */
function tagAt(source: string, start: number): TagToken | undefined {
  if (source.charCodeAt(start) !== 0x3c) return undefined;
  let i = start + 1;
  let closing = false;
  if (source.charCodeAt(i) === 0x2f) {
    closing = true;
    i += 1;
  }
  if (!isAsciiLetter(source.charCodeAt(i))) return undefined;
  const nameStart = i;
  while (i < source.length && isNameCharacter(source.charCodeAt(i))) i += 1;
  const nameEnd = i;
  const afterName = source.charCodeAt(i);
  if (i < source.length && afterName !== 0x3e && !isHtmlSpace(afterName)) return undefined;
  let quote = 0;
  while (i < source.length) {
    const code = source.charCodeAt(i);
    if (code === 0x3c) return undefined;
    if (quote !== 0) {
      if (code === quote) quote = 0;
    } else if (code === 0x22 || code === 0x27) {
      quote = code;
    } else if (code === 0x3e) {
      return {
        name: lowerAscii(source.slice(nameStart, nameEnd)),
        attributes: source.slice(nameEnd, i),
        closing,
        end: i + 1,
      };
    }
    i += 1;
  }
  return undefined;
}

/** Decode just the three references in the archive, exactly once and linearly. */
function decodeEntities(source: string): string {
  const fragments: string[] = [];
  let textStart = 0;
  let i = 0;
  while (i < source.length) {
    if (source.charCodeAt(i) !== 0x26) {
      i += 1;
      continue;
    }
    let entity: string | undefined;
    let length = 0;
    if (source.startsWith("&lt;", i)) {
      entity = "<";
      length = 4;
    } else if (source.startsWith("&gt;", i)) {
      entity = ">";
      length = 4;
    } else if (source.startsWith("&amp;", i)) {
      entity = "&";
      length = 5;
    }
    if (entity !== undefined) {
      fragments.push(source.slice(textStart, i), entity);
      i += length;
      textStart = i;
    } else {
      i += 1;
    }
  }
  if (fragments.length === 0) return source;
  fragments.push(source.slice(textStart));
  return fragments.join("");
}

/** Read one anchor attribute value; the cursor remains at the next separator. */
function attributeValue(source: string, cursor: { value: number }): string | null {
  let i = cursor.value;
  if (i >= source.length) return null;
  const first = source.charCodeAt(i);
  if (first === 0x22 || first === 0x27) {
    i += 1;
    const start = i;
    while (i < source.length && source.charCodeAt(i) !== first) i += 1;
    if (i >= source.length) {
      cursor.value = i;
      return null;
    }
    const result = decodeEntities(source.slice(start, i));
    cursor.value = i + 1;
    return result;
  }
  const start = i;
  while (i < source.length) {
    const code = source.charCodeAt(i);
    if (isHtmlSpace(code) || code === 0x22 || code === 0x27 || code === 0x3d || code === 0x3c || code === 0x3e) {
      break;
    }
    i += 1;
  }
  if (i === start) return null;
  cursor.value = i;
  return decodeEntities(source.slice(start, i));
}

/** Parse only the archived anchor attributes, preserving the first occurrence. */
function anchorAttributes(source: string): Record<string, string | null> {
  const attributes: Record<string, string | null> = {};
  let i = 0;
  while (i < source.length) {
    const spaceStart = i;
    while (i < source.length && isHtmlSpace(source.charCodeAt(i))) i += 1;
    if (i === spaceStart || i >= source.length || !isAsciiLetter(source.charCodeAt(i))) break;
    const nameStart = i;
    while (i < source.length && isNameCharacter(source.charCodeAt(i))) i += 1;
    const nameEnd = i;
    const name = lowerAscii(source.slice(nameStart, i));
    while (i < source.length && isHtmlSpace(source.charCodeAt(i))) i += 1;
    let value: string | null;
    if (source.charCodeAt(i) === 0x3d) {
      i += 1;
      while (i < source.length && isHtmlSpace(source.charCodeAt(i))) i += 1;
      const cursor = { value: i };
      value = attributeValue(source, cursor);
      i = cursor.value;
    } else {
      // Whitespace belongs to the next attribute; let its leading-space test
      // decide whether the sequence is well formed.
      i = nameEnd;
      value = null;
    }
    if (name === "href" || name === "name" || name === "type") {
      if (!Object.hasOwn(attributes, name)) attributes[name] = value;
    }
  }
  return attributes;
}

function appendNode(nodes: DraftNode[], roots: NodeId[], stack: NodeId[], node: DraftNode): NodeId {
  const id = nodes.length;
  nodes.push(node);
  const parent = stack.length === 0 ? undefined : nodes[stack[stack.length - 1]];
  if (parent === undefined) {
    roots.push(id);
  } else if (parent.kind === "element") {
    parent.children.push(id);
  } else {
    throw new Error("historical HTML stack points to a text node");
  }
  return id;
}

/** Append or accumulate one text run without quadratic string concatenation. */
function appendText(
  nodes: DraftNode[],
  roots: NodeId[],
  stack: NodeId[],
  value: string,
  decode: boolean,
): void {
  if (value.length === 0) return;
  const text = decode ? decodeEntities(value) : value;
  const parent = stack.length === 0 ? undefined : nodes[stack[stack.length - 1]];
  if (parent !== undefined && parent.kind !== "element") {
    throw new Error("historical HTML stack points to a text node");
  }
  const siblings = parent === undefined ? roots : parent.children;
  const previousId = siblings.length === 0 ? undefined : siblings[siblings.length - 1];
  const previous = previousId === undefined ? undefined : nodes[previousId];
  if (previous?.kind === "text") {
    previous.fragments.push(text);
  } else {
    const id = nodes.length;
    nodes.push({ kind: "text", fragments: [text] });
    siblings.push(id);
  }
}

/** The scope position table keeps closing and implied-close lookups bounded. */
function topPosition(positions: Map<string, number[]>, tag: string): number | undefined {
  const entries = positions.get(tag);
  return entries === undefined || entries.length === 0 ? undefined : entries[entries.length - 1];
}

function latestPosition(positions: Map<string, number[]>, tags: readonly string[]): number | undefined {
  let latest: number | undefined;
  for (const tag of tags) {
    const position = topPosition(positions, tag);
    if (position !== undefined && (latest === undefined || position > latest)) latest = position;
  }
  return latest;
}

function truncateStack(
  nodes: DraftNode[],
  stack: NodeId[],
  positions: Map<string, number[]>,
  at: number,
): void {
  while (stack.length > at) {
    const id = stack.pop();
    if (id === undefined) throw new Error("historical HTML stack underflow");
    const node = nodes[id];
    if (node?.kind !== "element") throw new Error("historical HTML stack points to a text node");
    positions.get(node.tag)?.pop();
  }
}

function asciiEqualsIgnoreCase(source: string, start: number, target: string): boolean {
  if (start + target.length > source.length) return false;
  for (let i = 0; i < target.length; i += 1) {
    let left = source.charCodeAt(start + i);
    let right = target.charCodeAt(i);
    if (left >= 0x41 && left <= 0x5a) left += 0x20;
    if (right >= 0x41 && right <= 0x5a) right += 0x20;
    if (left !== right) return false;
  }
  return true;
}

/** Search raw text for its exact ASCII closing-tag spelling, ignoring case. */
function rawEnd(source: string, target: string, from: number): number | undefined {
  let candidate = source.indexOf("<", from);
  while (candidate !== -1) {
    if (asciiEqualsIgnoreCase(source, candidate, target)) return candidate;
    candidate = source.indexOf("<", candidate + 1);
  }
  return undefined;
}

function finalizeDocument(nodes: DraftNode[], roots: NodeId[]): Result<HtmlDocument, HtmlParseError> {
  const finalized: HtmlNode[] = new Array(nodes.length);
  for (let id = 0; id < nodes.length; id += 1) {
    const node = nodes[id];
    if (node === undefined) return { ok: false, error: { kind: "invalid-tree-state" } };
    if (node.kind === "text") {
      finalized[id] = { kind: "text", text: node.fragments.join("") };
    } else {
      if (node.children.some((child) => child < 0 || child >= nodes.length)) {
        return { ok: false, error: { kind: "invalid-tree-state" } };
      }
      finalized[id] = {
        kind: "element",
        tag: node.tag,
        attributes: node.attributes,
        children: node.children,
      };
    }
  }
  if (roots.some((root) => root < 0 || root >= nodes.length)) {
    return { ok: false, error: { kind: "invalid-tree-state" } };
  }
  return { ok: true, value: { nodes: finalized, roots } };
}

/** Parse the HWEB00 HTML Tags 1992 profile into a nonrecursive node arena. */
export function parseHtmlTags1992(source: string): Result<HtmlDocument, HtmlParseError> {
  if (typeof source !== "string") return { ok: false, error: { kind: "invalid-source" } };
  // Normalize every CRLF pair before any token or attribute scanner runs.
  const normalized = source.replaceAll("\r\n", "\n");
  const nodes: DraftNode[] = [];
  const roots: NodeId[] = [];
  const stack: NodeId[] = [];
  const positions = new Map<string, number[]>();
  let i = 0;
  let textStart = 0;
  while (i < normalized.length) {
    const start = normalized.indexOf("<", i);
    if (start === -1) {
      appendText(nodes, roots, stack, normalized.slice(textStart), true);
      break;
    }
    if (start > textStart) appendText(nodes, roots, stack, normalized.slice(textStart, start), true);
    if (normalized.startsWith("<!--", start)) {
      const end = normalized.indexOf("-->", start + 4);
      if (end === -1) return finalizeDocument(nodes, roots);
      i = end + 3;
      textStart = i;
      continue;
    }
    const token = tagAt(normalized, start);
    if (token === undefined) {
      appendText(nodes, roots, stack, "<", false);
      i = start + 1;
      textStart = i;
      continue;
    }
    const name = token.name;
    if (!knownTags.has(name)) {
      i = token.end;
      textStart = i;
      continue;
    }
    if (!token.closing && name === "plaintext") {
      const id = appendNode(nodes, roots, stack, {
        kind: "element",
        tag: name,
        attributes: {},
        children: [],
      });
      appendText(nodes, roots, [id], normalized.slice(token.end), false);
      return finalizeDocument(nodes, roots);
    }
    if (!token.closing && (name === "xmp" || name === "listing")) {
      const id = appendNode(nodes, roots, stack, {
        kind: "element",
        tag: name,
        attributes: {},
        children: [],
      });
      const endTag = `</${name}>`;
      const end = rawEnd(normalized, endTag, token.end);
      if (end === undefined) {
        appendText(nodes, roots, [id], normalized.slice(token.end), false);
        return finalizeDocument(nodes, roots);
      }
      appendText(nodes, roots, [id], normalized.slice(token.end, end), false);
      i = end + endTag.length;
      textStart = i;
      continue;
    }
    if (token.closing) {
      if (name !== "p" && name !== "isindex") {
        const at = topPosition(positions, name);
        if (at !== undefined) truncateStack(nodes, stack, positions, at);
      }
      i = token.end;
      textStart = i;
      continue;
    }
    if (name === "p" || name === "isindex") {
      appendNode(nodes, roots, stack, {
        kind: "element",
        tag: name,
        attributes: {},
        children: [],
      });
      i = token.end;
      textStart = i;
      continue;
    }

    let closingNames: readonly string[] = [];
    let boundaries: readonly string[] = [];
    if (name === "li") {
      closingNames = ["li"];
      boundaries = ["ul", "menu", "dir"];
    } else if (name === "dt" || name === "dd") {
      closingNames = ["dt", "dd"];
      boundaries = ["dl"];
    } else if (name.length === 2 && name[0] === "h" && name[1] >= "1" && name[1] <= "6") {
      closingNames = ["h1", "h2", "h3", "h4", "h5", "h6"];
    }
    const closeAt = latestPosition(positions, closingNames);
    const boundaryAt = latestPosition(positions, boundaries);
    if (closeAt !== undefined && (boundaryAt === undefined || closeAt > boundaryAt)) {
      truncateStack(nodes, stack, positions, closeAt);
    }
    const attributes = name === "a" ? anchorAttributes(token.attributes) : {};
    const id = appendNode(nodes, roots, stack, {
      kind: "element",
      tag: name,
      attributes,
      children: [],
    });
    const tagPositions = positions.get(name) ?? [];
    tagPositions.push(stack.length);
    positions.set(name, tagPositions);
    stack.push(id);
    i = token.end;
    textStart = i;
  }
  return finalizeDocument(nodes, roots);
}
