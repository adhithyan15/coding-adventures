import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App.js";
import "./styles/app.lattice";

const root = document.getElementById("root");

if (root === null) {
  throw new Error("Expected #root container");
}

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
