/**
 * ViewRenderer.test.tsx — Unit tests for the ViewRenderer routing component.
 *
 * ViewRenderer is a "router inside the views engine": given a viewId, it
 * looks up the view in the store, reads config.type, and renders the right
 * component:
 *   "list"     → TaskList
 *   "kanban"   → KanbanView
 *   "calendar" → CalendarViewWrapper
 *
 * We mock three things:
 *   1. The store module — so we control which views are returned
 *   2. The child renderer components — so tests are fast and isolated
 *   3. The state module — so store.dispatch calls don't fail
 *
 * Each test verifies:
 *   - The correct child component is rendered
 *   - The "view not found" fallback works for unknown viewIds
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import "@testing-library/jest-dom";
import React from "react";

// ── Module mocks ─────────────────────────────────────────────────────────────
//
// We mock the store, state module, and all three child renderers.
// This way ViewRenderer can be tested in pure isolation.

vi.mock("@coding-adventures/store", () => ({
  useStore: vi.fn(),
}));

// Use real strings — the t() function just returns strings from the JSON catalog.
// No mock needed; importing it directly works fine in the test environment.

vi.mock("../state.js", () => ({
  store: { getState: vi.fn(), dispatch: vi.fn(), subscribe: vi.fn(), use: vi.fn() },
}));

vi.mock("../components/TaskList.js", () => ({
  TaskList: ({ onNavigate: _onNavigate }: { onNavigate: () => void }) => (
    <div data-testid="todo-list">TaskList</div>
  ),
}));

vi.mock("../components/KanbanView.js", () => ({
  KanbanView: () => <div data-testid="kanban-view">KanbanView</div>,
}));

vi.mock("../components/CalendarViewWrapper.js", () => ({
  CalendarViewWrapper: () => <div data-testid="calendar-view-wrapper">CalendarViewWrapper</div>,
}));

// ── Import after mocks ────────────────────────────────────────────────────────

import { ViewRenderer } from "../components/ViewRenderer.js";
import { useStore } from "@coding-adventures/store";

// ── Fixtures ─────────────────────────────────────────────────────────────────

const DEFAULT_FILTER = {
  statusFilter: null,
  priorityFilter: null,
  categoryFilter: "",
  searchQuery: "",
};

function makeListView(id: string) {
  return {
    id,
    name: "All Tasks",
    config: { type: "list" as const, filter: DEFAULT_FILTER, sortField: "createdAt" as const, sortDirection: "desc" as const },
    sortOrder: 0,
    isBuiltIn: true,
    createdAt: 0,
    updatedAt: 0,
  };
}

function makeKanbanView(id: string) {
  return {
    id,
    name: "Board",
    config: { type: "kanban" as const, filter: DEFAULT_FILTER, groupByField: "status" as const },
    sortOrder: 1,
    isBuiltIn: true,
    createdAt: 0,
    updatedAt: 0,
  };
}

function makeCalendarView(id: string) {
  return {
    id,
    name: "This Month",
    config: {
      type: "calendar" as const,
      filter: DEFAULT_FILTER,
      dateField: "dueDate" as const,
      granularity: "month" as const,
      dateRange: { type: "rolling" as const, unit: "month" as const, count: 1 },
      calendarId: "gregorian",
    },
    sortOrder: 2,
    isBuiltIn: true,
    createdAt: 0,
    updatedAt: 0,
  };
}

/**
 * makeAppState — builds a minimal AppState for mocking useStore.
 */
function makeAppState(views: ReturnType<typeof makeListView | typeof makeKanbanView | typeof makeCalendarView>[]) {
  return {
    tasks: [],
    views,
    calendars: [],
    activeViewId: views[0]?.id ?? "",
  };
}

const mockUseStore = vi.mocked(useStore);
const noop = () => {};

beforeEach(() => {
  vi.clearAllMocks();
});

// ── Tests ─────────────────────────────────────────────────────────────────────

describe("ViewRenderer", () => {
  // ── List view ──────────────────────────────────────────────────────────────

  it("renders TaskList for a list view config", () => {
    const listView = makeListView("all-tasks");
    mockUseStore.mockReturnValue(makeAppState([listView]));

    render(<ViewRenderer viewId="all-tasks" onNavigate={noop} />);
    expect(screen.getByTestId("todo-list")).toBeInTheDocument();
  });

  it("does not render KanbanView or CalendarViewWrapper for a list view config", () => {
    const listView = makeListView("all-tasks");
    mockUseStore.mockReturnValue(makeAppState([listView]));

    render(<ViewRenderer viewId="all-tasks" onNavigate={noop} />);
    expect(screen.queryByTestId("kanban-view")).not.toBeInTheDocument();
    expect(screen.queryByTestId("calendar-view-wrapper")).not.toBeInTheDocument();
  });

  // ── Kanban view ────────────────────────────────────────────────────────────

  it("renders KanbanView for a kanban view config", () => {
    const kanbanView = makeKanbanView("board");
    mockUseStore.mockReturnValue(makeAppState([kanbanView]));

    render(<ViewRenderer viewId="board" onNavigate={noop} />);
    expect(screen.getByTestId("kanban-view")).toBeInTheDocument();
  });

  it("does not render TaskList or CalendarViewWrapper for a kanban view", () => {
    const kanbanView = makeKanbanView("board");
    mockUseStore.mockReturnValue(makeAppState([kanbanView]));

    render(<ViewRenderer viewId="board" onNavigate={noop} />);
    expect(screen.queryByTestId("todo-list")).not.toBeInTheDocument();
    expect(screen.queryByTestId("calendar-view-wrapper")).not.toBeInTheDocument();
  });

  // ── Calendar view ──────────────────────────────────────────────────────────

  it("renders CalendarViewWrapper for a calendar view config", () => {
    const calView = makeCalendarView("this-month");
    mockUseStore.mockReturnValue(makeAppState([calView]));

    render(<ViewRenderer viewId="this-month" onNavigate={noop} />);
    expect(screen.getByTestId("calendar-view-wrapper")).toBeInTheDocument();
  });

  it("does not render TaskList or KanbanView for a calendar view", () => {
    const calView = makeCalendarView("this-month");
    mockUseStore.mockReturnValue(makeAppState([calView]));

    render(<ViewRenderer viewId="this-month" onNavigate={noop} />);
    expect(screen.queryByTestId("todo-list")).not.toBeInTheDocument();
    expect(screen.queryByTestId("kanban-view")).not.toBeInTheDocument();
  });

  // ── View not found ─────────────────────────────────────────────────────────

  it("renders 'View not found' for an unknown viewId", () => {
    mockUseStore.mockReturnValue(makeAppState([]));

    render(<ViewRenderer viewId="does-not-exist" onNavigate={noop} />);
    expect(screen.getByText("View not found")).toBeInTheDocument();
  });

  it("shows a generic 'doesn't exist' message (does not reflect URL input)", () => {
    mockUseStore.mockReturnValue(makeAppState([]));

    render(<ViewRenderer viewId="my-deleted-view" onNavigate={noop} />);
    // The error message should NOT reflect the raw viewId from the URL
    expect(screen.queryByText(/my-deleted-view/)).not.toBeInTheDocument();
    expect(screen.getByText(/doesn't exist or has been removed/)).toBeInTheDocument();
  });

  it("renders 'Go to All Tasks' button in the not-found state", () => {
    mockUseStore.mockReturnValue(makeAppState([]));

    render(<ViewRenderer viewId="stale-id" onNavigate={noop} />);
    expect(screen.getByRole("button", { name: "Go to All Tasks" })).toBeInTheDocument();
  });

  it("clicking 'Go to All Tasks' calls onNavigate with /view/all-tasks", () => {
    mockUseStore.mockReturnValue(makeAppState([]));
    const mockNavigate = vi.fn();

    render(<ViewRenderer viewId="stale-id" onNavigate={mockNavigate} />);
    screen.getByRole("button", { name: "Go to All Tasks" }).click();
    expect(mockNavigate).toHaveBeenCalledWith("/view/all-tasks");
  });

  // ── Multiple views in store ────────────────────────────────────────────────

  it("renders the correct view when store has multiple views", () => {
    const views = [
      makeListView("all-tasks"),
      makeKanbanView("board"),
      makeCalendarView("this-month"),
    ];
    mockUseStore.mockReturnValue(makeAppState(views));

    // Request the calendar view specifically
    render(<ViewRenderer viewId="this-month" onNavigate={noop} />);
    expect(screen.getByTestId("calendar-view-wrapper")).toBeInTheDocument();
    expect(screen.queryByTestId("todo-list")).not.toBeInTheDocument();
    expect(screen.queryByTestId("kanban-view")).not.toBeInTheDocument();
  });
});
