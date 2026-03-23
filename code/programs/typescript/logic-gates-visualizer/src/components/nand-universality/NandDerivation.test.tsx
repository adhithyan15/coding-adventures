/**
 * Tests for NandDerivation — interactive NAND universality diagrams.
 *
 * These tests verify:
 *   1. Each derivation renders without crashing
 *   2. Input toggles change values correctly
 *   3. Output values match the expected gate behavior
 *   4. Intermediate wire values are displayed
 *   5. Transistor cost comparison is shown
 *   6. Accessibility attributes are present
 */

import { describe, it, expect, beforeAll } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { initI18n } from "@coding-adventures/ui-components";
import { NandDerivation } from "./NandDerivation.js";

// Load English translations before tests.
import en from "../../i18n/locales/en.json";

beforeAll(() => {
  initI18n({ en });
});

describe("NandDerivation — NOT from NAND", () => {
  it("renders with initial output = 1 (NOT of 0)", () => {
    render(<NandDerivation type="not" />);
    expect(screen.getByText(/NAND → NOT/)).toBeTruthy();
    // Initial input is 0, NOT(0) = 1, so output should show 1
    expect(screen.getByLabelText(/Out: 1/)).toBeTruthy();
  });

  it("toggles input A and updates output", () => {
    render(<NandDerivation type="not" />);
    const toggleA = screen.getByLabelText(/Input A/);

    // Toggle A from 0 to 1: NOT(1) = 0
    fireEvent.click(toggleA);
    expect(screen.getByLabelText(/Out: 0/)).toBeTruthy();

    // Toggle A back to 0: NOT(0) = 1
    fireEvent.click(toggleA);
    expect(screen.getByLabelText(/Out: 1/)).toBeTruthy();
  });

  it("shows transistor cost comparison", () => {
    render(<NandDerivation type="not" />);
    expect(screen.getByText(/1 NAND = 4T/)).toBeTruthy();
    expect(screen.getByText(/2T/)).toBeTruthy();
  });

  it("has accessible SVG diagram", () => {
    render(<NandDerivation type="not" />);
    const svg = screen.getByRole("img");
    expect(svg.getAttribute("aria-label")).toBeTruthy();
  });
});

describe("NandDerivation — AND from NAND", () => {
  it("renders with initial output = 0 (AND(0,0))", () => {
    render(<NandDerivation type="and" />);
    expect(screen.getByText(/NAND → AND/)).toBeTruthy();
    expect(screen.getByLabelText(/Out: 0/)).toBeTruthy();
  });

  it("computes AND(1,1) = 1 when both inputs toggled", () => {
    render(<NandDerivation type="and" />);
    const toggleA = screen.getByLabelText(/Input A/);
    const toggleB = screen.getByLabelText(/Input B/);

    fireEvent.click(toggleA); // A=1, B=0 → AND = 0
    expect(screen.getByLabelText(/Out: 0/)).toBeTruthy();

    fireEvent.click(toggleB); // A=1, B=1 → AND = 1
    expect(screen.getByLabelText(/Out: 1/)).toBeTruthy();
  });

  it("shows Gate 1 and Gate 2 labels", () => {
    render(<NandDerivation type="and" />);
    expect(screen.getByText("Gate 1")).toBeTruthy();
    expect(screen.getByText("Gate 2")).toBeTruthy();
  });

  it("shows transistor cost: 2 NAND = 8T vs native 6T", () => {
    render(<NandDerivation type="and" />);
    expect(screen.getByText(/2 NAND = 8T/)).toBeTruthy();
    expect(screen.getByText(/6T/)).toBeTruthy();
  });
});

describe("NandDerivation — OR from NAND (De Morgan's)", () => {
  it("renders with initial output = 0 (OR(0,0))", () => {
    render(<NandDerivation type="or" />);
    expect(screen.getByText(/NAND → OR/)).toBeTruthy();
    expect(screen.getByLabelText(/Out: 0/)).toBeTruthy();
  });

  it("computes OR(0,1) = 1 when B toggled", () => {
    render(<NandDerivation type="or" />);
    const toggleB = screen.getByLabelText(/Input B/);

    fireEvent.click(toggleB); // A=0, B=1 → OR = 1
    expect(screen.getByLabelText(/Out: 1/)).toBeTruthy();
  });

  it("computes OR(1,0) = 1 when A toggled", () => {
    render(<NandDerivation type="or" />);
    const toggleA = screen.getByLabelText(/Input A/);

    fireEvent.click(toggleA); // A=1, B=0 → OR = 1
    expect(screen.getByLabelText(/Out: 1/)).toBeTruthy();
  });

  it("shows all 3 gate labels", () => {
    render(<NandDerivation type="or" />);
    expect(screen.getByText("Gate 1")).toBeTruthy();
    expect(screen.getByText("Gate 2")).toBeTruthy();
    expect(screen.getByText("Gate 3")).toBeTruthy();
  });

  it("shows transistor cost: 3 NAND = 12T vs native 6T", () => {
    render(<NandDerivation type="or" />);
    expect(screen.getByText(/3 NAND = 12T/)).toBeTruthy();
  });
});

describe("NandDerivation — XOR from NAND", () => {
  it("renders with initial output = 0 (XOR(0,0))", () => {
    render(<NandDerivation type="xor" />);
    expect(screen.getByText(/NAND → XOR/)).toBeTruthy();
    expect(screen.getByLabelText(/Out: 0/)).toBeTruthy();
  });

  it("computes XOR(1,0) = 1", () => {
    render(<NandDerivation type="xor" />);
    const toggleA = screen.getByLabelText(/Input A/);

    fireEvent.click(toggleA); // A=1, B=0 → XOR = 1
    expect(screen.getByLabelText(/Out: 1/)).toBeTruthy();
  });

  it("computes XOR(1,1) = 0", () => {
    render(<NandDerivation type="xor" />);
    const toggleA = screen.getByLabelText(/Input A/);
    const toggleB = screen.getByLabelText(/Input B/);

    fireEvent.click(toggleA); // A=1, B=0 → XOR = 1
    fireEvent.click(toggleB); // A=1, B=1 → XOR = 0
    expect(screen.getByLabelText(/Out: 0/)).toBeTruthy();
  });

  it("computes XOR(0,1) = 1", () => {
    render(<NandDerivation type="xor" />);
    const toggleB = screen.getByLabelText(/Input B/);

    fireEvent.click(toggleB); // A=0, B=1 → XOR = 1
    expect(screen.getByLabelText(/Out: 1/)).toBeTruthy();
  });

  it("shows all 4 gate labels", () => {
    render(<NandDerivation type="xor" />);
    expect(screen.getByText("Gate 1")).toBeTruthy();
    expect(screen.getByText("Gate 2")).toBeTruthy();
    expect(screen.getByText("Gate 3")).toBeTruthy();
    expect(screen.getByText("Gate 4")).toBeTruthy();
  });

  it("shows transistor cost: 4 NAND = 16T", () => {
    render(<NandDerivation type="xor" />);
    expect(screen.getByText(/4 NAND = 16T/)).toBeTruthy();
  });

  it("shows intermediate wire values", () => {
    render(<NandDerivation type="xor" />);
    // Initial: A=0, B=0, N=NAND(0,0)=1
    // w1=NAND(0,1)=1, w2=NAND(0,1)=1
    const svg = screen.getByRole("img");
    expect(svg.textContent).toContain("N=1");
    expect(svg.textContent).toContain("w1=1");
    expect(svg.textContent).toContain("w2=1");
  });
});
