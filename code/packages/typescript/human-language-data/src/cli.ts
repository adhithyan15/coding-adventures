// CLI: `human-language-data validate` — the CI-facing entry point.
// Loads the real curriculum, runs the validator, prints a report, and exits
// non-zero if there are any errors.

import { loadEverything } from "./loader.js";
import { validate, hasErrors, summarize } from "./validate.js";
import { coverageByLanguage } from "./queries.js";

export function runValidate(root?: string): number {
  const { taxonomy, lessons, scripts, dataset } = loadEverything(root);
  const issues = validate({ taxonomy, lessons, scripts });

  for (const issue of issues) {
    const tag = issue.level.toUpperCase().padEnd(7);
    process.stdout.write(`${tag} [${issue.code}] ${issue.message}\n`);
  }

  process.stdout.write(`\n${dataset.concepts.length} concepts, ${dataset.languages.length} languages\n`);
  const cov = coverageByLanguage(dataset);
  for (const lang of dataset.languages) {
    process.stdout.write(`  ${lang.padEnd(12)} ${cov[lang].core} core / ${cov[lang].total} total\n`);
  }
  process.stdout.write(`\n${summarize(issues)}\n`);
  return hasErrors(issues) ? 1 : 0;
}

// Run when invoked directly (node/tsx src/cli.ts validate).
if (import.meta.url === `file://${process.argv[1]}`) {
  const cmd = process.argv[2] ?? "validate";
  if (cmd !== "validate") {
    process.stderr.write(`unknown command '${cmd}'\n`);
    process.exit(2);
  }
  process.exit(runValidate());
}
