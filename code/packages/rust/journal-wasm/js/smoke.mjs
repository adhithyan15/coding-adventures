// Smoke test: load the built wasm and drive the engine from JS end to end.
//   ./build-wasm.sh && node js/smoke.mjs
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { createJournalEngine } from "./journal-engine.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const wasm = readFileSync(join(here, "..", "pkg", "journal_engine.wasm"));
let clock = 1_000;
const engine = createJournalEngine(wasm, { now: () => clock++ });

function assert(cond, msg) {
  if (!cond) {
    console.error(`FAIL: ${msg}`);
    process.exit(1);
  }
}

assert(engine.timeline().code === "uninitialised", "queries before init are envelopes");
assert(engine.snapshot() === "null", "snapshot before init is null");

assert(engine.init({ journalId: "personal", name: "Personal" }).ok, "init");
const create = (id, date, title, body) =>
  engine.apply({ type: "createEntry", id, journal: "personal", date, title, body });
assert(create("e1", "2025-09-23", "First light", "Walked to the lighthouse.").ok, "create e1");
assert(create("e2", "2026-09-23", "Rain", "Stayed in. Café au lait.").ok, "create e2");
assert(engine.apply({ type: "setTags", id: "e1", tags: ["Travel", "travel"] }).ok, "tags");

const days = engine.timeline();
assert(days.ok && days.data.length === 2 && days.data[0].date === "2026-09-23", "timeline");

const recall = engine.onThisDay("2026-09-23");
assert(recall.ok && recall.data[0].yearsAgo === 1, "on this day");

const hits = engine.search("CAFÉ");
assert(hits.ok && hits.data[0].entry === "e2", "non-ASCII case-insensitive search");

const tags = engine.tagCounts();
assert(tags.ok && tags.data.length === 1 && tags.data[0].tag === "Travel", "tag dedup");

// Rejections come back as coded envelopes, not throws.
assert(create("e1", "2026-01-01", "", "").code === "duplicateId", "duplicate id");
assert(engine.apply({ type: "deleteJournal", id: "personal" }).code === "lastJournal", "last journal");

// Legacy import: one good row, one impossible date.
const imp = engine.importLegacy("personal", [
  { id: "old1", title: "Old", content: "hi", createdAt: "2020-01-01", updatedAt: 5 },
  { id: "old2", title: "Bad", content: "", createdAt: "2020-02-30", updatedAt: 5 },
]);
assert(imp.ok && imp.data.imported === 1 && imp.data.skipped[0].code === "invalidDate", "import");

// Persistence round trip, and a hostile snapshot is refused whole.
const snap = engine.snapshot();
engine.reset();
assert(engine.load(snap).ok, "load snapshot");
assert(engine.entry("old1").data.title === "Old", "state restored");
assert(engine.load('{"journals":{},"entries":{}}').code === "lastJournal", "empty state refused");
assert(engine.entry("old1").ok, "old state kept after refusal");

console.log("journal-wasm smoke test passed");
