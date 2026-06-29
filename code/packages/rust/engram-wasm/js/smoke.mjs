// End-to-end smoke test for the compiled Engram WASM boundary.
//
// Run after build-wasm.sh:
//   node js/smoke.mjs

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { createEngramEngine, installEngramMosaicHost } from "./engram-mosaic-host-wasm.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const wasm = readFileSync(join(here, "..", "pkg", "engram_engine.wasm"));
const engine = createEngramEngine(wasm, { deckId: "deck", now: () => 1700000000000 });

let failures = 0;
function check(label, got, want) {
  const ok = JSON.stringify(got) === JSON.stringify(want);
  if (!ok) failures++;
  console.log(`${ok ? "ok  " : "FAIL"} ${label}`);
  if (!ok) {
    console.log("  got :", JSON.stringify(got));
    console.log("  want:", JSON.stringify(want));
  }
}

engine.reset();
const snapshot = {
  decks: [{ id: "deck", name: "Tamil", description: "Script", createdAt: 1700000000000 }],
  noteTypes: [],
  notes: [],
  cards: [{ id: "card", deckId: "deck", front: "letter-a", back: "a", createdAt: 1700000000000 }],
  cardProgress: [],
  sessions: [],
  reviews: [],
  activeSession: null,
};

check("load snapshot", engine.loadSnapshot(snapshot).ok, true);
check("deck stats", engine.getDeckStats("deck", 1700000000000).stats.total, 1);

const host = engine.createMosaicHost();
const initial = await host.getProps({ component: "EngramApp" });
check("host prop camelCase", initial.props.appTitle, "Engram");
check("host deck name", initial.props.deckName, "Tamil");
check("host list prop", initial.props.browserResultCardIds, ["card"]);

engine.dispatch({
  type: "startSession",
  sessionId: "session",
  deckId: "deck",
  queue: snapshot.cards,
  startedAt: 1700000000000,
});

const revealed = await host.handleEvent({ component: "EngramApp", event: { type: "reveal" } });
check("event updates props", revealed.props.answerVisible, true);

let seenIntent = null;
const intentHost = engine.createMosaicHost({
  onHostIntent: (intent) => {
    seenIntent = intent;
    return { handled: true };
  },
});
const imported = await intentHost.handleEvent({ component: "EngramApp", event: "onImportAnki" });
check("host intent type", imported.hostIntent.type, "importAnki");
check("host intent callback", seenIntent.type, "importAnki");
check("host intent result", imported.hostResult, { handled: true });

const opened = await intentHost.handleEvent({
  component: "EngramApp",
  event: "onBrowserOpenSelected",
});
check("browser open intent", opened.hostIntent.type, "openCard");
check("browser open card", opened.hostIntent.cardId, "card");

let readyEvent = null;
const fakeWindow = {
  CustomEvent: class CustomEvent {
    constructor(type, init) {
      this.type = type;
      this.detail = init?.detail;
    }
  },
  dispatchEvent(event) {
    readyEvent = event;
  },
};
installEngramMosaicHost(fakeWindow, wasm, { deckId: "deck", now: () => 1700000000000 });
check("install host platform", fakeWindow.mosaicHost.platform, "engram-wasm");
check("install ready event", readyEvent.type, "mosaic-host-ready");
check("install ready detail", readyEvent.detail, { platform: "engram-wasm" });

console.log(failures === 0 ? "\nALL PASS" : `\n${failures} FAILURE(S)`);
process.exitCode = failures === 0 ? 0 : 1;
