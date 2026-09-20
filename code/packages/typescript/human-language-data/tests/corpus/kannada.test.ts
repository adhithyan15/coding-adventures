import { loadCorpusTestShards } from "./corpus-test-shards.js";

await loadCorpusTestShards(
  "kannada",
  import.meta.url,
  import.meta.glob("./kannada/*.case.ts"),
);
