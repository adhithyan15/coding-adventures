import { lstatSync, readFileSync, readdirSync } from "node:fs";
import { basename, join } from "node:path";

export interface DocShardDigestOwner {
  readonly version: 1;
  readonly path: string;
  readonly sha256: string;
}

const OWNER_NAME = /^[a-z0-9-]+--(?:roadmap|session-map)\.json$/;
const OWNED_PATH = /^code\/learning\/human-languages\/([a-z0-9-]+)\/(roadmap|session-map)\.md$/;
const SHA256 = /^[0-9a-f]{64}$/;

export function docShardDigestOwnerName(path: string): string {
  const match = OWNED_PATH.exec(path);
  if (!match) {
    throw new Error(`unsafe reconstructed-document digest path '${path}'`);
  }
  return `${match[1]}--${match[2]}.json`;
}

export function assertSafeDocShardDigestOwnerNames(names: readonly string[]): void {
  const foldedNames = new Set<string>();
  for (const name of names) {
    const folded = name.toLocaleLowerCase("en-US");
    if (foldedNames.has(folded)) {
      throw new Error(`case-colliding reconstructed-document digest owner '${name}'`);
    }
    foldedNames.add(folded);
    if (!OWNER_NAME.test(name)) {
      throw new Error(`unsafe reconstructed-document digest owner '${name}'`);
    }
  }
}

export function readDocShardDigestOwners(directory: string): ReadonlyMap<string, DocShardDigestOwner> {
  const owners = new Map<string, DocShardDigestOwner>();
  const names = readdirSync(directory).sort();
  assertSafeDocShardDigestOwnerNames(names);
  for (const name of names) {

    const path = join(directory, name);
    const stat = lstatSync(path);
    if (stat.isSymbolicLink() || !stat.isFile()) {
      throw new Error(`reconstructed-document digest owner '${name}' must be a regular file`);
    }

    const parsed = JSON.parse(readFileSync(path, "utf8")) as Partial<DocShardDigestOwner>;
    const keys = Object.keys(parsed).sort();
    if (JSON.stringify(keys) !== JSON.stringify(["path", "sha256", "version"])) {
      throw new Error(`reconstructed-document digest owner '${name}' has unexpected fields`);
    }
    if (parsed.version !== 1 || typeof parsed.path !== "string" || typeof parsed.sha256 !== "string") {
      throw new Error(`reconstructed-document digest owner '${name}' has an invalid schema`);
    }
    if (!SHA256.test(parsed.sha256)) {
      throw new Error(`reconstructed-document digest owner '${name}' has an invalid SHA-256`);
    }
    if (owners.has(parsed.path)) {
      throw new Error(`duplicate reconstructed-document digest owner for '${parsed.path}'`);
    }
    if (basename(name) !== docShardDigestOwnerName(parsed.path)) {
      throw new Error(`reconstructed-document digest owner '${name}' does not own '${parsed.path}'`);
    }
    owners.set(parsed.path, parsed as DocShardDigestOwner);
  }
  return owners;
}
