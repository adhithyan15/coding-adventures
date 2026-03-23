/**
 * Tests for the Everything is Addition tab (Tab 2).
 *
 * Verifies:
 * - SubtractionView: two's complement transformation, correct results
 * - MultiplicationView: shift-and-add grid, step trace, products
 * - EverythingIsAddition container renders both views
 */

import { describe, it, expect, beforeAll } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../../i18n/locales/en.json";

import { SubtractionView } from "./SubtractionView.js";
import { MultiplicationView } from "./MultiplicationView.js";
import { EverythingIsAddition } from "./EverythingIsAddition.js";

beforeAll(() => {
  initI18n({ en });
});

// ---------------------------------------------------------------------------
// SubtractionView
// ---------------------------------------------------------------------------

describe("SubtractionView", () => {
  it("renders title and description", () => {
    render(<SubtractionView />);
    expect(screen.getByText(/Subtraction = Addition/)).toBeInTheDocument();
    // Multiple elements contain "two's complement", so use getAllBy
    expect(screen.getAllByText(/two's complement/i).length).toBeGreaterThan(0);
  });

  it("shows the 3-step transformation", () => {
    render(<SubtractionView />);
    expect(screen.getByText(/Step 1/)).toBeInTheDocument();
    expect(screen.getByText(/Step 2/)).toBeInTheDocument();
    expect(screen.getByText(/Step 3/)).toBeInTheDocument();
  });

  it("defaults to 7 - 3 = 4", () => {
    render(<SubtractionView />);
    expect(screen.getByText(/7 − 3/)).toBeInTheDocument();
    // Step 3 shows the result
    expect(screen.getByText(/7 \+ \(−3\) = 4/)).toBeInTheDocument();
  });

  it("shows the adder trace table", () => {
    render(<SubtractionView />);
    const table = screen.getByRole("table");
    const rows = table.querySelectorAll("tbody tr");
    expect(rows.length).toBe(4); // 4-bit adder trace
  });

  it("shows the educational insight callout", () => {
    render(<SubtractionView />);
    expect(screen.getByText(/NOT\(x\) \+ 1 = −x/)).toBeInTheDocument();
  });

  it("updates when inputs change", () => {
    render(<SubtractionView />);
    // Toggle A0 from 1 to 0: A changes from 7 (0111→LSB [1,1,1,0]) to 6 (0110→LSB [0,1,1,0])
    fireEvent.click(screen.getByLabelText(/Input A0: 1/i));
    expect(screen.getByText(/6 − 3/)).toBeInTheDocument();
    expect(screen.getByText(/6 \+ \(−3\) = 3/)).toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// MultiplicationView
// ---------------------------------------------------------------------------

describe("MultiplicationView", () => {
  it("renders title and description", () => {
    render(<MultiplicationView />);
    expect(screen.getByText(/Multiplication = Shift-and-Add/)).toBeInTheDocument();
  });

  it("defaults to 5 × 3 = 15", () => {
    render(<MultiplicationView />);
    expect(screen.getByText(/5 × 3 = 15/)).toBeInTheDocument();
  });

  it("shows decimal values for operands", () => {
    render(<MultiplicationView />);
    expect(screen.getByText("(5)")).toBeInTheDocument();
    expect(screen.getByText("(3)")).toBeInTheDocument();
  });

  it("shows the product in the result row", () => {
    render(<MultiplicationView />);
    expect(screen.getByText("(15)")).toBeInTheDocument();
  });

  it("shows step trace table with 4 rows", () => {
    render(<MultiplicationView />);
    const tables = screen.getAllByRole("table");
    const traceTable = tables.find(t =>
      t.querySelector("caption")?.textContent?.includes("Step")
    );
    expect(traceTable).toBeTruthy();
    const rows = traceTable!.querySelectorAll("tbody tr");
    expect(rows.length).toBe(4);
  });

  it("highlights active steps (bit=1) and dims skipped steps (bit=0)", () => {
    render(<MultiplicationView />);
    // B = 3 = [1,1,0,0] LSB first → bits 0 and 1 are active
    const tables = screen.getAllByRole("table");
    const traceTable = tables.find(t =>
      t.querySelector("caption")?.textContent?.includes("Step")
    );
    const rows = traceTable!.querySelectorAll("tbody tr");
    // Rows 0 and 1 should be active (highlighted)
    expect(rows[0]).toHaveClass("truth-table__row--active");
    expect(rows[1]).toHaveClass("truth-table__row--active");
    // Rows 2 and 3 should not be active
    expect(rows[2]).not.toHaveClass("truth-table__row--active");
    expect(rows[3]).not.toHaveClass("truth-table__row--active");
  });

  it("shows educational insight callout", () => {
    render(<MultiplicationView />);
    expect(screen.getByText(/conditional additions of shifted values/i)).toBeInTheDocument();
  });

  it("updates when inputs change", () => {
    render(<MultiplicationView />);
    // Set A to 3 by toggling A2 from 1 to 0: A changes from 5 ([1,0,1,0]) to 1 ([1,0,0,0])
    fireEvent.click(screen.getByLabelText(/Input A2: 1/i));
    // Now A = 1, B = 3, product = 3
    expect(screen.getByText(/1 × 3 = 3/)).toBeInTheDocument();
  });

  it("handles 0 × 0 = 0", () => {
    render(<MultiplicationView />);
    // Set A to 0 by toggling all set bits off
    fireEvent.click(screen.getByLabelText(/Input A0: 1/i));
    fireEvent.click(screen.getByLabelText(/Input A2: 1/i));
    // Set B to 0
    fireEvent.click(screen.getByLabelText(/Input B0: 1/i));
    fireEvent.click(screen.getByLabelText(/Input B1: 1/i));
    expect(screen.getByText(/0 × 0 = 0/)).toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// EverythingIsAddition container
// ---------------------------------------------------------------------------

describe("EverythingIsAddition", () => {
  it("renders intro text", () => {
    render(<EverythingIsAddition />);
    expect(screen.getByText(/only arithmetic hardware/i)).toBeInTheDocument();
  });

  it("renders both subtraction and multiplication views", () => {
    render(<EverythingIsAddition />);
    expect(screen.getByText(/Subtraction = Addition/)).toBeInTheDocument();
    expect(screen.getByText(/Multiplication = Shift-and-Add/)).toBeInTheDocument();
  });
});
