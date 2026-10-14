import type { EtymologyGapPinOwner } from "../etymology-gap-pin-owners.js";

export const etymologyGapPinOwner = {
  language: "russian",
  withoutHook: 0,
  withoutVisible: 0,
  rationale: [
    "The 1,135 / 1,148 baseline fell to 0 / 0. All 1,135 generated word lessons gained a taken-apart block and matching hook; thirteen earlier hand-written lessons gained the half they lacked. Only RU-C02-menya-zovut required a duration increase, from 240 to 275 seconds.",
    "The review paraphrased and cited Vasmer's Russisches etymologisches Woerterbuch in its Russian edition with Trubachev's notes. Church Slavonic forms are named by their sound change, loans name donor and route, and 86 uncertain or disputed origins retain Vasmer's hedges.",
  ],
} as const satisfies EtymologyGapPinOwner;
