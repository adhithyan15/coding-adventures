import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { DataTable } from "./DataTable.js";
import type { ColumnDef } from "./Table.js";

// ---------------------------------------------------------------------------
// Test data
// ---------------------------------------------------------------------------

interface Person {
  name: string;
  age: number;
}

const columns: ColumnDef<Person>[] = [
  { id: "name", header: "Name", accessor: "name" },
  { id: "age", header: "Age", accessor: "age", align: "right" },
];

const data: Person[] = [
  { name: "Alice", age: 30 },
  { name: "Bob", age: 25 },
  { name: "Carol", age: 35 },
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("DataTable", () => {
  describe("rendering", () => {
    it("renders a table element", () => {
      render(<DataTable columns={columns} data={data} />);
      expect(screen.getByRole("table")).toBeTruthy();
    });

    it("renders a region wrapper with aria-label", () => {
      render(
        <DataTable columns={columns} data={data} ariaLabel="People" />,
      );
      expect(screen.getByRole("region")).toBeTruthy();
      expect(
        screen.getByRole("region").getAttribute("aria-label"),
      ).toBe("People");
    });

    it("renders column headers with columnheader role", () => {
      render(<DataTable columns={columns} data={data} />);
      const headers = screen.getAllByRole("columnheader");
      expect(headers).toHaveLength(2);
      expect(headers[0]!.textContent).toBe("Name");
      expect(headers[1]!.textContent).toBe("Age");
    });

    it("renders the correct number of body rows", () => {
      render(<DataTable columns={columns} data={data} />);
      // getAllByRole("row") includes header row + body rows
      const rows = screen.getAllByRole("row");
      expect(rows).toHaveLength(4); // 1 header + 3 data
    });

    it("renders cell values from string-key accessor", () => {
      render(<DataTable columns={columns} data={data} />);
      expect(screen.getByText("Alice")).toBeTruthy();
      expect(screen.getByText("Bob")).toBeTruthy();
      expect(screen.getByText("Carol")).toBeTruthy();
    });

    it("renders cell values from function accessor", () => {
      const fnColumns: ColumnDef<Person>[] = [
        {
          id: "greeting",
          header: "Greeting",
          accessor: (row) => `Hello, ${row.name}!`,
        },
      ];
      render(<DataTable columns={fnColumns} data={data} />);
      expect(screen.getByText("Hello, Alice!")).toBeTruthy();
      expect(screen.getByText("Hello, Bob!")).toBeTruthy();
    });

    it("renders caption when provided", () => {
      render(
        <DataTable columns={columns} data={data} caption="People Table" />,
      );
      expect(screen.getByText("People Table")).toBeTruthy();
    });

    it("does not render caption when not provided", () => {
      const { container } = render(
        <DataTable columns={columns} data={data} />,
      );
      expect(container.querySelector(".table__caption")).toBeNull();
    });

    it("renders empty tbody when data is empty", () => {
      render(<DataTable columns={columns} data={[]} />);
      // Only the header row
      const rows = screen.getAllByRole("row");
      expect(rows).toHaveLength(1);
    });
  });

  describe("accessibility", () => {
    it("sets scope=col on header cells", () => {
      render(<DataTable columns={columns} data={data} />);
      const headers = screen.getAllByRole("columnheader");
      for (const header of headers) {
        expect(header.getAttribute("scope")).toBe("col");
      }
    });

    it("region wrapper has tabIndex=0 for keyboard scrolling", () => {
      render(<DataTable columns={columns} data={data} />);
      expect(screen.getByRole("region").tabIndex).toBe(0);
    });
  });

  describe("styling", () => {
    it("applies alignment BEM modifier classes", () => {
      const { container } = render(
        <DataTable columns={columns} data={data} />,
      );
      // Header cells
      const headerCells = container.querySelectorAll(".table__cell--header");
      expect(headerCells[0]!.classList.contains("table__cell--align-left")).toBe(
        true,
      );
      expect(
        headerCells[1]!.classList.contains("table__cell--align-right"),
      ).toBe(true);

      // Body cells — first row
      const bodyCells = container.querySelectorAll(
        ".table__body .table__cell",
      );
      expect(bodyCells[0]!.classList.contains("table__cell--align-left")).toBe(
        true,
      );
      expect(
        bodyCells[1]!.classList.contains("table__cell--align-right"),
      ).toBe(true);
    });

    it("applies column width as inline style on header cells", () => {
      const widthColumns: ColumnDef<Person>[] = [
        { id: "name", header: "Name", accessor: "name", width: "200px" },
        { id: "age", header: "Age", accessor: "age", width: 100 },
      ];
      render(<DataTable columns={widthColumns} data={data} />);
      const headers = screen.getAllByRole("columnheader");
      expect((headers[0] as HTMLElement).style.width).toBe("200px");
      expect((headers[1] as HTMLElement).style.width).toBe("100px");
    });

    it("does not set width style when column width is undefined", () => {
      render(<DataTable columns={columns} data={data} />);
      const headers = screen.getAllByRole("columnheader");
      expect((headers[0] as HTMLElement).style.width).toBe("");
    });

    it("applies custom className", () => {
      const { container } = render(
        <DataTable columns={columns} data={data} className="custom-table" />,
      );
      expect(container.querySelector(".custom-table")).toBeTruthy();
    });

    it("applies default className 'table'", () => {
      const { container } = render(
        <DataTable columns={columns} data={data} />,
      );
      expect(container.querySelector(".table")).toBeTruthy();
    });
  });

  describe("row keys", () => {
    it("uses custom rowKey function when provided", () => {
      const { container } = render(
        <DataTable
          columns={columns}
          data={data}
          rowKey={(row) => row.name}
        />,
      );
      // Verify rows render (the key is internal to React, but we can
      // verify the component doesn't crash with a custom key function)
      const rows = container.querySelectorAll(".table__body .table__row");
      expect(rows).toHaveLength(3);
    });
  });

  describe("edge cases", () => {
    it("handles null/undefined cell values gracefully", () => {
      interface MaybeData {
        val: string | null;
      }
      const cols: ColumnDef<MaybeData>[] = [
        { id: "val", header: "Value", accessor: "val" },
      ];
      const rows: MaybeData[] = [{ val: null }];
      render(<DataTable columns={cols} data={rows} />);
      // Should render empty string, not "null"
      const cells = screen.getAllByRole("cell");
      expect(cells[0]!.textContent).toBe("");
    });

    it("passes rowIndex to function accessor", () => {
      const indexColumns: ColumnDef<Person>[] = [
        {
          id: "index",
          header: "#",
          accessor: (_row, i) => i + 1,
        },
      ];
      render(<DataTable columns={indexColumns} data={data} />);
      expect(screen.getByText("1")).toBeTruthy();
      expect(screen.getByText("2")).toBeTruthy();
      expect(screen.getByText("3")).toBeTruthy();
    });
  });

  describe("column resizing", () => {
    it("does not render resize handles when resizable is false", () => {
      const { container } = render(
        <DataTable columns={columns} data={data} />,
      );
      expect(container.querySelector(".table__resize-handle")).toBeNull();
    });

    it("renders resize handles when resizable is true", () => {
      const { container } = render(
        <DataTable columns={columns} data={data} resizable />,
      );
      const handles = container.querySelectorAll(".table__resize-handle");
      expect(handles).toHaveLength(2); // one per column
    });

    it("resize handles have role=separator", () => {
      render(<DataTable columns={columns} data={data} resizable />);
      const separators = screen.getAllByRole("separator");
      expect(separators).toHaveLength(2);
    });

    it("resize handles have accessible labels", () => {
      render(<DataTable columns={columns} data={data} resizable />);
      const separators = screen.getAllByRole("separator");
      expect(separators[0]!.getAttribute("aria-label")).toBe(
        "Resize Name column",
      );
      expect(separators[1]!.getAttribute("aria-label")).toBe(
        "Resize Age column",
      );
    });

    it("resize handles have aria-valuenow and aria-valuemin", () => {
      render(<DataTable columns={columns} data={data} resizable />);
      const separators = screen.getAllByRole("separator");
      expect(separators[0]!.getAttribute("aria-valuemin")).toBe("40");
      expect(separators[0]!.getAttribute("aria-valuenow")).toBeTruthy();
    });

    it("resize handles are focusable", () => {
      render(<DataTable columns={columns} data={data} resizable />);
      const separators = screen.getAllByRole("separator");
      expect(separators[0]!.tabIndex).toBe(0);
    });

    it("applies table--resizing class during drag is false initially", () => {
      const { container } = render(
        <DataTable columns={columns} data={data} resizable />,
      );
      expect(container.querySelector(".table--resizing")).toBeNull();
    });
  });
});
