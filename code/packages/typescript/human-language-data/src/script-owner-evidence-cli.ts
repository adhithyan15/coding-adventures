import { defaultCurriculumRoot } from "./loader.js";
import {
  SCRIPT_OWNER_EVIDENCE_CONFIGS,
  checkScriptOwnerEvidence,
  writeScriptOwnerEvidence,
} from "./script-owner-evidence.js";

export function runScriptOwnerEvidenceCli(args: readonly string[], root = defaultCurriculumRoot()): number {
  if (args.length !== 1 || (args[0] !== "--check" && args[0] !== "--write")) {
    throw new Error("usage: script-owner-evidence-cli --check|--write");
  }
  for (const options of SCRIPT_OWNER_EVIDENCE_CONFIGS) {
    if (args[0] === "--write") writeScriptOwnerEvidence(root, options);
    checkScriptOwnerEvidence(root, options);
  }
  return 0;
}

if (process.argv[1]?.endsWith("script-owner-evidence-cli.js")) {
  process.exitCode = runScriptOwnerEvidenceCli(process.argv.slice(2));
}
