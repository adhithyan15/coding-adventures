import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { JSDOM } from "jsdom";

const backend = existsSync(resolve("VentureChrome.js")) ? "webcomponent" : "html";

test(`${backend} controls cross the Mosaic host seam`, async () => {
  const source = stripScriptTags(readFileSync(resolve("index.html"), "utf8"));
  const dom = new JSDOM(source, {
    url: "https://venture.test/",
    runScripts: "dangerously",
  });
  const installedGlobals = installDomGlobals(dom.window);

  try {
    const calls = [];
    const contentSurface = document.createElement("div");
    contentSurface.dataset.ventureHostSurface = backend;
    let props = {
      address: "https://venture.test/initial",
      pageTitle: "Initial page",
      statusText: "Ready from MosaicHost",
      backDisabled: true,
      forwardDisabled: false,
      bookmarkLabel: "Bookmark",
      bookmarkDisabled: true,
      viewSourceDisabled: true,
      navigationDisabled: true,
      contentSurface,
    };

    window.mosaicHost = {
      async getProps() {
        return { props };
      },
      async handleEvent(request) {
        calls.push(request.event);
        if (request.event.type === "addressChange") {
          props = { ...props, address: request.event.value };
        }
        if (request.event.type === "toggleBookmark") {
          props = { ...props, bookmarkLabel: "Remove Bookmark" };
        }
        props = {
          ...props,
          statusText: `Handled ${request.event.type} through MosaicHost`,
        };
        return { props };
      },
    };

    if (backend === "webcomponent") {
      await importFresh("VentureChrome.js");
    }
    await importFresh("main.js");
    await settle();

    const root = findRoot();
    let controls = readControls(root);
    assert.equal(controls.back.disabled, true);
    assert.equal(controls.forward.disabled, false);
    assert.equal(controls.reload.disabled, true);
    assert.equal(controls.bookmark.disabled, true);
    assert.equal(controls.viewSource.disabled, true);
    assert.equal(controls.go.disabled, true);
    assert.equal(controls.address.readOnly, true);
    assert.match(renderScope(root).textContent, /Ready from MosaicHost/);
    assert.ok(root.querySelector(`[data-venture-host-surface="${backend}"]`));

    controls.back.click();
    controls.bookmark.click();
    controls.viewSource.click();
    controls.go.click();
    await settle();
    assert.deepEqual(calls, [], "disabled native buttons must suppress dispatch");

    props = {
      ...props,
      backDisabled: false,
      bookmarkDisabled: false,
      viewSourceDisabled: false,
      navigationDisabled: false,
      statusText: "Enabled by mosaic-host-ready",
    };
    window.dispatchEvent(new Event("mosaic-host-ready"));
    await settle();

    controls = readControls(root);
    assert.equal(controls.back.disabled, false);
    assert.equal(controls.reload.disabled, false);
    assert.equal(controls.bookmark.disabled, false);
    assert.equal(controls.viewSource.disabled, false);
    assert.equal(controls.go.disabled, false);
    assert.equal(controls.address.readOnly, false);
    assert.match(renderScope(root).textContent, /Enabled by mosaic-host-ready/);

    controls.bookmark.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "toggleBookmark");
    controls = readControls(root);
    assert.equal(controls.bookmark.textContent.trim(), "Remove Bookmark");

    controls.viewSource.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "viewSource");

    controls = readControls(root);
    const nextAddress = "https://venture.test/next";
    controls.address.value = nextAddress;
    controls.address.dispatchEvent(
      new Event(backend === "html" ? "input" : "change", { bubbles: true }),
    );
    await settle();
    assert.deepEqual(calls.at(-1), { type: "addressChange", value: nextAddress });
    assert.match(renderScope(root).textContent, /Handled addressChange through MosaicHost/);

    controls = readControls(root);
    controls.address.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
    );
    await settle();
    assert.equal(calls.at(-1)?.type, "navigate");

    controls = readControls(root);
    controls.go.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "navigate");
    assert.deepEqual(
      calls.map(event => event.type),
      ["toggleBookmark", "viewSource", "addressChange", "navigate", "navigate"],
    );
    assert.match(renderScope(root).textContent, /Handled navigate through MosaicHost/);
  } finally {
    restoreDomGlobals(installedGlobals);
    dom.window.close();
  }
});

// A single-pass replace can reintroduce a "<script" sequence when two
// overlapping matches straddle each other (e.g. "<scr<script>ipt>ipt>"),
// so keep stripping until a pass makes no further change.
function stripScriptTags(html) {
  let current = html;
  let previous;
  do {
    previous = current;
    current = previous.replace(
      /<script\b[^>]*>[\s\S]*?<\/script>|<script\b[^>]*\/?>/gi,
      "",
    );
  } while (current !== previous);
  return current;
}

function findRoot() {
  const root =
    backend === "html"
      ? document.querySelector('[data-mosaic-html-root="VentureChrome"]')
      : document.querySelector("mos-venture-chrome");
  assert.ok(root, `${backend} root must exist`);
  return root;
}

function renderScope(root) {
  return backend === "html" ? root : root.shadowRoot;
}

function readControls(root) {
  const scope = renderScope(root);
  assert.ok(scope, `${backend} render scope must exist`);
  const buttons = new Map(
    [...scope.querySelectorAll("button")].map(button => [button.textContent.trim(), button]),
  );
  const address = scope.querySelector("input");
  assert.ok(address, "address input must exist");
  for (const label of ["Back", "Forward", "Reload", "Bookmark", "Remove Bookmark", "View Source", "Go"]) {
    if (label === "Bookmark" || label === "Remove Bookmark") continue;
    assert.ok(buttons.has(label), `${label} button must exist`);
  }
  const bookmark = buttons.get("Bookmark") ?? buttons.get("Remove Bookmark");
  assert.ok(bookmark, "bookmark button must exist");
  return {
    back: buttons.get("Back"),
    forward: buttons.get("Forward"),
    reload: buttons.get("Reload"),
    bookmark,
    viewSource: buttons.get("View Source"),
    go: buttons.get("Go"),
    address,
  };
}

async function importFresh(file) {
  const url = pathToFileURL(resolve(file));
  url.searchParams.set("acceptance", `${backend}-${Date.now()}-${Math.random()}`);
  await import(url.href);
}

async function settle() {
  await new Promise(resolvePromise => setTimeout(resolvePromise, 0));
  await new Promise(resolvePromise => setTimeout(resolvePromise, 0));
}

function installDomGlobals(windowObject) {
  const names = [
    "window",
    "document",
    "Node",
    "Element",
    "HTMLElement",
    "customElements",
    "CustomEvent",
    "Event",
    "KeyboardEvent",
  ];
  const previous = new Map();
  for (const name of names) {
    previous.set(name, globalThis[name]);
    globalThis[name] = windowObject[name];
  }
  return previous;
}

function restoreDomGlobals(previous) {
  for (const [name, value] of previous) {
    if (value === undefined) {
      delete globalThis[name];
    } else {
      globalThis[name] = value;
    }
  }
}
