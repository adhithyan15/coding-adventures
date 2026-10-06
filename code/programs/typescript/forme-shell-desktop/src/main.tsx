import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { AuthoringShell } from "@coding-adventures/forme-authoring-shell";
import { createDesktopHost } from "./desktop-host.js";
import "./main.css";

declare global {
  var __FORME_PREVIEW_URL__: string | undefined;
}

const previewUrl = globalThis.__FORME_PREVIEW_URL__;
if (typeof previewUrl !== "string") throw new Error("the native preview boundary is unavailable");
const root = document.getElementById("root");
if (root === null) throw new Error("the Forme application root is missing");

const host = createDesktopHost({ previewUrl, invoke });
createRoot(root).render(
  <StrictMode>
    <AuthoringShell host={host} />
  </StrictMode>,
);
