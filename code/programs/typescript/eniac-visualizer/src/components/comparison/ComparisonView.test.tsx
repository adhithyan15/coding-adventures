import { describe, it, expect, beforeAll } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../../i18n/locales/en.json";
import { ComparisonView } from "./ComparisonView.js";

beforeAll(() => { initI18n({ en }); });

describe("ComparisonView", () => {
  it("renders title", () => {
    render(<ComparisonView />);
    expect(screen.getByText(/ENIAC.*vs.*Binary/i)).toBeInTheDocument();
  });

  it("shows both panels", () => {
    render(<ComparisonView />);
    expect(screen.getAllByText(/ENIAC — Decimal/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Modern — Binary/).length).toBeGreaterThan(0);
  });

  it("defaults to 42 + 75 = 117", () => {
    render(<ComparisonView />);
    // ENIAC panel shows "0117" (zero-padded), binary panel shows "117"
    expect(screen.getByText("0117")).toBeInTheDocument();
    expect(screen.getByText("117")).toBeInTheDocument();
  });

  it("shows comparison table", () => {
    render(<ComparisonView />);
    expect(screen.getAllByText(/Pulse counting/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/XOR \+ AND \+ carry/).length).toBeGreaterThan(0);
  });

  it("shows tube count comparison (40 vs 14)", () => {
    render(<ComparisonView />);
    expect(screen.getAllByText("40").length).toBeGreaterThan(0);
    expect(screen.getAllByText("14").length).toBeGreaterThan(0);
  });

  it("shows binary bits for the result", () => {
    render(<ComparisonView />);
    const bits = document.querySelectorAll(".comp-bit");
    expect(bits.length).toBe(14); // 14-bit binary
  });

  it("updates when operands change", () => {
    render(<ComparisonView />);
    const inputs = document.querySelectorAll(".comp-input");
    fireEvent.change(inputs[0], { target: { value: "100" } });
    fireEvent.change(inputs[1], { target: { value: "200" } });
    // Binary panel shows 300, ENIAC panel shows 0300
    expect(screen.getByText("300")).toBeInTheDocument();
    expect(screen.getByText("0300")).toBeInTheDocument();
  });

  it("shows educational insight about von Neumann", () => {
    render(<ComparisonView />);
    expect(screen.getByText(/von Neumann/)).toBeInTheDocument();
  });
});
