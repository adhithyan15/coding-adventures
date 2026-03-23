/**
 * Tests for the useTabs hook.
 *
 * useTabs provides the keyboard navigation logic and ARIA attribute
 * generation for accessible tab interfaces. It separates the behavior
 * from the presentation, so it can be used with any tab UI.
 *
 * These tests use renderHook from @testing-library/react to test the
 * hook in isolation without rendering a full component.
 */

import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useTabs } from "./useTabs.js";

/* ── Test data ────────────────────────────────────────────────────── */

const items = [
  { id: "one", label: "One" },
  { id: "two", label: "Two" },
  { id: "three", label: "Three" },
];

/* ── Tests ────────────────────────────────────────────────────────── */

describe("useTabs", () => {
  /* ── getTabProps ────────────────────────────────────────────────── */

  it("returns getTabProps that produces correct ARIA attributes", () => {
    const { result } = renderHook(() =>
      useTabs({ items, activeTab: "one", onActiveChange: vi.fn() }),
    );

    const props = result.current.getTabProps("one");

    /**
     * Every tab button must have role="tab" for screen readers
     * to recognize it as part of a tab interface.
     */
    expect(props.role).toBe("tab");
    expect(props["aria-controls"]).toBe("panel-one");
    expect(props.tabIndex).toBe(0);
    expect(props["aria-selected"]).toBe(true);
  });

  it("marks the active tab as selected and others as not", () => {
    const { result } = renderHook(() =>
      useTabs({ items, activeTab: "two", onActiveChange: vi.fn() }),
    );

    /**
     * Only the active tab should have aria-selected=true and tabIndex=0.
     * All others get aria-selected=false and tabIndex=-1.
     * This is the "roving tabindex" pattern.
     */
    const activeProps = result.current.getTabProps("two");
    expect(activeProps["aria-selected"]).toBe(true);
    expect(activeProps.tabIndex).toBe(0);

    const inactiveProps = result.current.getTabProps("one");
    expect(inactiveProps["aria-selected"]).toBe(false);
    expect(inactiveProps.tabIndex).toBe(-1);
  });

  it("getTabProps onClick calls onActiveChange with the tab id", () => {
    const onActiveChange = vi.fn();
    const { result } = renderHook(() =>
      useTabs({ items, activeTab: "one", onActiveChange }),
    );

    const props = result.current.getTabProps("two");
    act(() => {
      props.onClick();
    });
    expect(onActiveChange).toHaveBeenCalledWith("two");
  });

  /* ── handleKeyDown ─────────────────────────────────────────────── */

  it("ArrowRight advances to the next tab", () => {
    const onActiveChange = vi.fn();
    const { result } = renderHook(() =>
      useTabs({ items, activeTab: "one", onActiveChange }),
    );

    /**
     * Simulating a keyboard event. We create a minimal event-like object
     * with the key and preventDefault — enough for the handler to process.
     */
    act(() => {
      result.current.handleKeyDown({
        key: "ArrowRight",
        preventDefault: vi.fn(),
      } as unknown as React.KeyboardEvent);
    });

    expect(onActiveChange).toHaveBeenCalledWith("two");
  });

  it("ArrowLeft moves to the previous tab", () => {
    const onActiveChange = vi.fn();
    const { result } = renderHook(() =>
      useTabs({ items, activeTab: "two", onActiveChange }),
    );

    act(() => {
      result.current.handleKeyDown({
        key: "ArrowLeft",
        preventDefault: vi.fn(),
      } as unknown as React.KeyboardEvent);
    });

    expect(onActiveChange).toHaveBeenCalledWith("one");
  });

  it("Home moves to the first tab", () => {
    const onActiveChange = vi.fn();
    const { result } = renderHook(() =>
      useTabs({ items, activeTab: "three", onActiveChange }),
    );

    act(() => {
      result.current.handleKeyDown({
        key: "Home",
        preventDefault: vi.fn(),
      } as unknown as React.KeyboardEvent);
    });

    expect(onActiveChange).toHaveBeenCalledWith("one");
  });

  it("End moves to the last tab", () => {
    const onActiveChange = vi.fn();
    const { result } = renderHook(() =>
      useTabs({ items, activeTab: "one", onActiveChange }),
    );

    act(() => {
      result.current.handleKeyDown({
        key: "End",
        preventDefault: vi.fn(),
      } as unknown as React.KeyboardEvent);
    });

    expect(onActiveChange).toHaveBeenCalledWith("three");
  });

  it("unrelated keys do not trigger onActiveChange", () => {
    const onActiveChange = vi.fn();
    const { result } = renderHook(() =>
      useTabs({ items, activeTab: "one", onActiveChange }),
    );

    act(() => {
      result.current.handleKeyDown({
        key: "Enter",
        preventDefault: vi.fn(),
      } as unknown as React.KeyboardEvent);
    });

    expect(onActiveChange).not.toHaveBeenCalled();
  });
});
