/**
 * Tests for the Binary Adders tab (Tab 1).
 *
 * Verifies:
 * - Half adder renders with toggleable inputs and correct outputs
 * - Full adder renders with 3 inputs and shows intermediates
 * - Ripple-carry adder renders 4-bit inputs with decimal display
 * - Truth tables highlight correct rows
 * - BitGroup shows correct decimal values
 */

import { describe, it, expect, beforeAll } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../../i18n/locales/en.json";

import { HalfAdderDiagram } from "./HalfAdderDiagram.js";
import { FullAdderDiagram } from "./FullAdderDiagram.js";
import { RippleCarryDiagram } from "./RippleCarryDiagram.js";
import { BinaryAdders } from "./BinaryAdders.js";

beforeAll(() => {
  initI18n({ en });
});

// ---------------------------------------------------------------------------
// Half Adder
// ---------------------------------------------------------------------------

describe("HalfAdderDiagram", () => {
  it("renders title and description", () => {
    render(<HalfAdderDiagram />);
    expect(screen.getByText(/Half Adder/)).toBeInTheDocument();
    expect(screen.getByText(/simplest arithmetic circuit/i)).toBeInTheDocument();
  });

  it("starts with A=0, B=0 → Sum=0, Carry=0", () => {
    render(<HalfAdderDiagram />);
    expect(screen.getByText("Sum: 0")).toBeInTheDocument();
    expect(screen.getByText("Carry: 0")).toBeInTheDocument();
  });

  it("toggling A to 1 gives Sum=1, Carry=0", () => {
    render(<HalfAdderDiagram />);
    const aButton = screen.getByLabelText(/Input A: 0/i);
    fireEvent.click(aButton);
    expect(screen.getByText("Sum: 1")).toBeInTheDocument();
    expect(screen.getByText("Carry: 0")).toBeInTheDocument();
  });

  it("A=1, B=1 gives Sum=0, Carry=1", () => {
    render(<HalfAdderDiagram />);
    fireEvent.click(screen.getByLabelText(/Input A: 0/i));
    fireEvent.click(screen.getByLabelText(/Input B: 0/i));
    expect(screen.getByText("Sum: 0")).toBeInTheDocument();
    expect(screen.getByText("Carry: 1")).toBeInTheDocument();
  });

  it("renders a truth table with 4 rows", () => {
    render(<HalfAdderDiagram />);
    const table = screen.getByRole("table");
    const rows = table.querySelectorAll("tbody tr");
    expect(rows.length).toBe(4);
  });

  it("highlights the active truth table row", () => {
    render(<HalfAdderDiagram />);
    // A=0, B=0 → row 0 should be active
    const table = screen.getByRole("table");
    const rows = table.querySelectorAll("tbody tr");
    expect(rows[0]).toHaveAttribute("aria-current", "true");
  });
});

// ---------------------------------------------------------------------------
// Full Adder
// ---------------------------------------------------------------------------

describe("FullAdderDiagram", () => {
  it("renders title and description", () => {
    render(<FullAdderDiagram />);
    expect(screen.getByText(/Full Adder/)).toBeInTheDocument();
    expect(screen.getByText(/carry-in/i)).toBeInTheDocument();
  });

  it("starts with A=0, B=0, Cin=0 → Sum=0, Cout=0", () => {
    render(<FullAdderDiagram />);
    expect(screen.getByText("Sum: 0")).toBeInTheDocument();
    expect(screen.getByText("Carry Out: 0")).toBeInTheDocument();
  });

  it("A=1, B=1, Cin=0 gives Sum=0, Carry=1", () => {
    render(<FullAdderDiagram />);
    fireEvent.click(screen.getByLabelText(/Input A: 0/i));
    fireEvent.click(screen.getByLabelText(/Input B: 0/i));
    expect(screen.getByText("Sum: 0")).toBeInTheDocument();
    expect(screen.getByText("Carry Out: 1")).toBeInTheDocument();
  });

  it("A=1, B=1, Cin=1 gives Sum=1, Carry=1", () => {
    render(<FullAdderDiagram />);
    fireEvent.click(screen.getByLabelText(/Input A: 0/i));
    fireEvent.click(screen.getByLabelText(/Input B: 0/i));
    fireEvent.click(screen.getByLabelText(/Input Cin: 0/i));
    expect(screen.getByText("Sum: 1")).toBeInTheDocument();
    expect(screen.getByText("Carry Out: 1")).toBeInTheDocument();
  });

  it("shows intermediate values", () => {
    render(<FullAdderDiagram />);
    expect(screen.getByText("HA1 Sum: 0")).toBeInTheDocument();
    expect(screen.getByText("HA1 Carry: 0")).toBeInTheDocument();
    expect(screen.getByText("HA2 Carry: 0")).toBeInTheDocument();
  });

  it("renders a truth table with 8 rows", () => {
    render(<FullAdderDiagram />);
    const table = screen.getByRole("table");
    const rows = table.querySelectorAll("tbody tr");
    expect(rows.length).toBe(8);
  });
});

// ---------------------------------------------------------------------------
// Ripple-Carry Adder
// ---------------------------------------------------------------------------

describe("RippleCarryDiagram", () => {
  it("renders title", () => {
    render(<RippleCarryDiagram />);
    expect(screen.getByText(/Ripple-Carry Adder/)).toBeInTheDocument();
  });

  it("starts with A=5, B=3 → shows 5 + 3 = 8", () => {
    render(<RippleCarryDiagram />);
    expect(screen.getByText(/5 \+ 3 = 8/)).toBeInTheDocument();
  });

  it("shows decimal values for both operands", () => {
    render(<RippleCarryDiagram />);
    // A = [1,0,1,0] = 5, B = [1,1,0,0] = 3
    expect(screen.getByText("= 5")).toBeInTheDocument();
    expect(screen.getByText("= 3")).toBeInTheDocument();
  });

  it("renders per-adder snapshot table with 4 rows", () => {
    render(<RippleCarryDiagram />);
    const tables = screen.getAllByRole("table");
    // Should have the snapshot table
    const snapshotTable = tables.find(t =>
      t.querySelector("caption")?.textContent?.includes("Snapshot")
    );
    expect(snapshotTable).toBeTruthy();
    const rows = snapshotTable!.querySelectorAll("tbody tr");
    expect(rows.length).toBe(4);
  });

  it("shows no overflow for 5 + 3", () => {
    render(<RippleCarryDiagram />);
    expect(screen.getByText(/No overflow/)).toBeInTheDocument();
  });

  it("shows overflow when result exceeds 4 bits", () => {
    render(<RippleCarryDiagram />);
    // Set A to 15 (1111) by toggling all bits to 1
    // A starts as [1,0,1,0] = 5, need to toggle A1 and A3
    fireEvent.click(screen.getByLabelText(/Input A1: 0/i));
    fireEvent.click(screen.getByLabelText(/Input A3: 0/i));
    // Now A = [1,1,1,1] = 15

    // B starts as [1,1,0,0] = 3, set to 1 by toggling B1
    fireEvent.click(screen.getByLabelText(/Input B1: 1/i));
    // Now B = [1,0,0,0] = 1

    // 15 + 1 = 16, which overflows 4 bits
    expect(screen.getByText(/Overflow/)).toBeInTheDocument();
  });
});

// ---------------------------------------------------------------------------
// BinaryAdders container
// ---------------------------------------------------------------------------

describe("BinaryAdders", () => {
  it("renders intro text", () => {
    render(<BinaryAdders />);
    expect(screen.getByText(/foundation of ALL arithmetic/i)).toBeInTheDocument();
  });

  it("renders all three adder diagrams", () => {
    render(<BinaryAdders />);
    expect(screen.getByText(/Half Adder/)).toBeInTheDocument();
    expect(screen.getByText(/Full Adder/)).toBeInTheDocument();
    expect(screen.getByText(/Ripple-Carry Adder/)).toBeInTheDocument();
  });
});
