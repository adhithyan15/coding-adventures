import { loadCorpusTestShards } from "./corpus-test-shards.js";

await loadCorpusTestShards(
  "bengali",
  import.meta.url,
  import.meta.glob("./bengali/*.case.ts"),
);
