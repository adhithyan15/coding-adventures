// One immutable default-root input for the real track snapshot shards.
// Consumers derive fresh reports/sets inside their original timed cases; live
// inventories, CLI checks and mutable temporary-root fixtures remain independent.
import { loadEverything } from "../../src/loader.js";

export const REAL_EVERYTHING = loadEverything();
