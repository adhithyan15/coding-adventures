/**
 * setup.ts — Vitest global test setup file.
 *
 * Polyfills that the jsdom environment doesn't provide out of the box:
 *
 *   • crypto.getRandomValues — used by @coding-adventures/uuid's v7()
 *     implementation. jsdom does not expose the Web Crypto API in all
 *     versions, so we fill it in from Node's built-in webcrypto module
 *     which has been stable since Node 19 and experimental since Node 15.
 *
 * This file is listed under `test.setupFiles` in vitest.config.ts and
 * runs once per test-file before any test code executes.
 */

import { webcrypto } from "node:crypto";
import { vi } from "vitest";

// Polyfill Web Crypto API for the jsdom test environment.
//
// jsdom 25 exposes window.crypto, but in some Vitest worker+jsdom
// configurations the bare `crypto` identifier resolves to the Node.js
// `crypto` built-in (which does NOT have getRandomValues) rather than
// the Web Crypto API. This is triggered when packages like
// @coding-adventures/uuid are transformed by Vite before the jsdom
// window is fully wired as globalThis.
//
// We use vi.stubGlobal (Vitest's official API) to install Node's
// webcrypto as the authoritative `crypto` object. vi.stubGlobal
// correctly handles the jsdom/globalThis relationship that plain
// Object.defineProperty misses.
vi.stubGlobal("crypto", webcrypto);
