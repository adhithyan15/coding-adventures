import { loadCorpusTestShards } from "./corpus-test-shards.js";

await loadCorpusTestShards(
  "sanskrit",
  import.meta.url,
  import.meta.glob("./sanskrit/*.case.ts"),
);
