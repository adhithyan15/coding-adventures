import { describe, it, expect } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useGridKeyboard } from "./useGridKeyboard.js";

describe("useGridKeyboard", () => {
  const defaultOptions = { rowCount: 3, colCount: 4 };

  it("initializes focused cell at (0, 0)", () => {
    const { result } = renderHook(() => useGridKeyboard(defaultOptions));
    expect(result.current.focusedCell).toEqual({ row: 0, col: 0 });
  });

  it("getCellTabIndex returns 0 for focused cell, -1 for others", () => {
    const { result } = renderHook(() => useGridKeyboard(defaultOptions));
    expect(result.current.getCellTabIndex(0, 0)).toBe(0);
    expect(result.current.getCellTabIndex(0, 1)).toBe(-1);
    expect(result.current.getCellTabIndex(1, 0)).toBe(-1);
  });

  it("setFocusedCell updates the focused cell", () => {
    const { result } = renderHook(() => useGridKeyboard(defaultOptions));
    act(() => {
      result.current.setFocusedCell({ row: 1, col: 2 });
    });
    expect(result.current.focusedCell).toEqual({ row: 1, col: 2 });
    expect(result.current.getCellTabIndex(1, 2)).toBe(0);
    expect(result.current.getCellTabIndex(0, 0)).toBe(-1);
  });

  describe("clamp behavior", () => {
    it("does not go below row 0", () => {
      const { result } = renderHook(() => useGridKeyboard(defaultOptions));
      // Focus is at (0,0), pressing ArrowUp should stay at row 0
      const event = createKeyEvent("ArrowUp");
      act(() => {
        result.current.onKeyDown(event);
      });
      expect(result.current.focusedCell.row).toBe(0);
    });

    it("does not go below col 0", () => {
      const { result } = renderHook(() => useGridKeyboard(defaultOptions));
      const event = createKeyEvent("ArrowLeft");
      act(() => {
        result.current.onKeyDown(event);
      });
      expect(result.current.focusedCell.col).toBe(0);
    });

    it("does not exceed max row", () => {
      const { result } = renderHook(() => useGridKeyboard(defaultOptions));
      // Move to last row
      act(() => {
        result.current.setFocusedCell({ row: 2, col: 0 });
      });
      const event = createKeyEvent("ArrowDown");
      act(() => {
        result.current.onKeyDown(event);
      });
      expect(result.current.focusedCell.row).toBe(2);
    });

    it("does not exceed max col", () => {
      const { result } = renderHook(() => useGridKeyboard(defaultOptions));
      // Move to last col
      act(() => {
        result.current.setFocusedCell({ row: 0, col: 3 });
      });
      const event = createKeyEvent("ArrowRight");
      act(() => {
        result.current.onKeyDown(event);
      });
      expect(result.current.focusedCell.col).toBe(3);
    });
  });

  describe("Home / End", () => {
    it("Home moves to first column in current row", () => {
      const { result } = renderHook(() => useGridKeyboard(defaultOptions));
      act(() => {
        result.current.setFocusedCell({ row: 1, col: 3 });
      });
      const event = createKeyEvent("Home");
      act(() => {
        result.current.onKeyDown(event);
      });
      expect(result.current.focusedCell).toEqual({ row: 1, col: 0 });
    });

    it("End moves to last column in current row", () => {
      const { result } = renderHook(() => useGridKeyboard(defaultOptions));
      act(() => {
        result.current.setFocusedCell({ row: 1, col: 0 });
      });
      const event = createKeyEvent("End");
      act(() => {
        result.current.onKeyDown(event);
      });
      expect(result.current.focusedCell).toEqual({ row: 1, col: 3 });
    });

    it("Ctrl+Home moves to first cell in grid", () => {
      const { result } = renderHook(() => useGridKeyboard(defaultOptions));
      act(() => {
        result.current.setFocusedCell({ row: 2, col: 3 });
      });
      const event = createKeyEvent("Home", { ctrlKey: true });
      act(() => {
        result.current.onKeyDown(event);
      });
      expect(result.current.focusedCell).toEqual({ row: 0, col: 0 });
    });

    it("Ctrl+End moves to last cell in grid", () => {
      const { result } = renderHook(() => useGridKeyboard(defaultOptions));
      const event = createKeyEvent("End", { ctrlKey: true });
      act(() => {
        result.current.onKeyDown(event);
      });
      expect(result.current.focusedCell).toEqual({ row: 2, col: 3 });
    });
  });

  it("prevents default on handled keys", () => {
    const { result } = renderHook(() => useGridKeyboard(defaultOptions));
    const event = createKeyEvent("ArrowRight");
    act(() => {
      result.current.onKeyDown(event);
    });
    expect(event.preventDefault).toHaveBeenCalled();
  });

  it("does not prevent default on unhandled keys", () => {
    const { result } = renderHook(() => useGridKeyboard(defaultOptions));
    const event = createKeyEvent("Tab");
    act(() => {
      result.current.onKeyDown(event);
    });
    expect(event.preventDefault).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Creates a minimal keyboard event object that satisfies the onKeyDown
 * handler. We can't use fireEvent here because the hook is tested in
 * isolation without a DOM element.
 */
function createKeyEvent(
  key: string,
  opts: { ctrlKey?: boolean; metaKey?: boolean } = {},
) {
  return {
    key,
    ctrlKey: opts.ctrlKey ?? false,
    metaKey: opts.metaKey ?? false,
    preventDefault: vi.fn(),
    currentTarget: {
      querySelector: vi.fn(() => null),
    },
  } as unknown as React.KeyboardEvent;
}

// vi is imported globally by vitest
import { vi } from "vitest";
import type React from "react";
