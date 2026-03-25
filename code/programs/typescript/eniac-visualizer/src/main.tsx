/**
 * Application entry point for the ENIAC visualizer.
 */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { initI18n } from "@coding-adventures/ui-components";
import en from "./i18n/locales/en.json";
import { App } from "./App.js";
import "./styles/app.css";
import "./styles/triode.css";
import "./styles/ring-counter.css";
import "./styles/accumulator.css";
import "./styles/comparison.css";

initI18n({ en });

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
