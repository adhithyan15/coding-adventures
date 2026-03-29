import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { CanvasTable } from "./CanvasTable.js";
import type { ColumnDef } from "./Table.js";

// ---------------------------------------------------------------------------
// Test data
// ---------------------------------------------------------------------------

interface Person {
  name: string;
  age: number;
}

const columns: ColumnDef<Person>[] = [
  { id: "name", header: "Name", accessor: "name", width: 200 },
  { id: "age", header: "Age", accessor: "age", width: 100, align: "right" },
];

const data: Person[] = [
  { name: "Alice", age: 30 },
  { name: "Bob", age: 25 },
];

// ---------------------------------------------------------------------------
// Canvas 2D context mock
//
// jsdom does not implement the Canvas API. We mock getContext("2d") to return
// a spy object so we can verify that the rendering pipeline calls the
// expected drawing methods.
// ---------------------------------------------------------------------------

function createMockContext() {
  return {
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
    textAlign: "left" as CanvasTextAlign,
  };
}

beforeEach(() => {
  // Mock getContext on all canvas elements
  const original = HTMLCanvasElement.prototype.getContext;
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
    function (this: HTMLCanvasElement, contextId: string, ...args: unknown[]) {
      if (contextId === "2d") {
        return createMockContext() as unknown as CanvasRenderingContext2D;
      }
      return original.call(this, contextId, ...(args as []));
    },
  );
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("CanvasTable", () => {
  describe("rendering", () => {
    it("renders a canvas element", () => {
      const { container } = render(
        <CanvasTable columns={columns} data={data} />,
      );
      expect(container.querySelector("canvas")).toBeTruthy();
    });

    it("canvas is aria-hidden", () => {
      const { container } = render(
        <CanvasTable columns={columns} data={data} />,
      );
      expect(
        container.querySelector("canvas")!.getAttribute("aria-hidden"),
      ).toBe("true");
    });

    it("renders a grid role on the container", () => {
      render(
        <CanvasTable columns={columns} data={data} ariaLabel="People" />,
      );
      const grid = screen.getByRole("grid");
      expect(grid).toBeTruthy();
      expect(grid.getAttribute("aria-label")).toBe("People");
    });
  });

  describe("ARIA grid overlay", () => {
    it("renders columnheader roles for each column", () => {
      render(<CanvasTable columns={columns} data={data} />);
      const headers = screen.getAllByRole("columnheader");
      expect(headers).toHaveLength(2);
      expect(headers[0]!.textContent).toBe("Name");
      expect(headers[1]!.textContent).toBe("Age");
    });

    it("renders gridcell roles for each data cell", () => {
      render(<CanvasTable columns={columns} data={data} />);
      const cells = screen.getAllByRole("gridcell");
      // 2 rows * 2 columns = 4 cells
      expect(cells).toHaveLength(4);
    });

    it("gridcells contain correct text content", () => {
      render(<CanvasTable columns={columns} data={data} />);
      const cells = screen.getAllByRole("gridcell");
      expect(cells[0]!.textContent).toBe("Alice");
      expect(cells[1]!.textContent).toBe("30");
      expect(cells[2]!.textContent).toBe("Bob");
      expect(cells[3]!.textContent).toBe("25");
    });

    it("sets aria-colindex on each cell", () => {
      render(<CanvasTable columns={columns} data={data} />);
      const headers = screen.getAllByRole("columnheader");
      expect(headers[0]!.getAttribute("aria-colindex")).toBe("1");
      expect(headers[1]!.getAttribute("aria-colindex")).toBe("2");

      const cells = screen.getAllByRole("gridcell");
      expect(cells[0]!.getAttribute("aria-colindex")).toBe("1");
      expect(cells[1]!.getAttribute("aria-colindex")).toBe("2");
    });

    it("sets aria-rowindex on each row", () => {
      render(<CanvasTable columns={columns} data={data} />);
      const rows = screen.getAllByRole("row");
      // Header row is rowindex 1, data rows are 2 and 3
      expect(rows[0]!.getAttribute("aria-rowindex")).toBe("1");
      expect(rows[1]!.getAttribute("aria-rowindex")).toBe("2");
      expect(rows[2]!.getAttribute("aria-rowindex")).toBe("3");
    });

    it("sets aria-rowcount and aria-colcount on the grid", () => {
      render(<CanvasTable columns={columns} data={data} />);
      const grid = screen.getByRole("grid");
      expect(grid.getAttribute("aria-rowcount")).toBe("3"); // 1 header + 2 data
      expect(grid.getAttribute("aria-colcount")).toBe("2");
    });

    it("renders row group elements", () => {
      render(<CanvasTable columns={columns} data={data} />);
      const rowgroups = screen.getAllByRole("rowgroup");
      expect(rowgroups).toHaveLength(2); // header group + body group
    });
  });

  describe("keyboard navigation", () => {
    it("ArrowRight moves focus to next cell", () => {
      render(<CanvasTable columns={columns} data={data} />);
      const grid = screen.getByRole("grid");

      // Focus the first cell
      const firstHeader = screen.getAllByRole("columnheader")[0]!;
      firstHeader.focus();

      // Press ArrowRight
      fireEvent.keyDown(grid, { key: "ArrowRight" });

      // The second header should now have tabIndex=0
      const headers = screen.getAllByRole("columnheader");
      expect(headers[1]!.tabIndex).toBe(0);
    });

    it("ArrowDown moves focus to cell below", () => {
      render(<CanvasTable columns={columns} data={data} />);
      const grid = screen.getByRole("grid");

      // Focus the first header
      const firstHeader = screen.getAllByRole("columnheader")[0]!;
      firstHeader.focus();

      // Press ArrowDown
      fireEvent.keyDown(grid, { key: "ArrowDown" });

      // The first body cell should now have tabIndex=0
      const cells = screen.getAllByRole("gridcell");
      expect(cells[0]!.tabIndex).toBe(0);
    });

    it("Home moves focus to first cell in row", () => {
      render(<CanvasTable columns={columns} data={data} />);
      const grid = screen.getByRole("grid");

      // Focus the second header
      const secondHeader = screen.getAllByRole("columnheader")[1]!;
      secondHeader.focus();
      fireEvent.keyDown(grid, { key: "ArrowRight" });

      // Press Home
      fireEvent.keyDown(grid, { key: "Home" });

      // First header should have tabIndex=0
      const headers = screen.getAllByRole("columnheader");
      expect(headers[0]!.tabIndex).toBe(0);
    });

    it("End moves focus to last cell in row", () => {
      render(<CanvasTable columns={columns} data={data} />);
      const grid = screen.getByRole("grid");

      // Focus the first header
      const firstHeader = screen.getAllByRole("columnheader")[0]!;
      firstHeader.focus();

      // Press End
      fireEvent.keyDown(grid, { key: "End" });

      // Last header should have tabIndex=0
      const headers = screen.getAllByRole("columnheader");
      expect(headers[1]!.tabIndex).toBe(0);
    });
  });

  describe("edge cases", () => {
    it("handles empty data array", () => {
      render(<CanvasTable columns={columns} data={[]} />);
      const grid = screen.getByRole("grid");
      expect(grid.getAttribute("aria-rowcount")).toBe("1"); // header only
      expect(screen.queryAllByRole("gridcell")).toHaveLength(0);
    });

    it("renders caption when provided", () => {
      render(
        <CanvasTable
          columns={columns}
          data={data}
          caption="People Table"
        />,
      );
      expect(screen.getByText("People Table")).toBeTruthy();
    });

    it("handles function accessor in overlay cells", () => {
      const fnColumns: ColumnDef<Person>[] = [
        {
          id: "greeting",
          header: "Greeting",
          accessor: (row) => `Hi ${row.name}`,
          width: 200,
        },
      ];
      render(<CanvasTable columns={fnColumns} data={data} />);
      const cells = screen.getAllByRole("gridcell");
      expect(cells[0]!.textContent).toBe("Hi Alice");
      expect(cells[1]!.textContent).toBe("Hi Bob");
    });

    it("applies custom className alongside table--canvas", () => {
      const { container } = render(
        <CanvasTable
          columns={columns}
          data={data}
          className="custom-table"
        />,
      );
      const el = container.querySelector(".custom-table");
      expect(el).toBeTruthy();
      expect(el!.classList.contains("table--canvas")).toBe(true);
    });
  });
});
