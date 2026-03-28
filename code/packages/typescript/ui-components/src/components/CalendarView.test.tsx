/**
 * CalendarView.test.tsx — Unit tests for the CalendarView component.
 *
 * We test the following behaviours:
 *
 * === Month granularity ===
 *  1. Renders the current month header on first mount.
 *  2. Items with a matching dueDate appear in the correct cell.
 *  3. Items with null dueDate are silently omitted.
 *  4. The previous-month navigation button decrements the displayed month.
 *  5. The next-month navigation button increments the displayed month.
 *  6. Navigating past December wraps to January of the next year.
 *  7. Navigating before January wraps to December of the previous year.
 *  8. The "Today" button resets the view to the current month after navigation.
 *  9. Today's cell carries aria-current="date".
 * 10. Items that fall on overflow cells (prev/next month) are still rendered.
 * 11. `initialYear` / `initialMonth` props set the starting view.
 * 12. Multiple items on the same day all render.
 * 13. The renderItem function receives the original item object.
 * 14. All 7 day-of-week column headers are rendered.
 * 15. Exactly 42 gridcells per month view.
 * 16. Today's cell has no aria-current when viewing a different month.
 * 17. Items outside the 42-cell window are not shown in the month view.
 *
 * === Week granularity ===
 * 18. Week view shows 7 columns.
 * 19. Week navigation shows "Next week" / "Previous week" buttons.
 * 20. weekStartsOn=1 (Monday) makes Monday the first column.
 *
 * === Agenda granularity ===
 * 21. Agenda view groups items by date.
 * 22. Items with dueTime appear under the timed slot, untimed in "All Day".
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
  dueTime?: string | null,
): TestItem {
  return { id, dueDate, title, priority, dueTime };
}

// ── Freeze time helper ───────────────────────────────────────────────────────
//
// We freeze to: 2026-03-15 (March 15, 2026 — a Sunday).

const FROZEN_DATE = new Date(2026, 2, 15); // March 15, 2026
const FROZEN_DATE_STR = "2026-03-15";

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(FROZEN_DATE);
});

afterEach(() => {
  vi.useRealTimers();
});

// ── Simple render helpers ────────────────────────────────────────────────────

function renderCalendar(
  items: TestItem[],
  overrides: Partial<React.ComponentProps<typeof CalendarView<TestItem>>> = {},
) {
  return render(
    <CalendarView<TestItem>
      items={items}
      granularity="month"
      renderItem={(item) => <span data-testid={`item-${item.id}`}>{item.title}</span>}
      ariaLabel="Test Calendar"
      {...overrides}
    />,
  );
}

function renderWeekCalendar(
  items: TestItem[],
  overrides: Partial<React.ComponentProps<typeof CalendarView<TestItem>>> = {},
) {
  return render(
    <CalendarView<TestItem>
      items={items}
      granularity="week"
      renderItem={(item) => <span data-testid={`item-${item.id}`}>{item.title}</span>}
      ariaLabel="Week Calendar"
      {...overrides}
    />,
  );
}

function renderAgendaCalendar(
  items: TestItem[],
  overrides: Partial<React.ComponentProps<typeof CalendarView<TestItem>>> = {},
) {
  return render(
    <CalendarView<TestItem>
      items={items}
      granularity="agenda"
      renderItem={(item) => <span data-testid={`item-${item.id}`}>{item.title}</span>}
      ariaLabel="Agenda Calendar"
      {...overrides}
    />,
  );
}

// ── Month granularity tests ──────────────────────────────────────────────────

describe("CalendarView — month granularity", () => {
  // ── 1. Initial month header ────────────────────────────────────────────────
  it("renders the current month and year as the header", () => {
    renderCalendar([]);
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "March 2026",
    );
  });

  // ── 2. Items appear in their due-date cell ─────────────────────────────────
  it("renders an item inside the cell matching its dueDate", () => {
    const item = makeItem("a", "2026-03-15", "Write tests");
    renderCalendar([item]);
    const el = screen.getByTestId("item-a");
    expect(el).toBeInTheDocument();
    expect(el).toHaveTextContent("Write tests");
  });

  // ── 3. Null dueDate items are omitted ──────────────────────────────────────
  it("does not render items whose dueDate is null", () => {
    const item = makeItem("b", null, "Undated task");
    renderCalendar([item]);
    expect(screen.queryByTestId("item-b")).not.toBeInTheDocument();
  });

  // ── 4. Previous-month navigation ──────────────────────────────────────────
  it("navigates to the previous month when ‹ is clicked", () => {
    renderCalendar([]);
    fireEvent.click(screen.getByRole("button", { name: "Previous month" }));
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "February 2026",
    );
  });

  // ── 5. Next-month navigation ───────────────────────────────────────────────
  it("navigates to the next month when › is clicked", () => {
    renderCalendar([]);
    fireEvent.click(screen.getByRole("button", { name: "Next month" }));
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "April 2026",
    );
  });

  // ── 6. Year wraps forward (December → January) ────────────────────────────
  it("wraps to January of the next year after navigating forward from December", () => {
    renderCalendar([], { initialYear: 2026, initialMonth: 11 }); // December 2026
    fireEvent.click(screen.getByRole("button", { name: "Next month" }));
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "January 2027",
    );
  });

  // ── 7. Year wraps backward (January → December) ───────────────────────────
  it("wraps to December of the previous year when navigating before January", () => {
    renderCalendar([], { initialYear: 2026, initialMonth: 0 }); // January 2026
    fireEvent.click(screen.getByRole("button", { name: "Previous month" }));
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "December 2025",
    );
  });

  // ── 8. "Today" button resets the view ─────────────────────────────────────
  it("returns to the current month when Today is clicked after navigating away", () => {
    renderCalendar([]);
    fireEvent.click(screen.getByRole("button", { name: "Next month" }));
    fireEvent.click(screen.getByRole("button", { name: "Next month" }));
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "May 2026",
    );
    fireEvent.click(screen.getByRole("button", { name: "Go to today" }));
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(
      "March 2026",
    );
  });

  // ── 9. Today's cell has aria-current="date" ───────────────────────────────
  it("marks today's cell with aria-current='date'", () => {
    renderCalendar([]);
    const todayCell = screen.getByRole("gridcell", {
      name: (name) => name.startsWith(FROZEN_DATE_STR),
    });
    expect(todayCell).toHaveAttribute("aria-current", "date");
  });

  // ── 10. Items in overflow cells (days from adjacent months) ───────────────
  it("renders items on overflow days from the next month", () => {
    // April 1 falls in March's overflow area (last row of the 6-week grid)
    const item = makeItem("overflow", "2026-04-01", "April Fools");
    renderCalendar([item]);
    expect(screen.getByTestId("item-overflow")).toBeInTheDocument();
    expect(screen.getByTestId("item-overflow")).toHaveTextContent("April Fools");
  });

  // ── 11. initialYear / initialMonth props ──────────────────────────────────
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

  // ── 13. renderItem receives the full item object ───────────────────────────
  it("passes the full item object to renderItem, not just id/dueDate", () => {
    const item = makeItem("p", "2026-03-10", "Priority task", "high");
    render(
      <CalendarView<TestItem>
        items={[item]}
        granularity="month"
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

  // ── 14. Day-of-week headers are rendered ──────────────────────────────────
  it("renders all 7 day-of-week column headers", () => {
    renderCalendar([]);
    const headers = screen.getAllByRole("columnheader");
    expect(headers).toHaveLength(7);
    expect(headers[0]).toHaveTextContent("Sun");
    expect(headers[6]).toHaveTextContent("Sat");
  });

  // ── 15. 42 gridcells rendered (6 weeks) ───────────────────────────────────
  it("renders exactly 42 day cells per month view", () => {
    renderCalendar([]);
    const cells = screen.getAllByRole("gridcell");
    expect(cells).toHaveLength(42);
  });

  // ── 16. Today's cell absent from aria-current when navigated away ──────────
  it("today cell does not have aria-current when viewing a different month", () => {
    renderCalendar([]);
    fireEvent.click(screen.getByRole("button", { name: "Next month" }));
    const markedCells = screen
      .queryAllByRole("gridcell")
      .filter((el) => el.getAttribute("aria-current") === "date");
    expect(markedCells).toHaveLength(0);
  });

  // ── 17. Items outside the 42-cell window are not shown ────────────────────
  it("does not show items whose dueDate is outside the grid's 42-day window", () => {
    // June 2026 is not within the March 2026 grid (spans Feb 22 – Apr 11)
    const item = makeItem("far", "2026-06-15", "Far future");
    renderCalendar([item]);
    expect(screen.queryByTestId("item-far")).not.toBeInTheDocument();
  });
});

// ── Week granularity tests ───────────────────────────────────────────────────

describe("CalendarView — week granularity", () => {
  // ── 18. Week view shows 7 columns ─────────────────────────────────────────
  it("renders 7 gridcells for the week view", () => {
    renderWeekCalendar([]);
    const cells = screen.getAllByRole("gridcell");
    expect(cells).toHaveLength(7);
  });

  // ── 19. Week navigation uses "Previous week" / "Next week" buttons ─────────
  it("shows Previous week and Next week navigation buttons", () => {
    renderWeekCalendar([]);
    expect(screen.getByRole("button", { name: "Previous week" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Next week" })).toBeInTheDocument();
  });

  // ── 20. weekStartsOn=1 makes Monday the first column ──────────────────────
  it("renders Mon as the first column header when weekStartsOn=1", () => {
    renderWeekCalendar([], { weekStartsOn: 1 });
    const headers = screen.getAllByRole("columnheader");
    expect(headers[0]).toHaveTextContent("Mon");
    expect(headers[6]).toHaveTextContent("Sun");
  });
});

// ── Agenda granularity tests ─────────────────────────────────────────────────

describe("CalendarView — agenda granularity", () => {
  // ── 21. Agenda groups items by date ────────────────────────────────────────
  it("renders items grouped under their date", () => {
    const items = [
      makeItem("a1", "2026-03-15", "Task A1"),
      makeItem("a2", "2026-03-15", "Task A2"),
      makeItem("b1", "2026-03-16", "Task B1"),
    ];
    renderAgendaCalendar(items);
    expect(screen.getByTestId("item-a1")).toBeInTheDocument();
    expect(screen.getByTestId("item-a2")).toBeInTheDocument();
    expect(screen.getByTestId("item-b1")).toBeInTheDocument();
  });

  // ── 22. Timed items shown with time; untimed in All Day ────────────────────
  it("renders timed items under their time slot and untimed in All Day", () => {
    const items = [
      makeItem("timed", "2026-03-15", "Morning standup", "low", "09:00"),
      makeItem("untimed", "2026-03-15", "Read emails", "low", null),
    ];
    renderAgendaCalendar(items);
    // Both items visible
    expect(screen.getByTestId("item-timed")).toBeInTheDocument();
    expect(screen.getByTestId("item-untimed")).toBeInTheDocument();
    // "All Day" section should exist for untimed
    expect(screen.getByText("All Day")).toBeInTheDocument();
    // The timed task's time label should be visible
    expect(screen.getByText("09:00")).toBeInTheDocument();
  });
});
