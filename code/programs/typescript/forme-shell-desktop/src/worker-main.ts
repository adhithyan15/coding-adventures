import { runProductWorker } from "./product-worker.js";
import { MAX_WORKER_MESSAGE_BYTES } from "./worker-protocol.js";

async function main(): Promise<void> {
  const chunks: Uint8Array[] = [];
  let length = 0;
  for await (const chunk of process.stdin) {
    const bytes = typeof chunk === "string" ? new TextEncoder().encode(chunk) : new Uint8Array(chunk);
    length += bytes.length;
    if (length > MAX_WORKER_MESSAGE_BYTES) throw new Error("invalid worker input");
    chunks.push(bytes);
  }
  const request = new Uint8Array(length);
  let offset = 0;
  for (const chunk of chunks) {
    request.set(chunk, offset);
    offset += chunk.length;
  }
  const response = await runProductWorker(request);
  await new Promise<void>((resolve, reject) => {
    process.stdout.write(response, (error) => error === null ? resolve() : reject(error));
  });
}

void main().catch(() => {
  process.exitCode = 2;
});
