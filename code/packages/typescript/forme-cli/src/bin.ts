import { createCancellationTokenSource } from "@coding-adventures/forme-stage";
import { run } from "./cli.js";

export async function main(): Promise<void> {
  const cancellation = createCancellationTokenSource();
  const cancel = () => cancellation.cancel("interrupted by SIGINT");
  process.once("SIGINT", cancel);
  try {
    process.exitCode = await run(process.argv.slice(2), undefined, {
      cancellation: cancellation.token,
    });
  } catch (error) {
    process.stderr.write(`forme: E_INTERNAL: ${message(error)}\n`);
    process.exitCode = 2;
  } finally {
    process.removeListener("SIGINT", cancel);
  }
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
