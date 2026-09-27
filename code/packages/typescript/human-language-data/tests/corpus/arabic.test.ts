import { loadCorpusTestShards } from "./corpus-test-shards.js";

await loadCorpusTestShards(
  "arabic",
  import.meta.url,
  import.meta.glob("./arabic/*.case.ts"),
);
