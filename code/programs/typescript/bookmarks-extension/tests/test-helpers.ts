/**
 * Test Helpers — InMemoryStorage
 * ==============================
 *
 * A simple in-memory implementation of BookmarkStorage for use in tests.
 *
 * Why not use IndexedDBStorage in all tests?
 * -------------------------------------------
 * Popup tests focus on UI behavior (button clicks, form submission,
 * view switching). They need a storage backend, but they don't need
 * to test IndexedDB itself. Using InMemoryStorage:
 *
 * 1. Runs faster (no IDB overhead)
 * 2. Keeps popup tests focused on popup logic
 * 3. Removes the IDB dependency from popup test failures
 *
 * Is this fake faithful to the real thing?
 * -----------------------------------------
 * Yes — we verify it by running the same contract tests that
 * IndexedDBStorage passes. If the contract tests pass for both,
 * they're interchangeable for consumer code.
 */

import type {
  Bookmark,
  BookmarkCreateInput,
  BookmarkUpdateInput,
  BookmarkStorage,
} from "../src/storage/bookmark-storage";

export class InMemoryStorage implements BookmarkStorage {
  /** Internal store: Map from bookmark ID to bookmark object */
  private store = new Map<string, Bookmark>();

  /** Track whether initialize() has been called */
  private initialized = false;

  async initialize(): Promise<void> {
    this.initialized = true;
  }

  async getAll(): Promise<Bookmark[]> {
    const all = Array.from(this.store.values());
    // Match IndexedDB behavior: sorted by updatedAt descending
    return all.sort(
      (a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
    );
  }

  async getByUrl(url: string): Promise<Bookmark | null> {
    for (const bookmark of this.store.values()) {
      if (bookmark.url === url) return bookmark;
    }
    return null;
  }

  async getById(id: string): Promise<Bookmark | null> {
    return this.store.get(id) ?? null;
  }

  async save(input: BookmarkCreateInput): Promise<Bookmark> {
    // Check for duplicate URL (matches IndexedDB unique index behavior)
    for (const existing of this.store.values()) {
      if (existing.url === input.url) {
        throw new Error(`Bookmark with URL already exists: ${input.url}`);
      }
    }

    const now = new Date().toISOString();
    const bookmark: Bookmark = {
      id: crypto.randomUUID(),
      ...input,
      createdAt: now,
      updatedAt: now,
    };
    this.store.set(bookmark.id, bookmark);
    return bookmark;
  }

  async update(id: string, input: BookmarkUpdateInput): Promise<Bookmark> {
    const existing = this.store.get(id);
    if (!existing) {
      throw new Error(`Bookmark not found: ${id}`);
    }

    const updated: Bookmark = {
      ...existing,
      ...input,
      updatedAt: new Date().toISOString(),
    };
    this.store.set(id, updated);
    return updated;
  }

  async delete(id: string): Promise<void> {
    this.store.delete(id);
  }
}
