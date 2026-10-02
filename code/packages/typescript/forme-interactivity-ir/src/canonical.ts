import type { InteractivityDocument } from "./types.js";

const validatedDocuments = new WeakSet<object>();

/** Deterministic JSON: lexicographic object keys, authored array order. */
export function canonicalInteractivityDocument(document: InteractivityDocument): string {
  if (document === null || typeof document !== "object" || !validatedDocuments.has(document)) {
    throw new TypeError("canonicalInteractivityDocument requires a validated InteractivityDocument");
  }
  return JSON.stringify(sortValue(document));
}

/** @internal Brand a fresh frozen validator snapshot for public serialization. */
export function registerValidatedInteractivityDocument(document: InteractivityDocument): void {
  validatedDocuments.add(document);
}

function sortValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortValue);
  if (value !== null && typeof value === "object") {
    const result: Record<string, unknown> = Object.create(null) as Record<string, unknown>;
    for (const key of Object.keys(value).sort(compareUnicodeScalars)) {
      result[key] = sortValue((value as Record<string, unknown>)[key]);
    }
    return result;
  }
  return value;
}

export function compareUnicodeScalars(left: string, right: string): number {
  let leftIndex = 0;
  let rightIndex = 0;
  while (leftIndex < left.length && rightIndex < right.length) {
    const leftPoint = left.codePointAt(leftIndex)!;
    const rightPoint = right.codePointAt(rightIndex)!;
    if (leftPoint !== rightPoint) return leftPoint - rightPoint;
    leftIndex += leftPoint > 0xffff ? 2 : 1;
    rightIndex += rightPoint > 0xffff ? 2 : 1;
  }
  return (left.length - leftIndex) - (right.length - rightIndex);
}
