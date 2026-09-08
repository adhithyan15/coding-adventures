#!/usr/bin/env node

import { readFile } from "node:fs/promises";
import { stdin, stderr, stdout } from "node:process";

import { runNetlistJson } from "./index.js";

export const CLI_USAGE = "usage: spice-netlist-parser run --json <deck|- >\n";

async function readStandardInput(): Promise<string> {
  let text = "";
  for await (const chunk of stdin) {
    text += chunk;
  }
  return text;
}

export async function main(argv: readonly string[]): Promise<number> {
  if (argv.length !== 3 || argv[0] !== "run" || argv[1] !== "--json") {
    stderr.write(CLI_USAGE);
    return 2;
  }

  try {
    const text = argv[2] === "-" ? await readStandardInput() : await readFile(argv[2]!, "utf8");
    stdout.write(runNetlistJson(text));
    return 0;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    stderr.write(`SPICE_CLI_ERROR: ${message}\n`);
    return 1;
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main(process.argv.slice(2)).then((status) => {
    process.exitCode = status;
  });
}
