import { loadCorpusTestShards } from "./corpus-test-shards.js";

await loadCorpusTestShards(
  "marathi",
  import.meta.url,
  import.meta.glob("./marathi/*.case.ts"),
);
