import { describe, it, expect, beforeAll } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../../i18n/locales/en.json";
import { AccumulatorView } from "./AccumulatorView.js";

beforeAll(() => { initI18n({ en }); });

describe("AccumulatorView", () => {
  it("renders title", () => {
    render(<AccumulatorView />);
    expect(screen.getByText(/ENIAC Accumulator/)).toBeInTheDocument();
  });

  it("starts at 0042", () => {
    render(<AccumulatorView />);
    expect(screen.getByText("0042")).toBeInTheDocument();
  });

  it("shows 4 decades", () => {
    render(<AccumulatorView />);
    expect(screen.getByText("ones")).toBeInTheDocument();
    expect(screen.getByText("tens")).toBeInTheDocument();
    expect(screen.getByText("hundreds")).toBeInTheDocument();
    expect(screen.getByText("thousands")).toBeInTheDocument();
  });

  it("adding produces correct result: 42 + 75 = 117", () => {
    render(<AccumulatorView />);
    fireEvent.click(screen.getByText("Add"));
    expect(screen.getByText("0117")).toBeInTheDocument();
  });

  it("shows trace table after addition", () => {
    render(<AccumulatorView />);
    fireEvent.click(screen.getByText("Add"));
    expect(screen.getByText("Addition Trace")).toBeInTheDocument();
  });

  it("trace shows carry for tens digit", () => {
    render(<AccumulatorView />);
    fireEvent.click(screen.getByText("Add"));
    // Tens digit carries: 4+7=11
    const carryRows = document.querySelectorAll(".eniac-table__row--carry");
    expect(carryRows.length).toBeGreaterThan(0);
  });

  it("reset clears accumulator to 0000", () => {
    render(<AccumulatorView />);
    fireEvent.click(screen.getByText("Add"));
    fireEvent.click(screen.getByText("Reset"));
    expect(screen.getByText("0000")).toBeInTheDocument();
  });

  it("shows 40 tubes total (4 decades × 10 tubes)", () => {
    render(<AccumulatorView />);
    const tubes = document.querySelectorAll(".tube");
    expect(tubes.length).toBe(40);
  });

  it("shows educational insight about 550 tubes per accumulator", () => {
    render(<AccumulatorView />);
    expect(screen.getByText(/550 vacuum tubes/)).toBeInTheDocument();
  });
});
