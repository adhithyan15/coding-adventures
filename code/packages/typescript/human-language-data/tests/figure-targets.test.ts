import { loadCorpusTestShards } from "./corpus/corpus-test-shards.js";

await loadCorpusTestShards(
  "figure-targets",
  import.meta.url,
  import.meta.glob("./figure-targets/*.case.ts"),
);
