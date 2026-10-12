import { loadCorpusTestShards } from "./corpus-test-shards.js";

await loadCorpusTestShards(
  "malayalam",
  import.meta.url,
  import.meta.glob("./malayalam/*.case.ts"),
);
