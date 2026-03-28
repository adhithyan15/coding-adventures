import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
  test: {
    environment: "jsdom",
    globals: true,
    exclude: ["e2e/**", "node_modules/**", "playwright-report/**"],
    coverage: {
      provider: "v8",
      // Only measure coverage on the pure-logic layer.
      // Excluded from thresholds (require browser/e2e tests, not unit tests):
      //   main.tsx       — app bootstrap, not unit testable
      //   seed.ts        — data seeding helpers, tested via integration
      //   state.ts       — single `new Store(...)` call
      //   electron/      — Electron main process
      //   vite.config.ts — build config
      //   components/**  — React UI components (except ViewRenderer which we test)
      //     TodoList, TodoCard, TodoEditor, FilterBar, EmptyState,
      //     TodoCalendar, KanbanView, CalendarViewWrapper are UI-only;
      //     ViewRenderer is covered by ViewRenderer.test.tsx
      include: [
        "src/actions.ts",
        "src/calendar-settings.ts",
        "src/persistence.ts",
        "src/reducer.ts",
        "src/types.ts",
        "src/views.ts",
        "src/components/ViewRenderer.tsx",
      ],
      thresholds: {
        lines: 80,
      },
    },
  },
  resolve: {
    // Deduplicate React — file: protocol deps may bundle their own copy,
    // which causes "Invalid hook call" errors. Force all imports of react
    // and react-dom to resolve to this project's single copy.
    dedupe: ["react", "react-dom"],
    alias: {
      react: path.resolve(__dirname, "node_modules/react"),
      "react-dom": path.resolve(__dirname, "node_modules/react-dom"),
    },
  },
});
