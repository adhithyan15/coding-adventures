import type { Manifest } from "@coding-adventures/forme-manifest";
import {
  Kinds,
  streamOf,
  type KindDescriptor,
  type KindName,
} from "@coding-adventures/forme-types";
import { PluginHostError } from "./errors.js";

export function descriptorForKindReference(
  reference: string,
  manifest: Manifest,
): KindDescriptor {
  const stream = /^Stream<([^<>]+)>$/.exec(reference);
  if (stream) return streamOf(descriptorForBareKind(stream[1]!, manifest));
  return descriptorForBareKind(reference, manifest);
}

function descriptorForBareKind(reference: string, manifest: Manifest): KindDescriptor {
  const builtin = (Kinds as Readonly<Record<string, KindDescriptor>>)[reference];
  if (builtin) return builtin;
  const contributed = manifest.contributes.kinds.find(kind => kind.name === reference);
  if (contributed) return { name: contributed.name as KindName, version: contributed.version };
  throw new PluginHostError("MANIFEST_INVALID", `unknown kind reference ${JSON.stringify(reference)}`);
}
