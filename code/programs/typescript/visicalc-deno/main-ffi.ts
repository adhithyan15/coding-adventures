// main-ffi.ts — VisiCalc on Deno Desktop, engine loaded via Deno.dlopen (FFI).
//
// This is the FFI sibling of main.ts. Where main.ts embeds the engine as
// WebAssembly and runs it INSIDE the webview, this host loads the SAME Rust
// `spreadsheet-core` engine as a NATIVE dynamic library (`spreadsheet-capi`'s C
// ABI — the very library the Qt and SwiftUI demos link) into the Deno process
// via `Deno.dlopen`, and calls it as native machine code. The webview becomes a
// thin HTTP client: it renders the grid and posts edits to a tiny local API that
// this process serves by calling the FFI engine. Same engine, two transports —
// WASM-in-browser (main.ts) vs native-FFI-in-Deno (here).
//
//   Run in a window:   deno task desktop:ffi   (deno desktop -A main-ffi.ts)
//   Server only (dev): deno task dev:ffi       (open http://localhost:8792)
//   Build an app:      deno task build:ffi
//
// Prerequisite: the native engine must be vendored first —
//   deno task engine     (bash scripts/build-engine.sh)
// which builds libspreadsheet_capi.{dylib,so,dll} from the crate into vendor/.

// ---- Locate the vendored native engine -------------------------------------
const LIB_BY_OS: Record<string, string> = {
  darwin: "libspreadsheet_capi.dylib",
  linux: "libspreadsheet_capi.so",
  windows: "spreadsheet_capi.dll",
};
const libName = LIB_BY_OS[Deno.build.os] ?? "libspreadsheet_capi.so";
const libPath = new URL(`./vendor/${libName}`, import.meta.url).pathname;

let lib: Deno.DynamicLibrary<typeof SYMBOLS>;
const SYMBOLS = {
  sc_session_new: { parameters: [], result: "pointer" },
  sc_session_free: { parameters: ["pointer"], result: "void" },
  sc_set_cell: { parameters: ["pointer", "buffer", "buffer"], result: "pointer" },
  sc_get_raw: { parameters: ["pointer", "buffer"], result: "pointer" },
  sc_get_display_window: {
    parameters: ["pointer", "u32", "u32", "u32", "u32"],
    result: "pointer",
  },
  sc_string_free: { parameters: ["pointer"], result: "void" },
} as const;

try {
  lib = Deno.dlopen(libPath, SYMBOLS);
} catch (e) {
  console.error(
    `Failed to load the native engine at ${libPath}.\n` +
      `Build + vendor it first:  deno task engine\n(${e instanceof Error ? e.message : e})`,
  );
  Deno.exit(1);
}

// ---- Thin JS view over the C ABI -------------------------------------------
const enc = new TextEncoder();
/** A null-terminated C string buffer for a `const char *` parameter. */
function cstr(s: string): Uint8Array {
  return enc.encode(s + "\0");
}
/** Read an owned `char *` return value, then free it with sc_string_free. */
function take(ptr: Deno.PointerValue): string {
  if (ptr === null) return "";
  const s = new Deno.UnsafePointerView(ptr).getCString();
  lib.symbols.sc_string_free(ptr);
  return s;
}

const session = lib.symbols.sc_session_new();
if (session === null) {
  console.error("sc_session_new returned NULL");
  Deno.exit(1);
}

function setCell(a1: string, raw: string): void {
  take(lib.symbols.sc_set_cell(session, cstr(a1), cstr(raw)));
}
function getRaw(a1: string): string {
  return take(lib.symbols.sc_get_raw(session, cstr(a1)));
}
/** Computed display window as a 2-D array of strings (1-based engine coords). */
function displayWindow(rows: number, cols: number): string[][] {
  const json = take(
    lib.symbols.sc_get_display_window(session, 1, 1, rows, cols),
  );
  try {
    const parsed = JSON.parse(json);
    return Array.isArray(parsed?.cells) ? parsed.cells : [];
  } catch {
    return [];
  }
}

// The classic cross-footing budget — the identical seed every VisiCalc demo
// uses (E column = row sums, row 5 = column sums, E5 = grand total 169).
const SEED: Record<string, string> = {
  A1: "15", B1: "3", C1: "12", D1: "8", E1: "=SUM(A1:D1)",
  A2: "8", B2: "14", C2: "7", D2: "22", E2: "=SUM(A2:D2)",
  A3: "12", B3: "9", C3: "18", D3: "6", E3: "=SUM(A3:D3)",
  A4: "4", B4: "11", C4: "3", D4: "17", E4: "=SUM(A4:D4)",
  A5: "=SUM(A1:A4)", B5: "=SUM(B1:B4)", C5: "=SUM(C1:C4)",
  D5: "=SUM(D1:D4)", E5: "=SUM(E1:E4)",
};
for (const [a1, raw] of Object.entries(SEED)) setCell(a1, raw);

const ROWS = 5, COLS = 5; // A..E

// ---- Thin webview client ----------------------------------------------------
// A dependency-free page that renders the engine's computed values (fetched from
// /api/window) and posts edits to /api/cell — which this process applies via the
// native FFI engine, then the page re-fetches. The engine never runs in the
// browser here; the webview is pure I/O.
const PAGE = /* html */ `<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<title>VisiCalc — Deno Desktop (FFI)</title>
<style>
  body { margin:0; background:#1e1e1e; color:#ccc; font-family:-apple-system,"Segoe UI",sans-serif; }
  .app { max-width:720px; margin:0 auto; padding:16px; }
  h1 { font-size:12px; font-weight:normal; color:#9d9d9d; letter-spacing:1px; text-transform:uppercase; }
  .bar { display:flex; background:#252526; padding:4px; border-bottom:1px solid #3f3f46; margin-bottom:8px; }
  .bar .addr { font-family:monospace; font-size:12px; color:#9d9d9d; background:#2d2d30; min-width:48px; padding:4px; text-align:right; margin-right:8px; }
  .bar input { flex:1; font-family:monospace; font-size:13px; border:none; border-bottom:1px solid #3f3f46; background:transparent; color:#ccc; padding:4px; outline:none; }
  table { font-family:monospace; font-size:12px; border-collapse:collapse; width:100%; }
  td { border:1px solid #3f3f46; padding:2px 6px; height:22px; text-align:right; cursor:cell; }
  td.sel { background:#264f78; color:#fff; outline:1px solid #007acc; }
  .note { color:#9d9d9d; font-size:11px; margin-top:16px; padding:8px; background:#252526; border-left:2px solid #007acc; }
</style></head>
<body><div class="app">
  <h1>VisiCalc · Deno Desktop · Rust engine via Deno.dlopen (FFI)</h1>
  <div class="bar"><span class="addr" id="addr">A1</span>
    <input id="f" placeholder="Enter a value or formula, e.g. =SUM(A1:A4)"></div>
  <table id="grid"></table>
  <div class="note">The spreadsheet engine is the native <code>libspreadsheet_capi</code>
    dynamic library, loaded into the Deno process with <code>Deno.dlopen</code>. This page
    fetches computed values from a local API and posts edits back — the engine runs as
    native code, not WebAssembly.</div>
</div>
<script>
  const COLS = ${COLS};
  let sel = { r: 0, c: 1 }; // A1 (col 0 is the row-label gutter)
  const colLetter = (c) => String.fromCharCode(64 + c);
  const addr = (r, c) => colLetter(c) + (r + 1);
  async function refresh() {
    const cells = (await (await fetch("/api/window")).json()).cells || [];
    const t = document.getElementById("grid");
    t.innerHTML = "";
    cells.forEach((row, r) => {
      const tr = document.createElement("tr");
      row.forEach((val, c) => {
        const td = document.createElement("td");
        td.textContent = val;
        if (c >= 1) {
          if (r === sel.r && c === sel.c) td.className = "sel";
          td.onclick = () => select(r, c);
        } else { td.style.color = "#9d9d9d"; td.style.cursor = "default"; }
        tr.appendChild(td);
      });
      t.appendChild(tr);
    });
    document.getElementById("addr").textContent = addr(sel.r, sel.c);
  }
  async function select(r, c) {
    sel = { r, c };
    const raw = (await (await fetch("/api/raw?a1=" + addr(r, c))).json()).raw || "";
    document.getElementById("f").value = raw;
    await refresh();
  }
  document.getElementById("f").addEventListener("keydown", async (e) => {
    if (e.key !== "Enter") return;
    await fetch("/api/cell", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ a1: addr(sel.r, sel.c), raw: e.target.value }),
    });
    await refresh();
  });
  refresh();
</script>
</body></html>`;

const json = (data: unknown, status = 200) =>
  new Response(JSON.stringify(data), {
    status,
    headers: { "content-type": "application/json" },
  });

// `deno desktop` points the webview at this server automatically; a fixed port
// also lets you open it in a browser during `deno run` dev. Port 8792 avoids
// clashing with the WASM path's 8791.
Deno.serve(
  { hostname: "127.0.0.1", port: 8792 },
  async (req: Request): Promise<Response> => {
    const url = new URL(req.url);
    if (url.pathname === "/" || url.pathname === "/index.html") {
      return new Response(PAGE, {
        headers: { "content-type": "text/html; charset=utf-8" },
      });
    }
    if (url.pathname === "/api/window") {
      return json({ cells: displayWindow(ROWS, COLS) });
    }
    if (url.pathname === "/api/raw") {
      const a1 = url.searchParams.get("a1") ?? "";
      // Only A1-style refs are ever produced by the client; reject anything else
      // so no arbitrary string reaches the engine through this read path.
      if (!/^[A-Z]+[0-9]+$/.test(a1)) return json({ raw: "" });
      return json({ raw: getRaw(a1) });
    }
    if (url.pathname === "/api/cell" && req.method === "POST") {
      const body = await req.json().catch(() => null);
      const a1 = typeof body?.a1 === "string" ? body.a1 : "";
      const raw = typeof body?.raw === "string" ? body.raw : "";
      if (!/^[A-Z]+[0-9]+$/.test(a1)) return json({ ok: false }, 400);
      setCell(a1, raw); // engine recomputes every dependent cell
      return json({ ok: true });
    }
    return new Response("Not found", { status: 404 });
  },
);
