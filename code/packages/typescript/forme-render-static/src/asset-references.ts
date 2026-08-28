import type { DocumentNode } from "@coding-adventures/document-ast";
import type { AssetRef, LogicalId } from "@coding-adventures/forme-types";

/** Collision-free placeholder consumed only by asset-aware emitters. */
export function assetPlaceholderUrl(id: LogicalId, urlSuffix = ""): string {
  return `forme-asset:${encodeURIComponent(id)}${urlSuffix}`;
}

/**
 * Return an immutable Document AST copy whose resolved images use Forme
 * placeholders. The authored document is returned unchanged when no refs exist.
 */
export function rewriteAssetReferences(
  document: DocumentNode,
  refs: readonly AssetRef[],
): DocumentNode {
  if (refs.length === 0) return document;
  const refsByPath = new Map<string, AssetRef>();
  for (const ref of refs) {
    if (ref.role !== "image") continue;
    const key = JSON.stringify(ref.nodePath);
    if (refsByPath.has(key)) {
      throw new Error(`forme-render-static: duplicate image AssetRef at node path ${key}`);
    }
    refsByPath.set(key, ref);
  }
  if (refsByPath.size === 0) return document;
  const matchedPaths = new Set<string>();
  const rewritten = rewriteNode(document, [], refsByPath, matchedPaths) as DocumentNode;
  if (matchedPaths.size !== refsByPath.size) {
    const missing = [...refsByPath.keys()].find(path => !matchedPaths.has(path));
    throw new Error(`forme-render-static: image AssetRef does not target an image node at path ${missing}`);
  }
  return rewritten;
}

function rewriteNode(
  node: unknown,
  path: readonly number[],
  refsByPath: ReadonlyMap<string, AssetRef>,
  matchedPaths: Set<string>,
): unknown {
  if (typeof node !== "object" || node === null) return node;
  const record = node as Readonly<Record<string, unknown>>;
  const key = JSON.stringify(path);
  const ref = refsByPath.get(key);
  if (record.type === "image" && ref !== undefined) {
    matchedPaths.add(key);
    return { ...record, destination: assetPlaceholderUrl(ref.id, ref.urlSuffix) };
  }
  if (!Array.isArray(record.children)) return node;
  const originalChildren = record.children;
  const children = originalChildren.map((child, index) =>
    rewriteNode(child, [...path, index], refsByPath, matchedPaths));
  if (children.every((child, index) => child === originalChildren[index])) return node;
  return { ...record, children };
}
