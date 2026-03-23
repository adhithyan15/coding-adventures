/**
 * Application entry point.
 *
 * Initializes the i18n system with English translations and mounts
 * the React app to the DOM. StrictMode enables additional development
 * checks (double-rendering, deprecated API warnings).
 */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { initI18n } from "@coding-adventures/ui-components";
import en from "./i18n/locales/en.json";
import { App } from "./App.js";
import "./styles/app.css";
import "./styles/adders.css";
import "./styles/addition.css";

// Initialize translations before rendering any components.
// All visible text comes from en.json — no hardcoded strings in components.
initI18n({ en });

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
