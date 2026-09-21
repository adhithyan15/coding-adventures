import { lstatSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { stripControlCharacters } from "./constants.js";
import { defaultCurriculumRoot } from "./loader.js";
import {
  ROOT_SLUG_BASELINE_PATH,
  diffRootSlugSplits,
  findRootSlugSplits,
  loadRootSlugBaseline,
  serialiseRootSlugBaseline,
} from "./root-slug-splits.js";
import { isSharded } from "./shard.js";

/**
 * The WRITE side's own guard.
 *
 * `readLedgerFile` refuses a symlinked ledger and refuses one superseded by a
 * sibling `X.d/`, so without this the two halves of this CLI disagree about
 * the same path: `--check` would refuse to read a symlinked baseline while
 * `--write` happily followed it and overwrote whatever it pointed at. This
 * repo states the rule in `shard.ts` -- "a guard living only inside the reader
 * is a guard the writer forgets" -- and `script-owner-evidence.ts` applies it
 * at its own write. This is the same guard for this writer.
 */
function assertWritableLedger(path: string): void {
  let prior;
  try {
    prior = lstatSync(path);
  } catch {
    return; // Not there yet: writing creates it, which is fine.
  }
  if (prior.isSymbolicLink() || !prior.isFile()) {
    throw new Error(`'${path}' must be a real regular file, not a symlink`);
  }
  if (isSharded(path)) {
    throw new Error(`'${path}' is superseded by its shard directory; refusing to write the monolith`);
  }
}

/**
 * Corpus text, made safe to print.
 *
 * A slug comes from lesson frontmatter, and this message is printed onto a CI
 * runner. `\u001b[2K\u001b[1G` erases the rendered line and homes the cursor;
 * a bare `\r` inside a slug survives `trim()` and can begin what a log viewer
 * reads as a fresh line, which is enough to forge a `##[error]` of somebody
 * else's. `constants.ts` already carries the stripper for exactly this.
 */
function printable(value: string): string {
  return stripControlCharacters(value);
}

export function runRootSlugSplitsCli(args: readonly string[], root = defaultCurriculumRoot()): number {
  const allowNew = args.includes("--allow-new");
  const rest = args.filter((arg) => arg !== "--allow-new");
  if (rest.length !== 1 || (rest[0] !== "--check" && rest[0] !== "--write")) {
    throw new Error("usage: root-slug-splits-cli --check|--write [--allow-new]");
  }
  const live = findRootSlugSplits(root);

  if (rest[0] === "--write") {
    const path = resolve(root, ROOT_SLUG_BASELINE_PATH);
    assertWritableLedger(path);
    // The baseline MAY ONLY SHRINK, and `--write` is the command a contributor
    // is told to run -- so without this it is also the command that quietly
    // launders a new split into the accepted set. Growing it takes an explicit
    // flag, which a reviewer can see in the diff of a package script or a
    // commit body.
    const { added } = diffRootSlugSplits(live, loadRootSlugBaseline(root));
    if (added.length > 0 && !allowNew) {
      throw new Error(
        `refusing to grow the baseline with ${added.length} new split(s):\n  ` +
          `${added.map((split) => printable(split.slugs.join(" / "))).join("\n  ")}\n` +
          `Fix the slug so it matches the spelling the corpus already uses. If the ` +
          `split is genuinely new and accepted, re-run with --allow-new and say why.`,
      );
    }
    writeFileSync(path, serialiseRootSlugBaseline(live));
    process.stdout.write(`wrote ${ROOT_SLUG_BASELINE_PATH} (${live.length} entries)\n`);
    return 0;
  }

  const baseline = loadRootSlugBaseline(root);
  const { added, resolved } = diffRootSlugSplits(live, baseline);
  if (added.length === 0 && resolved.length === 0) {
    process.stdout.write(`root slug splits: ${live.length} known entries, none new\n`);
    return 0;
  }
  const baselineSize = new Map(baseline.splits.map((split) => [split.key, split.slugs.length]));
  const lines: string[] = [];
  for (const split of added) {
    const before = baselineSize.get(split.key);
    // A split that LOST a spelling also lands here, because the baseline no
    // longer matches -- and telling that author "stop adding spellings" points
    // them at the opposite of what they did. Distinguish the two.
    const shrank = before !== undefined && split.slugs.length < before;
    lines.push(
      shrank
        ? `'${printable(split.key)}' now has ${split.slugs.length} spelling(s), was ${before}; ` +
          `run generate:root-slug-splits to record it`
        : `NEW split on '${printable(split.key)}': ${printable(split.slugs.join(" / "))}`,
    );
  }
  for (const key of resolved) {
    lines.push(
      `baseline entry '${printable(key)}' no longer splits; run generate:root-slug-splits to prune`,
    );
  }
  throw new Error(
    `root slug splits (HL-C419):\n  ${lines.join("\n  ")}\n` +
      `A slug is the cousins join key and the join is exact string equality, so two\n` +
      `spellings of one etymon silently split a cousin panel. Pick the spelling the\n` +
      `corpus already uses for that etymon rather than adding a second.`,
  );
}

if (process.argv[1]?.endsWith("root-slug-splits-cli.js")) {
  try {
    process.exitCode = runRootSlugSplitsCli(process.argv.slice(2));
  } catch (error) {
    process.stderr.write(`${(error as Error).message}\n`);
    process.exitCode = 1;
  }
}
