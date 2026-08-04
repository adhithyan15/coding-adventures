import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { loadEverything } from "./loader.js";
import { buildCurriculumGapReport, renderCurriculumGapReport } from "./report.js";

interface ReportOptions {
  root?: string;
  format: "json" | "text";
}

function parseOptions(args: string[]): ReportOptions {
  const options: ReportOptions = { format: "text" };
  for (let index = 0; index < args.length; index += 1) {
    const flag = args[index];
    if (flag !== "--root" && flag !== "--format") {
      throw new Error(`unknown argument '${flag}'`);
    }
    const value = args[index + 1];
    if (!value) throw new Error(`${flag} requires a value`);
    if (flag === "--root") options.root = resolve(value);
    else if (value === "json" || value === "text") options.format = value;
    else throw new Error(`--format must be 'json' or 'text'`);
    index += 1;
  }
  return options;
}

export function runCurriculumGapReport(args = process.argv.slice(2)): number {
  let options: ReportOptions;
  try {
    options = parseOptions(args);
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    return 2;
  }
  const { registry, lessons, books } = loadEverything(options.root);
  const report = buildCurriculumGapReport({ registry, lessons, books });
  const json = `${JSON.stringify(report, null, 2)}\n`;
  const text = renderCurriculumGapReport(report);
  process.stdout.write(options.format === "json" ? json : text);
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runCurriculumGapReport());
}
