// Vitest configuration for the twig-vscode extension.
//
// All tests run in a plain Node environment — VS Code's `vscode` module is
// not available in unit tests, so individual test files mock it with
// `vi.mock('vscode', ...)` where needed.  The `passWithNoTests` flag
// prevents CI from failing when the test suite is empty (e.g. while the
// extension is being bootstrapped).
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    passWithNoTests: true,
  },
});
