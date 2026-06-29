// End-to-end smoke test for the compiled Engram WASM boundary.
//
// Run after build-wasm.sh:
//   node js/smoke.mjs

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { createEngramEngine } from "./engram-mosaic-host-wasm.mjs";

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

console.log(failures === 0 ? "\nALL PASS" : `\n${failures} FAILURE(S)`);
process.exit(failures === 0 ? 0 : 1);
