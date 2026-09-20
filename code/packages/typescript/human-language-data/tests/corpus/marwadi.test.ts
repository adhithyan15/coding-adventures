import { loadCorpusTestShards } from "./corpus-test-shards.js";

await loadCorpusTestShards(
  "marwadi",
  import.meta.url,
  import.meta.glob("./marwadi/*.case.ts"),
);
