const BYTES_PER_RECORD = 16;
const RT_DATA = 0x00;
const RT_EOF = 0x01;
const MAX_IMAGE_SIZE = 0x1000;

function checksum(fields: readonly number[]): number {
  return (0x100 - (fields.reduce((total, value) => total + value, 0) % 256)) % 256;
}

function dataRecord(address: number, chunk: Uint8Array): string {
  const byteCount = chunk.length;
  const addrHi = (address >> 8) & 0xff;
  const addrLo = address & 0xff;
  const fields = [byteCount, addrHi, addrLo, RT_DATA, ...chunk];
  const cs = checksum(fields);
  const dataHex = Array.from(chunk, (value) => value.toString(16).toUpperCase().padStart(2, "0")).join("");
  return `:${byteCount.toString(16).toUpperCase().padStart(2, "0")}${addrHi
    .toString(16)
    .toUpperCase()
    .padStart(2, "0")}${addrLo.toString(16).toUpperCase().padStart(2, "0")}00${dataHex}${cs
    .toString(16)
    .toUpperCase()
    .padStart(2, "0")}\n`;
}

function eofRecord(): string {
  return ":00000001FF\n";
}

export function encodeHex(binary: Uint8Array, origin = 0): string {
  if (binary.length === 0) {
    throw new Error("binary must be non-empty");
  }
  if (origin < 0 || origin > 0xffff) {
    throw new Error(`origin must be 0-65535, got 0x${origin.toString(16).toUpperCase()}`);
  }
  if (origin + binary.length > 0x10000) {
    throw new Error(
      `image overflows 16-bit address space: origin=0x${origin
        .toString(16)
        .toUpperCase()}, size=${binary.length}`,
    );
  }

  const lines: string[] = [];
  for (let offset = 0; offset < binary.length; offset += BYTES_PER_RECORD) {
    lines.push(dataRecord(origin + offset, binary.slice(offset, offset + BYTES_PER_RECORD)));
  }
  lines.push(eofRecord());
  return lines.join("");
}

export interface DecodedHex {
  readonly origin: number;
  readonly binary: Uint8Array;
}

export function decodeHex(hexText: string): DecodedHex {
  const segments = new Map<number, Uint8Array>();

  const lines = hexText.split(/\r?\n/);
  for (let lineNumber = 0; lineNumber < lines.length; lineNumber += 1) {
    const rawLine = lines[lineNumber].trim();
    if (!rawLine) {
      continue;
    }
    if (!rawLine.startsWith(":")) {
      throw new Error(`line ${lineNumber + 1}: expected ':'`);
    }

    const payload = rawLine.slice(1);
    if (payload.length % 2 !== 0) {
      throw new Error(`line ${lineNumber + 1}: invalid hex length`);
    }

    const recordBytes = new Uint8Array(payload.length / 2);
    for (let index = 0; index < payload.length; index += 2) {
      const pair = payload.slice(index, index + 2);
      const parsed = Number.parseInt(pair, 16);
      if (Number.isNaN(parsed)) {
        throw new Error(`line ${lineNumber + 1}: invalid hex byte '${pair}'`);
      }
      recordBytes[index / 2] = parsed;
    }

    if (recordBytes.length < 5) {
      throw new Error(`line ${lineNumber + 1}: record too short`);
    }

    const byteCount = recordBytes[0];
    const address = (recordBytes[1] << 8) | recordBytes[2];
    const recordType = recordBytes[3];
    const expectedLength = 4 + byteCount + 1;
    if (recordBytes.length < expectedLength) {
      throw new Error(
        `line ${lineNumber + 1}: record claims ${byteCount} data bytes but only ${recordBytes.length} bytes are present`,
      );
    }

    const data = recordBytes.slice(4, 4 + byteCount);
    const storedChecksum = recordBytes[4 + byteCount];
    const computedChecksum = checksum(Array.from(recordBytes.slice(0, 4 + byteCount)));
    if (computedChecksum !== storedChecksum) {
      throw new Error(
        `line ${lineNumber + 1}: checksum mismatch (expected 0x${computedChecksum
          .toString(16)
          .toUpperCase()}, got 0x${storedChecksum.toString(16).toUpperCase()})`,
      );
    }

    if (recordType === RT_EOF) {
      break;
    }
    if (recordType !== RT_DATA) {
      throw new Error(`line ${lineNumber + 1}: unsupported record type 0x${recordType.toString(16)}`);
    }

    segments.set(address, data);
  }

  if (segments.size === 0) {
    return { origin: 0, binary: new Uint8Array() };
  }

  const addresses = Array.from(segments.keys()).sort((left, right) => left - right);
  const origin = addresses[0];
  const end = Math.max(...addresses.map((address) => address + (segments.get(address)?.length ?? 0)));

  if (end - origin > MAX_IMAGE_SIZE) {
    throw new Error(
      `decoded image too large: ${end - origin} bytes (maximum ${MAX_IMAGE_SIZE} bytes for Intel 4004 ROM)`,
    );
  }

  const buffer = new Uint8Array(end - origin);
  for (const address of addresses) {
    const segment = segments.get(address);
    if (!segment) {
      continue;
    }
    buffer.set(segment, address - origin);
  }

  return { origin, binary: buffer };
}
