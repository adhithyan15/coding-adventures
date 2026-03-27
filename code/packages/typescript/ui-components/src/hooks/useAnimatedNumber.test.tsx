import { describe, expect, it, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useAnimatedNumber } from "./useAnimatedNumber.js";

describe("useAnimatedNumber", () => {
  it("starts at the target value initially", () => {
    const { result } = renderHook(() => useAnimatedNumber(5));
    expect(result.current).toBe(5);
  });

  it("animates toward the next value", () => {
    vi.useFakeTimers();

    let target = 1;
    const { result, rerender } = renderHook(() => useAnimatedNumber(target, 100));

    target = 3;
    rerender();

    act(() => {
      vi.advanceTimersByTime(50);
    });

    expect(result.current).toBeGreaterThan(1);
    expect(result.current).toBeLessThan(3);

    act(() => {
      vi.advanceTimersByTime(100);
    });

    expect(result.current).toBeCloseTo(3, 5);
    vi.useRealTimers();
  });
});
