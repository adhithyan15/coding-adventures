// main.tsx — React 18 entry point (UI26 §11).

import React from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error(
    "VisiCalc demo: #root element missing from index.html — check the template.",
  );
}

createRoot(rootElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
