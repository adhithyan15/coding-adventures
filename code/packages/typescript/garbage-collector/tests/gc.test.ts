import { describe, expect, it } from "vitest";
import {
  ConsCell,
  GarbageCollector,
  HeapObject,
  LispClosure,
  MarkAndSweepGC,
  Symbol,
  SymbolTable,
} from "../src/index.js";

describe("MarkAndSweepGC", () => {
  it("allocates objects at monotonically increasing heap addresses", () => {
    const gc = new MarkAndSweepGC();

    const first = gc.allocate(new ConsCell(42, null));
    const second = gc.allocate(new Symbol("next"));

    expect(first).toBe(0x10000);
    expect(second).toBe(0x10001);
    expect(gc.heapSize()).toBe(2);
    expect(gc.deref(first)).toBeInstanceOf(ConsCell);
    expect(gc.isValidAddress(second)).toBe(true);
  });

  it("throws when dereferencing an invalid or freed object", () => {
    const gc = new MarkAndSweepGC();
    const address = gc.allocate(new Symbol("gone"));

    expect(gc.deref(address)).toBeInstanceOf(Symbol);
    expect(gc.collect([])).toBe(1);
    expect(() => gc.deref(address)).toThrow(RangeError);
    expect(gc.isValidAddress(address)).toBe(false);
  });

  it("keeps a root and its transitive references alive", () => {
    const gc = new MarkAndSweepGC();
    const tail = gc.allocate(new Symbol("tail"));
    const middle = gc.allocate(new ConsCell(tail, null));
    const head = gc.allocate(new ConsCell(1, middle));

    const freed = gc.collect([head]);

    expect(freed).toBe(0);
    expect(gc.heapSize()).toBe(3);
    expect(gc.isValidAddress(tail)).toBe(true);
  });

  it("collects unreachable cycles", () => {
    const gc = new MarkAndSweepGC();
    const left = new ConsCell();
    const right = new ConsCell();
    const leftAddress = gc.allocate(left);
    const rightAddress = gc.allocate(right);
    left.cdr = rightAddress;
    right.cdr = leftAddress;

    expect(gc.collect([])).toBe(2);
    expect(gc.heapSize()).toBe(0);
  });

  it("scans nested root arrays and objects", () => {
    const gc = new MarkAndSweepGC();
    const fromArray = gc.allocate(new Symbol("array-root"));
    const fromObject = gc.allocate(new Symbol("object-root"));
    const unreachable = gc.allocate(new Symbol("unreachable"));

    const freed = gc.collect([[fromArray], { global: fromObject, literal: 42 }]);

    expect(freed).toBe(1);
    expect(gc.isValidAddress(fromArray)).toBe(true);
    expect(gc.isValidAddress(fromObject)).toBe(true);
    expect(gc.isValidAddress(unreachable)).toBe(false);
  });

  it("tracks collection statistics", () => {
    const gc = new MarkAndSweepGC();
    const root = gc.allocate(new Symbol("root"));
    gc.allocate(new Symbol("temp"));

    expect(gc.collect([root])).toBe(1);

    expect(gc.stats()).toEqual({
      totalAllocations: 2,
      totalCollections: 1,
      totalFreed: 1,
      heapSize: 1,
    });
  });
});

describe("heap object types", () => {
  it("reports references from cons cells and closures", () => {
    const closure = new LispClosure("lambda", { x: 0x10000, y: "plain", z: 17.5 }, ["arg"]);

    expect(new ConsCell(0x10000, "tail").references()).toEqual([0x10000]);
    expect(new Symbol("plain").references()).toEqual([]);
    expect(closure.references()).toEqual([0x10000]);
    expect(closure.code).toBe("lambda");
    expect(closure.params).toEqual(["arg"]);
  });

  it("allows custom collectors to use the abstract validity helper", () => {
    class TinyCollector extends GarbageCollector {
      private readonly obj = new Symbol("one");

      allocate(_obj: HeapObject): number {
        return 1;
      }

      deref(address: number): HeapObject {
        if (address === 1) {
          return this.obj;
        }
        throw new RangeError("missing");
      }

      collect(_roots: number[]): number {
        return 0;
      }

      heapSize(): number {
        return 1;
      }

      stats() {
        return { totalAllocations: 1, totalCollections: 0, totalFreed: 0, heapSize: 1 };
      }
    }

    const gc = new TinyCollector();
    expect(gc.isValidAddress(1)).toBe(true);
    expect(gc.isValidAddress(2)).toBe(false);
  });
});

describe("SymbolTable", () => {
  it("interns equal names to the same live address", () => {
    const gc = new MarkAndSweepGC();
    const table = new SymbolTable(gc);

    const first = table.intern("foo");
    const second = table.intern("foo");
    const other = table.intern("bar");

    expect(first).toBe(second);
    expect(first).not.toBe(other);
    expect(table.lookup("foo")).toBe(first);
    expect(table.allSymbols()).toEqual({ foo: first, bar: other });
  });

  it("reallocates interned symbols after their heap object is collected", () => {
    const gc = new MarkAndSweepGC();
    const table = new SymbolTable(gc);
    const original = table.intern("foo");

    expect(gc.collect([])).toBe(1);
    expect(table.lookup("foo")).toBeUndefined();

    const fresh = table.intern("foo");
    expect(fresh).not.toBe(original);
    expect(table.allSymbols()).toEqual({ foo: fresh });
  });
});
