// Unit tests for dap.ts.
//
// The `vscode` module is not available in unit tests (it is injected by the
// VS Code runtime).  We mock it below so that the module-level import of
// `vscode` in dap.ts resolves cleanly, and we can test the parts that do not
// depend on real VS Code APIs.

import { describe, it, expect, vi } from "vitest";

// Mock the `vscode` module before importing any module that depends on it.
// `vi.mock` is hoisted by vitest so it runs before imports.
vi.mock("vscode", () => {
  // Minimal stub: only the APIs that dap.ts touches at import time are
  // needed here.  Other tests can extend this mock as needed.
  return {
    workspace: {
      getConfiguration: vi.fn(() => ({
        get: vi.fn((_key: string, fallback: unknown) => fallback),
      })),
    },
    debug: {
      registerDebugAdapterDescriptorFactory: vi.fn(),
    },
    DebugAdapterExecutable: vi
      .fn()
      .mockImplementation((cmd: string, _args: string[]) => ({ command: cmd })),
  };
});

// Import after mock setup so the mock is in place when the module loads.
import { LANGUAGE_NAME } from "./dap";

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("dap — LANGUAGE_NAME", () => {
  it("is the string 'Twig'", () => {
    // The constant is used in extension metadata, log messages, and error
    // dialogs.  Confirm it has the expected human-readable value so a
    // generator typo is caught immediately.
    expect(LANGUAGE_NAME).toBe("Twig");
  });

  it("is a non-empty string", () => {
    expect(typeof LANGUAGE_NAME).toBe("string");
    expect(LANGUAGE_NAME.length).toBeGreaterThan(0);
  });
});
