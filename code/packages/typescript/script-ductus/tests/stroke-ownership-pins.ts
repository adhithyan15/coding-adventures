// ---------------------------------------------------------------------------
// stroke-ownership-pins.ts — measure, load and write the per-script registry pins
// ---------------------------------------------------------------------------
//
// `stroke-ownership.test.ts` pins the whole `DUCTUS` registry: every key, the
// order of those keys, every byte of their data, and the stroke objects the
// Arabic family deliberately shares between scripts. It used to do that with
// one literal — `{ keys, keyHash, nonTamilDataHash, counts, … }` — and that
// literal moved whenever ANY script gained or changed a glyph. Two PRs, one
// adding a Kannada digit and one refitting a Malayalam vowel sign, both edited
// the same five lines and so conflicted by construction (#12118, #13193).
//
// This module splits that one literal into one pin file per script, plus one
// small file for the facts that genuinely belong to no single script:
//
//     tests/stroke-ownership/
//       _registry.json     scriptRuns, sharedIdentityGroups, sharedIdentityHash
//       arabic.json        count, runs, keyHash, dataHash
//       kannada.json       count, runs, keyHash, dataHash
//       tamil.json         count, runs, keyHash            (no dataHash — below)
//       …                  one file per canonical `script`, no more, no fewer
//
// A Kannada PR now rewrites kannada.json and nothing else, so it cannot meet a
// Malayalam PR in a merge. And the split loses nothing — see "Why the split is
// lossless" below.
//
// What each field pins
// --------------------
//
//   count       how many registry entries carry `script: <this script>`
//               (the old `counts[script]`)
//   runs        the length of each contiguous block this script occupies in
//               `Object.keys(DUCTUS)`, in order. Most scripts are one block, so
//               `runs` is `[count]`; Tamil is `[26, 10]` (a main owner and a
//               tail owner), and the Arabic family interleaves three scripts.
//   keyHash     SHA-256 of this script's keys, in registry order
//   dataHash    SHA-256 of `JSON.stringify` of this script's entries, in order
//
// and in `_registry.json`, for the registry as a whole:
//
//   scriptRuns            the script of each contiguous block, in order — the
//                         "shape" of the registry: japanese, chinese, …, latin
//   sharedIdentityGroups  how many objects are reachable from two different
//                         keys (the Arabic family shares checked stroke arrays)
//   sharedIdentityHash    SHA-256 of the exact paths that share each object
//
// Why the split is lossless
// -------------------------
//
// The old pin hashed the ORDERED list of all keys. The new pins never store
// that list, yet they still determine it exactly. Walk `scriptRuns`; the i-th
// time a script appears, take the next `runs[i]` keys from that script's own
// ordered key list. Concatenate. That rebuilds `Object.keys(DUCTUS)` key for
// key, and `interleaveScriptKeys` below does exactly that so the test can
// PROVE the reconstruction rather than assert it:
//
//     scriptRuns:  [ tamil , kannada , tamil ]
//     runs:        tamil [2, 1]   kannada [2]
//     keys:        tamil [அ, ஆ, எ]   kannada [ಕ, ಗ]
//     rebuilt:     அ ஆ | ಕ ಗ | எ
//
// So any change the old literal caught still fails here:
//
//   a glyph added, removed or renamed  -> that script's count/keyHash move
//   a glyph moved within its script    -> that script's keyHash moves
//   a block moved between scripts      -> scriptRuns or that script's runs move
//   any byte of non-Tamil data changed -> that script's dataHash moves
//   a shared stroke split or rejoined  -> the shared-identity values move
//   a new script appears               -> it has no pin file: fail
//   a script disappears                -> its pin file is stale: fail
//
// Why Tamil has no dataHash
// -------------------------
//
// The old literal hashed only NON-Tamil data (`nonTamilDataHash`): Tamil's data
// is pinned glyph by glyph in `tests/strokes/tamil/U-XXXX.test.ts`, so that two
// Tamil PRs on different letters do not collide even within Tamil. The shard
// keeps that boundary exactly: `tamil.json` pins Tamil's keys, order and count,
// and its data stays with the per-glyph owners. `DATA_PINNED_PER_GLYPH` is the
// one place that rule lives.
// ---------------------------------------------------------------------------

import { createHash } from "node:crypto";
import { lstatSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

/** Scripts whose data is pinned per glyph elsewhere, so their shard omits `dataHash`. */
export const DATA_PINNED_PER_GLYPH: ReadonlySet<string> = new Set(["tamil"]);

/** The one pin file that belongs to no script. Its leading `_` cannot name a script. */
export const REGISTRY_PIN_FILE = "_registry.json";

/** A canonical script id as it appears in a pin filename: `perso-arabic`, `latin`. */
const SCRIPT_NAME = /^[a-z][a-z0-9-]*$/;
const SHA256_HEX = /^[0-9a-f]{64}$/;

export interface ScriptPin {
  count: number;
  runs: number[];
  keyHash: string;
  dataHash?: string;
}

export interface RegistryPin {
  scriptRuns: string[];
  sharedIdentityGroups: number;
  sharedIdentityHash: string;
}

export interface StrokeOwnershipPins {
  registry: RegistryPin;
  scripts: Record<string, ScriptPin>;
}

/** The smallest shape of a registry entry the pins need: its canonical script. */
type ScriptedEntry = { readonly script: string };

const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

/**
 * Every object reachable from two DIFFERENT top-level keys, as the sorted list
 * of paths that reach it. The Arabic family shares checked stroke arrays and
 * stroke objects between Arabic, Perso-Arabic and Urdu-Nastaliq on purpose
 * (HL27 §3 item 6); this is how a test notices one being split or rejoined.
 */
export const sharedObjectIdentityGroups = (
  registry: Readonly<Record<string, object>>,
): string[][] => {
  const seen = new Map<object, { path: string; root: string }>();
  const shared = new Map<object, string[]>();

  const visit = (value: unknown, path: string, root: string): void => {
    if (value === null || typeof value !== "object") return;
    const previous = seen.get(value);
    if (previous !== undefined) {
      if (previous.root !== root) {
        const paths = shared.get(value) ?? [previous.path];
        paths.push(path);
        shared.set(value, paths);
      }
      return;
    }
    seen.set(value, { path, root });
    for (const [key, child] of Object.entries(value)) {
      visit(child, `${path}.${key}`, root);
    }
  };

  for (const [key, value] of Object.entries(registry)) {
    visit(value, JSON.stringify(key), key);
  }
  return [...shared.values()].sort(([a], [b]) => a.localeCompare(b));
};

/** One script's keys in registry order, plus its block lengths, before hashing. */
export interface ScriptLayout {
  keys: string[];
  runs: number[];
}

/**
 * Cut the registry's key order into contiguous same-script blocks.
 *
 *     keys:     a1 a2 b1 a3      (scripts a a b a)
 *     blocks:   a(2) b(1) a(1)
 *     result:   scriptRuns [a, b, a]   a { keys [a1 a2 a3], runs [2, 1] }
 *                                      b { keys [b1],       runs [1] }
 */
export const scriptLayout = (
  registry: Readonly<Record<string, ScriptedEntry>>,
): { scriptRuns: string[]; byScript: Map<string, ScriptLayout> } => {
  const scriptRuns: string[] = [];
  const byScript = new Map<string, ScriptLayout>();
  let previous: string | undefined;
  for (const [key, entry] of Object.entries(registry)) {
    const layout = byScript.get(entry.script) ?? { keys: [], runs: [] };
    byScript.set(entry.script, layout);
    layout.keys.push(key);
    if (entry.script === previous) {
      layout.runs[layout.runs.length - 1] = layout.runs.at(-1)! + 1;
    } else {
      scriptRuns.push(entry.script);
      layout.runs.push(1);
      previous = entry.script;
    }
  }
  return { scriptRuns, byScript };
};

/**
 * The inverse of `scriptLayout`: rebuild the full ordered key list from the
 * block shape and each script's own ordered keys. Throws if the pieces do not
 * fit together exactly — a block asks for keys a script does not have, or a
 * script has keys no block consumes.
 */
export const interleaveScriptKeys = (
  scriptRuns: readonly string[],
  byScript: ReadonlyMap<string, ScriptLayout>,
): string[] => {
  const keys: string[] = [];
  const cursor = new Map<string, { block: number; key: number }>();
  for (const script of scriptRuns) {
    const layout = byScript.get(script);
    if (layout === undefined) throw new Error(`block names unknown script ${script}`);
    const at = cursor.get(script) ?? { block: 0, key: 0 };
    const length = layout.runs[at.block];
    if (length === undefined) throw new Error(`${script} has fewer blocks than scriptRuns names`);
    const block = layout.keys.slice(at.key, at.key + length);
    if (block.length !== length) throw new Error(`${script} runs out of keys in block ${at.block}`);
    keys.push(...block);
    cursor.set(script, { block: at.block + 1, key: at.key + length });
  }
  for (const [script, layout] of byScript) {
    const at = cursor.get(script);
    if (at === undefined || at.block !== layout.runs.length || at.key !== layout.keys.length) {
      throw new Error(`${script} has keys or blocks that scriptRuns never consumes`);
    }
  }
  return keys;
};

/** Measure the pins the registry would need to pass, from the registry itself. */
export const measureStrokeOwnershipPins = (
  registry: Readonly<Record<string, ScriptedEntry & object>>,
): StrokeOwnershipPins => {
  const { scriptRuns, byScript } = scriptLayout(registry);
  const identityGroups = sharedObjectIdentityGroups(registry);
  const scripts: Record<string, ScriptPin> = {};
  for (const script of [...byScript.keys()].sort()) {
    const { keys, runs } = byScript.get(script)!;
    // Field order here is the field order on disk: count first, because the
    // count is what a reviewer reads in the diff ("kannada 54 -> 55").
    const pin: ScriptPin = {
      count: keys.length,
      runs,
      keyHash: sha256(JSON.stringify(keys)),
    };
    if (!DATA_PINNED_PER_GLYPH.has(script)) {
      // Object.fromEntries defines (never assigns) each key, so even a hostile
      // `__proto__` key serialises as data — the same reason the registry
      // itself is built with defineProperty.
      pin.dataHash = sha256(
        JSON.stringify(Object.fromEntries(keys.map((key) => [key, registry[key]]))),
      );
    }
    scripts[script] = pin;
  }
  return {
    registry: {
      scriptRuns,
      sharedIdentityGroups: identityGroups.length,
      sharedIdentityHash: sha256(JSON.stringify(identityGroups)),
    },
    scripts,
  };
};

/** The exact bytes each pin file should hold, keyed by filename. */
export const strokeOwnershipPinFiles = (pins: StrokeOwnershipPins): Map<string, string> => {
  const files = new Map<string, string>();
  files.set(REGISTRY_PIN_FILE, `${JSON.stringify(pins.registry, null, 2)}\n`);
  for (const [script, pin] of Object.entries(pins.scripts)) {
    // The name becomes a path. Script ids are source literals today, but the
    // writer refuses anything the loader would refuse ("../x" included), so a
    // bad id can never write outside the shard directory.
    if (!SCRIPT_NAME.test(script)) throw new Error(`stroke-ownership: bad script id ${JSON.stringify(script)}`);
    files.set(`${script}.json`, `${JSON.stringify(pin, null, 2)}\n`);
  }
  return files;
};

// ---------------------------------------------------------------------------
// Loading: fail closed on anything that is not exactly a pin
// ---------------------------------------------------------------------------
//
// The loader is the same shape as human-language-data's
// `filmstrip-target-count-pins.ts`: a real directory, real files, lowercase
// names, and an exact field set per file. A symlinked pin could point the gate
// at a file the PR never shows; a stray field could hold a value nothing
// checks. Both are refused before any comparison happens.

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const hasExactFields = (
  value: Record<string, unknown>,
  required: readonly string[],
  optional: readonly string[] = [],
): boolean =>
  required.every((field) => Object.hasOwn(value, field)) &&
  Object.keys(value).every((field) => required.includes(field) || optional.includes(field));

const isCount = (value: unknown, minimum: number): value is number =>
  Number.isSafeInteger(value) && (value as number) >= minimum;

const isHash = (value: unknown): value is string =>
  typeof value === "string" && SHA256_HEX.test(value);

const parseRegistryPin = (name: string, parsed: unknown): RegistryPin => {
  if (
    !isRecord(parsed) ||
    !hasExactFields(parsed, ["scriptRuns", "sharedIdentityGroups", "sharedIdentityHash"]) ||
    !Array.isArray(parsed.scriptRuns) ||
    parsed.scriptRuns.length === 0 ||
    !parsed.scriptRuns.every((script) => typeof script === "string" && SCRIPT_NAME.test(script)) ||
    !isCount(parsed.sharedIdentityGroups, 0) ||
    !isHash(parsed.sharedIdentityHash)
  ) {
    throw new Error(
      `${name}: expected exactly scriptRuns (non-empty script names), ` +
        "sharedIdentityGroups (non-negative integer) and sharedIdentityHash (SHA-256 hex)",
    );
  }
  return parsed as unknown as RegistryPin;
};

const parseScriptPin = (name: string, parsed: unknown): ScriptPin => {
  if (
    !isRecord(parsed) ||
    !hasExactFields(parsed, ["count", "runs", "keyHash"], ["dataHash"]) ||
    !isCount(parsed.count, 1) ||
    !Array.isArray(parsed.runs) ||
    parsed.runs.length === 0 ||
    !parsed.runs.every((run) => isCount(run, 1)) ||
    !isHash(parsed.keyHash) ||
    (Object.hasOwn(parsed, "dataHash") && !isHash(parsed.dataHash))
  ) {
    throw new Error(
      `${name}: expected exactly count (positive integer), runs (non-empty positive ` +
        "integers), keyHash (SHA-256 hex) and, unless its data is pinned per glyph, dataHash",
    );
  }
  return parsed as unknown as ScriptPin;
};

/** Read every pin file from a real directory, refusing anything unexpected. */
export const loadStrokeOwnershipPins = (directory: string): StrokeOwnershipPins => {
  const directoryStat = lstatSync(directory);
  if (directoryStat.isSymbolicLink() || !directoryStat.isDirectory()) {
    throw new Error(`stroke ownership pin directory must be a real directory: ${directory}`);
  }
  const entries = readdirSync(directory, { withFileTypes: true }).sort((left, right) =>
    left.name < right.name ? -1 : left.name > right.name ? 1 : 0,
  );
  let registry: RegistryPin | undefined;
  const scripts: Record<string, ScriptPin> = {};
  for (const entry of entries) {
    const isScriptPin =
      entry.name.endsWith(".json") && SCRIPT_NAME.test(entry.name.slice(0, -".json".length));
    if (
      (entry.name !== REGISTRY_PIN_FILE && !isScriptPin) ||
      entry.isSymbolicLink() ||
      !entry.isFile()
    ) {
      throw new Error(`unsafe stroke ownership pin '${entry.name}'`);
    }
    const parsed: unknown = JSON.parse(readFileSync(join(directory, entry.name), "utf8"));
    if (entry.name === REGISTRY_PIN_FILE) {
      registry = parseRegistryPin(entry.name, parsed);
    } else {
      scripts[entry.name.slice(0, -".json".length)] = parseScriptPin(entry.name, parsed);
    }
  }
  if (registry === undefined) throw new Error(`${REGISTRY_PIN_FILE} is missing`);
  if (Object.keys(scripts).length === 0) throw new Error("no per-script stroke ownership pins");
  return { registry, scripts };
};
