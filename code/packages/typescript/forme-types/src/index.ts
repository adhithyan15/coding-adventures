/**
 * @coding-adventures/forme-types
 *
 * The Forme kernel's shared type vocabulary — Kinds, KindDescriptors,
 * branded identity types, and JSON utilities.  Every other Forme
 * package imports from here.
 *
 * This package is types and constants only.  No I/O, no runtime side
 * effects, no dependencies beyond `@coding-adventures/document-ast`
 * (which supplies `DocumentNode`, the Content IR root).  The
 * implementations of identity hashing live in `forme-identity`; the
 * stage interface lives in `forme-stage`; etc.
 *
 * See FM01 §2 for the full design.  See the per-module headers for
 * the rationale behind each type group:
 *
 *   utility.ts  — JsonValue, ReadonlyRecord
 *   identity.ts — LogicalId, RevisionId branded aliases
 *   kinds.ts    — KIND name set, KindDescriptor, canonical Kinds object,
 *                 Stream helpers, KERNEL_API_VERSION
 *   shapes.ts   — Each kind's TypeScript interface (12 kernel kinds)
 *                 plus stub StyleDocument / Interactivity that FM04 / FM05
 *                 will replace.
 *   payload.ts  — KindPayload mapped type that infers the value type
 *                 for a stage's `In` and `Out` from its descriptors.
 */

export type { JsonValue, ReadonlyRecord } from "./utility.js";

export type { LogicalId, RevisionId } from "./identity.js";

export {
  KERNEL_API_VERSION,
  KINDS,
  Kinds,
  streamOf,
  isStreamDescriptor,
} from "./kinds.js";
export type {
  KernelApiVersion,
  BuiltinKindName,
  KindName,
  KindDescriptor,
} from "./kinds.js";

export {
  EMPTY_STYLE,
  EMPTY_INTERACTIVITY,
} from "./shapes.js";
export type {
  AssetRole,
  AssetRef,
  ContentSource,
  ContentNode,
  OrderKey,
  CollectionEntry,
  Collection,
  Asset,
  StyleDocument,
  Interactivity,
  Document,
  StyleRuleId,
  IslandId,
  PageMeta,
  RenderedPage,
  Length,
  Margins,
  PageSizeName,
  PageSize,
  PageSettings,
  RunningElement,
  PrintForme,
  RuntimeRequirement,
  RequestHandler,
  SearchIndex,
  FeedFormat,
  Feed,
  DeployVariant,
  DeployRoute,
  DeployAssetEntry,
  DeployManifest,
  DeployArtifact,
  Stream,
} from "./shapes.js";

export type { KindPayloadMap, KindPayload } from "./payload.js";
