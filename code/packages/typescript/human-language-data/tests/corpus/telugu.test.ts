import { loadCorpusTestShards } from "./corpus-test-shards.js";

await loadCorpusTestShards(
  "telugu",
  import.meta.url,
  import.meta.glob("./telugu/*.case.ts"),
);
