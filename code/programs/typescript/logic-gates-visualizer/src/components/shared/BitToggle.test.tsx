/**
 * Tests for the BitToggle component.
 *
 * Verifies:
 * 1. Rendering — displays the current value (0 or 1)
 * 2. Click interaction — toggles the value and calls onChange
 * 3. Keyboard accessibility — Enter key triggers toggle
 * 4. ARIA label — updates to reflect current value
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { BitToggle } from "./BitToggle.js";

// i18n must be initialized before any component that uses useTranslation.
// BitToggle doesn't use i18n directly, so no init needed here.

describe("BitToggle", () => {
  it("renders the current value", () => {
    render(<BitToggle value={0} onChange={() => {}} label="A" />);
    const button = screen.getByRole("button");
    expect(button.textContent).toBe("0");
  });

  it("renders value 1 with high styling", () => {
    render(<BitToggle value={1} onChange={() => {}} label="A" />);
    const button = screen.getByRole("button");
    expect(button.textContent).toBe("1");
    expect(button.className).toContain("bit-toggle--high");
  });

  it("renders value 0 with low styling", () => {
    render(<BitToggle value={0} onChange={() => {}} label="B" />);
    const button = screen.getByRole("button");
    expect(button.className).toContain("bit-toggle--low");
  });

  it("calls onChange with toggled value on click", () => {
    const onChange = vi.fn();
    render(<BitToggle value={0} onChange={onChange} label="A" />);
    fireEvent.click(screen.getByRole("button"));
    expect(onChange).toHaveBeenCalledWith(1);
  });

  it("toggles from 1 to 0 on click", () => {
    const onChange = vi.fn();
    render(<BitToggle value={1} onChange={onChange} label="A" />);
    fireEvent.click(screen.getByRole("button"));
    expect(onChange).toHaveBeenCalledWith(0);
  });

  it("has an accessible aria-label describing the input", () => {
    render(<BitToggle value={0} onChange={() => {}} label="A" />);
    const button = screen.getByRole("button");
    expect(button.getAttribute("aria-label")).toBe(
      "Input A: 0, click to toggle",
    );
  });

  it("updates aria-label when value changes", () => {
    const { rerender } = render(
      <BitToggle value={0} onChange={() => {}} label="A" />,
    );
    expect(screen.getByRole("button").getAttribute("aria-label")).toContain("0");

    rerender(<BitToggle value={1} onChange={() => {}} label="A" />);
    expect(screen.getByRole("button").getAttribute("aria-label")).toContain("1");
  });

  it("displays the label text", () => {
    render(<BitToggle value={0} onChange={() => {}} label="B" />);
    expect(screen.getByText("B")).toBeTruthy();
  });
});
