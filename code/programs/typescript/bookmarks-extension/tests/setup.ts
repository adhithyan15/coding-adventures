/**
 * Test Setup — fake-indexeddb
 * ===========================
 *
 * This file is loaded before every test via vitest's `setupFiles` config.
 *
 * What it does
 * ------------
 * `fake-indexeddb/auto` patches the global scope with a complete
 * IndexedDB implementation that runs in Node.js / jsdom. Without this,
 * `indexedDB`, `IDBDatabase`, `IDBTransaction`, etc. are all undefined
 * in the test environment.
 *
 * Why "auto"?
 * -----------
 * The `/auto` import automatically assigns the fake implementations to
 * `globalThis.indexedDB`, `globalThis.IDBKeyRange`, etc. — exactly what
 * our `IndexedDBStorage` class expects to find. No manual wiring needed.
 *
 * This approach is the standard way to test IndexedDB code outside a
 * browser. The `fake-indexeddb` package implements the full W3C
 * IndexedDB 2.0 spec, so our tests exercise real IDB behavior.
 */
import "fake-indexeddb/auto";
