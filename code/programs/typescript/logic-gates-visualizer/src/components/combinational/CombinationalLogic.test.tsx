/**
 * Tests for Combinational Logic components — MUX, Decoder, Priority Encoder.
 *
 * Verifies:
 *   1. Each component renders without errors
 *   2. Input toggles update outputs correctly
 *   3. MUX selects the correct data input based on select line
 *   4. Decoder activates exactly one output for each input combination
 *   5. Priority encoder selects the highest-priority active input
 *   6. SVG diagrams have proper ARIA labels
 */

import { describe, it, expect, beforeAll } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { initI18n } from "@coding-adventures/ui-components";
import { MuxDiagram } from "./MuxDiagram.js";
import { DecoderDiagram } from "./DecoderDiagram.js";
import { EncoderDiagram } from "./EncoderDiagram.js";
import { CombinationalLogic } from "./CombinationalLogic.js";

import en from "../../i18n/locales/en.json";

beforeAll(() => {
  initI18n({ en });
});

// =========================================================================
// MUX
// =========================================================================

describe("MuxDiagram — 2:1 Multiplexer", () => {
  it("renders with title and description", () => {
    render(<MuxDiagram />);
    expect(screen.getByText(/Multiplexer/)).toBeTruthy();
  });

  it("initial state: S=0, D0=0, D1=1 → output = D0 = 0", () => {
    render(<MuxDiagram />);
    // Default: D0=0, D1=1, S=0 → output = D0 = 0
    expect(screen.getByLabelText(/Out: 0/)).toBeTruthy();
  });

  it("S=0 selects D0: when D0 is toggled to 1, output becomes 1", () => {
    render(<MuxDiagram />);
    const toggleD0 = screen.getByLabelText(/Input D0/);
    fireEvent.click(toggleD0); // D0=1, D1=1, S=0 → output = D0 = 1
    expect(screen.getByLabelText(/Out: 1/)).toBeTruthy();
  });

  it("S=1 selects D1: output follows D1", () => {
    render(<MuxDiagram />);
    const toggleS = screen.getByLabelText(/Input S/);
    fireEvent.click(toggleS); // D0=0, D1=1, S=1 → output = D1 = 1
    expect(screen.getByLabelText(/Out: 1/)).toBeTruthy();
  });

  it("has accessible SVG diagram", () => {
    render(<MuxDiagram />);
    const svg = screen.getByRole("img");
    expect(svg.getAttribute("aria-label")).toContain("MUX");
  });

  it("shows truth table with active row", () => {
    render(<MuxDiagram />);
    // S=0 initially, so first row should be active
    const activeRow = document.querySelector(".truth-table__row--active");
    expect(activeRow).toBeTruthy();
    expect(activeRow?.textContent).toContain("D0");
  });
});

// =========================================================================
// Decoder
// =========================================================================

describe("DecoderDiagram — 2-to-4 Decoder", () => {
  it("renders with title", () => {
    render(<DecoderDiagram />);
    expect(screen.getByText(/Decoder/)).toBeTruthy();
  });

  it("initial state A0=0, A1=0 → Y0=1, Y1=Y2=Y3=0", () => {
    render(<DecoderDiagram />);
    // Only Y0 should be active
    expect(screen.getByText("Y0: 1")).toBeTruthy();
    expect(screen.getByText("Y1: 0")).toBeTruthy();
    expect(screen.getByText("Y2: 0")).toBeTruthy();
    expect(screen.getByText("Y3: 0")).toBeTruthy();
  });

  it("toggling A0 to 1 activates Y1 (input=01)", () => {
    render(<DecoderDiagram />);
    const toggleA0 = screen.getByLabelText(/Input A0/);
    fireEvent.click(toggleA0); // A0=1, A1=0 → Y1 active

    expect(screen.getByText("Y0: 0")).toBeTruthy();
    expect(screen.getByText("Y1: 1")).toBeTruthy();
    expect(screen.getByText("Y2: 0")).toBeTruthy();
    expect(screen.getByText("Y3: 0")).toBeTruthy();
  });

  it("toggling A1 to 1 activates Y2 (input=10)", () => {
    render(<DecoderDiagram />);
    const toggleA1 = screen.getByLabelText(/Input A1/);
    fireEvent.click(toggleA1); // A0=0, A1=1 → Y2 active

    expect(screen.getByText("Y0: 0")).toBeTruthy();
    expect(screen.getByText("Y1: 0")).toBeTruthy();
    expect(screen.getByText("Y2: 1")).toBeTruthy();
    expect(screen.getByText("Y3: 0")).toBeTruthy();
  });

  it("both inputs 1 activates Y3 (input=11)", () => {
    render(<DecoderDiagram />);
    const toggleA0 = screen.getByLabelText(/Input A0/);
    const toggleA1 = screen.getByLabelText(/Input A1/);
    fireEvent.click(toggleA0);
    fireEvent.click(toggleA1); // A0=1, A1=1 → Y3 active

    expect(screen.getByText("Y0: 0")).toBeTruthy();
    expect(screen.getByText("Y1: 0")).toBeTruthy();
    expect(screen.getByText("Y2: 0")).toBeTruthy();
    expect(screen.getByText("Y3: 1")).toBeTruthy();
  });

  it("has accessible SVG diagram", () => {
    render(<DecoderDiagram />);
    const svg = screen.getByRole("img");
    expect(svg.getAttribute("aria-label")).toContain("decoder");
  });
});

// =========================================================================
// Priority Encoder
// =========================================================================

describe("EncoderDiagram — 4-to-2 Priority Encoder", () => {
  it("renders with title", () => {
    render(<EncoderDiagram />);
    expect(screen.getByText(/Priority Encoder/)).toBeTruthy();
  });

  it("no inputs active: valid=0", () => {
    render(<EncoderDiagram />);
    expect(screen.getByText(/No active input/)).toBeTruthy();
  });

  it("I0 active only: winner is I0, output=00, valid=1", () => {
    render(<EncoderDiagram />);
    const toggleI0 = screen.getByLabelText(/Input I0/);
    fireEvent.click(toggleI0); // I0=1 → winner I0, A1=0 A0=0

    expect(screen.getByText(/I0 → 00/)).toBeTruthy();
  });

  it("I0 and I2 active: I2 wins (higher priority)", () => {
    render(<EncoderDiagram />);
    const toggleI0 = screen.getByLabelText(/Input I0/);
    const toggleI2 = screen.getByLabelText(/Input I2/);
    fireEvent.click(toggleI0);
    fireEvent.click(toggleI2); // I0=1, I2=1 → I2 wins, A1=1 A0=0

    expect(screen.getByText(/I2 → 10/)).toBeTruthy();
  });

  it("I3 always wins when active (highest priority)", () => {
    render(<EncoderDiagram />);
    const toggleI0 = screen.getByLabelText(/Input I0/);
    const toggleI1 = screen.getByLabelText(/Input I1/);
    const toggleI3 = screen.getByLabelText(/Input I3/);
    fireEvent.click(toggleI0);
    fireEvent.click(toggleI1);
    fireEvent.click(toggleI3); // I0=1, I1=1, I3=1 → I3 wins

    expect(screen.getByText(/I3 → 11/)).toBeTruthy();
  });

  it("has accessible SVG diagram", () => {
    render(<EncoderDiagram />);
    const svg = screen.getByRole("img");
    expect(svg.getAttribute("aria-label")).toContain("priority encoder");
  });
});

// =========================================================================
// Container
// =========================================================================

describe("CombinationalLogic — Tab 3 container", () => {
  it("renders all three circuit components", () => {
    render(<CombinationalLogic />);
    expect(screen.getByText(/Multiplexer/)).toBeTruthy();
    expect(screen.getByText(/Decoder/)).toBeTruthy();
    expect(screen.getByText(/Priority Encoder/)).toBeTruthy();
  });
});
