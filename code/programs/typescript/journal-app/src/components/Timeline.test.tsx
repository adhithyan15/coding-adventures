/**
 * Timeline.test.tsx — Component tests for the Timeline screen.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../i18n/locales/en.json";
import { store } from "../state.js";
import { ENTRIES_LOAD, ENTRY_CREATE } from "../actions.js";
import { Timeline } from "./Timeline.js";

// Initialise i18n and reset store state before each test
beforeEach(() => {
  initI18n({ en });
  store.dispatch({ type: ENTRIES_LOAD, entries: [] });
});

describe("Timeline", () => {
  it("renders empty state message when no entries", () => {
    render(<Timeline />);
    expect(screen.getByText(/no entries yet/i)).toBeTruthy();
  });

  it("renders 'New Entry' button", () => {
    render(<Timeline />);
    expect(screen.getByText(/new entry/i)).toBeTruthy();
  });

  it("renders entry cards when entries exist", () => {
    store.dispatch({
      type: ENTRY_CREATE,
      id: "e1",
      title: "My First Entry",
      content: "Hello world",
      createdAt: "2026-04-02",
      updatedAt: Date.now(),
    });

    render(<Timeline />);
    expect(screen.getByText("My First Entry")).toBeTruthy();
  });

  it("groups entries by date", () => {
    store.dispatch({
      type: ENTRY_CREATE,
      id: "e1",
      title: "Day One",
      content: "",
      createdAt: "2026-04-01",
      updatedAt: 1000,
    });
    store.dispatch({
      type: ENTRY_CREATE,
      id: "e2",
      title: "Day Two",
      content: "",
      createdAt: "2026-04-02",
      updatedAt: 2000,
    });

    render(<Timeline />);
    expect(screen.getByText("Day One")).toBeTruthy();
    expect(screen.getByText("Day Two")).toBeTruthy();
  });
});
