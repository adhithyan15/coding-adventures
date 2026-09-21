import { loadCorpusTestShards } from "./corpus-test-shards.js";

await loadCorpusTestShards(
  "urdu",
  import.meta.url,
  import.meta.glob("./urdu/*.case.ts"),
);
