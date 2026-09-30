import type { Readable, Writable } from "node:stream";
import type { Capability } from "@coding-adventures/forme-capability";
import type { Manifest, StageContribution } from "@coding-adventures/forme-manifest";
import type { Stage, StageContext } from "@coding-adventures/forme-stage";
import type { KindDescriptor } from "@coding-adventures/forme-types";
import type { StageRef } from "@coding-adventures/forme-pipeline-config";
import type { Logger } from "@coding-adventures/forme-stage";

export interface DiscoveredPlugin {
  readonly rootDirectory: string;
  readonly manifestPath: string;
  readonly entryPath: string;
  readonly entryBytes: Uint8Array;
  /** Exact private discovery snapshots keyed by stage id. */
  readonly configSchemas: Readonly<Record<string, VerifiedConfigSchemaSnapshot>>;
  readonly manifest: Manifest;
  readonly manifestHash: string;
}

export interface VerifiedConfigSchemaSnapshot {
  readonly relativePath: string;
  readonly bytes: Uint8Array;
  readonly hash: string;
}

/** Immutable code snapshot supplied to a trusted sandbox launcher. */
export type VerifiedPluginSnapshot = Pick<
  DiscoveredPlugin,
  "manifest" | "manifestHash" | "entryBytes"
>;

export interface PluginLaunchRequest {
  readonly plugin: VerifiedPluginSnapshot;
  readonly stage: StageContribution;
  readonly instanceId: string;
  readonly workingDirectory: string;
  readonly resources: Manifest["resources"];
  readonly configSchema: VerifiedConfigSchemaSnapshot | null;
}

export interface PluginProcessExit {
  readonly code: number | null;
  readonly signal: NodeJS.Signals | null;
}

export interface LaunchedPluginProcess {
  /** Only `sandboxed` is accepted. Anything else is killed and rejected. */
  readonly isolation: "sandboxed" | "none";
  readonly isolationProvider: string;
  /** Identity of the manifest plus exact entry bytes staged for execution. */
  readonly launchedManifestHash: string;
  /** Exact schema snapshot staged for this stage, or null when absent. */
  readonly launchedConfigSchemaHash: string | null;
  readonly stdin: Writable;
  readonly stdout: Readable;
  readonly stderr: Readable;
  readonly exited: Promise<PluginProcessExit>;
  signal(signal: NodeJS.Signals): void;
  cleanup?(): void | Promise<void>;
}

export interface PluginProcessFactory {
  launch(request: PluginLaunchRequest): Promise<LaunchedPluginProcess>;
}

export interface PluginStageLoader {
  loadStage(
    ref: StageRef,
    instanceId?: string,
    instanceCapabilities?: readonly Capability[],
  ): Promise<Stage<KindDescriptor, KindDescriptor>>;
  dispose?(): Promise<void>;
}

export interface PluginHost extends PluginStageLoader {
  readonly plugins: ReadonlyMap<string, DiscoveredPlugin>;
  dispose(): Promise<void>;
}

export interface PluginHostOptions {
  /**
   * Ordered, host-managed discovery roots. The caller must keep these roots
   * quiescent for the duration of createPluginHost(); FM-B015 installers own
   * atomic staging and immutable/owned install-root enforcement.
   */
  readonly roots: readonly string[];
  readonly grants?: Readonly<Record<string, readonly Capability[]>>;
  readonly processFactory?: PluginProcessFactory;
  readonly logger?: Logger;
  /** Trusted host implementations used only after plugin capability checks pass. */
  readonly capabilityApis?: Partial<Pick<
    StageContext,
    "storage" | "network" | "env" | "filesystem" | "shell"
  >>;
  readonly storageRoot?: string;
  readonly cacheDirectory?: string | null;
  readonly hostName?: string;
  readonly hostVersion?: string;
  readonly handshakeTimeoutMs?: number;
  readonly requestTimeoutMs?: number;
  readonly cancellationGracePeriodMs?: number;
  readonly disposeGracePeriodMs?: number;
  readonly killGracePeriodMs?: number;
  readonly maxFrameBytes?: number;
  readonly maxHeaderBytes?: number;
  readonly maxBufferedStreamValues?: number;
  /** Maximum estimated decoded bytes retained by one output stream queue. */
  readonly maxBufferedStreamBytes?: number;
  /** Lifetime limits for untrusted log traffic from one plugin process. */
  readonly maxLogEntries?: number;
  readonly maxLogBytes?: number;
}
