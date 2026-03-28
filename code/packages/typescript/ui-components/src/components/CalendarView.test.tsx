/**
 * CalendarView.test.tsx — Unit tests for the CalendarView component.
 *
 * We test the following behaviours:
 *
 *  1. Renders the current month header on first mount.
 *  2. Items with a matching dueDate appear in the correct cell.
 *  3. Items with null dueDate are silently omitted.
 *  4. The previous-month navigation button decrements the displayed month.
 *  5. The next-month navigation button increments the displayed month.
 *  6. Navigating past December wraps to January of the next year.
 *  7. Navigating before January wraps to December of the previous year.
 *  8. The "Today" button resets the view to the current month after navigation.
 *  9. Today's cell carries aria-current="date".
 * 10. Items that fall on days in the overflow area (prev/next month cells)
 *     are still rendered in those cells.
 * 11. `initialYear` / `initialMonth` props set the starting view.
 * 12. Multiple items on the same day all render.
 * 13. The renderItem function receives the original item object.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import React from "react";
import { CalendarView } from "./CalendarView.js";
import type { CalendarItem } from "./CalendarView.js";

// ── Test fixture type ────────────────────────────────────────────────────────

interface TestItem extends CalendarItem {
  title: string;
  priority: "low" | "high";
}

function makeItem(
  id: string,
  dueDate: string | null,
  title: string = `Task ${id}`,
  priority: "low" | "high" = "low",
): TestItem {
  return { id, dueDate, title, priority };
}

// ── Freeze time helper ───────────────────────────────────────────────────────
//
// The component calls `new Date()` on every render to determine "today".
// We freeze the clock to a known date so tests are deterministic regardless
// of when they run.
//
// We freeze to: 2026-03-15 (a Sunday — March 1st is a Sunday in 2026,
// and March 15 is also a Sunday, giving us a predictable grid layout).

const FROZEN_DATE = new Date(2026, 2, 15); // March 15, 2026 (month is 0-indexed)
const FROZEN_DATE_STR = "2026-03-15";

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(FROZEN_DATE);
});

afterEach(() => {
  vi.useRealTimers();
});

// ── Simple render helper ─────────────────────────────────────────────────────

function renderCalendar(
  items: TestItem[],
  overrides: Partial<React.ComponentProps<typeof CalendarView<TestItem>>> = {},
) {
  return render(
    <CalendarView<TestItem>
      items={items}
      renderItem={(item) => <span data-testid={`item-${item.id}`}>{item.title}</span>}
      ariaLabel="Test Calendar"
      {...overrides}
    />,
  );
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe("CalendarView", () => {
  // ── 1. Initial month header ──────────────────────────────────────────────
  it("renders the current month and year as the header", () => {
    renderCalendar([]);
    // The frozen date is March 15, 2026
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "March 2026",
    );
  });

  // ── 2. Items appear in their due-date cell ───────────────────────────────
  it("renders an item inside the cell matching its dueDate", () => {
    const item = makeItem("a", "2026-03-15", "Write tests");
    renderCalendar([item]);

    // The item should be visible
    const el = screen.getByTestId("item-a");
    expect(el).toBeInTheDocument();
    expect(el).toHaveTextContent("Write tests");
  });

  // ── 3. Null dueDate items are omitted ────────────────────────────────────
  it("does not render items whose dueDate is null", () => {
    const item = makeItem("b", null, "Undated task");
    renderCalendar([item]);
    expect(screen.queryByTestId("item-b")).not.toBeInTheDocument();
  });

  // ── 4. Previous-month navigation ─────────────────────────────────────────
  it("navigates to the previous month when ‹ is clicked", () => {
    renderCalendar([]);
    fireEvent.click(screen.getByRole("button", { name: "Previous month" }));
    // March 2026 → February 2026
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "February 2026",
    );
  });

  // ── 5. Next-month navigation ─────────────────────────────────────────────
  it("navigates to the next month when › is clicked", () => {
    renderCalendar([]);
    fireEvent.click(screen.getByRole("button", { name: "Next month" }));
    // March 2026 → April 2026
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "April 2026",
    );
  });

  // ── 6. Year wraps forward (December → January) ───────────────────────────
  it("wraps to January of the next year after clicking next 9 times from March", () => {
    renderCalendar([], { initialYear: 2026, initialMonth: 11 }); // December 2026
    fireEvent.click(screen.getByRole("button", { name: "Next month" }));
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "January 2027",
    );
  });

  // ── 7. Year wraps backward (January → December) ──────────────────────────
  it("wraps to December of the previous year when navigating before January", () => {
    renderCalendar([], { initialYear: 2026, initialMonth: 0 }); // January 2026
    fireEvent.click(screen.getByRole("button", { name: "Previous month" }));
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "December 2025",
    );
  });

  // ── 8. "Today" button resets the view ────────────────────────────────────
  it("returns to the current month when Today is clicked after navigating away", () => {
    renderCalendar([]);
    // Navigate away twice
    fireEvent.click(screen.getByRole("button", { name: "Next month" }));
    fireEvent.click(screen.getByRole("button", { name: "Next month" }));
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "May 2026",
    );
    // Jump back to today (March 2026)
    fireEvent.click(screen.getByRole("button", { name: "Go to today" }));
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "March 2026",
    );
  });

  // ── 9. Today's cell has aria-current="date" ───────────────────────────────
  it("marks today's cell with aria-current='date'", () => {
    renderCalendar([]);
    // Find the cell whose aria-label starts with "2026-03-15"
    const todayCell = screen.getByRole("gridcell", {
      name: (name) => name.startsWith(FROZEN_DATE_STR),
    });
    expect(todayCell).toHaveAttribute("aria-current", "date");
  });

  // ── 10. Items in overflow cells (days from adjacent months) ──────────────
  it("renders items on overflow days from the next month", () => {
    // April 1 is in March's overflow area (March has 31 days, last row has Apr 1–11)
    const item = makeItem("overflow", "2026-04-01", "April Fools");
    renderCalendar([item]);
    // Should still be rendered even though the cell is "other-month"
    expect(screen.getByTestId("item-overflow")).toBeInTheDocument();
    expect(screen.getByTestId("item-overflow")).toHaveTextContent("April Fools");
  });

  // ── 11. initialYear / initialMonth props ─────────────────────────────────
  it("respects initialYear and initialMonth props", () => {
    renderCalendar([], { initialYear: 2025, initialMonth: 6 }); // July 2025
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "July 2025",
    );
  });

  // ── 12. Multiple items on the same day all render ─────────────────────────
  it("renders multiple items on the same day", () => {
    const items = [
      makeItem("x", "2026-03-20", "Task X"),
      makeItem("y", "2026-03-20", "Task Y"),
      makeItem("z", "2026-03-20", "Task Z"),
    ];
    renderCalendar(items);
    expect(screen.getByTestId("item-x")).toBeInTheDocument();
    expect(screen.getByTestId("item-y")).toBeInTheDocument();
    expect(screen.getByTestId("item-z")).toBeInTheDocument();
  });

  // ── 13. renderItem receives the full item object ──────────────────────────
  it("passes the full item object to renderItem, not just id/dueDate", () => {
    // We render the priority field to verify the full object is passed
    const item = makeItem("p", "2026-03-10", "Priority task", "high");
    render(
      <CalendarView<TestItem>
        items={[item]}
        renderItem={(i) => (
          <span data-testid="rendered">
            {i.title}:{i.priority}
          </span>
        )}
        ariaLabel="Test"
      />,
    );
    expect(screen.getByTestId("rendered")).toHaveTextContent(
      "Priority task:high",
    );
  });

  // ── 14. Day-of-week headers are rendered ─────────────────────────────────
  it("renders all 7 day-of-week column headers", () => {
    renderCalendar([]);
    const headers = screen.getAllByRole("columnheader");
    expect(headers).toHaveLength(7);
    expect(headers[0]).toHaveTextContent("Sun");
    expect(headers[6]).toHaveTextContent("Sat");
  });

  // ── 15. 42 gridcells rendered (6 weeks) ──────────────────────────────────
  it("renders exactly 42 day cells per month view", () => {
    renderCalendar([]);
    const cells = screen.getAllByRole("gridcell");
    expect(cells).toHaveLength(42);
  });

  // ── 16. Today's cell is absent from aria-current when navigated away ──────
  it("today cell does not have aria-current when viewing a different month", () => {
    renderCalendar([]);
    // Navigate to a month that does not contain today
    fireEvent.click(screen.getByRole("button", { name: "Next month" }));
    // No cell should have aria-current="date"
    const markedCells = screen
      .queryAllByRole("gridcell")
      .filter((el) => el.getAttribute("aria-current") === "date");
    expect(markedCells).toHaveLength(0);
  });

  // ── 17. Items from a different month don't appear in current view ─────────
  it("does not show items whose dueDate is outside the grid's 42-day window", () => {
    // June 2026 is not within the March 2026 grid (which spans ~Feb 22 – Apr 4)
    const item = makeItem("far", "2026-06-15", "Far future");
    renderCalendar([item]);
    expect(screen.queryByTestId("item-far")).not.toBeInTheDocument();
  });
});
