import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { Table } from "./Table.js";
import { resolveCellValue } from "./Table.js";
import type { ColumnDef } from "./Table.js";

// ---------------------------------------------------------------------------
// Test data
// ---------------------------------------------------------------------------

interface Fruit {
  name: string;
  count: number;
}

const columns: ColumnDef<Fruit>[] = [
  { id: "name", header: "Fruit", accessor: "name" },
  { id: "count", header: "Count", accessor: "count" },
];

const data: Fruit[] = [
  { name: "Apple", count: 5 },
  { name: "Banana", count: 3 },
];

// Mock canvas context for the canvas renderer tests
beforeEach(() => {
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
    () =>
      ({
        clearRect: vi.fn(),
        fillRect: vi.fn(),
        fillText: vi.fn(),
        beginPath: vi.fn(),
        moveTo: vi.fn(),
        lineTo: vi.fn(),
        stroke: vi.fn(),
        rect: vi.fn(),
        clip: vi.fn(),
        save: vi.fn(),
        restore: vi.fn(),
        setTransform: vi.fn(),
        fillStyle: "",
        strokeStyle: "",
        lineWidth: 1,
        font: "",
        textAlign: "left",
      }) as unknown as CanvasRenderingContext2D,
  );
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Table", () => {
  describe("renderer routing", () => {
    it("defaults to HTML backend (renders a <table>)", () => {
      render(<Table columns={columns} data={data} />);
      expect(screen.getByRole("table")).toBeTruthy();
    });

    it("renders HTML backend when renderer='html'", () => {
      render(<Table columns={columns} data={data} renderer="html" />);
      expect(screen.getByRole("table")).toBeTruthy();
    });

    it("renders Canvas backend when renderer='canvas'", () => {
      render(<Table columns={columns} data={data} renderer="canvas" />);
      expect(screen.getByRole("grid")).toBeTruthy();
    });
  });

  describe("props pass-through", () => {
    it("passes ariaLabel to HTML backend", () => {
      render(
        <Table columns={columns} data={data} ariaLabel="Fruits" />,
      );
      expect(
        screen.getByRole("region").getAttribute("aria-label"),
      ).toBe("Fruits");
    });

    it("passes ariaLabel to Canvas backend", () => {
      render(
        <Table
          columns={columns}
          data={data}
          renderer="canvas"
          ariaLabel="Fruits"
        />,
      );
      expect(
        screen.getByRole("grid").getAttribute("aria-label"),
      ).toBe("Fruits");
    });

    it("passes data to HTML backend", () => {
      render(<Table columns={columns} data={data} />);
      expect(screen.getByText("Apple")).toBeTruthy();
      expect(screen.getByText("Banana")).toBeTruthy();
    });

    it("passes data to Canvas backend overlay", () => {
      render(<Table columns={columns} data={data} renderer="canvas" />);
      const cells = screen.getAllByRole("gridcell");
      expect(cells[0]!.textContent).toBe("Apple");
    });
  });
});

describe("resolveCellValue", () => {
  it("resolves a string-key accessor", () => {
    const col: ColumnDef<Fruit> = {
      id: "name",
      header: "Name",
      accessor: "name",
    };
    expect(resolveCellValue(col, { name: "Mango", count: 7 }, 0)).toBe(
      "Mango",
    );
  });

  it("resolves a function accessor", () => {
    const col: ColumnDef<Fruit> = {
      id: "desc",
      header: "Desc",
      accessor: (row) => `${row.count}x ${row.name}`,
    };
    expect(
      resolveCellValue(col, { name: "Kiwi", count: 2 }, 0),
    ).toBe("2x Kiwi");
  });

  it("converts null to empty string", () => {
    interface MaybeNull {
      val: string | null;
    }
    const col: ColumnDef<MaybeNull> = {
      id: "val",
      header: "Val",
      accessor: "val",
    };
    expect(resolveCellValue(col, { val: null }, 0)).toBe("");
  });

  it("converts undefined to empty string", () => {
    interface MaybeUndef {
      val?: string;
    }
    const col: ColumnDef<MaybeUndef> = {
      id: "val",
      header: "Val",
      accessor: "val",
    };
    expect(resolveCellValue(col, {}, 0)).toBe("");
  });

  it("converts numbers to strings", () => {
    const col: ColumnDef<Fruit> = {
      id: "count",
      header: "Count",
      accessor: "count",
    };
    expect(resolveCellValue(col, { name: "Fig", count: 42 }, 0)).toBe(
      "42",
    );
  });

  it("passes rowIndex to function accessor", () => {
    const col: ColumnDef<Fruit> = {
      id: "idx",
      header: "#",
      accessor: (_row, i) => i + 1,
    };
    expect(
      resolveCellValue(col, { name: "Grape", count: 1 }, 5),
    ).toBe("6");
  });
});
