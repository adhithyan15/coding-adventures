/**
 * Tests for the ALU tab (Tab 3).
 *
 * Verifies:
 * - OperationSelector renders all 6 operations
 * - ALUView computes correct results for each operation
 * - Condition flags display correctly
 * - B input hidden for NOT (unary)
 * - Switching operations updates result
 */

import { describe, it, expect, beforeAll } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../../i18n/locales/en.json";

import { ALUView } from "./ALUView.js";

beforeAll(() => {
  initI18n({ en });
});

describe("ALUView", () => {
  it("renders title and intro", () => {
    render(<ALUView />);
    expect(screen.getByText(/One Circuit to Rule/)).toBeInTheDocument();
    expect(screen.getByText(/operations come together/i)).toBeInTheDocument();
  });

  it("renders all 6 operation buttons", () => {
    render(<ALUView />);
    expect(screen.getByRole("radio", { name: /ADD/i })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /SUB/i })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /AND/i })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /^OR$/i })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /XOR/i })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /NOT/i })).toBeInTheDocument();
  });

  it("defaults to ADD with A=42, B=15 → result=57", () => {
    render(<ALUView />);
    expect(screen.getByText("= 57")).toBeInTheDocument();
  });

  it("ADD is initially selected", () => {
    render(<ALUView />);
    const addBtn = screen.getByRole("radio", { name: /ADD/i });
    expect(addBtn).toHaveAttribute("aria-checked", "true");
  });

  it("switching to SUB shows subtraction result", () => {
    render(<ALUView />);
    // A=42, B=15, SUB → 42-15=27
    fireEvent.click(screen.getByRole("radio", { name: /SUB/i }));
    expect(screen.getByText("= 27")).toBeInTheDocument();
  });

  it("switching to AND shows bitwise AND result", () => {
    render(<ALUView />);
    // A=42 (00101010), B=15 (00001111), AND → 00001010 = 10
    fireEvent.click(screen.getByRole("radio", { name: /AND/i }));
    expect(screen.getByText("= 10")).toBeInTheDocument();
  });

  it("switching to OR shows bitwise OR result", () => {
    render(<ALUView />);
    // A=42 (00101010), B=15 (00001111), OR → 00101111 = 47
    fireEvent.click(screen.getByRole("radio", { name: /^OR$/i }));
    expect(screen.getByText("= 47")).toBeInTheDocument();
  });

  it("switching to XOR shows bitwise XOR result", () => {
    render(<ALUView />);
    // A=42 (00101010), B=15 (00001111), XOR → 00100101 = 37
    fireEvent.click(screen.getByRole("radio", { name: /XOR/i }));
    expect(screen.getByText("= 37")).toBeInTheDocument();
  });

  it("switching to NOT shows bitwise NOT of A", () => {
    render(<ALUView />);
    // A=42 (00101010), NOT → 11010101 = 213
    fireEvent.click(screen.getByRole("radio", { name: /NOT/i }));
    expect(screen.getByText("= 213")).toBeInTheDocument();
  });

  it("shows unary note for NOT operation", () => {
    render(<ALUView />);
    fireEvent.click(screen.getByRole("radio", { name: /NOT/i }));
    expect(screen.getByText(/only uses operand A/i)).toBeInTheDocument();
  });

  it("displays four condition flags", () => {
    render(<ALUView />);
    expect(screen.getByText("Zero")).toBeInTheDocument();
    expect(screen.getByText("Carry")).toBeInTheDocument();
    expect(screen.getByText("Negative")).toBeInTheDocument();
    expect(screen.getByText("Overflow")).toBeInTheDocument();
  });

  it("displays result in hex format", () => {
    render(<ALUView />);
    // 57 = 0x39
    expect(screen.getByText("0x39")).toBeInTheDocument();
  });

  it("shows 8 result bits", () => {
    render(<ALUView />);
    const container = document.querySelector(".result-display__bits");
    const bits = container?.querySelectorAll(".result-display__bit");
    expect(bits?.length).toBe(8);
  });

  it("zero flag activates when result is 0", () => {
    render(<ALUView />);
    // Set A=0 by toggling all set bits off
    // A starts at 42 (00101010) → toggle A1, A3, A5
    fireEvent.click(screen.getByLabelText(/Input A1: 1/i));
    fireEvent.click(screen.getByLabelText(/Input A3: 1/i));
    fireEvent.click(screen.getByLabelText(/Input A5: 1/i));
    // Now A=0, SUB with B would give negative, but use AND
    fireEvent.click(screen.getByRole("radio", { name: /AND/i }));
    // A=0 AND B=anything = 0 → zero flag set
    const zeroFlag = document.querySelector(".flag-indicator--active");
    expect(zeroFlag).toBeTruthy();
  });
});
