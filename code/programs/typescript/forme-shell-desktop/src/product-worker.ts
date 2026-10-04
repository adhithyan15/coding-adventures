import type { RunResult } from "@coding-adventures/forme-orchestrator";
import type { DeployArtifact } from "@coding-adventures/forme-types";
import { buildAuthoringProduct } from "./product.js";
import {
  decodeWorkerRequest,
  encodeWorkerFailure,
  encodeWorkerSuccess,
} from "./worker-protocol.js";

export interface ProductWorkerDependencies {
  readonly build: typeof buildAuthoringProduct;
}

const defaultDependencies: ProductWorkerDependencies = Object.freeze({ build: buildAuthoringProduct });

/** Execute the worker's only operation against one already-bounded request. */
export async function runProductWorker(
  requestBytes: Uint8Array,
  dependencies: ProductWorkerDependencies = defaultDependencies,
): Promise<Uint8Array> {
  const request = decodeWorkerRequest(requestBytes);
  let result: RunResult;
  try {
    result = await dependencies.build({
      project: request.project,
      revision: request.revision,
      output: request.output,
    });
  } catch {
    return encodeWorkerFailure();
  }
  if (result.outcome !== "success") return encodeWorkerFailure();
  const artifact = result.outputs.site as DeployArtifact | undefined;
  if (artifact === undefined) return encodeWorkerFailure();
  try {
    return encodeWorkerSuccess(request.revision, artifact);
  } catch {
    return encodeWorkerFailure();
  }
}
