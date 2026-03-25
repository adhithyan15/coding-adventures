import { describe, it, expect, beforeAll } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../../i18n/locales/en.json";
import { TriodeSwitch } from "./TriodeSwitch.js";

beforeAll(() => { initI18n({ en }); });

describe("TriodeSwitch", () => {
  it("renders title", () => {
    render(<TriodeSwitch />);
    expect(screen.getByText(/Triode/)).toBeInTheDocument();
  });

  it("shows conducting state at default grid voltage (0V)", () => {
    render(<TriodeSwitch />);
    const readout = document.querySelector(".triode-readout__value--on");
    expect(readout?.textContent).toMatch(/Conducting/i);
  });

  it("shows plate current", () => {
    render(<TriodeSwitch />);
    expect(screen.getByText(/mA/)).toBeInTheDocument();
  });

  it("has a grid voltage slider", () => {
    render(<TriodeSwitch />);
    const slider = screen.getByRole("slider");
    expect(slider).toBeInTheDocument();
  });

  it("switching to negative voltage shows cutoff", () => {
    render(<TriodeSwitch />);
    const slider = screen.getByRole("slider");
    fireEvent.change(slider, { target: { value: "-15" } });
    expect(screen.getByText(/Cutoff/i)).toBeInTheDocument();
  });

  it("shows educational insight", () => {
    render(<TriodeSwitch />);
    expect(screen.getByText(/MOSFET/i)).toBeInTheDocument();
  });
});
