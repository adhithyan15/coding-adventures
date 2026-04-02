/**
 * EntryView.test.tsx — Component tests for the read-only entry view.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../i18n/locales/en.json";
import { store } from "../state.js";
import { ENTRIES_LOAD, ENTRY_CREATE } from "../actions.js";
import { EntryView } from "./EntryView.js";

beforeEach(() => {
  initI18n({ en });
  store.dispatch({ type: ENTRIES_LOAD, entries: [] });
  store.dispatch({
    type: ENTRY_CREATE,
    id: "view-1",
    title: "Viewable Entry",
    content: "**Bold content** here",
    createdAt: "2026-04-02",
    updatedAt: 1000,
  });
});

describe("EntryView", () => {
  it("renders the entry title", () => {
    render(<EntryView entryId="view-1" />);
    expect(screen.getByText("Viewable Entry")).toBeTruthy();
  });

  it("renders the entry content as HTML", () => {
    render(<EntryView entryId="view-1" />);
    // The GFM pipeline should render **Bold content** as <strong>
    const container = document.querySelector(".entry-view__content");
    expect(container?.innerHTML).toContain("<strong>");
  });

  it("shows Edit and Delete buttons", () => {
    render(<EntryView entryId="view-1" />);
    expect(screen.getByText(/edit/i)).toBeTruthy();
    expect(screen.getByText(/delete/i)).toBeTruthy();
  });

  it("shows not-found message for invalid entryId", () => {
    render(<EntryView entryId="nonexistent" />);
    expect(screen.getByText(/entry not found/i)).toBeTruthy();
  });

  it("shows a Back button", () => {
    render(<EntryView entryId="view-1" />);
    expect(screen.getByText(/back/i)).toBeTruthy();
  });
});
