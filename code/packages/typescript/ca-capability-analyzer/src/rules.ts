/**
 * Rules and patterns for ambient capability and VER15 CWE detection.
 */

import type { AmbientCapability, CWEId } from "./types.js";

export interface CapabilityRule {
  readonly capability: AmbientCapability;
  readonly category: string;
  readonly action: string;
  readonly description: string;
}

export const CAPABILITY_RULES: Record<AmbientCapability, CapabilityRule> = {
  "fs:read": {
    capability: "fs:read",
    category: "fs",
    action: "read",
    description: "Filesystem read operations (readFile, readdir, stat, open, access, etc.)",
  },
  "fs:write": {
    capability: "fs:write",
    category: "fs",
    action: "write",
    description: "Filesystem write operations (writeFile, appendFile, mkdir, unlink, chmod, etc.)",
  },
  "net:socket": {
    capability: "net:socket",
    category: "net",
    action: "connect",
    description: "Network socket or HTTP operations (fetch, WebSocket, net.connect, http.request, etc.)",
  },
  "process:spawn": {
    capability: "process:spawn",
    category: "proc",
    action: "exec",
    description: "Child process execution (spawn, exec, execFile, fork, etc.)",
  },
  "env:read": {
    capability: "env:read",
    category: "env",
    action: "read",
    description: "Environment variable read operations (process.env, Deno.env, Bun.env)",
  },
  "ffi:unsafe": {
    capability: "ffi:unsafe",
    category: "ffi",
    action: "call",
    description: "Foreign function interface or native module loading (process.dlopen, ffi-napi, etc.)",
  },
  "time:clock": {
    capability: "time:clock",
    category: "time",
    action: "read",
    description: "Wall-clock or system time queries (Date.now, new Date(), performance.now, process.hrtime)",
  },
  "random:entropy": {
    capability: "random:entropy",
    category: "rand",
    action: "read",
    description: "Random number or entropy generation (Math.random, crypto.getRandomValues, randomBytes)",
  },
};

export const FS_READ_METHODS = new Set([
  "readFile",
  "readFileSync",
  "createReadStream",
  "readdir",
  "readdirSync",
  "stat",
  "statSync",
  "lstat",
  "lstatSync",
  "access",
  "accessSync",
  "open",
  "openSync",
  "read",
  "readSync",
  "watch",
  "watchFile",
]);

export const FS_WRITE_METHODS = new Set([
  "writeFile",
  "writeFileSync",
  "appendFile",
  "appendFileSync",
  "createWriteStream",
  "mkdir",
  "mkdirSync",
  "rm",
  "rmSync",
  "rmdir",
  "rmdirSync",
  "unlink",
  "unlinkSync",
  "chmod",
  "chmodSync",
  "chown",
  "chownSync",
  "copyFile",
  "copyFileSync",
  "rename",
  "renameSync",
  "truncate",
  "truncateSync",
]);

export const NET_MODULES = new Set([
  "net",
  "node:net",
  "http",
  "node:http",
  "https",
  "node:https",
  "dgram",
  "node:dgram",
  "tls",
  "node:tls",
  "http2",
  "node:http2",
  "undici",
  "ws",
  "axios",
]);

export const PROCESS_METHODS = new Set([
  "spawn",
  "spawnSync",
  "exec",
  "execSync",
  "execFile",
  "execFileSync",
  "fork",
]);

export const BANNED_PROTO_PROPERTIES = new Set(["__proto__", "prototype", "constructor"]);

/**
 * Checks if a regex pattern exhibits catastrophic backtracking (ReDoS / CWE-1333).
 */
export function isCatastrophicRegex(pattern: string): boolean {
  // Nested quantifiers like (a+)+, (a*)*, ([a-z]+)*, (.*)+
  if (/\([^()]*[+*][^()]*\)[+*]/.test(pattern)) {
    return true;
  }
  // Repeated alternation like (a|a)+, (a|b+)+
  if (/\([^()|]+(?:\|[^()|]+)+\)[+*]/.test(pattern)) {
    if (/\(\w+\|\w+\)[+*]/.test(pattern) || /\(\w+\|[^\)]*[+*][^\)]*\)[+*]/.test(pattern)) {
      return true;
    }
  }
  // Multiple overlapping wildcards with quantifiers: .*.* or \d+\d+
  if (/\.\*.*\.\*/.test(pattern) || /\\d\+.*\\d\+/.test(pattern)) {
    return true;
  }
  return false;
}
