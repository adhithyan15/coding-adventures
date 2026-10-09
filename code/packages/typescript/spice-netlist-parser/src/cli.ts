#!/usr/bin/env node

import { readFile } from "node:fs/promises";
import { stdin, stderr, stdout } from "node:process";
import { pathToFileURL } from "node:url";

import { CLI_ERROR_CODE, inspectNetlistJson, runNetlistJson } from "./index.js";

export const CLI_USAGE = "usage: spice-netlist-parser <inspect|run> --json <deck|- >\n";

async function readStandardInput(): Promise<string> {
  let text = "";
  for await (const chunk of stdin) {
    text += chunk;
  }
  return text;
}

export async function main(argv: readonly string[]): Promise<number> {
  if (argv.length !== 3 || !["inspect", "run"].includes(argv[0]!) || argv[1] !== "--json") {
    stderr.write(CLI_USAGE);
    return 2;
  }

  try {
    const text = argv[2] === "-" ? await readStandardInput() : await readFile(argv[2]!, "utf8");
    stdout.write(argv[0] === "inspect" ? inspectNetlistJson(text) : runNetlistJson(text));
    return 0;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    stderr.write(`${CLI_ERROR_CODE}: ${message}\n`);
    return 1;
  }
}

// Run only when executed directly, not when imported by the tests.
// `pathToFileURL` matters on Windows: `file://${argv[1]}` would compare
// `file:///D:/.../cli.js` against `file://D:\...\cli.js` and never match, so
// the bundled CLI exited 0 with no output.
if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2)).then((status) => {
    process.exitCode = status;
  });
}
