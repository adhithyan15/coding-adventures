/**
 * Tests for the TabList component.
 *
 * The TabList implements the WAI-ARIA Tabs pattern, which defines how
 * keyboard-navigable tab interfaces should work. These tests verify:
 *
 *   1. Correct rendering of tab items with proper ARIA roles
 *   2. Roving tabindex — only the active tab is focusable via Tab key
 *   3. Full keyboard navigation (Arrow keys, Home, End)
 *   4. Wraparound behavior at the edges of the tab list
 *   5. CSS class application for styling hooks
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { TabList } from "./TabList.js";

/* ── Test data ────────────────────────────────────────────────────── */

const tabs = [
  { id: "alpha", label: "Alpha" },
  { id: "beta", label: "Beta" },
  { id: "gamma", label: "Gamma" },
];

/* ── Helpers ──────────────────────────────────────────────────────── */

/**
 * Renders a TabList with sensible defaults.
 * The onActiveChange callback is a vitest mock so we can inspect calls.
 */
function renderTabList(overrides: Partial<Parameters<typeof TabList>[0]> = {}) {
  const onActiveChange = vi.fn();
  const result = render(
    <TabList
      items={tabs}
      activeTab="alpha"
      onActiveChange={onActiveChange}
      ariaLabel="Test tabs"
      {...overrides}
    />,
  );
  return { onActiveChange, ...result };
}

/* ── Tests ────────────────────────────────────────────────────────── */

describe("TabList", () => {
  /* ── Rendering ─────────────────────────────────────────────────── */

  it("renders all tab items", () => {
    renderTabList();
    expect(screen.getByText("Alpha")).toBeDefined();
    expect(screen.getByText("Beta")).toBeDefined();
    expect(screen.getByText("Gamma")).toBeDefined();
  });

  it("has role='tablist' on the container", () => {
    renderTabList();
    const tablist = screen.getByRole("tablist");
    expect(tablist).toBeDefined();
  });

  it("each button has role='tab'", () => {
    renderTabList();
    const tabButtons = screen.getAllByRole("tab");
    expect(tabButtons).toHaveLength(3);
  });

  /* ── ARIA attributes ───────────────────────────────────────────── */

  it("sets aria-selected=true on the active tab", () => {
    renderTabList({ activeTab: "beta" });
    const betaTab = screen.getByText("Beta");
    expect(betaTab.getAttribute("aria-selected")).toBe("true");
  });

  it("sets aria-selected=false on inactive tabs", () => {
    renderTabList({ activeTab: "beta" });
    const alphaTab = screen.getByText("Alpha");
    const gammaTab = screen.getByText("Gamma");
    expect(alphaTab.getAttribute("aria-selected")).toBe("false");
    expect(gammaTab.getAttribute("aria-selected")).toBe("false");
  });

  it("uses aria-controls pointing to panel-{id}", () => {
    renderTabList();
    const tabButtons = screen.getAllByRole("tab");
    expect(tabButtons[0]!.getAttribute("aria-controls")).toBe("panel-alpha");
    expect(tabButtons[1]!.getAttribute("aria-controls")).toBe("panel-beta");
    expect(tabButtons[2]!.getAttribute("aria-controls")).toBe("panel-gamma");
  });

  /* ── Roving tabindex ───────────────────────────────────────────── */

  it("active tab has tabIndex 0", () => {
    renderTabList({ activeTab: "beta" });
    const betaTab = screen.getByText("Beta");
    expect(betaTab.tabIndex).toBe(0);
  });

  it("inactive tabs have tabIndex -1", () => {
    renderTabList({ activeTab: "beta" });
    const alphaTab = screen.getByText("Alpha");
    const gammaTab = screen.getByText("Gamma");
    expect(alphaTab.tabIndex).toBe(-1);
    expect(gammaTab.tabIndex).toBe(-1);
  });

  /* ── Click behavior ────────────────────────────────────────────── */

  it("clicking a tab calls onActiveChange with its id", () => {
    const { onActiveChange } = renderTabList({ activeTab: "alpha" });
    fireEvent.click(screen.getByText("Beta"));
    expect(onActiveChange).toHaveBeenCalledWith("beta");
  });

  /* ── Keyboard navigation ───────────────────────────────────────── */

  it("ArrowRight moves to the next tab", () => {
    const { onActiveChange } = renderTabList({ activeTab: "alpha" });
    const tablist = screen.getByRole("tablist");
    fireEvent.keyDown(tablist, { key: "ArrowRight" });
    expect(onActiveChange).toHaveBeenCalledWith("beta");
  });

  it("ArrowLeft moves to the previous tab", () => {
    const { onActiveChange } = renderTabList({ activeTab: "beta" });
    const tablist = screen.getByRole("tablist");
    fireEvent.keyDown(tablist, { key: "ArrowLeft" });
    expect(onActiveChange).toHaveBeenCalledWith("alpha");
  });

  it("Home moves to the first tab", () => {
    const { onActiveChange } = renderTabList({ activeTab: "gamma" });
    const tablist = screen.getByRole("tablist");
    fireEvent.keyDown(tablist, { key: "Home" });
    expect(onActiveChange).toHaveBeenCalledWith("alpha");
  });

  it("End moves to the last tab", () => {
    const { onActiveChange } = renderTabList({ activeTab: "alpha" });
    const tablist = screen.getByRole("tablist");
    fireEvent.keyDown(tablist, { key: "End" });
    expect(onActiveChange).toHaveBeenCalledWith("gamma");
  });

  /* ── Wraparound ────────────────────────────────────────────────── */

  it("ArrowRight from last tab wraps to first", () => {
    const { onActiveChange } = renderTabList({ activeTab: "gamma" });
    const tablist = screen.getByRole("tablist");
    fireEvent.keyDown(tablist, { key: "ArrowRight" });
    expect(onActiveChange).toHaveBeenCalledWith("alpha");
  });

  it("ArrowLeft from first tab wraps to last", () => {
    const { onActiveChange } = renderTabList({ activeTab: "alpha" });
    const tablist = screen.getByRole("tablist");
    fireEvent.keyDown(tablist, { key: "ArrowLeft" });
    expect(onActiveChange).toHaveBeenCalledWith("gamma");
  });

  /* ── CSS class application ─────────────────────────────────────── */

  it("applies className to the container", () => {
    renderTabList({ className: "my-tabs" });
    const tablist = screen.getByRole("tablist");
    expect(tablist.className).toBe("my-tabs");
  });

  it("applies activeTabClassName to the active tab", () => {
    renderTabList({
      activeTab: "beta",
      tabClassName: "tab",
      activeTabClassName: "tab--active",
    });
    const betaTab = screen.getByText("Beta");
    expect(betaTab.className).toContain("tab--active");
  });

  it("does not apply activeTabClassName to inactive tabs", () => {
    renderTabList({
      activeTab: "beta",
      tabClassName: "tab",
      activeTabClassName: "tab--active",
    });
    const alphaTab = screen.getByText("Alpha");
    expect(alphaTab.className).not.toContain("tab--active");
  });
});
