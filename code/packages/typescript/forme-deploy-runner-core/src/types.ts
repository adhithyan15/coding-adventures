export type DeploySource =
  | "page-bundle"
  | "sitemap"
  | "robots"
  | "web-app-manifest"
  | "extra";

export interface DeployFileEntry {
  readonly outputPath: string;
  readonly contentType: string;
  readonly sizeBytes: number;
  readonly sha256: string;
  readonly source: DeploySource;
  readonly route?: string;
  readonly lastmod?: string;
}

export interface DeployManifest {
  readonly version: 1;
  readonly baseUrl?: string;
  readonly fileCount: number;
  readonly totalSizeBytes: number;
  readonly files: Readonly<Record<string, DeployFileEntry>>;
}

export interface ContentStore {
  readonly get: (sha256: string, signal?: AbortSignal) => Promise<Uint8Array>;
  readonly has: (sha256: string, signal?: AbortSignal) => Promise<boolean>;
  readonly hashes: () => AsyncIterable<string>;
}

export interface ContentPreflightOptions {
  readonly signal?: AbortSignal;
}

export interface ContentPreflightResult {
  readonly fileCount: number;
  readonly uniqueContentCount: number;
  readonly totalSizeBytes: number;
}

export interface VerifiedContentReader {
  readonly preflight: () => Promise<ContentPreflightResult>;
  readonly read: (outputPath: string) => Promise<Uint8Array>;
}

export type DeployAction = "create" | "update" | "skip" | "delete";

export interface DeployPlanEntry {
  readonly outputPath: string;
  readonly action: DeployAction;
  readonly current?: DeployFileEntry;
  readonly previous?: DeployFileEntry;
}

export interface DeployPlan {
  readonly entries: readonly DeployPlanEntry[];
  readonly summary: {
    readonly created: number;
    readonly updated: number;
    readonly skipped: number;
    readonly deleted: number;
  };
}

export interface DeployReportFile {
  readonly action: DeployAction;
  readonly bytesWritten: number;
  readonly elapsedMs: number;
  readonly error?: {
    readonly code: string;
    readonly message: string;
  };
}

export interface DeployReport {
  readonly version: 1;
  readonly manifestSha256: string;
  readonly target: string;
  readonly startedAt: string;
  readonly finishedAt: string;
  readonly status: "success" | "partial" | "failed" | "rolled-back";
  readonly files: Readonly<Record<string, DeployReportFile>>;
  readonly summary: {
    readonly created: number;
    readonly updated: number;
    readonly skipped: number;
    readonly deleted: number;
    readonly failed: number;
    readonly totalBytesWritten: number;
    readonly totalElapsedMs: number;
  };
}
