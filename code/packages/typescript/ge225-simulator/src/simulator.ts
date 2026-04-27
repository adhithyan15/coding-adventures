const MASK_20 = (1 << 20) - 1;
const DATA_MASK = (1 << 19) - 1;
const SIGN_BIT = 1 << 19;
const ADDR_MASK = 0x1fff;
const X_MASK = 0x7fff;
const N_MASK = 0x3f;
const WORD_BYTES = 3;
const MAX_X_GROUPS = 32;

const OP_LDA = 0o00;
const OP_ADD = 0o01;
const OP_SUB = 0o02;
const OP_STA = 0o03;
const OP_BXL = 0o04;
const OP_BXH = 0o05;
const OP_LDX = 0o06;
const OP_SPB = 0o07;
const OP_DLD = 0o10;
const OP_DAD = 0o11;
const OP_DSU = 0o12;
const OP_DST = 0o13;
const OP_INX = 0o14;
const OP_MPY = 0o15;
const OP_DVD = 0o16;
const OP_STX = 0o17;
const OP_EXT = 0o20;
const OP_CAB = 0o21;
const OP_DCB = 0o22;
const OP_ORY = 0o23;
const OP_MOY = 0o24;
const OP_RCD = 0o25;
const OP_BRU = 0o26;
const OP_STO = 0o27;

const BASE_OPCODE_NAMES = new Map<number, string>([
  [OP_LDA, "LDA"],
  [OP_ADD, "ADD"],
  [OP_SUB, "SUB"],
  [OP_STA, "STA"],
  [OP_BXL, "BXL"],
  [OP_BXH, "BXH"],
  [OP_LDX, "LDX"],
  [OP_SPB, "SPB"],
  [OP_DLD, "DLD"],
  [OP_DAD, "DAD"],
  [OP_DSU, "DSU"],
  [OP_DST, "DST"],
  [OP_INX, "INX"],
  [OP_MPY, "MPY"],
  [OP_DVD, "DVD"],
  [OP_STX, "STX"],
  [OP_EXT, "EXT"],
  [OP_CAB, "CAB"],
  [OP_DCB, "DCB"],
  [OP_ORY, "ORY"],
  [OP_MOY, "MOY"],
  [OP_RCD, "RCD"],
  [OP_BRU, "BRU"],
  [OP_STO, "STO"],
]);

const NON_MODIFYING_MEMORY_REFERENCE = new Set(["BXL", "BXH", "LDX", "SPB", "INX", "STX", "MOY"]);

const FIXED_WORDS = new Map<string, number>([
  ["OFF", parseInt("2500005", 8)],
  ["TYP", parseInt("2500006", 8)],
  ["TON", parseInt("2500007", 8)],
  ["RCS", parseInt("2500011", 8)],
  ["HPT", parseInt("2500016", 8)],
  ["LDZ", parseInt("2504002", 8)],
  ["LDO", parseInt("2504022", 8)],
  ["LMO", parseInt("2504102", 8)],
  ["CPL", parseInt("2504502", 8)],
  ["NEG", parseInt("2504522", 8)],
  ["CHS", parseInt("2504040", 8)],
  ["NOP", parseInt("2504012", 8)],
  ["LAQ", parseInt("2504001", 8)],
  ["LQA", parseInt("2504004", 8)],
  ["XAQ", parseInt("2504005", 8)],
  ["MAQ", parseInt("2504006", 8)],
  ["ADO", parseInt("2504032", 8)],
  ["SBO", parseInt("2504112", 8)],
  ["SET_DECMODE", parseInt("2506011", 8)],
  ["SET_BINMODE", parseInt("2506012", 8)],
  ["SXG", parseInt("2506013", 8)],
  ["SET_PST", parseInt("2506015", 8)],
  ["SET_PBK", parseInt("2506016", 8)],
  ["BOD", parseInt("2514000", 8)],
  ["BEV", parseInt("2516000", 8)],
  ["BMI", parseInt("2514001", 8)],
  ["BPL", parseInt("2516001", 8)],
  ["BZE", parseInt("2514002", 8)],
  ["BNZ", parseInt("2516002", 8)],
  ["BOV", parseInt("2514003", 8)],
  ["BNO", parseInt("2516003", 8)],
  ["BPE", parseInt("2514004", 8)],
  ["BPC", parseInt("2516004", 8)],
  ["BNR", parseInt("2514005", 8)],
  ["BNN", parseInt("2516005", 8)],
]);

const FIXED_NAMES = new Map<number, string>(Array.from(FIXED_WORDS.entries()).map(([k, v]) => [v, k]));

const SHIFT_BASES = new Map<string, number>([
  ["SRA", parseInt("2510000", 8)],
  ["SNA", parseInt("2510100", 8)],
  ["SCA", parseInt("2510040", 8)],
  ["SAN", parseInt("2510400", 8)],
  ["SRD", parseInt("2511000", 8)],
  ["NAQ", parseInt("2511100", 8)],
  ["SCD", parseInt("2511200", 8)],
  ["ANQ", parseInt("2511400", 8)],
  ["SLA", parseInt("2512000", 8)],
  ["SLD", parseInt("2512200", 8)],
  ["NOR", parseInt("2513000", 8)],
  ["DNO", parseInt("2513200", 8)],
]);

const TYPEWRITER_CODES = new Map<number, string>([
  [0o00, "0"], [0o01, "1"], [0o02, "2"], [0o03, "3"], [0o04, "4"], [0o05, "5"], [0o06, "6"], [0o07, "7"],
  [0o10, "8"], [0o11, "9"], [0o13, "/"], [0o21, "A"], [0o22, "B"], [0o23, "C"], [0o24, "D"], [0o25, "E"],
  [0o26, "F"], [0o27, "G"], [0o30, "H"], [0o31, "I"], [0o33, "-"], [0o40, "."], [0o41, "J"], [0o42, "K"],
  [0o43, "L"], [0o44, "M"], [0o45, "N"], [0o46, "O"], [0o47, "P"], [0o50, "Q"], [0o51, "R"], [0o53, "$"],
  [0o60, " "], [0o62, "S"], [0o63, "T"], [0o64, "U"], [0o65, "V"], [0o66, "W"], [0o67, "X"], [0o70, "Y"],
  [0o71, "Z"],
]);

export interface GE225Indicators {
  carry: boolean;
  zero: boolean;
  negative: boolean;
  overflow: boolean;
  parityError: boolean;
}

export interface GE225State {
  a: number;
  q: number;
  m: number;
  n: number;
  pc: number;
  ir: number;
  indicators: GE225Indicators;
  overflow: boolean;
  parityError: boolean;
  decimalMode: boolean;
  automaticInterruptMode: boolean;
  selectedXGroup: number;
  nReady: boolean;
  typewriterPower: boolean;
  controlSwitches: number;
  xWords: readonly number[];
  halted: boolean;
  memory: readonly number[];
}

export interface GE225Trace {
  address: number;
  instructionWord: number;
  mnemonic: string;
  aBefore: number;
  aAfter: number;
  qBefore: number;
  qAfter: number;
  effectiveAddress: number | null;
}

interface DecodedInstruction {
  mnemonic: string;
  opcode: number | null;
  modifier: number | null;
  address: number | null;
  count: number | null;
  fixedWord: boolean;
}

function toSigned20(value: number): number {
  const word = value & MASK_20;
  return (word & SIGN_BIT) !== 0 ? word - (1 << 20) : word;
}

function fromSigned20(value: number): number {
  return value & MASK_20;
}

function signOf(word: number): number {
  return (word & SIGN_BIT) !== 0 ? 1 : 0;
}

function withSign(word: number, sign: number): number {
  return ((sign & 1) << 19) | (word & DATA_MASK);
}

function combineWords(high: number, low: number): bigint {
  return (BigInt(high & MASK_20) << 20n) | BigInt(low & MASK_20);
}

function splitSigned40(value: bigint): [number, number] {
  const masked = BigInt.asUintN(40, value);
  return [Number((masked >> 20n) & BigInt(MASK_20)), Number(masked & BigInt(MASK_20))];
}

function toSigned40(value: bigint): bigint {
  return BigInt.asIntN(40, value);
}

function arithCompare(left: number, right: number): number {
  const l = toSigned20(left);
  const r = toSigned20(right);
  if (l < r) return -1;
  if (l > r) return 1;
  return 0;
}

function arithCompareDouble(leftHigh: number, leftLow: number, rightHigh: number, rightLow: number): number {
  const left = toSigned40(combineWords(leftHigh, leftLow));
  const right = toSigned40(combineWords(rightHigh, rightLow));
  if (left < right) return -1;
  if (left > right) return 1;
  return 0;
}

export function encodeInstruction(opcode: number, modifier: number, address: number): number {
  if (opcode < 0 || opcode > 0o37) throw new Error(`opcode out of range: ${opcode}`);
  if (modifier < 0 || modifier > 0o3) throw new Error(`modifier out of range: ${modifier}`);
  if (address < 0 || address > ADDR_MASK) throw new Error(`address out of range: ${address}`);
  return ((opcode & 0x1f) << 15) | ((modifier & 0x03) << 13) | (address & ADDR_MASK);
}

export function decodeInstruction(word: number): [number, number, number] {
  const normalized = word & MASK_20;
  return [(normalized >> 15) & 0x1f, (normalized >> 13) & 0x03, normalized & ADDR_MASK];
}

export function assembleFixed(mnemonic: string): number {
  const word = FIXED_WORDS.get(mnemonic);
  if (word === undefined) throw new Error(`unknown fixed GE-225 instruction: ${mnemonic}`);
  return word;
}

export function assembleShift(mnemonic: string, count: number): number {
  if (count < 0 || count > 0o37) throw new Error(`shift count out of range: ${count}`);
  const base = SHIFT_BASES.get(mnemonic);
  if (base === undefined) throw new Error(`unknown GE-225 shift instruction: ${mnemonic}`);
  return base | count;
}

export function packWords(words: number[]): Uint8Array {
  const blob = new Uint8Array(words.length * WORD_BYTES);
  words.forEach((word, index) => {
    const normalized = word & MASK_20;
    blob[index * WORD_BYTES] = (normalized >> 16) & 0xff;
    blob[index * WORD_BYTES + 1] = (normalized >> 8) & 0xff;
    blob[index * WORD_BYTES + 2] = normalized & 0xff;
  });
  return blob;
}

export function unpackWords(program: Uint8Array): number[] {
  if (program.length % WORD_BYTES !== 0) {
    throw new Error(`GE-225 byte stream must be a multiple of ${WORD_BYTES} bytes, got ${program.length}`);
  }
  const words: number[] = [];
  for (let i = 0; i < program.length; i += WORD_BYTES) {
    words.push(((program[i] << 16) | (program[i + 1] << 8) | program[i + 2]) & MASK_20);
  }
  return words;
}

export class GE225Simulator {
  private readonly memorySize: number;
  private readonly memory: number[];
  private readonly cardReaderQueue: number[][] = [];
  private a = 0;
  private q = 0;
  private m = 0;
  private n = 0;
  private pc = 0;
  private ir = 0;
  private overflow = false;
  private parityError = false;
  private decimalMode = false;
  private automaticInterruptMode = false;
  private selectedXGroup = 0;
  private nReady = true;
  private typewriterPower = false;
  private readonly typewriterOutput: string[] = [];
  private controlSwitches = 0;
  private halted = false;
  private readonly xGroups: number[][] = Array.from({ length: MAX_X_GROUPS }, () => [0, 0, 0, 0]);

  constructor(memoryWords = 4096) {
    if (memoryWords <= 0) throw new Error("memoryWords must be positive");
    this.memorySize = memoryWords;
    this.memory = new Array(memoryWords).fill(0);
  }

  reset(): void {
    this.a = 0;
    this.q = 0;
    this.m = 0;
    this.n = 0;
    this.pc = 0;
    this.ir = 0;
    this.overflow = false;
    this.parityError = false;
    this.decimalMode = false;
    this.automaticInterruptMode = false;
    this.selectedXGroup = 0;
    this.nReady = true;
    this.typewriterPower = false;
    this.typewriterOutput.length = 0;
    this.controlSwitches = 0;
    this.halted = false;
    this.xGroups.forEach((group) => group.fill(0));
  }

  getState(): GE225State {
    return {
      a: this.a,
      q: this.q,
      m: this.m,
      n: this.n,
      pc: this.pc,
      ir: this.ir,
      indicators: {
        carry: this.overflow,
        zero: this.a === 0,
        negative: (this.a & SIGN_BIT) !== 0,
        overflow: this.overflow,
        parityError: this.parityError,
      },
      overflow: this.overflow,
      parityError: this.parityError,
      decimalMode: this.decimalMode,
      automaticInterruptMode: this.automaticInterruptMode,
      selectedXGroup: this.selectedXGroup,
      nReady: this.nReady,
      typewriterPower: this.typewriterPower,
      controlSwitches: this.controlSwitches,
      xWords: [...this.xGroups[this.selectedXGroup]],
      halted: this.halted,
      memory: [...this.memory],
    };
  }

  setControlSwitches(value: number): void {
    this.controlSwitches = value & MASK_20;
  }

  queueCardReaderRecord(words: number[]): void {
    this.cardReaderQueue.push(words.map((word) => word & MASK_20));
  }

  getTypewriterOutput(): string {
    return this.typewriterOutput.join("");
  }

  loadWords(words: number[], startAddress = 0): void {
    words.forEach((word, index) => this.writeWord(startAddress + index, word));
  }

  readWord(address: number): number {
    this.checkAddress(address);
    return this.memory[address];
  }

  writeWord(address: number, value: number): void {
    this.checkAddress(address);
    this.memory[address] = value & MASK_20;
  }

  disassembleWord(word: number): string {
    const decoded = this.decodeWord(word);
    if (decoded.fixedWord) {
      return decoded.count === null ? decoded.mnemonic : `${decoded.mnemonic} ${decoded.count}`;
    }
    return `${decoded.mnemonic} 0x${decoded.address!.toString(16).toUpperCase().padStart(3, "0")},X${decoded.modifier}`;
  }

  step(): GE225Trace {
    if (this.halted) throw new Error("cannot step a halted GE-225 simulator");

    const pcBefore = this.pc;
    this.ir = this.readWord(this.pc);
    this.pc = (this.pc + 1) % this.memorySize;

    const decoded = this.decodeWord(this.ir);
    const aBefore = this.a;
    const qBefore = this.q;
    let effectiveAddress: number | null = null;

    if (!decoded.fixedWord) {
      const address = decoded.address!;
      if (!NON_MODIFYING_MEMORY_REFERENCE.has(decoded.mnemonic)) {
        effectiveAddress = this.resolveEffectiveAddress(address, decoded.modifier!);
      }
      this.executeMemoryReference(
        decoded.mnemonic,
        decoded.modifier!,
        effectiveAddress === null ? address : effectiveAddress,
        address,
        pcBefore,
      );
    } else {
      this.executeFixed(decoded);
    }

    return {
      address: pcBefore,
      instructionWord: this.ir,
      mnemonic: this.disassembleWord(this.ir),
      aBefore,
      aAfter: this.a,
      qBefore,
      qAfter: this.q,
      effectiveAddress,
    };
  }

  run(maxSteps = 100000): GE225Trace[] {
    const traces: GE225Trace[] = [];
    let steps = 0;
    while (!this.halted && steps < maxSteps) {
      traces.push(this.step());
      steps += 1;
    }
    return traces;
  }

  private getXWord(slot: number): number {
    return this.xGroups[this.selectedXGroup][slot] & X_MASK;
  }

  private setXWord(slot: number, value: number): void {
    this.xGroups[this.selectedXGroup][slot] = value & X_MASK;
  }

  private executeMemoryReference(
    mnemonic: string,
    modifier: number,
    effectiveOrRawAddress: number,
    rawAddress: number,
    pcBefore: number,
  ): void {
    const effectiveAddress = effectiveOrRawAddress % this.memorySize;

    if (mnemonic === "LDA") {
      this.m = this.readWord(effectiveAddress);
      this.a = this.m;
    } else if (mnemonic === "ADD") {
      this.m = this.readWord(effectiveAddress);
      const total = toSigned20(this.a) + toSigned20(this.m);
      this.a = fromSigned20(total);
      this.overflow = total < -(1 << 19) || total > ((1 << 19) - 1);
    } else if (mnemonic === "SUB") {
      this.m = this.readWord(effectiveAddress);
      const total = toSigned20(this.a) - toSigned20(this.m);
      this.a = fromSigned20(total);
      this.overflow = total < -(1 << 19) || total > ((1 << 19) - 1);
    } else if (mnemonic === "STA") {
      this.writeWord(effectiveAddress, this.a);
    } else if (mnemonic === "BXL") {
      if ((this.getXWord(modifier) & ADDR_MASK) >= rawAddress) this.pc = (this.pc + 1) % this.memorySize;
    } else if (mnemonic === "BXH") {
      if ((this.getXWord(modifier) & ADDR_MASK) < rawAddress) this.pc = (this.pc + 1) % this.memorySize;
    } else if (mnemonic === "LDX") {
      this.setXWord(modifier, this.readWord(rawAddress % this.memorySize));
    } else if (mnemonic === "SPB") {
      this.setXWord(modifier, pcBefore);
      this.pc = rawAddress % this.memorySize;
    } else if (mnemonic === "DLD") {
      const first = this.readWord(effectiveAddress);
      if ((effectiveAddress & 1) !== 0) {
        this.a = first;
        this.q = first;
      } else {
        this.a = first;
        this.q = this.readWord((effectiveAddress + 1) % this.memorySize);
      }
    } else if (mnemonic === "DAD") {
      const left = toSigned40(combineWords(this.a, this.q));
      const first = this.readWord(effectiveAddress);
      const second = (effectiveAddress & 1) !== 0 ? first : this.readWord((effectiveAddress + 1) % this.memorySize);
      const total = left + toSigned40(combineWords(first, second));
      [this.a, this.q] = splitSigned40(total);
      this.overflow = total < -(1n << 39n) || total > ((1n << 39n) - 1n);
    } else if (mnemonic === "DSU") {
      const left = toSigned40(combineWords(this.a, this.q));
      const first = this.readWord(effectiveAddress);
      const second = (effectiveAddress & 1) !== 0 ? first : this.readWord((effectiveAddress + 1) % this.memorySize);
      const total = left - toSigned40(combineWords(first, second));
      [this.a, this.q] = splitSigned40(total);
      this.overflow = total < -(1n << 39n) || total > ((1n << 39n) - 1n);
    } else if (mnemonic === "DST") {
      if ((effectiveAddress & 1) !== 0) {
        this.writeWord(effectiveAddress, this.q);
      } else {
        this.writeWord(effectiveAddress, this.a);
        this.writeWord((effectiveAddress + 1) % this.memorySize, this.q);
      }
    } else if (mnemonic === "INX") {
      this.setXWord(modifier, (this.getXWord(modifier) + rawAddress) & X_MASK);
    } else if (mnemonic === "MPY") {
      this.m = this.readWord(effectiveAddress);
      const product = BigInt(toSigned20(this.q)) * BigInt(toSigned20(this.m)) + BigInt(toSigned20(this.a));
      [this.a, this.q] = splitSigned40(product);
      this.overflow = product < -(1n << 39n) || product > ((1n << 39n) - 1n);
    } else if (mnemonic === "DVD") {
      this.m = this.readWord(effectiveAddress);
      const divisor = BigInt(toSigned20(this.m));
      if (divisor === 0n) throw new Error("GE-225 divide by zero");
      if (BigInt(Math.abs(toSigned20(this.a))) >= (divisor < 0n ? -divisor : divisor)) {
        this.overflow = true;
        return;
      }
      const dividend = toSigned40(combineWords(this.a, this.q));
      const absDividend = dividend < 0n ? -dividend : dividend;
      const absDivisor = divisor < 0n ? -divisor : divisor;
      const quotientMag = absDividend / absDivisor;
      const remainderMag = absDividend % absDivisor;
      const quotient = (dividend < 0n) !== (divisor < 0n) ? -quotientMag : quotientMag;
      const remainder = quotient < 0n ? -remainderMag : remainderMag;
      this.a = fromSigned20(Number(quotient));
      this.q = fromSigned20(Number(remainder));
      this.overflow = quotient < -(1n << 19n) || quotient > ((1n << 19n) - 1n);
    } else if (mnemonic === "STX") {
      this.writeWord(rawAddress % this.memorySize, this.getXWord(modifier));
    } else if (mnemonic === "EXT") {
      this.m = this.readWord(effectiveAddress);
      this.a &= (~this.m) & MASK_20;
    } else if (mnemonic === "CAB") {
      this.m = this.readWord(effectiveAddress);
      const relation = arithCompare(this.m, this.a);
      if (relation === 0) this.pc = (this.pc + 1) % this.memorySize;
      else if (relation < 0) this.pc = (this.pc + 2) % this.memorySize;
    } else if (mnemonic === "DCB") {
      const first = this.readWord(effectiveAddress);
      const second = (effectiveAddress & 1) !== 0 ? first : this.readWord((effectiveAddress + 1) % this.memorySize);
      const relation = arithCompareDouble(first, second, this.a, this.q);
      if (relation === 0) this.pc = (this.pc + 1) % this.memorySize;
      else if (relation < 0) this.pc = (this.pc + 2) % this.memorySize;
    } else if (mnemonic === "ORY") {
      this.writeWord(effectiveAddress, this.readWord(effectiveAddress) | this.a);
    } else if (mnemonic === "MOY") {
      const wordCount = Math.max(0, -toSigned20(this.q));
      const destination = this.a & X_MASK;
      for (let offset = 0; offset < wordCount; offset += 1) {
        this.writeWord((destination + offset) % this.memorySize, this.readWord((rawAddress + offset) % this.memorySize));
      }
      this.setXWord(0, this.pc);
      this.a = 0;
    } else if (mnemonic === "RCD") {
      const record = this.cardReaderQueue.shift();
      if (record === undefined) throw new Error("RCD executed with no queued card-reader record");
      record.forEach((word, offset) => this.writeWord((effectiveAddress + offset) % this.memorySize, word));
    } else if (mnemonic === "BRU") {
      this.pc = effectiveAddress;
    } else if (mnemonic === "STO") {
      const existing = this.readWord(effectiveAddress);
      this.writeWord(effectiveAddress, (existing & ~ADDR_MASK) | (this.a & ADDR_MASK));
    } else {
      throw new Error(`unimplemented GE-225 memory-reference instruction: ${mnemonic}`);
    }
  }

  private executeFixed(decoded: DecodedInstruction): void {
    const { mnemonic, count } = decoded;
    if (mnemonic === "OFF") {
      this.typewriterPower = false;
      this.nReady = true;
    } else if (mnemonic === "TYP") {
      if (!this.typewriterPower) {
        this.nReady = false;
        return;
      }
      const code = this.n & N_MASK;
      if (code === 0o37) this.typewriterOutput.push("\r");
      else if (code === 0o76) this.typewriterOutput.push("\t");
      else if (code !== 0o72 && code !== 0o75) {
        const char = TYPEWRITER_CODES.get(code);
        if (char === undefined) {
          this.nReady = false;
          return;
        }
        this.typewriterOutput.push(char);
      }
      this.nReady = true;
    } else if (mnemonic === "TON") {
      this.typewriterPower = true;
    } else if (mnemonic === "RCS") {
      this.a |= this.controlSwitches;
    } else if (mnemonic === "HPT") {
      this.nReady = false;
    } else if (mnemonic === "LDZ") {
      this.a = 0;
    } else if (mnemonic === "LDO") {
      this.a = 1;
    } else if (mnemonic === "LMO") {
      this.a = MASK_20;
    } else if (mnemonic === "CPL") {
      this.a = (~this.a) & MASK_20;
    } else if (mnemonic === "NEG") {
      const before = toSigned20(this.a);
      this.a = fromSigned20(-before);
      this.overflow = before === -(1 << 19);
    } else if (mnemonic === "CHS") {
      this.a ^= SIGN_BIT;
    } else if (mnemonic === "NOP") {
      return;
    } else if (mnemonic === "LAQ") {
      this.a = this.q;
    } else if (mnemonic === "LQA") {
      this.q = this.a;
    } else if (mnemonic === "XAQ") {
      [this.a, this.q] = [this.q, this.a];
    } else if (mnemonic === "MAQ") {
      this.q = this.a;
      this.a = 0;
    } else if (mnemonic === "ADO") {
      const total = toSigned20(this.a) + 1;
      this.a = fromSigned20(total);
      this.overflow = total < -(1 << 19) || total > ((1 << 19) - 1);
    } else if (mnemonic === "SBO") {
      const total = toSigned20(this.a) - 1;
      this.a = fromSigned20(total);
      this.overflow = total < -(1 << 19) || total > ((1 << 19) - 1);
    } else if (mnemonic === "SET_DECMODE") {
      this.decimalMode = true;
    } else if (mnemonic === "SET_BINMODE") {
      this.decimalMode = false;
    } else if (mnemonic === "SXG") {
      this.selectedXGroup = this.a & 0x1f;
    } else if (mnemonic === "SET_PST") {
      this.automaticInterruptMode = true;
    } else if (mnemonic === "SET_PBK") {
      this.automaticInterruptMode = false;
    } else if (["BOD", "BEV", "BMI", "BPL", "BZE", "BNZ", "BOV", "BNO", "BPE", "BPC", "BNR", "BNN"].includes(mnemonic)) {
      this.executeBranchTest(mnemonic);
    } else if (SHIFT_BASES.has(mnemonic)) {
      this.executeShift(mnemonic, count ?? 0);
    } else {
      throw new Error(`unimplemented GE-225 fixed instruction: ${mnemonic}`);
    }
  }

  private executeBranchTest(mnemonic: string): void {
    const cond =
      mnemonic === "BOD" ? (this.a & 1) !== 0 :
      mnemonic === "BEV" ? (this.a & 1) === 0 :
      mnemonic === "BMI" ? (this.a & SIGN_BIT) !== 0 :
      mnemonic === "BPL" ? (this.a & SIGN_BIT) === 0 :
      mnemonic === "BZE" ? this.a === 0 :
      mnemonic === "BNZ" ? this.a !== 0 :
      mnemonic === "BOV" ? this.overflow :
      mnemonic === "BNO" ? !this.overflow :
      mnemonic === "BPE" ? this.parityError :
      mnemonic === "BPC" ? !this.parityError :
      mnemonic === "BNR" ? this.nReady :
      !this.nReady;
    if (mnemonic === "BOV" || mnemonic === "BNO") this.overflow = false;
    if (mnemonic === "BPE" || mnemonic === "BPC") this.parityError = false;
    if (!cond) this.pc = (this.pc + 1) % this.memorySize;
  }

  private executeShift(mnemonic: string, count: number): void {
    if (count === 0) {
      if (mnemonic === "SRD") this.q = withSign(this.q, signOf(this.a));
      else if (mnemonic === "SLD") this.a = withSign(this.a, signOf(this.q));
      return;
    }

    const aSign = signOf(this.a);
    let aData = this.a & DATA_MASK;
    const qSign = signOf(this.q);
    let qData = this.q & DATA_MASK;

    if (mnemonic === "SRA") {
      this.a = fromSigned20(toSigned20(this.a) >> Math.min(count, 19));
    } else if (mnemonic === "SLA") {
      this.overflow = (aData >> Math.max(0, 19 - count)) !== 0;
      this.a = withSign((aData << count) & DATA_MASK, aSign);
    } else if (mnemonic === "SCA") {
      const rotation = count % 19;
      if (rotation !== 0) aData = ((aData >> rotation) | (aData << (19 - rotation))) & DATA_MASK;
      this.a = withSign(aData, aSign);
    } else if (mnemonic === "SAN") {
      const fill = aSign === 1 ? ((1 << count) - 1) : 0;
      let combined = ((aData & DATA_MASK) << 6) | (this.n & N_MASK);
      combined = ((fill << 25) | combined) >> count;
      this.a = withSign((combined >> 6) & DATA_MASK, aSign);
      this.n = combined & N_MASK;
    } else if (mnemonic === "SNA") {
      const combined = (((this.n & N_MASK) << 19) | aData) >>> count;
      this.n = (combined >> 19) & N_MASK;
      this.a = withSign(combined & DATA_MASK, aSign);
    } else if (mnemonic === "SRD") {
      const value = combineWords(this.a, this.q) >> BigInt(count);
      this.a = withSign(Number((value >> 20n) & BigInt(DATA_MASK)), aSign);
      this.q = withSign(Number(value & BigInt(DATA_MASK)), aSign);
    } else if (mnemonic === "NAQ") {
      const combined = (((this.n & N_MASK) << 38) | ((aData & DATA_MASK) << 19) | qData) >>> count;
      this.n = (combined >> 38) & N_MASK;
      this.a = withSign((combined >> 19) & DATA_MASK, aSign);
      this.q = withSign(combined & DATA_MASK, aSign);
    } else if (mnemonic === "SCD") {
      const rotation = count % 38;
      let combined = ((aData & DATA_MASK) << 19) | qData;
      if (rotation !== 0) combined = ((combined >> rotation) | (combined << (38 - rotation))) & ((1 << 38) - 1);
      this.a = withSign((combined >> 19) & DATA_MASK, aSign);
      this.q = withSign(combined & DATA_MASK, aSign);
    } else if (mnemonic === "ANQ") {
      for (let i = 0; i < count; i += 1) {
        const bit = this.a & 1;
        this.a = fromSigned20(toSigned20(this.a) >> 1);
        qData = ((bit << 18) | ((this.q & DATA_MASK) >> 1)) & DATA_MASK;
        this.q = withSign(qData, aSign);
        this.n = ((bit << 5) | (this.n >> 1)) & N_MASK;
      }
    } else if (mnemonic === "SLD") {
      let combined = ((aData & DATA_MASK) << 19) | qData;
      this.overflow = (combined >> Math.max(0, 38 - count)) !== 0;
      combined = (combined << count) & ((1 << 38) - 1);
      this.a = withSign((combined >> 19) & DATA_MASK, qSign);
      this.q = withSign(combined & DATA_MASK, qSign);
    } else if (mnemonic === "NOR") {
      let shifts = 0;
      const targetBit = aSign === 0 ? 0 : 1;
      while (shifts < count) {
        const lead = (aData >> 18) & 1;
        if (lead !== targetBit) break;
        this.overflow = this.overflow || lead === 1;
        aData = (aData << 1) & DATA_MASK;
        shifts += 1;
      }
      this.a = withSign(aData, aSign);
      this.setXWord(0, count - shifts);
    } else if (mnemonic === "DNO") {
      let shifts = 0;
      const targetBit = aSign === 0 ? 0 : 1;
      let combined = ((aData & DATA_MASK) << 19) | qData;
      while (shifts < count) {
        const lead = (combined >> 37) & 1;
        if (lead !== targetBit) break;
        this.overflow = this.overflow || lead === 1;
        combined = (combined << 1) & ((1 << 38) - 1);
        shifts += 1;
      }
      this.a = withSign((combined >> 19) & DATA_MASK, qSign);
      this.q = withSign(combined & DATA_MASK, qSign);
      this.setXWord(0, count - shifts);
    }
  }

  private decodeWord(word: number): DecodedInstruction {
    const normalized = word & MASK_20;
    const fixedName = FIXED_NAMES.get(normalized);
    if (fixedName !== undefined) return { mnemonic: fixedName, opcode: null, modifier: null, address: null, count: null, fixedWord: true };

    for (const [mnemonic, base] of SHIFT_BASES.entries()) {
      if ((normalized & ~0o37) === base) {
        return { mnemonic, opcode: null, modifier: null, address: null, count: normalized & 0o37, fixedWord: true };
      }
    }

    const [opcode, modifier, address] = decodeInstruction(normalized);
    const mnemonic = BASE_OPCODE_NAMES.get(opcode);
    if (mnemonic === undefined) throw new Error(`unknown GE-225 opcode field ${opcode.toString(8)}`);
    return { mnemonic, opcode, modifier, address, count: null, fixedWord: false };
  }

  private resolveEffectiveAddress(address: number, modifier: number): number {
    const base = address % this.memorySize;
    if (modifier === 0) return base;
    return (base + (this.getXWord(modifier) % this.memorySize)) % this.memorySize;
  }

  private checkAddress(address: number): void {
    if (address < 0 || address >= this.memorySize) throw new Error(`address out of range: ${address}`);
  }
}
