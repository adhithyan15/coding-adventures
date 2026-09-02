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
check("host status hidden", initial.props.hostStatusVisible, false);

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
    return intent.type === "importAnki"
      ? { status: "imported", name: "deck.apkg", size: 2048 }
      : { status: "captured" };
  },
});
const imported = await intentHost.handleEvent({ component: "EngramApp", event: "onImportAnki" });
check("host intent type", imported.hostIntent.type, "importAnki");
check("host intent callback", seenIntent.type, "importAnki");
check("host intent result", imported.hostResult, {
  status: "imported",
  name: "deck.apkg",
  size: 2048,
});
check("host status visible", imported.props.hostStatusVisible, true);
check("host status kind", imported.props.hostStatusKind, "imported");
check("host status label", imported.props.hostStatusLabel, "Import complete");
check("host status message", imported.props.hostStatusMessage, "Imported deck.apkg (2.0 KB).");

const opened = await intentHost.handleEvent({
  component: "EngramApp",
  event: "onBrowserOpenSelected",
});
check("browser open intent", opened.hostIntent.type, "openCard");
check("browser open card", opened.hostIntent.cardId, "card");

const demoEngine = createEngramEngine(wasm, { demo: true, now: () => 1700000000000 });
const demoSnapshot = demoEngine.demoSnapshot();
check("demo snapshot first deck", demoSnapshot.decks[0].id, "tamil-script");
const demoHost = demoEngine.createMosaicHost();
const demo = await demoHost.getProps({ component: "EngramApp" });
check("demo host deck name", demo.props.deckName, "Tamil::Script and Roots");
check("demo host deck total", demo.props.deckTotalValue, "2");
check("demo host note count", demo.props.collectionNoteCountValue, "5");
// Each row is [ name, due, new ]. Only Tamil has anything due, so only its row
// carries a due count -- an empty string rather than "0 due", so the decks with
// work waiting are the ones that stand out in the list.
check("demo host deck rows", demo.props.deckRows, [
  ["Tamil::Script and Roots", "1 due", "1 new"],
  ["Hindi::Devanagari", "", "1 new"],
  ["Kannada::Script", "", "1 new"],
  ["Spanish::Latin Roots", "", "1 new"],
]);

// Legacy APKG now runs IN the browser build. These assertions used to expect
// delegation — `ok: false` with an error mentioning "native hosts" — because the
// package layer was compiled out of wasm entirely. It no longer is.
//
// They kept passing after that changed, because this file runs against the
// checked-in `pkg/engram_engine.wasm`, which was itself stale. Two stale
// artifacts agreeing with each other reads exactly like a passing test, which is
// why the wasm is now rebuilt and compared in CI rather than trusted.
const exportedApkg = demoEngine.exportAnkiApkg();
check("apkg export succeeds in the browser build", exportedApkg.ok, true);
check("apkg export returns bytes", exportedApkg.apkg.length > 0, true);

// A legacy package round-trips back in through the same ABI.
const mergeEngine = createEngramEngine(wasm, { now: () => 1700000000000 });
const mergedApkg = mergeEngine.mergeAnkiApkg(Uint8Array.from(exportedApkg.apkg));
check("apkg merge succeeds in the browser build", mergedApkg.ok, true);

// Something that is not a package at all is still refused, so the assertions
// above cannot be satisfied by an implementation that accepts anything.
const rejectEngine = createEngramEngine(wasm, { now: () => 1700000000000 });
const rejected = rejectEngine.mergeAnkiApkg(new Uint8Array([1, 2, 3]));
check("garbage bytes are rejected", rejected.ok, false);

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
