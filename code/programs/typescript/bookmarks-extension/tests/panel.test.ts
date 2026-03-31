/**
 * Panel Tests
 * ============
 *
 * Testing a browser extension panel requires mocking two things:
 *
 * 1. **The DOM** — The panel manipulates HTML elements from panel.html.
 *    We set up a minimal DOM in each test that matches the real markup.
 *
 * 2. **The browser API** — The panel calls `getBrowserAPI()` which looks
 *    for `chrome` or `browser` on globalThis. We mock the `tabs.query()`
 *    method since that's what the bookmarks panel uses.
 *
 * We also inject `InMemoryStorage` to test panel logic without IndexedDB.
 * This keeps the tests fast and focused on UI behavior.
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import { initPanel } from "../src/panel/panel";
import { InMemoryStorage } from "./test-helpers";

/** The minimal DOM structure that panel.ts expects. */
const PANEL_HTML = `
  <div class="container">
    <nav class="tab-bar">
      <button class="tab-btn active" data-tab="add">Add</button>
      <button class="tab-btn" data-tab="list">Bookmarks</button>
    </nav>
    <div id="add-view" class="view active">
      <div class="field">
        <label for="bookmark-url">URL</label>
        <input type="text" id="bookmark-url" readonly>
      </div>
      <div class="field">
        <label for="bookmark-title">Title</label>
        <input type="text" id="bookmark-title" placeholder="Page title">
      </div>
      <div class="field">
        <label for="bookmark-note">Note</label>
        <textarea id="bookmark-note" rows="4"></textarea>
      </div>
      <button id="save-btn" class="primary-btn">Save Bookmark</button>
      <div id="status-message" class="status"></div>
    </div>
    <div id="list-view" class="view">
      <div class="field">
        <input type="text" id="search-input" placeholder="Search bookmarks...">
      </div>
      <div id="bookmarks-list"></div>
      <div id="empty-state" class="empty-state">No bookmarks saved yet.</div>
    </div>
  </div>
`;

describe("initPanel", () => {
  let storage: InMemoryStorage;

  beforeEach(() => {
    document.body.innerHTML = PANEL_HTML;
    storage = new InMemoryStorage();

    // Clean up globals between tests
    const g = globalThis as Record<string, unknown>;
    delete g.chrome;
    delete g.browser;
  });

  // =================================================================
  // Tab info display
  // =================================================================

  it("displays the current tab URL and title", async () => {
    await initPanel(storage, {
      url: "https://example.com/article",
      title: "Example Article",
    });

    const urlInput = document.getElementById("bookmark-url") as HTMLInputElement;
    const titleInput = document.getElementById("bookmark-title") as HTMLInputElement;

    expect(urlInput.value).toBe("https://example.com/article");
    expect(titleInput.value).toBe("Example Article");
  });

  it("handles missing tab gracefully", async () => {
    await initPanel(storage, null);

    const urlInput = document.getElementById("bookmark-url") as HTMLInputElement;
    const saveBtn = document.getElementById("save-btn") as HTMLButtonElement;

    expect(urlInput.value).toBe("No tab available");
    expect(saveBtn.disabled).toBe(true);
  });

  // =================================================================
  // Saving bookmarks
  // =================================================================

  it("saves a new bookmark when Save is clicked", async () => {
    await initPanel(storage, {
      url: "https://example.com",
      title: "Example",
    });

    // Add a note
    const noteInput = document.getElementById("bookmark-note") as HTMLTextAreaElement;
    noteInput.value = "Great article about testing";

    // Click save
    const saveBtn = document.getElementById("save-btn") as HTMLButtonElement;
    saveBtn.click();

    // Wait for async save
    await new Promise((r) => setTimeout(r, 10));

    // Verify it was saved
    const saved = await storage.getByUrl("https://example.com");
    expect(saved).not.toBeNull();
    expect(saved!.title).toBe("Example");
    expect(saved!.note).toBe("Great article about testing");
  });

  it("shows success message after saving", async () => {
    await initPanel(storage, {
      url: "https://example.com",
      title: "Example",
    });

    const saveBtn = document.getElementById("save-btn") as HTMLButtonElement;
    saveBtn.click();
    await new Promise((r) => setTimeout(r, 10));

    const status = document.getElementById("status-message")!;
    expect(status.textContent).toBe("Bookmark saved!");
    expect(status.classList.contains("success")).toBe(true);
  });

  it("changes button text to 'Update Bookmark' after first save", async () => {
    await initPanel(storage, {
      url: "https://example.com",
      title: "Example",
    });

    const saveBtn = document.getElementById("save-btn") as HTMLButtonElement;
    expect(saveBtn.textContent).toBe("Save Bookmark");

    saveBtn.click();
    await new Promise((r) => setTimeout(r, 10));

    expect(saveBtn.textContent).toBe("Update Bookmark");
  });

  // =================================================================
  // Editing existing bookmarks
  // =================================================================

  it("pre-fills form for an already-bookmarked URL", async () => {
    // Pre-save a bookmark
    await storage.initialize();
    await storage.save({
      url: "https://example.com",
      title: "Saved Title",
      note: "Saved Note",
    });

    await initPanel(storage, {
      url: "https://example.com",
      title: "Current Tab Title",
    });

    const titleInput = document.getElementById("bookmark-title") as HTMLInputElement;
    const noteInput = document.getElementById("bookmark-note") as HTMLTextAreaElement;
    const saveBtn = document.getElementById("save-btn") as HTMLButtonElement;

    // Should show the saved data, not the current tab title
    expect(titleInput.value).toBe("Saved Title");
    expect(noteInput.value).toBe("Saved Note");
    expect(saveBtn.textContent).toBe("Update Bookmark");
  });

  it("updates an existing bookmark when Update is clicked", async () => {
    await storage.initialize();
    const saved = await storage.save({
      url: "https://example.com",
      title: "Old Title",
      note: "Old Note",
    });

    await initPanel(storage, {
      url: "https://example.com",
      title: "Tab Title",
    });

    // Change the note
    const noteInput = document.getElementById("bookmark-note") as HTMLTextAreaElement;
    noteInput.value = "Updated Note";

    const saveBtn = document.getElementById("save-btn") as HTMLButtonElement;
    saveBtn.click();
    await new Promise((r) => setTimeout(r, 10));

    // Verify update
    const updated = await storage.getById(saved.id);
    expect(updated!.note).toBe("Updated Note");

    const status = document.getElementById("status-message")!;
    expect(status.textContent).toBe("Bookmark updated!");
  });

  // =================================================================
  // Tab navigation
  // =================================================================

  it("switches to the list view when Bookmarks tab is clicked", async () => {
    await initPanel(storage, {
      url: "https://example.com",
      title: "Example",
    });

    const listTab = document.querySelectorAll(".tab-btn")[1] as HTMLButtonElement;
    listTab.click();
    await new Promise((r) => setTimeout(r, 10));

    const addView = document.getElementById("add-view")!;
    const listView = document.getElementById("list-view")!;

    expect(addView.classList.contains("active")).toBe(false);
    expect(listView.classList.contains("active")).toBe(true);
  });

  // =================================================================
  // Bookmarks list
  // =================================================================

  it("renders saved bookmarks in the list view", async () => {
    await storage.initialize();
    await storage.save({ url: "https://a.com", title: "Alpha", note: "First" });
    await storage.save({ url: "https://b.com", title: "Beta", note: "Second" });

    await initPanel(storage, {
      url: "https://other.com",
      title: "Other",
    });

    // Switch to list view
    const listTab = document.querySelectorAll(".tab-btn")[1] as HTMLButtonElement;
    listTab.click();
    await new Promise((r) => setTimeout(r, 10));

    const cards = document.querySelectorAll(".bookmark-card");
    expect(cards.length).toBe(2);
  });

  it("shows empty state when no bookmarks exist", async () => {
    await initPanel(storage, {
      url: "https://example.com",
      title: "Example",
    });

    // Switch to list view
    const listTab = document.querySelectorAll(".tab-btn")[1] as HTMLButtonElement;
    listTab.click();
    await new Promise((r) => setTimeout(r, 10));

    const emptyState = document.getElementById("empty-state")!;
    expect(emptyState.style.display).toBe("block");
  });

  it("deletes a bookmark from the list", async () => {
    await storage.initialize();
    await storage.save({ url: "https://a.com", title: "Alpha", note: "" });

    await initPanel(storage, {
      url: "https://other.com",
      title: "Other",
    });

    // Switch to list view
    const listTab = document.querySelectorAll(".tab-btn")[1] as HTMLButtonElement;
    listTab.click();
    await new Promise((r) => setTimeout(r, 10));

    // Click delete on the first card
    const deleteBtn = document.querySelector(".delete-btn") as HTMLButtonElement;
    deleteBtn.click();
    await new Promise((r) => setTimeout(r, 10));

    // Should be gone from storage
    const all = await storage.getAll();
    expect(all).toHaveLength(0);

    // Should show empty state
    const emptyState = document.getElementById("empty-state")!;
    expect(emptyState.style.display).toBe("block");
  });

  it("filters bookmarks by search query", async () => {
    await storage.initialize();
    await storage.save({ url: "https://a.com", title: "TypeScript Guide", note: "" });
    await storage.save({ url: "https://b.com", title: "Python Tutorial", note: "" });

    await initPanel(storage, {
      url: "https://other.com",
      title: "Other",
    });

    // Switch to list view
    const listTab = document.querySelectorAll(".tab-btn")[1] as HTMLButtonElement;
    listTab.click();
    await new Promise((r) => setTimeout(r, 10));

    // Type in search
    const searchInput = document.getElementById("search-input") as HTMLInputElement;
    searchInput.value = "typescript";
    searchInput.dispatchEvent(new Event("input"));
    await new Promise((r) => setTimeout(r, 10));

    const cards = document.querySelectorAll(".bookmark-card");
    expect(cards.length).toBe(1);
  });

  it("switches to add view with bookmark data when Edit is clicked", async () => {
    await storage.initialize();
    await storage.save({
      url: "https://edit-me.com",
      title: "Edit Me",
      note: "Original note",
    });

    await initPanel(storage, {
      url: "https://other.com",
      title: "Other",
    });

    // Switch to list view
    const listTab = document.querySelectorAll(".tab-btn")[1] as HTMLButtonElement;
    listTab.click();
    await new Promise((r) => setTimeout(r, 10));

    // Click edit
    const editBtn = document.querySelector(".edit-btn") as HTMLButtonElement;
    editBtn.click();
    await new Promise((r) => setTimeout(r, 10));

    // Should switch to add view with bookmark data
    const addView = document.getElementById("add-view")!;
    expect(addView.classList.contains("active")).toBe(true);

    const urlInput = document.getElementById("bookmark-url") as HTMLInputElement;
    const titleInput = document.getElementById("bookmark-title") as HTMLInputElement;
    const noteInput = document.getElementById("bookmark-note") as HTMLTextAreaElement;

    expect(urlInput.value).toBe("https://edit-me.com");
    expect(titleInput.value).toBe("Edit Me");
    expect(noteInput.value).toBe("Original note");
  });

  // =================================================================
  // Edge cases
  // =================================================================

  it("does not crash with missing DOM elements", async () => {
    document.body.innerHTML = "<div>empty</div>";
    // Should return without error
    await expect(initPanel(storage, null)).resolves.toBeUndefined();
  });

  it("handles storage errors gracefully", async () => {
    await initPanel(storage, {
      url: "https://example.com",
      title: "Example",
    });

    // Save first bookmark
    const saveBtn = document.getElementById("save-btn") as HTMLButtonElement;
    saveBtn.click();
    await new Promise((r) => setTimeout(r, 10));

    // Create a second panel trying to save the same URL (would fail due
    // to duplicate URL in a fresh storage)
    const storage2 = new InMemoryStorage();
    await storage2.initialize();
    await storage2.save({
      url: "https://example.com",
      title: "Already Saved",
      note: "",
    });

    document.body.innerHTML = PANEL_HTML;
    // This time the URL is new for storage2's perspective, but let's
    // test by making save throw
    const errorStorage = new InMemoryStorage();
    await errorStorage.initialize();
    const originalSave = errorStorage.save.bind(errorStorage);
    errorStorage.save = async () => {
      throw new Error("Storage full");
    };

    await initPanel(errorStorage, {
      url: "https://will-fail.com",
      title: "Fail",
    });

    const saveBtn2 = document.getElementById("save-btn") as HTMLButtonElement;
    saveBtn2.click();
    await new Promise((r) => setTimeout(r, 10));

    const status = document.getElementById("status-message")!;
    expect(status.textContent).toBe("Storage full");
    expect(status.classList.contains("error")).toBe(true);
  });
});
