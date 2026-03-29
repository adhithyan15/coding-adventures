import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useColumnResize, MIN_COL_WIDTH } from "./useColumnResize.js";
import type { KeyboardEvent } from "react";

describe("useColumnResize", () => {
  describe("getColumnWidth", () => {
    it("returns the default width when no resize has occurred", () => {
      const { result } = renderHook(() => useColumnResize());
      expect(result.current.getColumnWidth("col1", 200)).toBe(200);
    });

    it("returns the overridden width after resize", () => {
      const { result } = renderHook(() => useColumnResize());

      // Simulate a drag: start, move via document events, end
      act(() => {
        result.current.startResize("col1", 100, 200);
      });

      // Simulate mousemove via document
      act(() => {
        document.dispatchEvent(
          new MouseEvent("mousemove", { clientX: 150 }),
        );
      });

      // Width should be 200 + (150 - 100) = 250
      expect(result.current.getColumnWidth("col1", 200)).toBe(250);

      // End drag
      act(() => {
        document.dispatchEvent(new MouseEvent("mouseup"));
      });

      // Width should persist after mouseup
      expect(result.current.getColumnWidth("col1", 200)).toBe(250);
    });
  });

  describe("minimum width enforcement", () => {
    it("clamps width to MIN_COL_WIDTH when dragging too far left", () => {
      const { result } = renderHook(() => useColumnResize());

      act(() => {
        result.current.startResize("col1", 300, 100);
      });

      // Drag far to the left: 300 - 500 = -200 delta, so 100 + (-200) = -100
      // Should clamp to MIN_COL_WIDTH
      act(() => {
        document.dispatchEvent(
          new MouseEvent("mousemove", { clientX: -200 }),
        );
      });

      expect(result.current.getColumnWidth("col1", 100)).toBe(MIN_COL_WIDTH);

      act(() => {
        document.dispatchEvent(new MouseEvent("mouseup"));
      });
    });
  });

  describe("isResizing", () => {
    it("is false initially", () => {
      const { result } = renderHook(() => useColumnResize());
      expect(result.current.isResizing).toBe(false);
    });

    it("is true during a drag", () => {
      const { result } = renderHook(() => useColumnResize());

      act(() => {
        result.current.startResize("col1", 100, 200);
      });

      expect(result.current.isResizing).toBe(true);

      act(() => {
        document.dispatchEvent(new MouseEvent("mouseup"));
      });

      expect(result.current.isResizing).toBe(false);
    });
  });

  describe("onColumnResize callback", () => {
    it("fires on mouseup with final width", () => {
      const onResize = vi.fn();
      const { result } = renderHook(() =>
        useColumnResize({ onColumnResize: onResize }),
      );

      act(() => {
        result.current.startResize("col1", 100, 200);
      });

      act(() => {
        document.dispatchEvent(
          new MouseEvent("mousemove", { clientX: 180 }),
        );
      });

      act(() => {
        document.dispatchEvent(new MouseEvent("mouseup"));
      });

      expect(onResize).toHaveBeenCalledWith("col1", 280);
    });
  });

  describe("RTL support", () => {
    it("flips drag delta in RTL mode", () => {
      const { result } = renderHook(() =>
        useColumnResize({ direction: "rtl" }),
      );

      act(() => {
        result.current.startResize("col1", 300, 200);
      });

      // In RTL, dragging left (decreasing clientX) should widen the column
      act(() => {
        document.dispatchEvent(
          new MouseEvent("mousemove", { clientX: 250 }),
        );
      });

      // RTL delta = -(250 - 300) = 50, so width = 200 + 50 = 250
      expect(result.current.getColumnWidth("col1", 200)).toBe(250);

      act(() => {
        document.dispatchEvent(new MouseEvent("mouseup"));
      });
    });
  });

  describe("keyboard resize", () => {
    it("increases width on ArrowRight in LTR", () => {
      const onResize = vi.fn();
      const { result } = renderHook(() =>
        useColumnResize({ onColumnResize: onResize }),
      );

      act(() => {
        result.current.handleResizeKeyDown(
          "col1",
          200,
          createKeyEvent("ArrowRight"),
        );
      });

      expect(result.current.getColumnWidth("col1", 200)).toBe(210);
      expect(onResize).toHaveBeenCalledWith("col1", 210);
    });

    it("decreases width on ArrowLeft in LTR", () => {
      const { result } = renderHook(() => useColumnResize());

      act(() => {
        result.current.handleResizeKeyDown(
          "col1",
          200,
          createKeyEvent("ArrowLeft"),
        );
      });

      expect(result.current.getColumnWidth("col1", 200)).toBe(190);
    });

    it("uses coarse step (50px) with Shift+Arrow", () => {
      const { result } = renderHook(() => useColumnResize());

      act(() => {
        result.current.handleResizeKeyDown(
          "col1",
          200,
          createKeyEvent("ArrowRight", { shiftKey: true }),
        );
      });

      expect(result.current.getColumnWidth("col1", 200)).toBe(250);
    });

    it("clamps to minimum width on keyboard resize", () => {
      const { result } = renderHook(() => useColumnResize());

      act(() => {
        result.current.handleResizeKeyDown(
          "col1",
          MIN_COL_WIDTH,
          createKeyEvent("ArrowLeft"),
        );
      });

      expect(result.current.getColumnWidth("col1", MIN_COL_WIDTH)).toBe(
        MIN_COL_WIDTH,
      );
    });

    it("flips arrow keys in RTL mode", () => {
      const { result } = renderHook(() =>
        useColumnResize({ direction: "rtl" }),
      );

      // In RTL, ArrowRight should narrow (negative delta)
      act(() => {
        result.current.handleResizeKeyDown(
          "col1",
          200,
          createKeyEvent("ArrowRight"),
        );
      });

      expect(result.current.getColumnWidth("col1", 200)).toBe(190);

      // ArrowLeft should widen in RTL
      act(() => {
        result.current.handleResizeKeyDown(
          "col1",
          190,
          createKeyEvent("ArrowLeft"),
        );
      });

      // 190 + 10 = 200 (but it reads from columnWidths which has 190)
      expect(result.current.getColumnWidth("col1", 200)).toBe(200);
    });

    it("does not prevent default on unhandled keys", () => {
      const { result } = renderHook(() => useColumnResize());
      const event = createKeyEvent("Tab");

      act(() => {
        result.current.handleResizeKeyDown("col1", 200, event);
      });

      expect(event.preventDefault).not.toHaveBeenCalled();
    });
  });

  describe("cleanup", () => {
    it("removes document listeners on unmount", () => {
      const removeListenerSpy = vi.spyOn(document, "removeEventListener");
      const { result, unmount } = renderHook(() => useColumnResize());

      act(() => {
        result.current.startResize("col1", 100, 200);
      });

      unmount();

      expect(removeListenerSpy).toHaveBeenCalledWith(
        "mousemove",
        expect.any(Function),
      );
      expect(removeListenerSpy).toHaveBeenCalledWith(
        "mouseup",
        expect.any(Function),
      );

      removeListenerSpy.mockRestore();
    });
  });
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function createKeyEvent(
  key: string,
  opts: { shiftKey?: boolean } = {},
): KeyboardEvent {
  return {
    key,
    shiftKey: opts.shiftKey ?? false,
    preventDefault: vi.fn(),
  } as unknown as KeyboardEvent;
}
