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
      bookmarksLabel: "Bookmarks (0)",
      bookmarksDisabled: true,
      bookmarksOpen: false,
      bookmarksPosition: "",
      bookmarksTitle: "",
      bookmarksAddress: "",
      bookmarksPreviousDisabled: true,
      bookmarksNextDisabled: true,
      bookmarksNavigateDisabled: true,
      copyAddressDisabled: true,
      openPageDisabled: true,
      savePageDisabled: true,
      printPageDisabled: true,
      sharePageDisabled: true,
      pageInfoDisabled: true,
      pageInfoOpen: false,
      pageInfoTitle: "Initial page",
      pageInfoAddress: "https://venture.test/final",
      pageInfoRequestedAddress: "https://venture.test/initial",
      pageInfoStatus: "HTTP 200",
      pageInfoResources: "Images: 2 (0 failed)  Stylesheets: 1 (0 failed)",
      zoomLabel: "100%",
      zoomOutDisabled: true,
      zoomResetDisabled: true,
      zoomInDisabled: true,
      viewSourceDisabled: true,
      viewSourceOpen: false,
      viewSourceTitle: "Initial page",
      viewSourceAddress: "https://venture.test/final",
      viewSourceContent: "<html><title>Initial page</title></html>",
      findOpen: false,
      findQuery: "",
      findResultLabel: "",
      findDisabled: true,
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
          props = {
            ...props,
            bookmarkLabel: "Remove Bookmark",
            bookmarksLabel: "Bookmarks (1)",
            bookmarksDisabled: false,
            bookmarksPosition: "1 of 1",
            bookmarksTitle: "Venture web acceptance",
            bookmarksAddress: props.address,
            bookmarksNavigateDisabled: false,
          };
        }
        if (request.event.type === "bookmarksOpen") {
          props = { ...props, bookmarksOpen: true };
        }
        if (request.event.type === "bookmarksClose" || request.event.type === "bookmarksNavigate") {
          props = { ...props, bookmarksOpen: false };
        }
        if (request.event.type === "findChange") {
          props = { ...props, findQuery: request.event.value, findResultLabel: "1 of 2" };
        }
        if (request.event.type === "findNext") {
          props = { ...props, findResultLabel: "2 of 2" };
        }
        if (request.event.type === "findOpen") {
          props = { ...props, findOpen: true };
        }
        if (request.event.type === "findClose") {
          props = { ...props, findOpen: false, findQuery: "", findResultLabel: "" };
        }
        if (request.event.type === "zoomIn") {
          props = { ...props, zoomLabel: "125%", zoomResetDisabled: false };
        }
        if (request.event.type === "zoomReset") {
          props = { ...props, zoomLabel: "100%", zoomResetDisabled: true };
        }
        if (request.event.type === "pageInfo") {
          props = { ...props, pageInfoOpen: true };
        }
        if (request.event.type === "pageInfoClose") {
          props = { ...props, pageInfoOpen: false };
        }
        if (request.event.type === "viewSource") {
          props = { ...props, viewSourceOpen: true };
        }
        if (request.event.type === "viewSourceClose") {
          props = { ...props, viewSourceOpen: false };
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
    assert.equal(controls.bookmarks.disabled, true);
    assert.equal(controls.copyAddress.disabled, true);
    assert.equal(controls.openPage.disabled, true);
    assert.equal(controls.savePage.disabled, true);
    assert.equal(controls.printPage.disabled, true);
    assert.equal(controls.sharePage.disabled, true);
    assert.equal(controls.pageInfo.disabled, true);
    assert.equal(controls.zoomOut.disabled, true);
    assert.equal(controls.zoomReset.disabled, true);
    assert.equal(controls.zoomIn.disabled, true);
    assert.equal(controls.viewSource.disabled, true);
    assert.equal(controls.go.disabled, true);
    assert.equal(controls.address.readOnly, true);
    assert.equal(controls.find, undefined);
    assert.match(renderScope(root).textContent, /Ready from MosaicHost/);
    assert.ok(root.querySelector(`[data-venture-host-surface="${backend}"]`));

    controls.back.click();
    controls.bookmark.click();
    controls.bookmarks.click();
    controls.copyAddress.click();
    controls.openPage.click();
    controls.savePage.click();
    controls.printPage.click();
    controls.sharePage.click();
    controls.pageInfo.click();
    controls.zoomOut.click();
    controls.zoomReset.click();
    controls.zoomIn.click();
    controls.viewSource.click();
    controls.findOpen.click();
    controls.go.click();
    await settle();
    assert.deepEqual(calls, [], "disabled native buttons must suppress dispatch");

    props = {
      ...props,
      backDisabled: false,
      bookmarkDisabled: false,
      copyAddressDisabled: false,
      openPageDisabled: false,
      savePageDisabled: false,
      printPageDisabled: false,
      sharePageDisabled: false,
      pageInfoDisabled: false,
      zoomOutDisabled: false,
      zoomResetDisabled: true,
      zoomInDisabled: false,
      viewSourceDisabled: false,
      findDisabled: false,
      navigationDisabled: false,
      statusText: "Enabled by mosaic-host-ready",
    };
    window.dispatchEvent(new Event("mosaic-host-ready"));
    await settle();

    controls = readControls(root);
    assert.equal(controls.back.disabled, false);
    assert.equal(controls.reload.disabled, false);
    assert.equal(controls.bookmark.disabled, false);
    assert.equal(controls.copyAddress.disabled, false);
    assert.equal(controls.openPage.disabled, false);
    assert.equal(controls.savePage.disabled, false);
    assert.equal(controls.printPage.disabled, false);
    assert.equal(controls.sharePage.disabled, false);
    assert.equal(controls.pageInfo.disabled, false);
    assert.equal(controls.zoomOut.disabled, false);
    assert.equal(controls.zoomReset.disabled, true);
    assert.equal(controls.zoomIn.disabled, false);
    assert.equal(controls.viewSource.disabled, false);
    assert.equal(controls.go.disabled, false);
    assert.equal(controls.address.readOnly, false);
    assert.equal(controls.find, undefined);
    assert.match(renderScope(root).textContent, /Enabled by mosaic-host-ready/);

    controls.bookmark.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "toggleBookmark");
    controls = readControls(root);
    assert.equal(controls.bookmark.textContent.trim(), "Remove Bookmark");
    assert.equal(controls.bookmarks.disabled, false);

    controls.bookmarks.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "bookmarksOpen");
    assert.match(renderScope(root).textContent, /1 of 1/);
    assert.match(renderScope(root).textContent, /Venture web acceptance/);
    buttonByLabel(root, "Close").click();
    await settle();
    assert.equal(calls.at(-1)?.type, "bookmarksClose");
    controls = readControls(root);

    controls.copyAddress.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "copyAddress");

    controls = readControls(root);
    controls.openPage.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "openPageInNewWindow");

    controls = readControls(root);
    controls.savePage.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "savePage");

    controls = readControls(root);
    controls.printPage.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "printPage");

    controls = readControls(root);
    controls.sharePage.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "sharePage");

    controls = readControls(root);
    controls.pageInfo.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "pageInfo");
    assert.match(renderScope(root).textContent, /Initial page/);
    assert.match(renderScope(root).textContent, /HTTP 200/);
    buttonByLabel(root, "Close").click();
    await settle();
    assert.equal(calls.at(-1)?.type, "pageInfoClose");
    assert.doesNotMatch(renderScope(root).textContent, /Images: 2 \(0 failed\)/);

    controls = readControls(root);
    controls.zoomIn.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "zoomIn");
    controls = readControls(root);
    assert.equal(controls.zoomReset.textContent.trim(), "125%");
    controls.zoomReset.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "zoomReset");

    controls = readControls(root);
    controls.viewSource.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "viewSource");
    assert.match(renderScope(root).textContent, /<html><title>Initial page<\/title><\/html>/);
    assert.match(renderScope(root).textContent, /https:\/\/venture\.test\/final/);
    buttonByLabel(root, "Copy Source").click();
    await settle();
    assert.equal(calls.at(-1)?.type, "viewSourceCopy");
    assert.match(renderScope(root).textContent, /<html><title>Initial page<\/title><\/html>/);
    buttonByLabel(root, "Close Source").click();
    await settle();
    assert.equal(calls.at(-1)?.type, "viewSourceClose");
    assert.doesNotMatch(renderScope(root).textContent, /<html><title>Initial page<\/title><\/html>/);

    controls = readControls(root);
    controls.findOpen.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "findOpen");
    controls = readControls(root);
    assert.equal(controls.find.readOnly, false);
    controls.find.value = "venture";
    controls.find.dispatchEvent(new Event(backend === "html" ? "input" : "change", { bubbles: true }));
    await settle();
    assert.deepEqual(calls.at(-1), { type: "findChange", value: "venture" });
    assert.match(renderScope(root).textContent, /1 of 2/);
    controls = readControls(root);
    controls.findNext.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "findNext");
    assert.match(renderScope(root).textContent, /2 of 2/);
    controls = readControls(root);
    controls.findClose.click();
    await settle();
    assert.equal(calls.at(-1)?.type, "findClose");
    controls = readControls(root);
    assert.equal(controls.find, undefined);

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
      ["toggleBookmark", "bookmarksOpen", "bookmarksClose", "copyAddress", "openPageInNewWindow", "savePage", "printPage", "sharePage", "pageInfo", "pageInfoClose", "zoomIn", "zoomReset", "viewSource", "viewSourceCopy", "viewSourceClose", "findOpen", "findChange", "findNext", "findClose", "addressChange", "navigate", "navigate"],
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

function buttonByLabel(root, label) {
  const button = [...renderScope(root).querySelectorAll("button")]
    .find(candidate => candidate.textContent.trim() === label);
  assert.ok(button, `${label} button must exist`);
  return button;
}

function readControls(root) {
  const scope = renderScope(root);
  assert.ok(scope, `${backend} render scope must exist`);
  const buttons = new Map(
    [...scope.querySelectorAll("button")].map(button => [button.textContent.trim(), button]),
  );
  const [address, find] = scope.querySelectorAll("input");
  assert.ok(address, "address input must exist");
  for (const label of ["Back", "Forward", "Reload", "Bookmark", "Remove Bookmark", "Copy", "New Window", "Save", "Print", "Share", "Info", "Zoom Out", "Zoom In", "Source", "Find", "Go"]) {
    if (label === "Bookmark" || label === "Remove Bookmark") continue;
    assert.ok(buttons.has(label), `${label} button must exist`);
  }
  const bookmark = buttons.get("Bookmark") ?? buttons.get("Remove Bookmark");
  assert.ok(bookmark, "bookmark button must exist");
  const bookmarks = [...buttons.entries()].find(([label]) => /^Bookmarks \(\d+\)$/.test(label))?.[1];
  assert.ok(bookmarks, "bookmarks button must exist");
  return {
    back: buttons.get("Back"),
    forward: buttons.get("Forward"),
    reload: buttons.get("Reload"),
    bookmark,
    bookmarks,
    copyAddress: buttons.get("Copy"),
    openPage: buttons.get("New Window"),
    savePage: buttons.get("Save"),
    printPage: buttons.get("Print"),
    sharePage: buttons.get("Share"),
    pageInfo: buttons.get("Info"),
    zoomOut: buttons.get("Zoom Out"),
    zoomReset: [...buttons.entries()].find(([label]) => /^\d+%$/.test(label))?.[1],
    zoomIn: buttons.get("Zoom In"),
    viewSource: buttons.get("Source"),
    findOpen: buttons.get("Find"),
    go: buttons.get("Go"),
    findNext: buttons.get("Next"),
    findClose: buttons.get("Close Find"),
    address,
    find,
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
