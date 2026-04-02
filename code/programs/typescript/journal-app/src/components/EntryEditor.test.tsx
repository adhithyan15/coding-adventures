/**
 * EntryEditor.test.tsx — Component tests for the split-pane editor.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../i18n/locales/en.json";
import { store } from "../state.js";
import { ENTRIES_LOAD, ENTRY_CREATE } from "../actions.js";
import { EntryEditor } from "./EntryEditor.js";

beforeEach(() => {
  initI18n({ en });
  store.dispatch({ type: ENTRIES_LOAD, entries: [] });
});

describe("EntryEditor", () => {
  it("renders title input and textarea for new entry", () => {
    render(<EntryEditor />);
    expect(screen.getByPlaceholderText(/entry title/i)).toBeTruthy();
    expect(screen.getByPlaceholderText(/write your journal/i)).toBeTruthy();
  });

  it("shows 'New Entry' heading when creating", () => {
    render(<EntryEditor />);
    expect(screen.getByText(/new entry/i)).toBeTruthy();
  });

  it("disables save button when title is empty", () => {
    render(<EntryEditor />);
    const saveButton = screen.getByText(/save/i);
    expect(saveButton).toHaveProperty("disabled", true);
  });

  it("enables save button when title is provided", () => {
    render(<EntryEditor />);
    const titleInput = screen.getByPlaceholderText(/entry title/i);
    fireEvent.change(titleInput, { target: { value: "My Entry" } });
    const saveButton = screen.getByText(/save/i);
    expect(saveButton).toHaveProperty("disabled", false);
  });

  it("pre-populates fields when editing existing entry", () => {
    store.dispatch({
      type: ENTRY_CREATE,
      id: "edit-me",
      title: "Existing Title",
      content: "Existing content",
      createdAt: "2026-04-02",
      updatedAt: 1000,
    });

    render(<EntryEditor entryId="edit-me" />);
    const titleInput = screen.getByDisplayValue("Existing Title");
    expect(titleInput).toBeTruthy();
  });

  it("shows 'Edit Entry' heading when editing", () => {
    store.dispatch({
      type: ENTRY_CREATE,
      id: "edit-me",
      title: "Existing",
      content: "",
      createdAt: "2026-04-02",
      updatedAt: 1000,
    });

    render(<EntryEditor entryId="edit-me" />);
    expect(screen.getByText(/edit entry/i)).toBeTruthy();
  });

  it("shows not-found message for invalid entryId", () => {
    render(<EntryEditor entryId="nonexistent" />);
    expect(screen.getByText(/entry not found/i)).toBeTruthy();
  });
});
