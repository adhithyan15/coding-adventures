/**
 * Tests for the GateCard component.
 *
 * Verifies:
 * 1. Renders the gate name from i18n
 * 2. Toggling inputs changes the output
 * 3. Truth table highlights the correct row
 * 4. CMOS panel is present (collapsed by default)
 */

import { describe, it, expect, beforeAll } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { initI18n } from "@coding-adventures/ui-components";
import { AND, NOT } from "@coding-adventures/logic-gates";
import type { Bit } from "@coding-adventures/logic-gates";
import en from "../../i18n/locales/en.json";
import { GateCard } from "./GateCard.js";

beforeAll(() => {
  initI18n({ en });
});

describe("GateCard", () => {
  it("renders the gate name", () => {
    render(
      <GateCard
        gateType="and"
        gateFn={(a: Bit, b: Bit) => AND(a, b)}
        inputLabels={["A", "B"]}
        i18nPrefix="gate.and"
      />,
    );
    expect(screen.getByText("AND")).toBeTruthy();
  });

  it("renders the gate description", () => {
    render(
      <GateCard
        gateType="not"
        gateFn={(a: Bit) => NOT(a)}
        inputLabels={["A"]}
        i18nPrefix="gate.not"
      />,
    );
    expect(screen.getByText(/opposite of its input/)).toBeTruthy();
  });

  it("shows initial output of 0 for AND gate (both inputs 0)", () => {
    render(
      <GateCard
        gateType="and"
        gateFn={(a: Bit, b: Bit) => AND(a, b)}
        inputLabels={["A", "B"]}
        i18nPrefix="gate.and"
      />,
    );
    // Output should show "Out: 0"
    expect(screen.getByText("Out: 0")).toBeTruthy();
  });

  it("shows initial output of 1 for NOT gate (input 0)", () => {
    render(
      <GateCard
        gateType="not"
        gateFn={(a: Bit) => NOT(a)}
        inputLabels={["A"]}
        i18nPrefix="gate.not"
      />,
    );
    // NOT(0) = 1, so output should show "Out: 1"
    expect(screen.getByText("Out: 1")).toBeTruthy();
  });

  it("updates output when input is toggled", () => {
    render(
      <GateCard
        gateType="and"
        gateFn={(a: Bit, b: Bit) => AND(a, b)}
        inputLabels={["A", "B"]}
        i18nPrefix="gate.and"
      />,
    );

    // Initially AND(0,0) = 0
    expect(screen.getByText("Out: 0")).toBeTruthy();

    // Toggle both inputs to 1
    const toggleButtons = screen.getAllByRole("button").filter(
      (btn) => btn.getAttribute("aria-label")?.startsWith("Input"),
    );
    fireEvent.click(toggleButtons[0]); // A -> 1
    fireEvent.click(toggleButtons[1]); // B -> 1

    // AND(1,1) = 1
    expect(screen.getByText("Out: 1")).toBeTruthy();
  });

  it("highlights the correct truth table row", () => {
    render(
      <GateCard
        gateType="and"
        gateFn={(a: Bit, b: Bit) => AND(a, b)}
        inputLabels={["A", "B"]}
        i18nPrefix="gate.and"
      />,
    );

    // Initial inputs: A=0, B=0 -> row index 0
    const rows = screen.getAllByRole("row");
    // rows[0] is header, rows[1] is first data row (A=0, B=0)
    expect(rows[1].getAttribute("aria-current")).toBe("true");
  });

  it("renders the CMOS panel toggle", () => {
    render(
      <GateCard
        gateType="and"
        gateFn={(a: Bit, b: Bit) => AND(a, b)}
        inputLabels={["A", "B"]}
        i18nPrefix="gate.and"
      />,
    );
    expect(screen.getByText(/Show CMOS implementation/)).toBeTruthy();
  });

  it("renders a truth table with correct number of rows", () => {
    render(
      <GateCard
        gateType="and"
        gateFn={(a: Bit, b: Bit) => AND(a, b)}
        inputLabels={["A", "B"]}
        i18nPrefix="gate.and"
      />,
    );
    // 2-input gate: 4 data rows + 1 header = 5
    const rows = screen.getAllByRole("row");
    expect(rows.length).toBe(5);
  });

  it("renders the gate symbol SVG", () => {
    render(
      <GateCard
        gateType="and"
        gateFn={(a: Bit, b: Bit) => AND(a, b)}
        inputLabels={["A", "B"]}
        i18nPrefix="gate.and"
      />,
    );
    const svg = screen.getByLabelText("AND gate symbol");
    expect(svg).toBeTruthy();
  });
});
