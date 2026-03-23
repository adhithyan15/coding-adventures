/**
 * Tests for the CmosPanel component.
 *
 * Verifies:
 * 1. Initial state — collapsed, toggle button visible
 * 2. Expand/collapse — clicking toggle reveals/hides content
 * 3. Transistor count badge — displays correct count
 * 4. ARIA — aria-expanded reflects state
 */

import { describe, it, expect, beforeAll } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../../i18n/locales/en.json";
import { CmosPanel } from "./CmosPanel.js";

beforeAll(() => {
  initI18n({ en });
});

describe("CmosPanel", () => {
  it("renders collapsed by default", () => {
    render(<CmosPanel gateType="not" inputA={0} />);
    const button = screen.getByRole("button");
    expect(button.getAttribute("aria-expanded")).toBe("false");
    expect(button.textContent).toContain("Show CMOS implementation");
  });

  it("expands when toggle is clicked", () => {
    render(<CmosPanel gateType="not" inputA={0} />);
    const button = screen.getByRole("button");
    fireEvent.click(button);
    expect(button.getAttribute("aria-expanded")).toBe("true");
    expect(button.textContent).toContain("Hide CMOS implementation");
  });

  it("collapses when toggle is clicked again", () => {
    render(<CmosPanel gateType="not" inputA={0} />);
    const button = screen.getByRole("button");
    fireEvent.click(button); // expand
    fireEvent.click(button); // collapse
    expect(button.getAttribute("aria-expanded")).toBe("false");
  });

  it("displays transistor count for NOT (2)", () => {
    render(<CmosPanel gateType="not" inputA={0} />);
    expect(screen.getByText(/2/)).toBeTruthy();
    expect(screen.getByText(/transistors/)).toBeTruthy();
  });

  it("displays transistor count for NAND (4)", () => {
    render(<CmosPanel gateType="nand" inputA={0} inputB={0} />);
    expect(screen.getByText(/4/)).toBeTruthy();
  });

  it("displays transistor count for AND (6)", () => {
    render(<CmosPanel gateType="and" inputA={0} inputB={0} />);
    expect(screen.getByText(/6/)).toBeTruthy();
  });

  it("displays transistor count for OR (6)", () => {
    render(<CmosPanel gateType="or" inputA={0} inputB={0} />);
    expect(screen.getByText(/6/)).toBeTruthy();
  });

  it("shows natural gate note for NAND when expanded", () => {
    render(<CmosPanel gateType="nand" inputA={0} inputB={0} />);
    fireEvent.click(screen.getByRole("button"));
    expect(screen.getByText("CMOS natural gate")).toBeTruthy();
  });

  it("shows natural gate note for NOR when expanded", () => {
    render(<CmosPanel gateType="nor" inputA={0} inputB={0} />);
    fireEvent.click(screen.getByRole("button"));
    expect(screen.getByText("CMOS natural gate")).toBeTruthy();
  });

  it("shows inverter note for AND when expanded", () => {
    render(<CmosPanel gateType="and" inputA={0} inputB={0} />);
    fireEvent.click(screen.getByRole("button"));
    expect(screen.getByText("Requires extra inverter stage")).toBeTruthy();
  });

  it("renders SVG diagram for NOT when expanded", () => {
    render(<CmosPanel gateType="not" inputA={1} />);
    fireEvent.click(screen.getByRole("button"));
    // The SVG should be present with the correct aria-label
    const svg = screen.getByLabelText(/CMOS NOT gate/);
    expect(svg).toBeTruthy();
  });
});
