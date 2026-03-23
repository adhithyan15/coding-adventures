/**
 * main.tsx — Application entry point.
 *
 * Responsibilities:
 *   1. Initialise i18n with the English locale strings.
 *   2. Seed the in-memory state with example templates (once, on first load).
 *   3. Mount the React app into <div id="root">.
 *
 * The i18n initialisation must happen before the first render so that
 * every component calling useTranslation() gets the correct strings
 * immediately, without a loading flash.
 *
 * Seed data is added only when state.templates is empty. In V0 this is
 * always true on page load because there is no persistence. In V1
 * (localStorage), the seed would be skipped once a user has their own
 * templates.
 */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { initI18n } from "@coding-adventures/ui-components";
import { appState } from "./state.js";
import { seedTemplates } from "./seed.js";
import { App } from "./App.js";
import "@coding-adventures/ui-components/src/styles/theme.css";
import "./styles/app.css";
import en from "./i18n/locales/en.json";

// ── 1. Initialise i18n ────────────────────────────────────────────────────
initI18n({ en });

// ── 2. Seed example templates ─────────────────────────────────────────────
if (appState.templates.length === 0) {
  seedTemplates(appState);
}

// ── 3. Mount React ────────────────────────────────────────────────────────
const root = document.getElementById("root");
if (!root) throw new Error("Root element #root not found");

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
