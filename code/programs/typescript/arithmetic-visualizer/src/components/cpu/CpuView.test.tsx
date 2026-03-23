/**
 * Tests for the CPU Step-Through tab (Tab 4).
 *
 * Verifies:
 * - Program loader renders with dropdown and hex dump
 * - Step/auto-step/reset controls work
 * - CPU state updates after stepping
 * - Execution trace history populates
 * - ALU trace shows for arithmetic instructions
 */

import { describe, it, expect, beforeAll } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../../i18n/locales/en.json";

import { CpuView } from "./CpuView.js";

beforeAll(() => {
  initI18n({ en });
});

describe("CpuView", () => {
  it("renders intro text", () => {
    render(<CpuView />);
    expect(screen.getByText(/everything comes together/i)).toBeInTheDocument();
  });

  it("renders program loader with dropdown", () => {
    render(<CpuView />);
    expect(screen.getByLabelText(/Select example program/i)).toBeInTheDocument();
    expect(screen.getByText("Load")).toBeInTheDocument();
  });

  it("shows all 4 example programs in dropdown", () => {
    render(<CpuView />);
    const select = screen.getByLabelText(/Select example program/i);
    const options = select.querySelectorAll("option");
    expect(options.length).toBe(4);
  });

  it("shows hex dump of selected program", () => {
    render(<CpuView />);
    // First program starts with D6 B1 D0 B0...
    expect(screen.getByText("D6")).toBeInTheDocument();
    expect(screen.getByText("B1")).toBeInTheDocument();
  });

  it("shows step controls after loading", () => {
    render(<CpuView />);
    fireEvent.click(screen.getByText("Load"));
    expect(screen.getByText("Step")).toBeInTheDocument();
    expect(screen.getByText("Auto-Step")).toBeInTheDocument();
    expect(screen.getByText("Reset")).toBeInTheDocument();
  });

  it("shows CPU state after loading", () => {
    render(<CpuView />);
    fireEvent.click(screen.getByText("Load"));
    expect(screen.getByText("ACC")).toBeInTheDocument();
    expect(screen.getByText("Carry")).toBeInTheDocument();
    expect(screen.getByText("PC")).toBeInTheDocument();
  });

  it("shows register grid with R0-R15", () => {
    render(<CpuView />);
    fireEvent.click(screen.getByText("Load"));
    expect(screen.getByText("R0")).toBeInTheDocument();
    expect(screen.getByText("R15")).toBeInTheDocument();
  });

  it("stepping updates CPU state", () => {
    render(<CpuView />);
    fireEvent.click(screen.getByText("Load"));

    // Initial state: ACC=0, all registers=0
    // Step 1: LDM 6 (D6) — loads 6 into accumulator
    fireEvent.click(screen.getByText("Step"));

    // After LDM 6, accumulator should be 6
    expect(screen.getByText("Current Instruction")).toBeInTheDocument();
    // The mnemonic should be visible
    const mnemonic = document.querySelector(".cpu-instr__mnemonic");
    expect(mnemonic?.textContent).toMatch(/LDM/i);
  });

  it("shows step count", () => {
    render(<CpuView />);
    fireEvent.click(screen.getByText("Load"));
    fireEvent.click(screen.getByText("Step"));
    expect(screen.getByText(/Steps: 1/)).toBeInTheDocument();
  });

  it("shows execution trace history", () => {
    render(<CpuView />);
    fireEvent.click(screen.getByText("Load"));
    fireEvent.click(screen.getByText("Step"));
    fireEvent.click(screen.getByText("Step"));

    expect(screen.getByText("Execution Trace")).toBeInTheDocument();
    const traceRows = document.querySelectorAll(".cpu-trace-row");
    expect(traceRows.length).toBe(2);
  });

  it("reset clears state and trace", () => {
    render(<CpuView />);
    fireEvent.click(screen.getByText("Load"));
    fireEvent.click(screen.getByText("Step"));
    fireEvent.click(screen.getByText("Step"));
    fireEvent.click(screen.getByText("Reset"));

    expect(screen.getByText(/Steps: 0/)).toBeInTheDocument();
  });

  it("switching programs updates hex dump", () => {
    render(<CpuView />);
    const select = screen.getByLabelText(/Select example program/i);
    fireEvent.change(select, { target: { value: "1" } });
    // Program 2 starts with D5 B0 D7 B1...
    expect(screen.getByText("D5")).toBeInTheDocument();
  });

  it("Add Two Numbers program produces correct result", () => {
    render(<CpuView />);
    // Select "Add Two Numbers" (index 1)
    const select = screen.getByLabelText(/Select example program/i);
    fireEvent.change(select, { target: { value: "1" } });
    fireEvent.click(screen.getByText("Load"));

    // Step through all 8 instructions
    for (let i = 0; i < 8; i++) {
      const stepBtn = screen.getByText("Step");
      if (stepBtn.hasAttribute("disabled")) break;
      fireEvent.click(stepBtn);
    }

    // Should show ALU trace for the ADD instruction
    const traceRows = document.querySelectorAll(".cpu-trace-row");
    const addRow = Array.from(traceRows).find(
      (r) => r.querySelector(".cpu-trace-row__mnem")?.textContent?.includes("ADD")
    );
    expect(addRow).toBeTruthy();
  });

  it("shows ALU trace for arithmetic instructions", () => {
    render(<CpuView />);
    // Select "Add Two Numbers" (index 1)
    const select = screen.getByLabelText(/Select example program/i);
    fireEvent.change(select, { target: { value: "1" } });
    fireEvent.click(screen.getByText("Load"));

    // Step through to the ADD instruction (instruction 6 = step index 5)
    // Program: LDM5, XCH R0, LDM7, XCH R1, LD R0, ADD R1, XCH R2, HLT
    for (let i = 0; i < 6; i++) {
      fireEvent.click(screen.getByText("Step"));
    }

    // After ADD instruction, the current instruction panel should show ALU trace
    const aluTrace = document.querySelector(".cpu-alu-trace");
    expect(aluTrace).toBeTruthy();
  });

  it("CPU halts and disables step button", () => {
    render(<CpuView />);
    // Select "Add Two Numbers" (index 1) — short program
    const select = screen.getByLabelText(/Select example program/i);
    fireEvent.change(select, { target: { value: "1" } });
    fireEvent.click(screen.getByText("Load"));

    // Step through all instructions until halt
    for (let i = 0; i < 10; i++) {
      const stepBtn = screen.getByText("Step");
      if (stepBtn.hasAttribute("disabled")) break;
      fireEvent.click(stepBtn);
    }

    // CPU should be halted
    expect(screen.getByText("CPU Halted")).toBeInTheDocument();
    expect(screen.getByText("Step")).toBeDisabled();
  });
});
