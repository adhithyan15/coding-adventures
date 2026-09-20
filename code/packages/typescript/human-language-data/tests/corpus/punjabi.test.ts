import { loadCorpusTestShards } from "./corpus-test-shards.js";

await loadCorpusTestShards(
  "punjabi",
  import.meta.url,
  import.meta.glob("./punjabi/*.case.ts"),
);
