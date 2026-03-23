/**
 * Tests for the TruthTable component.
 *
 * Verifies:
 * 1. Rendering — correct number of rows and columns
 * 2. Active row highlighting — the matching row gets a highlight class
 * 3. Accessibility — proper table semantics and aria-current
 */

import { describe, it, expect, beforeAll } from "vitest";
import { render, screen } from "@testing-library/react";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../../i18n/locales/en.json";
import { TruthTable } from "./TruthTable.js";
import type { Bit } from "@coding-adventures/logic-gates";

beforeAll(() => {
  initI18n({ en });
});

const andRows: Array<{ inputs: Bit[]; output: Bit }> = [
  { inputs: [0, 0], output: 0 },
  { inputs: [0, 1], output: 0 },
  { inputs: [1, 0], output: 0 },
  { inputs: [1, 1], output: 1 },
];

describe("TruthTable", () => {
  it("renders the correct number of rows", () => {
    render(
      <TruthTable inputs={["A", "B"]} output="Out" rows={andRows} />,
    );
    // 4 data rows + 1 header row = 5 total <tr> elements
    const rows = screen.getAllByRole("row");
    expect(rows.length).toBe(5);
  });

  it("renders column headers", () => {
    render(
      <TruthTable inputs={["A", "B"]} output="Out" rows={andRows} />,
    );
    expect(screen.getByText("A")).toBeTruthy();
    expect(screen.getByText("B")).toBeTruthy();
    expect(screen.getByText("Out")).toBeTruthy();
  });

  it("renders the caption from i18n", () => {
    render(
      <TruthTable inputs={["A", "B"]} output="Out" rows={andRows} />,
    );
    expect(screen.getByText("Truth Table")).toBeTruthy();
  });

  it("highlights the active row", () => {
    render(
      <TruthTable
        inputs={["A", "B"]}
        output="Out"
        rows={andRows}
        activeRow={3}
      />,
    );
    // The 4th data row (index 3) should have the active class.
    // getAllByRole("row") includes header, so data rows start at index 1.
    const allRows = screen.getAllByRole("row");
    const activeRow = allRows[4]; // header + 3 non-active + 1 active
    expect(activeRow.className).toContain("truth-table__row--active");
  });

  it("sets aria-current on the active row", () => {
    render(
      <TruthTable
        inputs={["A", "B"]}
        output="Out"
        rows={andRows}
        activeRow={0}
      />,
    );
    const allRows = screen.getAllByRole("row");
    // First data row (index 1 in allRows because index 0 is header)
    expect(allRows[1].getAttribute("aria-current")).toBe("true");
    // Other data rows should not have aria-current
    expect(allRows[2].getAttribute("aria-current")).toBeNull();
  });

  it("renders without active row (no highlight)", () => {
    render(
      <TruthTable inputs={["A", "B"]} output="Out" rows={andRows} />,
    );
    const allRows = screen.getAllByRole("row");
    // No row should have the active class
    allRows.forEach((row) => {
      expect(row.className).not.toContain("truth-table__row--active");
    });
  });

  it("works with a single-input gate (NOT)", () => {
    const notRows: Array<{ inputs: Bit[]; output: Bit }> = [
      { inputs: [0], output: 1 },
      { inputs: [1], output: 0 },
    ];
    render(
      <TruthTable inputs={["A"]} output="Out" rows={notRows} activeRow={0} />,
    );
    const allRows = screen.getAllByRole("row");
    // 1 header + 2 data = 3
    expect(allRows.length).toBe(3);
  });
});
