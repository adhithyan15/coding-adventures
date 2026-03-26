import { describe, it, expect, beforeAll } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../../i18n/locales/en.json";
import { RingCounterView } from "./RingCounterView.js";

beforeAll(() => { initI18n({ en }); });

describe("RingCounterView", () => {
  it("renders title", () => {
    render(<RingCounterView />);
    expect(screen.getByText(/Decade Ring Counter/)).toBeInTheDocument();
  });

  it("shows 10 tube indicators", () => {
    render(<RingCounterView />);
    const tubes = document.querySelectorAll(".tube");
    expect(tubes.length).toBe(10);
  });

  it("starts at digit 0", () => {
    render(<RingCounterView />);
    const digitDisplay = document.querySelector(".ring-digit__value");
    expect(digitDisplay?.textContent).toBe("0");
  });

  it("pulse button advances digit", () => {
    render(<RingCounterView />);
    fireEvent.click(screen.getByText("+1 Pulse"));
    const digitDisplay = document.querySelector(".ring-digit__value");
    expect(digitDisplay?.textContent).toBe("1");
  });

  it("shows carry when wrapping 9→0", () => {
    render(<RingCounterView />);
    // Set to 9 via dropdown
    const select = screen.getByLabelText(/Set Digit/i);
    fireEvent.change(select, { target: { value: "9" } });
    // Pulse to wrap
    fireEvent.click(screen.getByText("+1 Pulse"));
    expect(screen.getByText("CARRY!")).toBeInTheDocument();
  });

  it("shows no carry for normal increment", () => {
    render(<RingCounterView />);
    fireEvent.click(screen.getByText("+1 Pulse"));
    expect(screen.getByText(/No carry/)).toBeInTheDocument();
  });

  it("set digit dropdown works", () => {
    render(<RingCounterView />);
    const select = screen.getByLabelText(/Set Digit/i);
    fireEvent.change(select, { target: { value: "7" } });
    const digitDisplay = document.querySelector(".ring-digit__value");
    expect(digitDisplay?.textContent).toBe("7");
  });

  it("exactly one tube is on at a time", () => {
    render(<RingCounterView />);
    const onTubes = document.querySelectorAll(".tube--on");
    expect(onTubes.length).toBe(1);
  });

  it("shows tube count note", () => {
    render(<RingCounterView />);
    expect(screen.getAllByText(/10 vacuum tubes/i).length).toBeGreaterThan(0);
  });
});
