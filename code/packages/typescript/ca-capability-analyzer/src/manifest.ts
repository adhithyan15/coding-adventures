/**
 * Manifest parsing and capability normalization for required_capabilities.json.
 */

import * as fs from "node:fs";
import * as path from "node:path";
import type { CapabilityManifest, BannedConstructException } from "./types.js";
import { CAPABILITY_RULES } from "./rules.js";

export interface ParsedManifest {
  readonly manifestPath?: string;
  readonly packageName?: string;
  readonly declaredCapabilities: Set<string>;
  readonly exceptions: Map<string, BannedConstructException>;
  readonly raw?: CapabilityManifest;
}

export function loadManifest(packageDir: string): ParsedManifest {
  const manifestPath = path.join(packageDir, "required_capabilities.json");
  if (!fs.existsSync(manifestPath)) {
    return {
      declaredCapabilities: new Set<string>(),
      exceptions: new Map(),
    };
  }

  let content: string;
  try {
    content = fs.readFileSync(manifestPath, "utf-8");
  } catch (err) {
    throw new Error(`Failed to read manifest at ${manifestPath}: ${String(err)}`);
  }

  let json: any;
  try {
    json = JSON.parse(content);
  } catch (err) {
    throw new Error(`Malformed JSON in required_capabilities.json at ${manifestPath}: ${String(err)}`);
  }

  const declared = new Set<string>();
  const exceptions = new Map<string, BannedConstructException>();

  if (Array.isArray(json)) {
    // Legacy string array format: ["fs:read", "net:socket"]
    for (const item of json) {
      if (typeof item === "string") {
        normalizeAndAddCapability(item, declared);
      }
    }
  } else if (typeof json === "object" && json !== null) {
    if (Array.isArray(json.capabilities)) {
      for (const cap of json.capabilities) {
        if (typeof cap === "string") {
          normalizeAndAddCapability(cap, declared);
        } else if (typeof cap === "object" && cap !== null) {
          if (cap.category && cap.action) {
            const pair = `${cap.category}:${cap.action}`;
            declared.add(pair);
            // Also map to shorthand if applicable
            for (const [shorthand, rule] of Object.entries(CAPABILITY_RULES)) {
              if (rule.category === cap.category && rule.action === cap.action) {
                declared.add(shorthand);
              }
            }
          }
        }
      }
    }

    if (Array.isArray(json.banned_construct_exceptions)) {
      for (const exc of json.banned_construct_exceptions) {
        if (exc && typeof exc.construct === "string") {
          exceptions.set(exc.construct.toLowerCase(), exc);
        }
      }
    }
  }

  return {
    manifestPath,
    packageName: json?.package,
    declaredCapabilities: declared,
    exceptions,
    raw: json,
  };
}

function normalizeAndAddCapability(capStr: string, set: Set<string>): void {
  const trimmed = capStr.trim();
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
