import { loadCorpusTestShards } from "./corpus-test-shards.js";

await loadCorpusTestShards(
  "german",
  import.meta.url,
  import.meta.glob("./german/*.case.ts"),
);
