/**
 * combined.ts — `generateMetaTags({ basic?, og?, twitter? })`.
 *
 * Convenience wrapper combining all three generators into one call.
 * Tag-block order is: basic → opengraph → twitter (mirrors what
 * hand-authored heads look like; search-engine basic tags first,
 * then social preview cards).
 *
 * @module combined
 */

import { generateBasicTags } from "./basic.js";
import { generateOpenGraphTags } from "./opengraph.js";
import { generateTwitterCardTags } from "./twitter.js";
import type { BasicMeta, OpenGraphMeta, TwitterCardMeta } from "./types.js";

export interface CombinedMeta {
  readonly basic?: BasicMeta;
  readonly og?: OpenGraphMeta;
  readonly twitter?: TwitterCardMeta;
}

export function generateMetaTags(combined: CombinedMeta): string {
  const blocks: string[] = [];
  if (combined.basic   !== undefined) blocks.push(generateBasicTags(combined.basic));
  if (combined.og      !== undefined) blocks.push(generateOpenGraphTags(combined.og));
  if (combined.twitter !== undefined) blocks.push(generateTwitterCardTags(combined.twitter));
  // Filter empty strings (a meta block with zero supplied fields
  // produces "" — don't emit double blank lines).
  return blocks.filter((b) => b.length > 0).join("\n");
}
