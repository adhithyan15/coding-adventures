/**
 * Type definitions for ca-capability-analyzer.
 *
 * Conforms to Spec 13 (Capability Security) and VER15 (Weakness Registry).
 */

export type AmbientCapability =
  | "fs:read"
  | "fs:write"
  | "net:socket"
  | "process:spawn"
  | "env:read"
  | "ffi:unsafe"
  | "time:clock"
  | "random:entropy";

export type CapabilityCategory =
  | "fs"
  | "net"
  | "proc"
  | "env"
  | "ffi"
  | "time"
  | "stdin"
  | "stdout"
  | "rand"
  | "thread"
  | "ui";

export type CapabilityAction =
  | "read"
  | "write"
  | "create"
  | "delete"
  | "list"
  | "connect"
  | "listen"
  | "dns"
  | "exec"
  | "fork"
  | "signal"
  | "call"
  | "load"
  | "sleep"
  | "spawn"
  | "message"
  | "event";

export type CWEId =
  | "CWE-1321" // Prototype Pollution
  | "CWE-1333" // Inefficient Regular Expression / ReDoS
  | "CWE-674"  // Uncontrolled Recursion
  | "CWE-94"   // Code Injection / eval / new Function
  | "CWE-78"   // OS Command Injection
  | "CWE-400"  // Uncontrolled Resource Consumption
  | "CWE-20";  // Improper Input Validation

export interface DetectedCapability {
  readonly capability: AmbientCapability;
  readonly category: string;
  readonly action: string;
  readonly file: string;
  readonly line: number;
  readonly col: number;
  readonly evidence: string;
}

export interface DetectedWeakness {
  readonly cwe: CWEId;
  readonly name: string;
  readonly file: string;
  readonly line: number;
  readonly col: number;
  readonly evidence: string;
  readonly description: string;
}

export interface BannedConstructException {
  readonly construct: string;
  readonly language: string;
  readonly justification: string;
}

export interface CapabilityItem {
  readonly category: string;
  readonly action: string;
  readonly target?: string;
  readonly justification?: string;
}

export interface CapabilityManifest {
  readonly $schema?: string;
  readonly version: number;
  readonly package?: string;
  readonly capabilities: Array<CapabilityItem | string>;
  readonly justification?: string;
  readonly banned_construct_exceptions?: BannedConstructException[];
}

export interface AnalysisOptions {
  readonly strict?: boolean;
  readonly verbose?: boolean;
  readonly includeTests?: boolean;
}

export interface AnalysisResult {
  readonly packageDir: string;
  readonly packageName?: string;
  readonly inspectedFiles: string[];
  readonly detectedCapabilities: DetectedCapability[];
  readonly declaredCapabilities: string[];
  readonly undeclaredCapabilities: DetectedCapability[];
  readonly detectedWeaknesses: DetectedWeakness[];
  readonly exemptWeaknesses: DetectedWeakness[];
  readonly violations: string[];
  readonly ok: boolean;
}
