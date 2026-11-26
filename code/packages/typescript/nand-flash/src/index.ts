/**
 * # Logical SLC NAND flash
 *
 * This small educational model puts the important media rules in code: erased
 * bits are one, a page accepts one full-page program between erases, and erase
 * acts on an entire block. Database translation and silicon details come later.
 *
 * A one-page write is like writing in pencil on a page that can only be erased
 * as part of a bound notebook. To rewrite one page safely, a controller must
 * preserve live pages, erase the block, and then write the desired pages back.
 * This leaf intentionally does not implement that controller.
 *
 * All numeric inputs and counters are safe JavaScript integers. This is why the
 * portable contract caps the erase counter at 2^53−1 even in an unbounded mode.
 */

export const MAX_BLOCKS = 1024;
export const MAX_PAGES_PER_BLOCK = 256;
export const MAX_PAGE_SIZE_BYTES = 65_536;
export const MAX_TOTAL_BYTES = 16 * 1024 * 1024;
export const MAX_SAFE_ERASE_COUNT = Number.MAX_SAFE_INTEGER;
const typedArrayLength = Object.getOwnPropertyDescriptor(
  Object.getPrototypeOf(Uint8Array.prototype),
  "length",
)!.get!;
const typedArrayBuffer = Object.getOwnPropertyDescriptor(
  Object.getPrototypeOf(Uint8Array.prototype),
  "buffer",
)!.get!;
const typedArrayTag = Object.getOwnPropertyDescriptor(
  Object.getPrototypeOf(Uint8Array.prototype),
  Symbol.toStringTag,
)!.get!;
const arrayBufferByteLength = Object.getOwnPropertyDescriptor(
  ArrayBuffer.prototype,
  "byteLength",
)!.get!;

/** Base class for expected device-command failures. */
export class NANDFlashError extends Error {
  constructor(message: string) {
    super(message);
    this.name = new.target.name;
  }
}

/** Invalid or excessively large device geometry. */
export class GeometryError extends NANDFlashError {}
/** A block or page address is outside the device. */
export class AddressError extends NANDFlashError {}
/** A payload is not an exact Uint8Array page of the configured size. */
export class PayloadError extends NANDFlashError {}
/** A page has already been programmed since its containing block was erased. */
export class ProgramError extends NANDFlashError {}
/** A block has reached its configured or portable erase-count ceiling. */
export class EraseLimitError extends NANDFlashError {}

export interface NANDFlashOptions {
  blockCount: number;
  pagesPerBlock: number;
  pageSizeBytes: number;
  maxEraseCycles?: number | null;
}

export interface NANDFlashSnapshot {
  pages: string[][];
  erase_counts: number[];
  programmed: boolean[][];
}

/** Deterministic logical media with bounded geometry and atomic rejection. */
export class NANDFlash {
  readonly #blockCount: number;
  readonly #pagesPerBlock: number;
  readonly #pageSizeBytes: number;
  readonly #maxEraseCycles: number | null;
  readonly #pages: Uint8Array[][];
  readonly #programmed: boolean[][];
  readonly #eraseCounts: number[];

  constructor(options: NANDFlashOptions) {
    const { blockCount, pagesPerBlock, pageSizeBytes } = options;
    const maxEraseCycles = options.maxEraseCycles ?? null;
    if (
      !Number.isSafeInteger(blockCount) ||
      !Number.isSafeInteger(pagesPerBlock) ||
      !Number.isSafeInteger(pageSizeBytes) ||
      blockCount < 1 ||
      blockCount > MAX_BLOCKS ||
      pagesPerBlock < 1 ||
      pagesPerBlock > MAX_PAGES_PER_BLOCK ||
      pageSizeBytes < 1 ||
      pageSizeBytes > MAX_PAGE_SIZE_BYTES ||
      blockCount * pagesPerBlock * pageSizeBytes > MAX_TOTAL_BYTES ||
      (maxEraseCycles !== null &&
        (!Number.isSafeInteger(maxEraseCycles) ||
          maxEraseCycles < 0 ||
          maxEraseCycles > MAX_SAFE_ERASE_COUNT))
    ) {
      throw new GeometryError("geometry values are outside supported bounds");
    }

    this.#blockCount = blockCount;
    this.#pagesPerBlock = pagesPerBlock;
    this.#pageSizeBytes = pageSizeBytes;
    this.#maxEraseCycles = maxEraseCycles;
    this.#pages = Array.from({ length: blockCount }, () =>
      Array.from({ length: pagesPerBlock }, () => new Uint8Array(pageSizeBytes).fill(0xff)),
    );
    this.#programmed = Array.from({ length: blockCount }, () =>
      Array.from({ length: pagesPerBlock }, () => false),
    );
    this.#eraseCounts = Array.from({ length: blockCount }, () => 0);
  }

  get blockCount(): number { return this.#blockCount; }
  get pagesPerBlock(): number { return this.#pagesPerBlock; }
  get pageSizeBytes(): number { return this.#pageSizeBytes; }
  get maxEraseCycles(): number | null { return this.#maxEraseCycles; }

  #checkBlock(block: number): number {
    if (!Number.isSafeInteger(block) || block < 0 || block >= this.#blockCount) {
      throw new AddressError("block index out of range");
    }
    return block;
  }

  #checkPage(page: number): number {
    if (!Number.isSafeInteger(page) || page < 0 || page >= this.#pagesPerBlock) {
      throw new AddressError("page index out of range");
    }
    return page;
  }

  /** Return a copy so callers cannot mutate device state through a read. */
  read(block: number, page: number): Uint8Array {
    const b = this.#checkBlock(block);
    const p = this.#checkPage(page);
    return this.#pages[b][p].slice();
  }

  /** Program one full page, consuming its program marker even for all-FF data. */
  program(block: number, page: number, data: Uint8Array): void {
    const b = this.#checkBlock(block);
    const p = this.#checkPage(page);
    if (
      !ArrayBuffer.isView(data) ||
      Object.getPrototypeOf(data) !== Uint8Array.prototype
    ) {
      throw new PayloadError("page data must be bytes");
    }
    const typedArrayKind = typedArrayTag.call(data) as string | undefined;
    if (typedArrayKind !== "Uint8Array") {
      throw new PayloadError("page data must be bytes");
    }
    const backingBuffer = typedArrayBuffer.call(data) as ArrayBufferLike;
    try {
      // instanceof is forgeable by changing a SharedArrayBuffer's prototype;
      // this borrowed intrinsic verifies the actual internal ArrayBuffer brand.
      arrayBufferByteLength.call(backingBuffer);
    } catch {
      throw new PayloadError("page data must be bytes");
    }
    const inputLength = typedArrayLength.call(data) as number;
    if (inputLength !== this.#pageSizeBytes) {
      throw new PayloadError("page data must match page_size_bytes");
    }
    if (this.#programmed[b][p]) {
      throw new ProgramError("page has already been programmed since erase");
    }

    // Copy only after validating the built-in typed-array brand and bounded size.
    const payload = new Uint8Array(data);
    const next = new Uint8Array(this.#pageSizeBytes);
    for (let index = 0; index < this.#pageSizeBytes; index += 1) {
      // Erased bytes are FF, so this mask applies only one-to-zero changes.
      next[index] = this.#pages[b][p][index] & payload[index];
    }
    this.#pages[b][p] = next;
    this.#programmed[b][p] = true;
  }

  /** Erase every page in one block, incrementing only its cycle counter. */
  eraseBlock(block: number): void {
    const b = this.#checkBlock(block);
    if (
      this.#eraseCounts[b] >= MAX_SAFE_ERASE_COUNT ||
      (this.#maxEraseCycles !== null && this.#eraseCounts[b] >= this.#maxEraseCycles)
    ) {
      throw new EraseLimitError("block erase limit reached");
    }
    this.#pages[b] = Array.from({ length: this.#pagesPerBlock }, () =>
      new Uint8Array(this.#pageSizeBytes).fill(0xff),
    );
    this.#programmed[b] = Array.from({ length: this.#pagesPerBlock }, () => false);
    this.#eraseCounts[b] += 1;
  }

  /** Return fresh nested arrays and lowercase hex page strings. */
  snapshot(): NANDFlashSnapshot {
    return {
      pages: this.#pages.map((block) => block.map((page) => bytesToHex(page))),
      erase_counts: [...this.#eraseCounts],
      programmed: this.#programmed.map((block) => [...block]),
    };
  }
}

/** Avoid Node Buffer so this package has no Node-specific implementation path. */
function bytesToHex(bytes: Uint8Array): string {
  let result = "";
  for (const byte of bytes) result += byte.toString(16).padStart(2, "0");
  return result;
}
