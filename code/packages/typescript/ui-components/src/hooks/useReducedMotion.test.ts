/**
 * Tests for the useReducedMotion hook.
 *
 * This hook checks the `prefers-reduced-motion: reduce` media query
 * and keeps its return value in sync when the user's preference changes.
 *
 * Testing media queries requires mocking window.matchMedia, which
 * jsdom does not implement by default. We provide a minimal mock
 * that supports both initial state and change events.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useReducedMotion } from "./useReducedMotion.js";

/* ── matchMedia mock ─────────────────────────────────────────────── */

/**
 * We need to mock matchMedia because jsdom doesn't implement it.
 * The mock tracks event listeners so we can simulate preference changes.
 */
let changeHandler: ((e: { matches: boolean }) => void) | null = null;
let mockMatches = false;

function createMatchMediaMock(matches: boolean) {
  return (query: string) => ({
    matches,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: (_event: string, handler: (e: { matches: boolean }) => void) => {
      changeHandler = handler;
    },
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  });
}

/* ── Tests ────────────────────────────────────────────────────────── */

describe("useReducedMotion", () => {
  const originalMatchMedia = window.matchMedia;

  beforeEach(() => {
    changeHandler = null;
    mockMatches = false;
  });

  afterEach(() => {
    /**
     * Restore the original matchMedia after each test to avoid
     * leaking mock state between tests.
     */
    window.matchMedia = originalMatchMedia;
  });

  it("returns false when prefers-reduced-motion is not set", () => {
    /**
     * Default case: the user has no motion preference.
     * matchMedia("(prefers-reduced-motion: reduce)").matches is false.
     */
    window.matchMedia = createMatchMediaMock(false) as unknown as typeof window.matchMedia;
    const { result } = renderHook(() => useReducedMotion());
    expect(result.current).toBe(false);
  });

  it("returns true when prefers-reduced-motion is reduce", () => {
    /**
     * The user has explicitly requested reduced motion in their OS settings.
     * Animations should be disabled or replaced with static alternatives.
     */
    window.matchMedia = createMatchMediaMock(true) as unknown as typeof window.matchMedia;
    const { result } = renderHook(() => useReducedMotion());
    expect(result.current).toBe(true);
  });

  it("updates when the media query changes", () => {
    /**
     * The user can change their motion preference while the app is running
     * (e.g., toggling "Reduce motion" in System Preferences). The hook
     * subscribes to change events so it stays in sync.
     */
    window.matchMedia = createMatchMediaMock(false) as unknown as typeof window.matchMedia;
    const { result } = renderHook(() => useReducedMotion());

    expect(result.current).toBe(false);

    /* Simulate the user enabling reduced motion */
    act(() => {
      if (changeHandler) {
        changeHandler({ matches: true });
      }
    });

    expect(result.current).toBe(true);
  });
});
