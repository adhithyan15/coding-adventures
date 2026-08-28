// Infrastructure-only assembly for stable Tamil glyph owners.

import { entry as uB85 } from "./tamil/U-B85.ts";
import { entry as uB86 } from "./tamil/U-B86.ts";
import { entry as uB87 } from "./tamil/U-B87.ts";
import { entry as uB89 } from "./tamil/U-B89.ts";
import { entry as uB8A } from "./tamil/U-B8A.ts";
import { entry as uB92 } from "./tamil/U-B92.ts";
import { entry as uB95 } from "./tamil/U-B95.ts";
import { entry as uB99 } from "./tamil/U-B99.ts";
import { entry as uB9E } from "./tamil/U-B9E.ts";
import { entry as uB9A } from "./tamil/U-B9A.ts";
import { entry as uBB5 } from "./tamil/U-BB5.ts";
import { entry as uBB2 } from "./tamil/U-BB2.ts";
import { entry as uBB3 } from "./tamil/U-BB3.ts";
import { entry as uBB4 } from "./tamil/U-BB4.ts";
import { entry as uBB0 } from "./tamil/U-BB0.ts";
import { entry as uBB1 } from "./tamil/U-BB1.ts";
import { entry as uBA8 } from "./tamil/U-BA8.ts";
import { entry as uBA9 } from "./tamil/U-BA9.ts";
import { entry as uBA3 } from "./tamil/U-BA3.ts";
import { entry as uBAA } from "./tamil/U-BAA.ts";
import { entry as uB9F } from "./tamil/U-B9F.ts";
import { entry as uBA4 } from "./tamil/U-BA4.ts";
import { entry as uBAF } from "./tamil/U-BAF.ts";
import { entry as uBAE } from "./tamil/U-BAE.ts";
import { entry as uB8E } from "./tamil/U-B8E.ts";

import type { DuctusEntry } from "./registry.ts";

export const mainEntries: DuctusEntry[] = [
  uB85,
  uB86,
  uB87,
  uB89,
  uB8A,
  uB92,
  uB95,
  uB99,
  uB9E,
  uB9A,
  uBB5,
  uBB2,
  uBB3,
  uBB4,
  uBB0,
  uBB1,
  uBA8,
  uBA9,
  uBA3,
  uBAA,
  uB9F,
  uBA4,
  uBAF,
  uBAE,
];

// The historical Tamil tail entry followed the later Indic owner blocks.
export const tailEntries: DuctusEntry[] = [uB8E];
