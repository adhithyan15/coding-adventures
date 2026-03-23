/**
 * Tests for Sequential Logic components — SR Latch, D Flip-Flop, Counter.
 *
 * Verifies:
 *   1. SR Latch: set/reset/hold/forbidden states
 *   2. D Flip-Flop: clock pulse captures data
 *   3. Counter: step increments, reset returns to 0
 *   4. Accessibility: ARIA labels, button types
 */

import { describe, it, expect, beforeAll, vi, afterEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import { initI18n } from "@coding-adventures/ui-components";
import { SrLatchDiagram } from "./SrLatchDiagram.js";
import { DFlipFlopDiagram } from "./DFlipFlopDiagram.js";
import { CounterView } from "./CounterView.js";
import { SequentialLogic } from "./SequentialLogic.js";

import en from "../../i18n/locales/en.json";

beforeAll(() => {
  initI18n({ en });
});

afterEach(() => {
  vi.useRealTimers();
});

// =========================================================================
// SR Latch
// =========================================================================

describe("SrLatchDiagram — SR Latch", () => {
  it("renders with title", () => {
    render(<SrLatchDiagram />);
    expect(screen.getByText(/SR Latch/)).toBeTruthy();
  });

  it("initial state: S=0, R=0 → Hold state, Q=0", () => {
    render(<SrLatchDiagram />);
    expect(screen.getByLabelText(/Q: 0/)).toBeTruthy();
    // "Holding" appears in the state indicator
    expect(screen.getByText(/Holding/i)).toBeTruthy();
  });

  it("setting S=1 sets Q=1", () => {
    render(<SrLatchDiagram />);
    const toggleS = screen.getByLabelText(/Input S/);
    fireEvent.click(toggleS); // S=1, R=0 → Set
    expect(screen.getByLabelText(/Q: 1/)).toBeTruthy();
  });

  it("setting S=1 then S=0 holds Q=1", () => {
    render(<SrLatchDiagram />);
    const toggleS = screen.getByLabelText(/Input S/);
    fireEvent.click(toggleS); // S=1 → Set, Q=1
    fireEvent.click(toggleS); // S=0 → Hold, Q should stay 1
    expect(screen.getByLabelText(/Q: 1/)).toBeTruthy();
  });

  it("S=1 then R=1 shows forbidden state", () => {
    render(<SrLatchDiagram />);
    const toggleS = screen.getByLabelText(/Input S/);
    const toggleR = screen.getByLabelText(/Input R/);
    fireEvent.click(toggleS);
    fireEvent.click(toggleR);
    // "Forbidden" appears in state indicator and truth table — use getAllByText
    const forbiddenElements = screen.getAllByText(/Forbidden/i);
    expect(forbiddenElements.length).toBeGreaterThanOrEqual(1);
  });

  it("has accessible SVG", () => {
    render(<SrLatchDiagram />);
    const svg = screen.getByRole("img");
    expect(svg.getAttribute("aria-label")).toContain("SR");
  });

  it("shows truth table with actions", () => {
    render(<SrLatchDiagram />);
    // Truth table has Hold, Set, Reset, Forbidden as action column values
    const holdCells = screen.getAllByText("Hold");
    expect(holdCells.length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("Set")).toBeTruthy();
    expect(screen.getByText("Reset")).toBeTruthy();
  });
});

// =========================================================================
// D Flip-Flop
// =========================================================================

describe("DFlipFlopDiagram — D Flip-Flop", () => {
  it("renders with title", () => {
    render(<DFlipFlopDiagram />);
    expect(screen.getByText(/D Flip-Flop/)).toBeTruthy();
  });

  it("initial state: Q=0", () => {
    render(<DFlipFlopDiagram />);
    expect(screen.getByLabelText(/Q: 0/)).toBeTruthy();
  });

  it("pulsing clock with D=0 keeps Q=0", () => {
    render(<DFlipFlopDiagram />);
    const pulseBtn = screen.getByLabelText(/Pulse clock/);
    fireEvent.click(pulseBtn);
    expect(screen.getByLabelText(/Q: 0/)).toBeTruthy();
  });

  it("setting D=1 then pulsing clock captures Q=1", () => {
    render(<DFlipFlopDiagram />);
    const toggleD = screen.getByLabelText(/Input D/);
    const pulseBtn = screen.getByLabelText(/Pulse clock/);

    fireEvent.click(toggleD); // D=1
    fireEvent.click(pulseBtn); // Rising edge captures D
    expect(screen.getByLabelText(/Q: 1/)).toBeTruthy();
  });

  it("shows last capture info", () => {
    render(<DFlipFlopDiagram />);
    const toggleD = screen.getByLabelText(/Input D/);
    const pulseBtn = screen.getByLabelText(/Pulse clock/);

    fireEvent.click(toggleD);
    fireEvent.click(pulseBtn);
    expect(screen.getByText(/D=1 → Q=1/)).toBeTruthy();
  });

  it("has master and slave labels", () => {
    render(<DFlipFlopDiagram />);
    expect(screen.getByText("MASTER")).toBeTruthy();
    expect(screen.getByText("SLAVE")).toBeTruthy();
  });
});

// =========================================================================
// Counter
// =========================================================================

describe("CounterView — 4-bit Counter", () => {
  it("renders with title", () => {
    render(<CounterView />);
    expect(screen.getByText(/Counter/)).toBeTruthy();
  });

  it("initial value is 0", () => {
    render(<CounterView />);
    const decimalValue = document.querySelector(".counter-display__decimal-value");
    expect(decimalValue?.textContent).toBe("0");
    expect(screen.getByText("/ 15")).toBeTruthy();
  });

  it("stepping increments the counter", () => {
    render(<CounterView />);
    const stepBtn = screen.getByLabelText(/Step/);
    const getDecimal = () => document.querySelector(".counter-display__decimal-value")?.textContent;

    fireEvent.click(stepBtn); // 0 → 1
    expect(getDecimal()).toBe("1");

    fireEvent.click(stepBtn); // 1 → 2
    expect(getDecimal()).toBe("2");

    fireEvent.click(stepBtn); // 2 → 3
    expect(getDecimal()).toBe("3");
  });

  it("reset returns counter to 0", () => {
    render(<CounterView />);
    const stepBtn = screen.getByLabelText(/Step/);
    const resetBtn = screen.getByLabelText(/Reset/);

    fireEvent.click(stepBtn); // 0 → 1
    fireEvent.click(stepBtn); // 1 → 2
    fireEvent.click(resetBtn); // → 0

    // Should find a decimal value of 0 in the display
    const decimalValue = document.querySelector(".counter-display__decimal-value");
    expect(decimalValue?.textContent).toBe("0");
  });

  it("auto-step button toggles", () => {
    render(<CounterView />);
    const autoBtn = screen.getByLabelText(/Auto/);
    expect(autoBtn.getAttribute("aria-pressed")).toBe("false");

    fireEvent.click(autoBtn);
    expect(autoBtn.getAttribute("aria-pressed")).toBe("true");
  });

  it("auto-step increments over time", () => {
    vi.useFakeTimers();
    render(<CounterView />);
    const autoBtn = screen.getByLabelText(/Auto/);

    fireEvent.click(autoBtn); // Start auto

    act(() => {
      vi.advanceTimersByTime(1500); // 3 intervals at 500ms
    });

    const decimalValue = document.querySelector(".counter-display__decimal-value");
    const val = parseInt(decimalValue?.textContent ?? "0", 10);
    expect(val).toBeGreaterThan(0);
  });
});

// =========================================================================
// Container
// =========================================================================

describe("SequentialLogic — Tab 4 container", () => {
  it("renders all three circuit components", () => {
    render(<SequentialLogic />);
    expect(screen.getByText(/SR Latch/)).toBeTruthy();
    expect(screen.getByText(/D Flip-Flop/)).toBeTruthy();
    expect(screen.getByText(/Counter/)).toBeTruthy();
  });
});
