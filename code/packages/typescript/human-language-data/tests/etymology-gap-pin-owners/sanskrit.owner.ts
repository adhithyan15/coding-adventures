import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "sanskrit",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 1200 / 1099 baseline fell to 0 / 0, and the 12 empty taken-apart blocks to 0. 1200 lessons gained an etymology_hook and 1099 a taken-apart block; the 12 empty blocks gained one origin sentence each, and the 91 lessons that already showed an origin had it checked before their hook was added. Each origin was written from that word's own entries: Monier-Williams 1899 for the root, its homonym number and its cognate notes; CDIAL for the entry number, a descendant the entry lists and the entry's hedges; DEDR for Dravidian loans; EtymDB 2.1 for Indo-European cousins and loan chains.",
    "Most blocks derive the word from a verbal root or name the members of a compound; about 260 give an Indo-European cousin backed by MW's cognate note or EtymDB; loans from Dravidian, Munda, Iranian and Greek (horā, kendra) carry the hedges their sources carry; modern pairings MW does not record (vimānapattana, cikitsālaya) are said to be modern; about 170 say the origin is not settled. Six independent reviews read every lesson against its sources and corrected 167. No max_seconds value was raised.",
  ],
} as const satisfies EtymologyGapPinOwner;
