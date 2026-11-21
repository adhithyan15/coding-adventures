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

export const MAX_REGEX_PATTERN_LEN = 16_384;

function isWordCharCode(code: number): boolean {
  return (
    (code >= 48 && code <= 57) || // 0-9
    (code >= 65 && code <= 90) || // A-Z
    code === 95 ||                // _
    (code >= 97 && code <= 122)   // a-z
  );
}

/**
 * Checks if a regex pattern exhibits catastrophic backtracking (ReDoS / CWE-1333)
 * using a linear-time O(N) single-pass scanner (zero backtracking regexes).
 */
export function isCatastrophicRegex(pattern: string): boolean {
  if (typeof pattern !== "string") {
    throw new TypeError("pattern must be a string");
  }
  if (pattern.length > MAX_REGEX_PATTERN_LEN) {
    return true;
  }

  let dotStarCount = 0;
  let digitPlusCount = 0;
  let groupStart = -1;
  let hasInnerQuantifier = false;
  let hasPipe = false;
  let allWordBranches = true;
  let currentBranchLen = 0;

  for (let i = 0; i < pattern.length; i++) {
    const ch = pattern.charCodeAt(i);
    const next = i + 1 < pattern.length ? pattern.charCodeAt(i + 1) : -1;

    // Count .* occurrences (46 = '.', 42 = '*')
    if (ch === 46 && next === 42) {
      dotStarCount++;
      if (dotStarCount >= 2) return true;
    }

    // Count \d+ occurrences (92 = '\', 100 = 'd', 43 = '+')
    if (
      ch === 92 &&
      i + 2 < pattern.length &&
      pattern.charCodeAt(i + 1) === 100 &&
      pattern.charCodeAt(i + 2) === 43
    ) {
      digitPlusCount++;
      if (digitPlusCount >= 2) return true;
    }

    if (ch === 40) {
      // '(' starts innermost candidate group
      groupStart = i;
      hasInnerQuantifier = false;
      hasPipe = false;
      allWordBranches = true;
      currentBranchLen = 0;
    } else if (ch === 41 && groupStart !== -1) {
      // ')' closes group; check if followed by '+' (43) or '*' (42)
      const groupNonEmpty = i > groupStart + 1;
      const followedByQuantifier = next === 43 || next === 42;
      if (groupNonEmpty && followedByQuantifier) {
        if (hasInnerQuantifier) {
          return true;
        }
        if (hasPipe && allWordBranches && currentBranchLen > 0) {
          return true;
        }
      }
      groupStart = -1;
    } else if (groupStart !== -1) {
      if (ch === 43 || ch === 42) {
        hasInnerQuantifier = true;
      }
      if (ch === 124) {
        // '|'
        if (currentBranchLen === 0) {
          allWordBranches = false;
        }
        hasPipe = true;
        currentBranchLen = 0;
      } else if (isWordCharCode(ch)) {
        currentBranchLen++;
      } else {
        allWordBranches = false;
      }
    }
  }

  return false;
}
