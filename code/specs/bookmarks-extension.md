# Browser Extensions — Bookmarks

## Overview

A browser extension that lets you annotate and save bookmarks. Click the toolbar icon,
see the current tab's URL, write a note about it, and save. Your bookmarks are stored
locally in IndexedDB, with the storage layer designed as a pluggable abstraction so it
can be swapped to cloud backends (Google Drive, OneDrive) in the future.

This is the second extension in the repo, building on the patterns established by the
hello-world extension and the browser-extension-toolkit.

## What We're Building

**bookmarks-extension** — A side-panel-based extension with:
- A form to annotate the current tab's URL with a title and note
- Persistent storage via IndexedDB
- A list view to browse, search, edit, and delete saved bookmarks
- A storage abstraction layer enabling future cloud sync backends
- Cross-browser sidebar abstraction (Chrome sidePanel + Firefox sidebar_action)

**Implementation note:** The original spec described a popup UI. During implementation,
this was changed to a side panel approach because side panels persist while the user
browses (popups are destroyed on click-away), providing a better UX for annotation tasks.

---

## Concepts

### IndexedDB — Browser-native structured storage

IndexedDB is a low-level API for storing significant amounts of structured data in the
browser. Unlike `localStorage` (which only stores strings), IndexedDB stores JavaScript
objects directly. It's transactional, asynchronous, and has no practical size limits.

```
┌──────────────────────────────────────────┐
│              IndexedDB                    │
│                                          │
│  ┌─────────────────────────────────┐     │
│  │    Database: "bookmarks-ext"    │     │
│  │                                 │     │
│  │  ┌───────────────────────────┐  │     │
│  │  │  Object Store: "bookmarks"│  │     │
│  │  │                           │  │     │
│  │  │  keyPath: "id"            │  │     │
│  │  │  index: "url" (unique)    │  │     │
│  │  │                           │  │     │
│  │  │  { id, url, title,        │  │     │
│  │  │    note, createdAt,       │  │     │
│  │  │    updatedAt }            │  │     │
│  │  └───────────────────────────┘  │     │
│  └─────────────────────────────────┘     │
└──────────────────────────────────────────┘
```

**Key concepts:**

| Concept | Explanation |
|---------|-------------|
| Database | A named container. Each extension can have multiple databases. |
| Object Store | Like a table in SQL. Holds records (JavaScript objects). |
| Key Path | The property used as the primary key (`id` in our case). |
| Index | An alternate lookup path (`url` lets us find bookmarks by URL). |
| Transaction | A wrapper around read/write operations. Ensures atomicity. |
| Version | A schema version number. Bumping it triggers `onupgradeneeded`. |

**Why IndexedDB over `chrome.storage`?**

| Feature | IndexedDB | chrome.storage.local |
|---------|-----------|---------------------|
| Storage limit | Essentially unlimited | 10 MB (5 MB without `unlimitedStorage`) |
| Query by index | Yes (e.g., find by URL) | No (key-value only) |
| Transactions | Yes (atomic operations) | No |
| Requires permission | No | Yes (`storage` permission) |
| Available in popup | Yes | Yes |
| Available in service worker | Yes | Yes |

For a bookmarks tool that may store hundreds of annotated URLs, IndexedDB is the
better fit. It gives us indexed lookups (find bookmark by URL) and doesn't require
any extension permissions.

### The Strategy pattern — Pluggable storage backends

We want to start with IndexedDB but later support Google Drive, OneDrive, or any
other backend. The Strategy pattern makes this possible:

```
┌─────────────────────────┐
│    BookmarkStorage      │  ← Interface (the contract)
│                         │
│  initialize()           │
│  getAll()               │
│  getByUrl(url)          │
│  getById(id)            │
│  save(input)            │
│  update(id, input)      │
│  delete(id)             │
└────────┬────────────────┘
         │ implements
         │
    ┌────┴────────────────────────────────────┐
    │              │              │            │
    ▼              ▼              ▼            ▼
┌─────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐
│IndexedDB│  │  Google   │  │ OneDrive │  │ In-Mem │
│ Storage │  │  Drive    │  │ Storage  │  │(tests) │
└─────────┘  └──────────┘  └──────────┘  └────────┘
   (v1)        (future)      (future)     (testing)
```

The popup code never knows which backend it's talking to — it only sees the
`BookmarkStorage` interface. Swapping backends is a one-line change in the
factory function.

### Contract testing — One test suite, many backends

When you have multiple implementations of the same interface, you want to be sure
they all behave identically. Contract tests are a reusable test function that any
backend must pass:

```typescript
function runStorageContractTests(
  name: string,
  createStorage: () => BookmarkStorage
) {
  describe(`${name} contract`, () => {
    it("saves and retrieves a bookmark", async () => { ... });
    it("finds by URL", async () => { ... });
    it("updates a bookmark", async () => { ... });
    it("deletes a bookmark", async () => { ... });
    // ... every operation the interface defines
  });
}

// Each backend runs the same tests:
runStorageContractTests("IndexedDB", () => new IndexedDBStorage());
runStorageContractTests("InMemory",  () => new InMemoryStorage());
// Future: runStorageContractTests("GoogleDrive", () => new DriveStorage());
```

This guarantees that adding a new backend doesn't break the contract.

---

## Data Model

```typescript
/**
 * A saved bookmark — the core entity of the extension.
 *
 * Each bookmark captures a URL the user wants to remember, along with
 * a title and a free-form note (why they saved it, what they found
 * useful, etc.).
 */
interface Bookmark {
  id: string;           // Unique identifier (crypto.randomUUID())
  url: string;          // The bookmarked URL
  title: string;        // Page title (pre-filled from tab, editable)
  note: string;         // User's annotation
  createdAt: string;    // ISO 8601 timestamp of creation
  updatedAt: string;    // ISO 8601 timestamp of last update
}

/**
 * Input for creating a new bookmark.
 * The storage layer generates id, createdAt, and updatedAt automatically.
 */
type BookmarkCreateInput = Omit<Bookmark, 'id' | 'createdAt' | 'updatedAt'>;

/**
 * Input for updating an existing bookmark.
 * Only title and note are editable — the URL is immutable once saved.
 */
type BookmarkUpdateInput = Partial<Pick<Bookmark, 'title' | 'note'>>;
```

**Why is the URL immutable?** A bookmark is an annotation about a specific URL. If
you want to annotate a different URL, create a new bookmark. Allowing URL changes
would break the "find by URL" lookup and create confusing UX.

---

## Storage Abstraction

### Interface

```typescript
interface BookmarkStorage {
  /** Open the database / authenticate with the backend */
  initialize(): Promise<void>;

  /** Retrieve all bookmarks, ordered by most recently updated */
  getAll(): Promise<Bookmark[]>;

  /** Find a bookmark by its URL (null if not found) */
  getByUrl(url: string): Promise<Bookmark | null>;

  /** Find a bookmark by its ID (null if not found) */
  getById(id: string): Promise<Bookmark | null>;

  /** Create a new bookmark. Returns the created bookmark with generated fields. */
  save(input: BookmarkCreateInput): Promise<Bookmark>;

  /** Update an existing bookmark. Returns the updated bookmark. */
  update(id: string, input: BookmarkUpdateInput): Promise<Bookmark>;

  /** Delete a bookmark by ID. No-op if the bookmark doesn't exist. */
  delete(id: string): Promise<void>;
}
```

**Design decisions:**

1. **`initialize()` is separate from construction.** IndexedDB needs to open a
   database (async). Google Drive would need OAuth. By making init explicit, the
   popup can show a loading state while the backend warms up.

2. **All methods return Promises.** Even if IndexedDB operations are fast, cloud
   backends will have network latency. Promises keep the interface uniform.

3. **`delete()` is idempotent.** Deleting a non-existent bookmark succeeds silently.
   This avoids race conditions (e.g., user clicks delete twice quickly).

4. **`save()` vs `update()` are separate.** `save()` creates a new record with
   auto-generated fields. `update()` patches an existing record. This prevents
   accidental overwrites.

### Factory

```typescript
// src/storage/index.ts
import { IndexedDBStorage } from "./indexeddb-storage";
import type { BookmarkStorage } from "./bookmark-storage";

export function createStorage(): BookmarkStorage {
  return new IndexedDBStorage();
}
```

To swap to Google Drive later, change one line:
```typescript
export function createStorage(): BookmarkStorage {
  return new GoogleDriveStorage();
}
```

---

## IndexedDB Implementation

### Database schema

- **Database name:** `bookmarks-extension`
- **Version:** `1`
- **Object store:** `bookmarks`
  - Key path: `id`
  - Index: `url` (unique — one bookmark per URL)

### Operation mapping

| Interface method | IndexedDB operation |
|-----------------|---------------------|
| `initialize()` | `indexedDB.open()` with `onupgradeneeded` handler |
| `getAll()` | `store.getAll()` in a `readonly` transaction |
| `getByUrl(url)` | `store.index("url").get(url)` in a `readonly` transaction |
| `getById(id)` | `store.get(id)` in a `readonly` transaction |
| `save(input)` | `store.add(bookmark)` in a `readwrite` transaction |
| `update(id, input)` | `store.get(id)` → merge → `store.put(merged)` in `readwrite` |
| `delete(id)` | `store.delete(id)` in a `readwrite` transaction |

### Wrapping IDB requests in Promises

IndexedDB's native API uses event listeners (`onsuccess`, `onerror`). Each method
wraps these in a Promise for ergonomic async/await usage:

```typescript
function wrapRequest<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}
```

---

## Side Panel UI Design

### Side Panel vs Popup

The extension uses a **side panel** instead of a popup. Key differences:

| Aspect | Popup | Side Panel |
|--------|-------|------------|
| Lifetime | Destroyed on click-away | Persists while browsing |
| Size | Small fixed-size window | Full browser height, adjustable width |
| Chrome API | `action.default_popup` | `sidePanel` API (Chrome 114+) |
| Firefox API | `action.default_popup` | `sidebar_action` API |
| Safari | Supported | Not supported (falls back to popup) |

The service worker registers an `action.onClicked` listener that opens the
appropriate sidebar API for the current browser.

### Two-view layout

The panel has two views, toggled by a tab bar at the top:

```
┌─────────────────────────────────────┐
│  [✚ Add]  [📋 Bookmarks]           │  ← Tab bar
├─────────────────────────────────────┤
│                                     │
│  Add / Edit View:                   │
│                                     │
│  URL: https://example.com/article   │  ← Read-only, from current tab
│                                     │
│  Title: [Example Article         ]  │  ← Pre-filled from tab.title
│                                     │
│  Note:                              │
│  [                                ] │
│  [  Why I saved this page...      ] │
│  [                                ] │  ← Free-form textarea
│                                     │
│  [ Save Bookmark ]                  │  ← "Update" if already saved
│                                     │
│  ✓ Bookmark saved!                  │  ← Status message
│                                     │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│  [✚ Add]  [📋 Bookmarks]           │
├─────────────────────────────────────┤
│                                     │
│  🔍 [Search bookmarks...        ]  │
│                                     │
│  ┌─────────────────────────────┐    │
│  │ Example Article             │    │
│  │ example.com/article         │    │
│  │ "Great intro to..."        │    │
│  │              [Edit] [Delete]│    │
│  └─────────────────────────────┘    │
│                                     │
│  ┌─────────────────────────────┐    │
│  │ TypeScript Handbook         │    │
│  │ typescriptlang.org/docs     │    │
│  │ "Reference for generics"   │    │
│  │              [Edit] [Delete]│    │
│  └─────────────────────────────┘    │
│                                     │
│  No more bookmarks.                 │
│                                     │
└─────────────────────────────────────┘
```

### Panel lifecycle and storage

```
User clicks extension icon
       │
       ▼
Service worker opens side panel (sidePanel or sidebarAction)
       │
       ▼
panel.html loads
       │
       ▼
initPanel(storage?) called
       │
       ├── Creates storage (or uses injected one)
       ├── Calls storage.initialize()
       ├── Gets current tab via browser.tabs.query()
       ├── Checks if URL is already bookmarked
       │   ├── Yes → pre-fills form with existing data, shows "Update"
       │   └── No  → pre-fills title from tab.title, shows "Save"
       └── Sets up event listeners (save, tab switch, search, etc.)
       │
       ▼
User interacts (saves, edits, deletes, searches)
       │
       ▼
Panel stays open until user closes it → state is in IndexedDB
```

### Dependency injection for testability

The `initPanel()` function accepts an optional `BookmarkStorage`:

```typescript
export async function initPanel(storage?: BookmarkStorage): Promise<void> {
  const store = storage ?? createStorage();
  await store.initialize();
  // ... wire up UI
}
```

Tests inject an `InMemoryStorage` to avoid IndexedDB complexity. Production
uses the default `createStorage()` which returns `IndexedDBStorage`.

---

## Permissions

| Permission | Why | Browsers |
|-----------|-----|----------|
| `activeTab` | Access the current tab's URL and title when the user clicks the icon | All |
| `tabs` | Use `browser.tabs.query()` to get the active tab's info | All |
| `sidePanel` | Open Chrome's side panel via `chrome.sidePanel.open()` | Chrome only |

**What we DON'T need:**
- `storage` — IndexedDB doesn't require any permission
- `bookmarks` — We're not reading/writing the browser's built-in bookmarks
- `<all_urls>` — We don't inject content scripts into pages

**Note:** The `sidePanel` permission is stripped from the Firefox and Safari
manifests by the manifest transformer, since those browsers don't support it.

---

## Testing Strategy

### Contract tests (`bookmark-storage.test.ts`)

A reusable `runStorageContractTests(name, factory)` function that verifies the
full `BookmarkStorage` interface:

- `save()` creates a bookmark with generated id and timestamps
- `save()` twice with same URL throws (unique constraint)
- `getAll()` returns all saved bookmarks
- `getByUrl()` returns the bookmark for a known URL
- `getByUrl()` returns null for an unknown URL
- `getById()` returns the bookmark for a known ID
- `getById()` returns null for an unknown ID
- `update()` patches title and note, bumps `updatedAt`
- `update()` throws for a non-existent ID
- `delete()` removes a bookmark
- `delete()` is idempotent (no error for missing ID)

### IndexedDB tests (`indexeddb-storage.test.ts`)

Uses `fake-indexeddb` npm package (provides a complete IDB implementation in Node).
Runs the contract tests with `IndexedDBStorage`, plus IDB-specific checks:

- Database and object store are created on `initialize()`
- URL index exists and is unique

### InMemory test helper (`test-helpers.ts`)

A simple `InMemoryStorage` backed by a `Map`. Used in popup tests to avoid
IndexedDB complexity. Also verified by the contract tests to ensure the fake
is faithful to the real interface.

### Popup tests (`popup.test.ts`)

Mock `chrome`/`browser` globals (following hello-world's pattern), inject
`InMemoryStorage`, test all UI flows:

- Displays current tab URL
- Pre-fills title from tab
- Saves a new bookmark on button click
- Shows "Update" button for existing bookmark
- Shows success/error status messages
- Switches between Add and List views
- Renders bookmark list
- Deletes a bookmark from the list
- Filters bookmarks by search query
- Handles missing tab gracefully
- Handles storage errors gracefully

---

## Build Pipeline

Same as hello-world — Vite compiles TypeScript, custom plugin copies manifest
and icons, `build-all-browsers.ts` produces per-browser variants.

### Release workflow

Tag-triggered (`bookmarks-extension-v*`), produces three zip files
(`bookmarks-extension-chrome.zip`, `bookmarks-extension-firefox.zip`,
`bookmarks-extension-safari.zip`), attached to a GitHub Release.

---

## Future Work

- **Cloud sync backends** — Google Drive and OneDrive implementations of
  `BookmarkStorage`, with OAuth flows in the service worker
- **Import/export** — Download bookmarks as JSON, import from other tools
- **Tags/categories** — Organize bookmarks with labels
- **Full-text search** — Search through note content, not just titles
- **Content script** — Show a badge on the toolbar icon when the current
  page is already bookmarked
