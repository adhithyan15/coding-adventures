export type GCValue =
  | number
  | string
  | boolean
  | null
  | undefined
  | GCValue[]
  | { [key: string]: GCValue };

export interface GCStats {
  totalAllocations: number;
  totalCollections: number;
  totalFreed: number;
  heapSize: number;
}

export abstract class HeapObject {
  marked = false;

  references(): number[] {
    return [];
  }
}

export class ConsCell extends HeapObject {
  constructor(
    public car: GCValue = null,
    public cdr: GCValue = null,
  ) {
    super();
  }

  references(): number[] {
    return [this.car, this.cdr].filter(isAddressCandidate);
  }
}

export class Symbol extends HeapObject {
  constructor(public name: string = "") {
    super();
  }
}

export class LispClosure extends HeapObject {
  constructor(
    public code: unknown = null,
    public env: Record<string, GCValue> = {},
    public params: string[] = [],
  ) {
    super();
  }

  references(): number[] {
    return Object.values(this.env).filter(isAddressCandidate);
  }
}

export abstract class GarbageCollector {
  abstract allocate(obj: HeapObject): number;
  abstract deref(address: number): HeapObject;
  abstract collect(roots: GCValue[]): number;
  abstract heapSize(): number;
  abstract stats(): GCStats;

  isValidAddress(address: number): boolean {
    try {
      this.deref(address);
      return true;
    } catch {
      return false;
    }
  }
}

export class MarkAndSweepGC extends GarbageCollector {
  private readonly heap = new Map<number, HeapObject>();
  private nextAddress = 0x10000;
  private totalAllocations = 0;
  private totalCollections = 0;
  private totalFreed = 0;

  allocate(obj: HeapObject): number {
    const address = this.nextAddress;
    this.nextAddress += 1;
    this.heap.set(address, obj);
    this.totalAllocations += 1;
    return address;
  }

  deref(address: number): HeapObject {
    const obj = this.heap.get(address);
    if (obj === undefined) {
      throw new RangeError(`Invalid heap address: ${address}`);
    }
    return obj;
  }

  collect(roots: GCValue[]): number {
    this.totalCollections += 1;

    for (const root of roots) {
      this.markValue(root);
    }

    const toDelete: number[] = [];
    for (const [address, obj] of this.heap) {
      if (obj.marked) {
        obj.marked = false;
      } else {
        toDelete.push(address);
      }
    }

    for (const address of toDelete) {
      this.heap.delete(address);
    }

    this.totalFreed += toDelete.length;
    return toDelete.length;
  }

  heapSize(): number {
    return this.heap.size;
  }

  stats(): GCStats {
    return {
      totalAllocations: this.totalAllocations,
      totalCollections: this.totalCollections,
      totalFreed: this.totalFreed,
      heapSize: this.heapSize(),
    };
  }

  isValidAddress(address: number): boolean {
    return this.heap.has(address);
  }

  private markValue(value: GCValue): void {
    if (isAddressCandidate(value)) {
      const obj = this.heap.get(value);
      if (obj !== undefined && !obj.marked) {
        obj.marked = true;
        for (const ref of obj.references()) {
          this.markValue(ref);
        }
      }
      return;
    }

    if (Array.isArray(value)) {
      for (const item of value) {
        this.markValue(item);
      }
      return;
    }

    if (isRecord(value)) {
      for (const item of Object.values(value)) {
        this.markValue(item);
      }
    }
  }
}

export class SymbolTable {
  private readonly table = new Map<string, number>();

  constructor(private readonly gc: GarbageCollector) {}

  intern(name: string): number {
    const existing = this.table.get(name);
    if (existing !== undefined && this.gc.isValidAddress(existing)) {
      return existing;
    }

    const address = this.gc.allocate(new Symbol(name));
    this.table.set(name, address);
    return address;
  }

  lookup(name: string): number | undefined {
    const address = this.table.get(name);
    if (address !== undefined && this.gc.isValidAddress(address)) {
      return address;
    }
    return undefined;
  }

  allSymbols(): Record<string, number> {
    const symbols: Record<string, number> = {};
    for (const [name, address] of this.table) {
      if (this.gc.isValidAddress(address)) {
        symbols[name] = address;
      }
    }
    return symbols;
  }
}

function isAddressCandidate(value: GCValue): value is number {
  return typeof value === "number" && Number.isInteger(value);
}

function isRecord(value: GCValue): value is { [key: string]: GCValue } {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}
