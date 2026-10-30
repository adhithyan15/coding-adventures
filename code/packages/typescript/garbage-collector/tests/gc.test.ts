import assert from "node:assert/strict";
import { describe, it } from "node:test";
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

    assert.equal(first, 0x10000);
    assert.equal(second, 0x10001);
    assert.equal(gc.heapSize(), 2);
    assert.ok(gc.deref(first) instanceof ConsCell);
    assert.equal(gc.isValidAddress(second), true);
  });

  it("throws when dereferencing an invalid or freed object", () => {
    const gc = new MarkAndSweepGC();
    const address = gc.allocate(new Symbol("gone"));

    assert.ok(gc.deref(address) instanceof Symbol);
    assert.equal(gc.collect([]), 1);
    assert.throws(() => gc.deref(address), RangeError);
    assert.equal(gc.isValidAddress(address), false);
  });

  it("keeps a root and its transitive references alive", () => {
    const gc = new MarkAndSweepGC();
    const tail = gc.allocate(new Symbol("tail"));
    const middle = gc.allocate(new ConsCell(tail, null));
    const head = gc.allocate(new ConsCell(1, middle));

    const freed = gc.collect([head]);

    assert.equal(freed, 0);
    assert.equal(gc.heapSize(), 3);
    assert.equal(gc.isValidAddress(tail), true);
  });

  it("collects unreachable cycles", () => {
    const gc = new MarkAndSweepGC();
    const left = new ConsCell();
    const right = new ConsCell();
    const leftAddress = gc.allocate(left);
    const rightAddress = gc.allocate(right);
    left.cdr = rightAddress;
    right.cdr = leftAddress;

    assert.equal(gc.collect([]), 2);
    assert.equal(gc.heapSize(), 0);
  });

  it("scans nested root arrays and objects", () => {
    const gc = new MarkAndSweepGC();
    const fromArray = gc.allocate(new Symbol("array-root"));
    const fromObject = gc.allocate(new Symbol("object-root"));
    const unreachable = gc.allocate(new Symbol("unreachable"));

    const freed = gc.collect([[fromArray], { global: fromObject, literal: 42 }]);

    assert.equal(freed, 1);
    assert.equal(gc.isValidAddress(fromArray), true);
    assert.equal(gc.isValidAddress(fromObject), true);
    assert.equal(gc.isValidAddress(unreachable), false);
  });

  it("tracks collection statistics", () => {
    const gc = new MarkAndSweepGC();
    const root = gc.allocate(new Symbol("root"));
    gc.allocate(new Symbol("temp"));

    assert.equal(gc.collect([root]), 1);

    assert.deepEqual(gc.stats(), {
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

    assert.deepEqual(new ConsCell(0x10000, "tail").references(), [0x10000]);
    assert.deepEqual(new Symbol("plain").references(), []);
    assert.deepEqual(closure.references(), [0x10000]);
    assert.equal(closure.code, "lambda");
    assert.deepEqual(closure.params, ["arg"]);
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
    assert.equal(gc.isValidAddress(1), true);
    assert.equal(gc.isValidAddress(2), false);
  });
});

describe("SymbolTable", () => {
  it("interns equal names to the same live address", () => {
    const gc = new MarkAndSweepGC();
    const table = new SymbolTable(gc);

    const first = table.intern("foo");
    const second = table.intern("foo");
    const other = table.intern("bar");

    assert.equal(first, second);
    assert.notEqual(first, other);
    assert.equal(table.lookup("foo"), first);
    assert.deepEqual(table.allSymbols(), { foo: first, bar: other });
  });

  it("reallocates interned symbols after their heap object is collected", () => {
    const gc = new MarkAndSweepGC();
    const table = new SymbolTable(gc);
    const original = table.intern("foo");

    assert.equal(gc.collect([]), 1);
    assert.equal(table.lookup("foo"), undefined);

    const fresh = table.intern("foo");
    assert.notEqual(fresh, original);
    assert.deepEqual(table.allSymbols(), { foo: fresh });
  });
});
