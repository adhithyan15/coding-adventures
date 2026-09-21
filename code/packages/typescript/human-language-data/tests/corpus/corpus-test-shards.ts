import { lstatSync, readdirSync, realpathSync } from "node:fs";
import { dirname, join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const LANGUAGE_ID = /^[a-z][a-z0-9-]*$/;
const SHARD_NAME = /^[a-z0-9]+(?:-[a-z0-9]+)*-[0-9a-f]{8}\.case\.ts$/;

export function validateCorpusTestShardNames(
  language: string,
  names: readonly string[],
): readonly string[] {
  if (!LANGUAGE_ID.test(language)) {
    throw new Error(`Unsafe corpus test language id: ${language}`);
  }
  if (names.length === 0) {
    throw new Error(`Corpus test suite ${language} has no shard owners`);
  }

  const folded = new Set<string>();
  for (const name of names) {
    const caseFolded = name.toLocaleLowerCase("en-US");
    if (folded.has(caseFolded)) {
      throw new Error(`Case-fold-colliding corpus test shard owner for ${language}: ${name}`);
    }
    folded.add(caseFolded);
    if (!SHARD_NAME.test(name)) {
      throw new Error(`Unsafe corpus test shard owner for ${language}: ${name}`);
    }
  }
  return [...names].sort();
}

export async function loadCorpusTestShards(
  language: string,
  importerUrl: string,
  loaders: Readonly<Record<string, () => Promise<unknown>>>,
): Promise<void> {
  if (!LANGUAGE_ID.test(language)) {
    throw new Error(`Unsafe corpus test language id: ${language}`);
  }

  const corpusRoot = realpathSync(dirname(fileURLToPath(importerUrl)));
  const shardRootPath = join(corpusRoot, language);
  const shardRoot = realpathSync(shardRootPath);
  if (lstatSync(shardRootPath).isSymbolicLink()) {
    throw new Error(`Corpus test shard directory may not be a symlink: ${language}`);
  }
  const containment = relative(corpusRoot, shardRoot);
  if (containment === ".." || containment.startsWith(`..${sep}`)) {
    throw new Error(`Corpus test shard directory escapes the corpus root: ${language}`);
  }

  const entries = readdirSync(shardRootPath, { withFileTypes: true })
    .filter((entry) => entry.name.endsWith(".case.ts"));
  for (const entry of entries) {
    if (entry.isSymbolicLink() || !entry.isFile()) {
      throw new Error(`Corpus test shard owner must be a regular file: ${language}/${entry.name}`);
    }
  }
  const diskNames = validateCorpusTestShardNames(
    language,
    entries.map((entry) => entry.name),
  );

  const prefix = `./${language}/`;
  const loaderNames = Object.keys(loaders).map((key) => {
    if (!key.startsWith(prefix)) {
      throw new Error(`Corpus test loader escaped ${language}: ${key}`);
    }
    return key.slice(prefix.length);
  });
  const validatedLoaderNames = validateCorpusTestShardNames(language, loaderNames);
  if (JSON.stringify(diskNames) !== JSON.stringify(validatedLoaderNames)) {
    throw new Error(`Corpus test shard discovery mismatch for ${language}`);
  }

  for (const name of validatedLoaderNames) {
    await loaders[`${prefix}${name}`]!();
  }
}
