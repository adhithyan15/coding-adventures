/**
 * EntryCard.test.tsx — Component tests for the EntryCard component.
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { EntryCard } from "./EntryCard.js";
import type { Entry } from "../types.js";

const entry: Entry = {
  id: "test-1",
  title: "Test Title",
  content: "This is some content that should be shown as a preview in the card.",
  createdAt: "2026-04-02",
  updatedAt: new Date("2026-04-02T14:30:00Z").getTime(),
};

describe("EntryCard", () => {
  it("displays the entry title", () => {
    render(<EntryCard entry={entry} />);
    expect(screen.getByText("Test Title")).toBeTruthy();
  });

  it("displays a preview of the content", () => {
    render(<EntryCard entry={entry} />);
    expect(screen.getByText(/This is some content/)).toBeTruthy();
  });

  it("truncates long content with ellipsis", () => {
    const longEntry: Entry = {
      ...entry,
      content: "A".repeat(200),
    };
    render(<EntryCard entry={longEntry} />);
    const preview = screen.getByText(/A+\.\.\./);
    expect(preview).toBeTruthy();
  });

  it("renders as a button for click navigation", () => {
    render(<EntryCard entry={entry} />);
    const button = screen.getByRole("button");
    expect(button).toBeTruthy();
  });
});
