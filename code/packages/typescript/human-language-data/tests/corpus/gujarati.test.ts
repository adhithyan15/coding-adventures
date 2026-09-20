import { loadCorpusTestShards } from "./corpus-test-shards.js";

await loadCorpusTestShards(
  "gujarati",
  import.meta.url,
  import.meta.glob("./gujarati/*.case.ts"),
);
