/**
 * Bookmarks Extension — Side Panel Logic
 * ========================================
 *
 * This file wires up the side panel's UI to the storage layer. It handles:
 * - Getting the current tab's URL and title
 * - Saving new bookmarks / updating existing ones
 * - Switching between the Add/Edit and List views
 * - Rendering the bookmarks list with search, edit, and delete
 *
 * Side Panel vs Popup
 * --------------------
 * Unlike a popup (which is destroyed when the user clicks away), the
 * side panel persists while the user browses. The same HTML/CSS/JS
 * powers both Chrome's sidePanel API and Firefox's sidebar_action.
 *
 * Dependency Injection
 * ---------------------
 * The `initPanel()` function accepts an optional `BookmarkStorage`.
 * In production, it uses `createStorage()` (IndexedDB). In tests,
 * we inject `InMemoryStorage` to test panel logic without IDB.
 *
 * ```
 * // Production (panel.html loads this file):
 * initPanel();                       // uses IndexedDB
 *
 * // Tests:
 * initPanel(new InMemoryStorage());  // uses in-memory Map
 * ```
 *
 * Why export initPanel() instead of running code at the top level?
 * Because we need to call it from tests with a mock storage and
 * a mock DOM. Top-level code would run on import, before the test
 * can set up its mocks.
 */

import { getBrowserAPI } from "../lib/browser-api";
import { createStorage } from "../storage";
import type { BookmarkStorage, Bookmark } from "../storage";

// =========================================================================
// Tab Info — getting the current tab's URL and title
// =========================================================================

/**
 * Represents the info we need from the current browser tab.
 *
 * We define our own type rather than using the browser's Tab type
 * because (a) it keeps us decoupled from browser-specific types and
 * (b) tests can provide simple objects without mocking the full Tab.
 */
interface TabInfo {
  url: string;
  title: string;
}

/**
 * Get the current (active) tab's URL and title.
 *
 * Uses `browser.tabs.query()` with `active: true` and `currentWindow: true`
 * to find the tab that was active when the user clicked the extension icon.
 *
 * Returns null if we can't get the tab info (e.g., on chrome:// pages
 * or if the extension doesn't have the right permissions).
 */
async function getCurrentTab(): Promise<TabInfo | null> {
  try {
    const api = getBrowserAPI();
    const tabs = await api.tabs.query({ active: true, currentWindow: true });

    if (tabs.length === 0 || !tabs[0].url) {
      return null;
    }

    return {
      url: tabs[0].url,
      title: tabs[0].title ?? "",
    };
  } catch {
    // Can't access tabs API (e.g., running outside extension context)
    return null;
  }
}

// =========================================================================
// Status Messages
// =========================================================================

/**
 * Show a success or error message in the status area.
 *
 * The message auto-clears after 3 seconds. We use `aria-live="polite"`
 * on the status element so screen readers announce changes.
 */
function showStatus(
  statusEl: HTMLElement,
  message: string,
  type: "success" | "error",
): void {
  statusEl.textContent = message;
  statusEl.className = `status ${type}`;

  // Auto-clear after 3 seconds
  setTimeout(() => {
    statusEl.textContent = "";
    statusEl.className = "status";
  }, 3000);
}

// =========================================================================
// Bookmark List Rendering
// =========================================================================

/**
 * Render a list of bookmarks as cards in the given container.
 *
 * Each card shows the title, truncated URL, note snippet, and
 * Edit / Delete buttons. The callbacks handle user actions.
 */
function renderBookmarkList(
  bookmarks: Bookmark[],
  container: HTMLElement,
  emptyState: HTMLElement,
  onEdit: (bookmark: Bookmark) => void,
  onDelete: (bookmark: Bookmark) => Promise<void>,
): void {
  container.innerHTML = "";

  if (bookmarks.length === 0) {
    emptyState.style.display = "block";
    return;
  }

  emptyState.style.display = "none";

  for (const bookmark of bookmarks) {
    const card = document.createElement("div");
    card.className = "bookmark-card";
    card.dataset.id = bookmark.id;

    const titleEl = document.createElement("div");
    titleEl.className = "bookmark-title";
    titleEl.textContent = bookmark.title || bookmark.url;

    const urlEl = document.createElement("div");
    urlEl.className = "bookmark-url";
    urlEl.textContent = bookmark.url;

    const noteEl = document.createElement("div");
    noteEl.className = "bookmark-note";
    noteEl.textContent = bookmark.note;

    const actions = document.createElement("div");
    actions.className = "card-actions";

    const editBtn = document.createElement("button");
    editBtn.className = "edit-btn";
    editBtn.textContent = "Edit";
    editBtn.addEventListener("click", () => onEdit(bookmark));

    const deleteBtn = document.createElement("button");
    deleteBtn.className = "delete-btn";
    deleteBtn.textContent = "Delete";
    deleteBtn.addEventListener("click", () => onDelete(bookmark));

    actions.appendChild(editBtn);
    actions.appendChild(deleteBtn);

    card.appendChild(titleEl);
    card.appendChild(urlEl);
    if (bookmark.note) {
      card.appendChild(noteEl);
    }
    card.appendChild(actions);

    container.appendChild(card);
  }
}

// =========================================================================
// Main Initialization
// =========================================================================

/**
 * Initialize the popup.
 *
 * This is the entry point — called on DOMContentLoaded in production,
 * or directly by tests with a mock storage.
 *
 * @param storage - Optional storage backend. Defaults to createStorage()
 *   (IndexedDB). Tests inject InMemoryStorage.
 * @param tabInfo - Optional tab info override. Defaults to querying the
 *   browser. Tests provide a mock tab.
 */
export async function initPanel(
  storage?: BookmarkStorage,
  tabInfo?: TabInfo | null,
): Promise<void> {
  const store = storage ?? createStorage();
  await store.initialize();

  // Get DOM elements
  const urlInput = document.getElementById("bookmark-url") as HTMLInputElement | null;
  const titleInput = document.getElementById("bookmark-title") as HTMLInputElement | null;
  const noteInput = document.getElementById("bookmark-note") as HTMLTextAreaElement | null;
  const saveBtn = document.getElementById("save-btn") as HTMLButtonElement | null;
  const statusEl = document.getElementById("status-message");
  const searchInput = document.getElementById("search-input") as HTMLInputElement | null;
  const listContainer = document.getElementById("bookmarks-list");
  const emptyState = document.getElementById("empty-state");

  // Bail if required elements are missing (shouldn't happen, but safe)
  if (!urlInput || !titleInput || !noteInput || !saveBtn || !statusEl || !listContainer || !emptyState) {
    return;
  }

  // Get current tab info (or use provided mock)
  const tab = tabInfo !== undefined ? tabInfo : await getCurrentTab();

  // Track whether we're editing an existing bookmark
  let editingId: string | null = null;

  // ------------------------------------------------------------------
  // Set up the Add/Edit view
  // ------------------------------------------------------------------

  if (tab) {
    urlInput.value = tab.url;
    titleInput.value = tab.title;

    // Check if this URL is already bookmarked
    const existing = await store.getByUrl(tab.url);
    if (existing) {
      editingId = existing.id;
      titleInput.value = existing.title;
      noteInput.value = existing.note;
      saveBtn.textContent = "Update Bookmark";
    }
  } else {
    urlInput.value = "No tab available";
    urlInput.disabled = true;
    titleInput.disabled = true;
    noteInput.disabled = true;
    saveBtn.disabled = true;
  }

  // ------------------------------------------------------------------
  // Save / Update button
  // ------------------------------------------------------------------

  saveBtn.addEventListener("click", async () => {
    const url = urlInput.value;
    const title = titleInput.value.trim();
    const note = noteInput.value.trim();

    if (!url || url === "No tab available") return;

    try {
      if (editingId) {
        await store.update(editingId, { title, note });
        showStatus(statusEl, "Bookmark updated!", "success");
      } else {
        const saved = await store.save({ url, title, note });
        editingId = saved.id;
        saveBtn.textContent = "Update Bookmark";
        showStatus(statusEl, "Bookmark saved!", "success");
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : "Failed to save";
      showStatus(statusEl, message, "error");
    }
  });

  // ------------------------------------------------------------------
  // Tab navigation
  // ------------------------------------------------------------------

  const tabButtons = document.querySelectorAll(".tab-btn");
  const views = document.querySelectorAll(".view");

  tabButtons.forEach((btn) => {
    btn.addEventListener("click", async () => {
      const targetTab = (btn as HTMLElement).dataset.tab;

      // Update active states
      tabButtons.forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");

      views.forEach((v) => v.classList.remove("active"));
      const targetView = document.getElementById(`${targetTab}-view`);
      targetView?.classList.add("active");

      // Refresh the bookmark list when switching to it
      if (targetTab === "list") {
        await refreshList("");
      }
    });
  });

  // ------------------------------------------------------------------
  // Bookmark list
  // ------------------------------------------------------------------

  /** Refresh the bookmark list, optionally filtered by a search query. */
  async function refreshList(query: string): Promise<void> {
    let bookmarks = await store.getAll();

    if (query) {
      const lower = query.toLowerCase();
      bookmarks = bookmarks.filter(
        (b) =>
          b.title.toLowerCase().includes(lower) ||
          b.url.toLowerCase().includes(lower) ||
          b.note.toLowerCase().includes(lower),
      );
    }

    renderBookmarkList(
      bookmarks,
      listContainer!,
      emptyState!,
      // onEdit — switch to Add view with this bookmark's data
      (bookmark) => {
        editingId = bookmark.id;
        urlInput.value = bookmark.url;
        titleInput.value = bookmark.title;
        noteInput.value = bookmark.note;
        saveBtn.textContent = "Update Bookmark";

        // Switch to Add view
        tabButtons.forEach((b) => b.classList.remove("active"));
        tabButtons[0]?.classList.add("active");
        views.forEach((v) => v.classList.remove("active"));
        document.getElementById("add-view")?.classList.add("active");
      },
      // onDelete — remove the bookmark and refresh
      async (bookmark) => {
        await store.delete(bookmark.id);
        await refreshList(searchInput?.value ?? "");
      },
    );
  }

  // Search input — filter as the user types
  if (searchInput) {
    searchInput.addEventListener("input", () => {
      refreshList(searchInput.value);
    });
  }
}

// =========================================================================
// Auto-initialize when loaded in the browser
// =========================================================================

document.addEventListener("DOMContentLoaded", () => {
  initPanel();
});
