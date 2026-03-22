/**
 * Application entry point.
 *
 * Mounts the React app into the DOM. This is the only file that touches
 * the real DOM directly — everything else is pure React components.
 */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App.js";
import "./styles/calculator.css";
import "./styles/views.css";

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("Root element not found — check index.html has <div id='root'>");
}

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
