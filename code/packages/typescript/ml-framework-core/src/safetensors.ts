/**
 * # safetensors.ts — first HF interop (Phase A.7)
 *
 * Read and write the Hugging Face `safetensors` file format.  This is
 * the format every modern HF model checkpoint ships in (replacing
 * pickle for security reasons — safetensors deliberately stores ONLY
 * tensor bytes + a tiny JSON header, so loading can't execute
 * arbitrary code the way `pickle.load` can).
 *
 * Once `loadSafetensors` works, this framework can consume any HF
 * checkpoint whose tensors are all F32.  Phase B (next) builds on
 * top to assemble those tensors into actual transformer
 * architectures.
 *
 * ## On-disk format (canonical spec)
 *
 *   ┌─────────────────────────┐
 *   │ 8 bytes: header length  │  little-endian u64
 *   ├─────────────────────────┤
 *   │ JSON header (UTF-8)     │  exactly `header length` bytes
 *   ├─────────────────────────┤
 *   │ Raw tensor bytes        │  rest of file, no alignment
 *   └─────────────────────────┘
 *
 * The JSON header maps tensor name → metadata:
 *
 *   {
 *     "weight":     { "dtype": "F32", "shape": [10, 20], "data_offsets": [0, 800] },
 *     "bias":       { "dtype": "F32", "shape": [20],     "data_offsets": [800, 880] },
 *     "__metadata__": { "format": "pt" }
 *   }
 *
 * `data_offsets` are byte ranges into the payload (the bytes AFTER
 * the JSON header).  The optional `__metadata__` key holds free-form
 * string key/value pairs (preserved on round-trip; otherwise ignored).
 *
 * ## v1.7 scope
 *
 * - **F32 only** for now.  F16/BF16/I64/U8/etc throw a clear error
 *   at load time.  Storage in this framework is F32 anyway (see
 *   `Tensor` design); supporting other dtypes is post-A.7 work.
 * - **Synchronous file I/O** via `fs.writeFileSync` / `readFileSync`.
 *   Async is straightforward to add later (just swap to `fs/promises`).
 * - **Defensive parsing**: the on-disk header is attacker-controllable
 *   (anyone can hand you a malicious .safetensors).  We validate
 *   header length, JSON well-formedness, that each `data_offsets`
 *   range is inside the payload, and that the byte length matches
 *   the shape × 4-bytes-per-f32.  Out-of-bounds offsets, overlapping
 *   ranges, or size mismatches throw clear errors instead of
 *   silently OOB-reading.
 */

import * as fs from "node:fs";
import { Tensor } from "./tensor.js";

// ── Constants ──────────────────────────────────────────────────────

/** Bytes per F32 cell. */
const F32_BYTES = 4;

/**
 * Maximum JSON header size we'll accept on load.  Defends against
 * a malicious file with `headerLength = 2^60` from causing a giant
 * Buffer allocation.  100 MB is generous — real HF safetensors
 * headers are typically a few hundred KB even for huge models.
 */
const MAX_HEADER_BYTES = 100 * 1024 * 1024;

/** Type tag for the single dtype we support in v1.7. */
const SUPPORTED_DTYPE = "F32" as const;

/** All recognized safetensors dtype tags (for clearer error messages). */
const KNOWN_DTYPES = new Set([
  "F64", "F32", "F16", "BF16",
  "I64", "I32", "I16", "I8",
  "U64", "U32", "U16", "U8",
  "BOOL",
]);

// ── Header types ───────────────────────────────────────────────────

/** Shape of an entry in the JSON header (per tensor). */
interface TensorEntry {
  dtype: string;
  shape: number[];
  data_offsets: [number, number];
}

/** Optional free-form metadata that may appear alongside tensor entries. */
type Metadata = Record<string, string>;

// ── save ───────────────────────────────────────────────────────────

/**
 * Write a Record of name → Tensor to a `.safetensors` file.
 *
 * Tensors are laid out back-to-back in iteration order of the input
 * Record (JS preserves insertion order for string keys).  Each
 * tensor's bytes are its `data` buffer cast to f32 little-endian
 * (which is the native representation in V8 on all platforms we
 * care about — Node runs LE on x86 + arm64).
 *
 * @param tensors map of tensor name → Tensor
 * @param filePath where to write
 * @param metadata optional `__metadata__` map (preserved on load)
 */
export function saveSafetensors(
  tensors: Record<string, Tensor>,
  filePath: string,
  metadata?: Metadata,
): void {
  // 1. Build header object — compute data_offsets in iteration order.
  const header: Record<string, TensorEntry | Metadata> = {};
  let offset = 0;
  for (const [name, t] of Object.entries(tensors)) {
    if (name === "__metadata__") {
      throw new RangeError(
        `tensor name "__metadata__" is reserved by the safetensors format`,
      );
    }
    // Reject names that would mutate the prototype chain of the load-side
    // record on the read path.  See the matching guard in loadSafetensors.
    if (name === "__proto__" || name === "constructor" || name === "prototype") {
      throw new RangeError(`tensor name "${name}" is reserved`);
    }
    const byteLen = t.numel * F32_BYTES;
    header[name] = {
      dtype: SUPPORTED_DTYPE,
      shape: t.shape.slice(),
      data_offsets: [offset, offset + byteLen],
    };
    offset += byteLen;
  }
  if (metadata) {
    header["__metadata__"] = { ...metadata };
  }

  // 2. Serialize the header to UTF-8 bytes.
  const headerBytes = Buffer.from(JSON.stringify(header), "utf-8");
  const headerLen = headerBytes.length;

  // 3. Assemble the on-disk buffer: 8-byte LE length + header + payload.
  const totalBytes = 8 + headerLen + offset;
  const out = Buffer.alloc(totalBytes);
  // 8-byte little-endian u64 header length.  Node's `writeBigUInt64LE`
  // takes a BigInt — convert via BigInt().  headerLen always fits in
  // JS's safe-integer range (< 2^53), so the BigInt cast is safe.
  out.writeBigUInt64LE(BigInt(headerLen), 0);
  headerBytes.copy(out, 8);

  // 4. Copy each tensor's raw f32 bytes into the payload region.
  let cursor = 8 + headerLen;
  for (const t of Object.values(tensors)) {
    const view = Buffer.from(t.data.buffer, t.data.byteOffset, t.data.byteLength);
    view.copy(out, cursor);
    cursor += t.numel * F32_BYTES;
  }

  fs.writeFileSync(filePath, out);
}

// ── load ───────────────────────────────────────────────────────────

/**
 * Result of `loadSafetensors`.  The map is the same shape as what
 * `saveSafetensors` accepts, plus an optional `metadata` field with
 * the round-tripped `__metadata__`.
 */
export interface LoadResult {
  tensors: Record<string, Tensor>;
  metadata: Metadata | null;
}

/**
 * Read a `.safetensors` file and return its tensors + metadata.
 *
 * Strict validation: throws on
 *   - file < 8 bytes (no room for the header length)
 *   - header length > `MAX_HEADER_BYTES` or > file size
 *   - JSON header that doesn't parse
 *   - an entry whose dtype isn't F32 (clear "unsupported dtype X" message)
 *   - shape that isn't an array of non-negative integers
 *   - data_offsets outside [0, payloadLen], or end < start
 *   - data_offset byte length that doesn't match `numel * 4`
 *
 * For F16, BF16, etc., the error message names the dtype so callers
 * know what kind of unsupported file they handed us — they may want
 * to convert it before retrying.
 */
export function loadSafetensors(filePath: string): LoadResult {
  const fileBuf = fs.readFileSync(filePath);

  if (fileBuf.length < 8) {
    throw new RangeError(
      `safetensors file too short: ${fileBuf.length} bytes (need at least 8 for header length)`,
    );
  }

  // 1. Parse 8-byte LE u64 header length.  Reject pathologically large values.
  const headerLenBig = fileBuf.readBigUInt64LE(0);
  if (headerLenBig > BigInt(MAX_HEADER_BYTES)) {
    throw new RangeError(
      `safetensors header length ${headerLenBig} exceeds maximum ${MAX_HEADER_BYTES} bytes`,
    );
  }
  const headerLen = Number(headerLenBig);
  if (headerLen < 2) {
    // Even an empty header would be "{}" — 2 bytes.
    throw new RangeError(`safetensors header length ${headerLen} is too small (min 2 for "{}"`);
  }
  if (8 + headerLen > fileBuf.length) {
    throw new RangeError(
      `safetensors header length ${headerLen} extends past end of file (file is ${fileBuf.length} bytes)`,
    );
  }

  // 2. Parse the JSON header.
  let header: Record<string, unknown>;
  try {
    const jsonStr = fileBuf.slice(8, 8 + headerLen).toString("utf-8");
    header = JSON.parse(jsonStr) as Record<string, unknown>;
  } catch (e) {
    throw new SyntaxError(`safetensors header is not valid JSON: ${(e as Error).message}`);
  }
  if (typeof header !== "object" || header === null || Array.isArray(header)) {
    throw new SyntaxError("safetensors header must be a JSON object at the top level");
  }

  // Payload region: bytes after the header.
  const payloadStart = 8 + headerLen;
  const payloadLen = fileBuf.length - payloadStart;

  // 3. Walk entries.  Extract metadata if present; everything else is a tensor.
  //
  // Use null-prototype objects for both records so that a malicious file
  // declaring a tensor named "__proto__" can't mutate the returned record's
  // prototype chain.  We ALSO explicitly reject the three reserved names
  // ("__proto__", "constructor", "prototype") with a clear error, since
  // even with a null-prototype object those names would be surprising and
  // are never used in real HF checkpoints.
  const tensors: Record<string, Tensor> = Object.create(null);
  let metadata: Metadata | null = null;
  for (const [name, raw] of Object.entries(header)) {
    if (name === "__metadata__") {
      if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
        throw new SyntaxError(`__metadata__ must be a JSON object`);
      }
      // Coerce to Metadata; sanity-check all values are strings.
      // Null-prototype for the same reason as `tensors` above.
      const m: Metadata = Object.create(null);
      for (const [k, v] of Object.entries(raw as Record<string, unknown>)) {
        if (typeof v !== "string") {
          throw new SyntaxError(`__metadata__["${k}"] must be a string, got ${typeof v}`);
        }
        m[k] = v;
      }
      metadata = m;
      continue;
    }

    // Reject names that would mutate the prototype chain of the returned
    // tensors record on the read path.  This complements the null-prototype
    // base — belt-and-suspenders against a maliciously-crafted file.
    if (name === "__proto__" || name === "constructor" || name === "prototype") {
      throw new RangeError(
        `safetensors entry "${name}" uses a reserved tensor name`,
      );
    }
    const entry = validateEntry(name, raw, payloadLen);
    if (entry.dtype !== SUPPORTED_DTYPE) {
      const knownTag = KNOWN_DTYPES.has(entry.dtype) ? entry.dtype : `${entry.dtype}?`;
      throw new RangeError(
        `safetensors entry "${name}": unsupported dtype "${knownTag}". ` +
          `v1.7 supports only F32. (Consider converting to F32 before loading.)`,
      );
    }
    const [start, end] = entry.data_offsets;
    const byteLen = end - start;
    const expectedNumel = entry.shape.reduce((acc, d) => acc * d, 1);
    if (byteLen !== expectedNumel * F32_BYTES) {
      throw new RangeError(
        `safetensors entry "${name}": data_offsets byte length ${byteLen} ` +
          `does not match shape ${JSON.stringify(entry.shape)} × ${F32_BYTES} = ${expectedNumel * F32_BYTES}`,
      );
    }
    // Slice payload + copy into a fresh Float32Array so the Tensor
    // owns its own buffer (independent of the file Buffer's lifetime).
    const tensorBytes = fileBuf.slice(payloadStart + start, payloadStart + end);
    const f32 = new Float32Array(expectedNumel);
    Buffer.from(f32.buffer).set(tensorBytes);
    tensors[name] = new Tensor(Array.from(f32), { shape: entry.shape.slice() });
  }

  return { tensors, metadata };
}

/** Validate one header entry's shape; throws SyntaxError on malformed entry. */
function validateEntry(name: string, raw: unknown, payloadLen: number): TensorEntry {
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
    throw new SyntaxError(`safetensors entry "${name}" must be a JSON object`);
  }
  const obj = raw as Record<string, unknown>;
  const dtype = obj["dtype"];
  const shape = obj["shape"];
  const offs = obj["data_offsets"];

  if (typeof dtype !== "string") {
    throw new SyntaxError(`safetensors entry "${name}": dtype must be a string`);
  }
  if (!Array.isArray(shape)) {
    throw new SyntaxError(`safetensors entry "${name}": shape must be an array`);
  }
  for (let i = 0; i < shape.length; i++) {
    const d = shape[i];
    if (typeof d !== "number" || !Number.isInteger(d) || d < 0) {
      throw new SyntaxError(
        `safetensors entry "${name}": shape[${i}] must be a non-negative integer, got ${JSON.stringify(d)}`,
      );
    }
  }
  if (
    !Array.isArray(offs) ||
    offs.length !== 2 ||
    typeof offs[0] !== "number" ||
    typeof offs[1] !== "number" ||
    !Number.isInteger(offs[0]) ||
    !Number.isInteger(offs[1])
  ) {
    throw new SyntaxError(
      `safetensors entry "${name}": data_offsets must be [start, end] of two integers`,
    );
  }
  const [start, end] = offs as [number, number];
  if (start < 0 || end < start || end > payloadLen) {
    throw new RangeError(
      `safetensors entry "${name}": data_offsets [${start}, ${end}] out of payload bounds [0, ${payloadLen}]`,
    );
  }
  return { dtype, shape: shape as number[], data_offsets: [start, end] };
}
