// main.ts — VisiCalc on Deno Desktop (Deno 2.9's `deno desktop`).
//
// Tenth cross-backend VisiCalc host. Deno Desktop bundles your code + the Deno
// runtime + a webview into one native app per platform; `deno desktop main.ts`
// opens a window pointed at whatever your `Deno.serve()` handler serves (Deno.serve
// automatically binds to the address the webview navigates to). So this backend
// serves the SAME engine-backed web VisiCalc the HTML demo runs — the Rust
// `spreadsheet-core` engine compiled to WebAssembly — and Deno wraps it in a
// desktop window. Sibling to the Electron host; here the "wrapper" is Deno's
// built-in webview, with zero third-party dependencies.
//
//   Run in a window:   deno desktop -A main.ts
//   Build an app:      deno desktop -A --output VisiCalc.app main.ts
//   Server only (dev): deno run --allow-net main.ts   (open http://localhost:8791)
//
// The two files the HTML demo needs (its `index.html` and the one script it
// loads, `vendor/spreadsheet-engine-wasm.js` — the base64-embedded WASM engine)
// are EMBEDDED here via text-import attributes. That's what makes the compiled
// `deno desktop` app self-contained: it serves them from memory, so there is no
// runtime filesystem dependency on the sibling demo's directory (an earlier
// version read them at runtime and 404'd once compiled, because the source tree
// isn't inside the app bundle).
//
// The FFI variant (loading libspreadsheet_capi via Deno.dlopen instead of WASM)
// is a follow-up — this is the WASM path, reusing the browser engine as-is.

import indexHtml from "../visicalc-html/index.html" with { type: "text" };
import engineJs from "../visicalc-html/vendor/spreadsheet-engine-wasm.js" with {
  type: "text",
};

// Path → [content-type, body]. index.html loads exactly one asset
// (`vendor/spreadsheet-engine-wasm.js`); both are embedded above.
const HTML = "text/html; charset=utf-8";
const JS = "text/javascript; charset=utf-8";
const ASSETS: Record<string, [string, string]> = {
  "/": [HTML, indexHtml],
  "/index.html": [HTML, indexHtml],
  "/vendor/spreadsheet-engine-wasm.js": [JS, engineJs],
};

// `deno desktop` points the webview at this server automatically; a fixed port
// also lets you open it in a browser during `deno run` dev.
Deno.serve({ hostname: "127.0.0.1", port: 8791 }, (req: Request): Response => {
  const { pathname } = new URL(req.url);
  const hit = ASSETS[pathname];
  if (hit) {
    return new Response(hit[1], { headers: { "content-type": hit[0] } });
  }
  return new Response("Not found", { status: 404 });
});
