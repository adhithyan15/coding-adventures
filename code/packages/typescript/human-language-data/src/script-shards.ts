import {
  mergeScriptInventoryShards as mergeUntypedScriptInventoryShards,
  type Shard,
} from "./shard.js";
import type { ScriptData } from "./types.js";

export { scriptEntryId } from "./shard.js";

/** Typed public facade over the config-safe implementation in `shard.ts`. */
export function mergeScriptInventoryShards(shards: Shard[]): ScriptData {
  return mergeUntypedScriptInventoryShards<ScriptData>(shards);
}
