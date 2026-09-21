import { loadCorpusTestShards } from "./corpus-test-shards.js";

await loadCorpusTestShards(
  "tamil",
  import.meta.url,
  import.meta.glob("./tamil/*.case.ts"),
);
