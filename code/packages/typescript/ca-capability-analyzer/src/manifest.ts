/**
 * Manifest parsing and capability normalization for required_capabilities.json.
 */

import * as fs from "node:fs";
import * as path from "node:path";
import type { CapabilityManifest, BannedConstructException } from "./types.js";
import { CAPABILITY_RULES } from "./rules.js";

export const MAX_MANIFEST_BYTES = 1024 * 1024;
export const MAX_MANIFEST_ITEMS = 4096;
export const MAX_CAP_TOKEN_LEN = 128;

export interface ParsedManifest {
  readonly manifestPath?: string;
  readonly packageName?: string;
  readonly declaredCapabilities: Set<string>;
  readonly exceptions: Map<string, BannedConstructException>;
  readonly raw?: CapabilityManifest;
}

export function loadManifest(packageDir: string): ParsedManifest {
  if (typeof packageDir !== "string" || packageDir.length === 0 || packageDir.includes("\0")) {
    throw new Error("Invalid package directory path");
  }
  const manifestPath = path.join(packageDir, "required_capabilities.json");
  let stat: fs.Stats;
  try {
    stat = fs.lstatSync(manifestPath);
  } catch (err) {
    if ((err as NodeJS.ErrnoException)?.code === "ENOENT") {
      return {
        declaredCapabilities: new Set<string>(),
        exceptions: new Map(),
      };
    }
    throw new Error(`Failed to stat manifest at ${manifestPath}: ${String(err)}`);
  }

  if (stat.isSymbolicLink() || !stat.isFile()) {
    throw new Error(`Manifest at ${manifestPath} must be a regular non-symlink file`);
  }
  if (stat.size > MAX_MANIFEST_BYTES) {
    throw new Error(
      `Manifest at ${manifestPath} exceeds maximum size of ${MAX_MANIFEST_BYTES} bytes`
    );
  }

  let content: string;
  try {
    content = fs.readFileSync(manifestPath, "utf-8");
  } catch (err) {
    throw new Error(`Failed to read manifest at ${manifestPath}: ${String(err)}`);
  }

  let json: unknown;
  try {
    json = JSON.parse(content);
  } catch (err) {
    throw new Error(`Malformed JSON in required_capabilities.json at ${manifestPath}: ${String(err)}`);
  }

  if (typeof json !== "object" || json === null) {
    throw new Error(`Malformed manifest root in required_capabilities.json at ${manifestPath}`);
  }

  const declared = new Set<string>();
  const exceptions = new Map<string, BannedConstructException>();

  if (Array.isArray(json)) {
    if (json.length > MAX_MANIFEST_ITEMS) {
      throw new Error(`Manifest capability list exceeds ${MAX_MANIFEST_ITEMS} entries`);
    }
    // Legacy string array format: ["fs:read", "net:socket"]
    for (const item of json) {
      if (typeof item === "string") {
        normalizeAndAddCapability(item, declared);
      }
    }
    return {
      manifestPath,
      declaredCapabilities: declared,
      exceptions,
    };
  }

  const record = json as Record<string, unknown>;
  if (Array.isArray(record.capabilities)) {
    if (record.capabilities.length > MAX_MANIFEST_ITEMS) {
      throw new Error(`Manifest capabilities list exceeds ${MAX_MANIFEST_ITEMS} entries`);
    }
    for (const cap of record.capabilities) {
      if (typeof cap === "string") {
        normalizeAndAddCapability(cap, declared);
      } else if (typeof cap === "object" && cap !== null) {
        const capObj = cap as Record<string, unknown>;
        if (typeof capObj.category === "string" && typeof capObj.action === "string") {
          if (
            capObj.category.length > MAX_CAP_TOKEN_LEN ||
            capObj.action.length > MAX_CAP_TOKEN_LEN
          ) {
            throw new Error("Capability category/action exceeds maximum token length");
          }
          const cat = capObj.category.trim();
          const act = capObj.action.trim();
          if (cat.length > 0 && act.length > 0) {
            const pair = `${cat}:${act}`;
            declared.add(pair);
            // Also map to shorthand if applicable
            for (const [shorthand, rule] of Object.entries(CAPABILITY_RULES)) {
              if (rule.category === cat && rule.action === act) {
                declared.add(shorthand);
              }
            }
          }
        }
      }
    }
  }

  if (Array.isArray(record.banned_construct_exceptions)) {
    if (record.banned_construct_exceptions.length > MAX_MANIFEST_ITEMS) {
      throw new Error(`Manifest exceptions list exceeds ${MAX_MANIFEST_ITEMS} entries`);
    }
    for (const exc of record.banned_construct_exceptions) {
      if (exc && typeof exc === "object") {
        const excObj = exc as Record<string, unknown>;
        if (
          typeof excObj.construct === "string" &&
          excObj.construct.length <= MAX_CAP_TOKEN_LEN
        ) {
          exceptions.set(
            excObj.construct.toLowerCase(),
            exc as BannedConstructException
          );
        }
      }
    }
  }

  const packageName =
    typeof record.package === "string" && record.package.length <= 256
      ? record.package
      : undefined;

  return {
    manifestPath,
    packageName,
    declaredCapabilities: declared,
    exceptions,
    raw: json as CapabilityManifest,
  };
}

function normalizeAndAddCapability(capStr: string, set: Set<string>): void {
  if (capStr.length > MAX_CAP_TOKEN_LEN) {
    throw new Error("Capability entry exceeds maximum token length");
  }
  const trimmed = capStr.trim();
  if (trimmed.length === 0) {
    return;
  }
  set.add(trimmed);

  // If it's a shorthand like "fs:read", add category:action
  if (Object.hasOwn(CAPABILITY_RULES, trimmed)) {
    const rule = CAPABILITY_RULES[trimmed as keyof typeof CAPABILITY_RULES];
    set.add(`${rule.category}:${rule.action}`);
  }

  // If it's "category:action", add any matching shorthand
  for (const [shorthand, rule] of Object.entries(CAPABILITY_RULES)) {
    if (`${rule.category}:${rule.action}` === trimmed) {
      set.add(shorthand);
    }
  }
}
