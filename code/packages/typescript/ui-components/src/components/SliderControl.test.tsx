/**
 * Tests for the SliderControl component.
 *
 * SliderControl renders a labeled range input with value display.
 * These tests verify:
 *
 *   1. The label text renders and is linked to the input via htmlFor/id
 *   2. The range input has correct min/max/step/value attributes
 *   3. The current value is displayed, optionally with a unit
 *   4. Custom formatValue functions are respected
 *   5. Full ARIA support for assistive technology
 *   6. The onChange callback fires with the numeric value
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { SliderControl } from "./SliderControl.js";

/* ── Helpers ──────────────────────────────────────────────────────── */

function renderSlider(
  overrides: Partial<Parameters<typeof SliderControl>[0]> = {},
) {
  const onChange = vi.fn();
  const result = render(
    <SliderControl
      label="Gate Voltage"
      value={1.5}
      min={0}
      max={3.3}
      step={0.1}
      onChange={onChange}
      {...overrides}
    />,
  );
  return { onChange, ...result };
}

/* ── Tests ────────────────────────────────────────────────────────── */

describe("SliderControl", () => {
  /* ── Label ─────────────────────────────────────────────────────── */

  it("renders the label text", () => {
    renderSlider();
    expect(screen.getByText("Gate Voltage")).toBeDefined();
  });

  it("links the label to the input via htmlFor/id", () => {
    renderSlider();
    const label = screen.getByText("Gate Voltage");
    const input = screen.getByRole("slider");
    /**
     * The htmlFor on the <label> and the id on the <input> must match.
     * React's useId() generates the id, so we just check they agree.
     */
    expect(label.getAttribute("for")).toBe(input.id);
  });

  /* ── Range input attributes ────────────────────────────────────── */

  it("renders range input with correct min/max/step/value", () => {
    renderSlider({ min: 0, max: 5, step: 0.5, value: 2.5 });
    const input = screen.getByRole("slider") as HTMLInputElement;
    expect(input.type).toBe("range");
    expect(input.min).toBe("0");
    expect(input.max).toBe("5");
    expect(input.step).toBe("0.5");
    expect(input.value).toBe("2.5");
  });

  /* ── Value display ─────────────────────────────────────────────── */

  it("displays the value formatted with toFixed(2) by default", () => {
    renderSlider({ value: 1.5 });
    /**
     * toFixed(2) on 1.5 produces "1.50".
     * The output element should contain this text.
     */
    expect(screen.getByText("1.50")).toBeDefined();
  });

  it("displays the unit when provided", () => {
    renderSlider({ unit: "V" });
    expect(screen.getByText("V")).toBeDefined();
  });

  it("does not render a unit span when unit is empty", () => {
    const { container } = renderSlider({ unit: "" });
    const unitSpan = container.querySelector(".slider-control__unit");
    expect(unitSpan).toBeNull();
  });

  it("uses custom formatValue function when provided", () => {
    /**
     * A custom formatter that rounds to the nearest integer and
     * adds a prefix — useful for displaying percentages, for example.
     */
    renderSlider({
      value: 2.789,
      formatValue: (v) => `~${Math.round(v)}`,
    });
    expect(screen.getByText("~3")).toBeDefined();
  });

  /* ── ARIA attributes ───────────────────────────────────────────── */

  it("renders ARIA value attributes on the input", () => {
    renderSlider({ min: 0, max: 10, value: 5 });
    const input = screen.getByRole("slider");
    expect(input.getAttribute("aria-valuemin")).toBe("0");
    expect(input.getAttribute("aria-valuemax")).toBe("10");
    expect(input.getAttribute("aria-valuenow")).toBe("5");
  });

  /* ── onChange callback ─────────────────────────────────────────── */

  it("calls onChange with the numeric value when the slider is moved", () => {
    const { onChange } = renderSlider();
    const input = screen.getByRole("slider");
    fireEvent.change(input, { target: { value: "2.5" } });
    expect(onChange).toHaveBeenCalledWith(2.5);
  });

  /* ── CSS class ─────────────────────────────────────────────────── */

  it("applies the className to the container div", () => {
    const { container } = renderSlider({ className: "custom-slider" });
    expect(container.firstElementChild!.className).toBe("custom-slider");
  });
});
