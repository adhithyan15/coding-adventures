/**
 * Built-in naive instrument timbre presets (MUS05 §8).
 */

import { AdsrEnvelope } from "./envelope.js";
import { HarmonicPartial } from "./partial.js";
import { InstrumentProfile } from "./profile.js";
import { VariationProfile } from "./variation.js";

function p(multiplier: number, amplitude: number, phaseCycles = 0.0): HarmonicPartial {
  return new HarmonicPartial(multiplier, amplitude, phaseCycles);
}

function env(
  attackSeconds: number,
  decaySeconds: number,
  sustainLevel: number,
  releaseSeconds: number,
): AdsrEnvelope {
  return new AdsrEnvelope(attackSeconds, decaySeconds, sustainLevel, releaseSeconds);
}

const defaultVariation = new VariationProfile();

export function sine(): InstrumentProfile {
  return new InstrumentProfile(
    "sine",
    "Pure Sine",
    "sine",
    1.0,
    [p(1.0, 1.0)],
    env(0.005, 0.01, 0.9, 0.03),
    defaultVariation,
    null,
    "synth",
  );
}

export function silence(): InstrumentProfile {
  return new InstrumentProfile(
    "silence",
    "Silence",
    "silence",
    0.0,
    [],
    env(0.0, 0.0, 0.0, 0.0),
    defaultVariation,
    null,
    "utility",
  );
}

export function fluteNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "flute_naive",
    "Naive Flute",
    "additive",
    0.95,
    [p(1.0, 1.0), p(2.0, 0.1), p(3.0, 0.03)],
    env(0.08, 0.05, 0.85, 0.08),
    defaultVariation,
    null,
    "pipe",
  );
}

export function clarinetNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "clarinet_naive",
    "Naive Clarinet",
    "additive",
    0.78,
    [p(1.0, 1.0), p(3.0, 0.45), p(5.0, 0.2), p(7.0, 0.08)],
    env(0.04, 0.06, 0.75, 0.07),
    defaultVariation,
    null,
    "reed",
  );
}

export function violinNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "violin_naive",
    "Naive Violin",
    "additive",
    0.62,
    [p(1.0, 1.0), p(2.0, 0.55), p(3.0, 0.35), p(4.0, 0.2), p(5.0, 0.12)],
    env(0.07, 0.08, 0.8, 0.12),
    defaultVariation,
    null,
    "strings",
  );
}

export function pianoNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "piano_naive",
    "Naive Piano",
    "additive",
    0.82,
    [p(1.0, 1.0), p(2.0, 0.45), p(3.0, 0.25), p(4.0, 0.14), p(5.0, 0.08)],
    env(0.005, 0.35, 0.18, 0.18),
    defaultVariation,
    null,
    "piano",
  );
}

export function pluckNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "pluck_naive",
    "Naive Plucked String",
    "additive",
    0.66,
    [
      p(1.0, 1.0),
      p(2.0, 0.6),
      p(3.0, 0.38),
      p(4.0, 0.22),
      p(5.0, 0.13),
      p(6.0, 0.08),
    ],
    env(0.003, 0.45, 0.05, 0.08),
    defaultVariation,
    null,
    "plucked",
  );
}

export function brassNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "brass_naive",
    "Naive Brass",
    "additive",
    0.64,
    [p(1.0, 1.0), p(2.0, 0.7), p(3.0, 0.42), p(4.0, 0.22), p(5.0, 0.14)],
    env(0.03, 0.07, 0.78, 0.09),
    defaultVariation,
    null,
    "brass",
  );
}

export function organNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "organ_naive",
    "Naive Organ",
    "additive",
    0.55,
    [p(1.0, 1.0), p(2.0, 0.75), p(3.0, 0.45), p(4.0, 0.25)],
    env(0.01, 0.01, 0.95, 0.04),
    defaultVariation,
    null,
    "organ",
  );
}

export function malletNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "mallet_naive",
    "Naive Mallet",
    "additive",
    0.74,
    [p(1.0, 1.0), p(2.7, 0.28), p(5.1, 0.12)],
    env(0.002, 0.6, 0.02, 0.12),
    defaultVariation,
    null,
    "chromatic percussion",
  );
}

export function celestaNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "celesta_naive",
    "Naive Celesta",
    "additive",
    0.6,
    [p(1.0, 1.0), p(2.0, 0.35), p(4.0, 0.18), p(6.3, 0.08)],
    env(0.004, 0.9, 0.1, 0.25),
    defaultVariation,
    null,
    "chromatic percussion",
  );
}

export function glockenspielNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "glockenspiel_naive",
    "Naive Glockenspiel",
    "additive",
    0.5,
    [p(1.0, 1.0), p(2.8, 0.55), p(5.4, 0.3), p(8.9, 0.15)],
    env(0.001, 0.9, 0.02, 0.5),
    defaultVariation,
    null,
    "chromatic percussion",
  );
}

export function vibraphoneNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "vibraphone_naive",
    "Naive Vibraphone",
    "additive",
    0.58,
    [p(1.0, 1.0), p(2.0, 0.35), p(3.9, 0.16), p(6.8, 0.06)],
    env(0.003, 0.8, 0.28, 0.45),
    defaultVariation,
    null,
    "chromatic percussion",
  );
}

export function marimbaNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "marimba_naive",
    "Naive Marimba",
    "additive",
    0.68,
    [p(1.0, 1.0), p(4.0, 0.3), p(10.2, 0.1)],
    env(0.002, 0.55, 0.03, 0.12),
    defaultVariation,
    null,
    "chromatic percussion",
  );
}

export function xylophoneNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "xylophone_naive",
    "Naive Xylophone",
    "additive",
    0.64,
    [p(1.0, 1.0), p(3.2, 0.32), p(6.1, 0.14)],
    env(0.001, 0.35, 0.01, 0.08),
    defaultVariation,
    null,
    "chromatic percussion",
  );
}

export function tubularBellsNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "tubular_bells_naive",
    "Naive Tubular Bells",
    "additive",
    0.46,
    [p(1.0, 1.0), p(2.0, 0.2), p(3.0, 0.12), p(4.2, 0.28), p(6.8, 0.14)],
    env(0.003, 1.3, 0.04, 0.8),
    defaultVariation,
    null,
    "chromatic percussion",
  );
}

export function timpaniNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "timpani_naive",
    "Naive Timpani",
    "additive",
    0.7,
    [p(1.0, 1.0), p(1.5, 0.25), p(2.0, 0.38), p(2.44, 0.18), p(3.0, 0.08)],
    env(0.003, 0.7, 0.06, 0.2),
    defaultVariation,
    null,
    "pitched percussion",
  );
}

export function kalimbaNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "kalimba_naive",
    "Naive Kalimba",
    "additive",
    0.63,
    [p(1.0, 1.0), p(2.0, 0.22), p(3.0, 0.08), p(6.0, 0.18)],
    env(0.002, 0.45, 0.04, 0.15),
    defaultVariation,
    null,
    "pitched percussion",
  );
}

export function synthLeadNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "synth_lead_naive",
    "Naive Synth Lead",
    "additive",
    0.58,
    [
      p(1.0, 1.0),
      p(2.0, 0.85),
      p(3.0, 0.65),
      p(4.0, 0.48),
      p(5.0, 0.36),
      p(6.0, 0.24),
    ],
    env(0.004, 0.04, 0.8, 0.04),
    defaultVariation,
    null,
    "synth lead",
  );
}

export function synthPadNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "synth_pad_naive",
    "Naive Synth Pad",
    "additive",
    0.46,
    [p(1.0, 1.0), p(2.0, 0.35), p(3.0, 0.18), p(5.0, 0.1)],
    env(0.45, 0.4, 0.75, 0.55),
    defaultVariation,
    null,
    "synth pad",
  );
}

export function effectNaive(): InstrumentProfile {
  return new InstrumentProfile(
    "effect_naive",
    "Naive Effect",
    "additive",
    0.45,
    [p(1.0, 1.0), p(1.5, 0.55), p(2.25, 0.35), p(3.7, 0.18)],
    env(0.02, 0.7, 0.2, 0.25),
    defaultVariation,
    null,
    "sound effects",
  );
}

export const flute_naive = fluteNaive;
export const clarinet_naive = clarinetNaive;
export const violin_naive = violinNaive;
export const piano_naive = pianoNaive;
export const pluck_naive = pluckNaive;
export const brass_naive = brassNaive;
export const organ_naive = organNaive;
export const mallet_naive = malletNaive;
export const celesta_naive = celestaNaive;
export const glockenspiel_naive = glockenspielNaive;
export const vibraphone_naive = vibraphoneNaive;
export const marimba_naive = marimbaNaive;
export const xylophone_naive = xylophoneNaive;
export const tubular_bells_naive = tubularBellsNaive;
export const timpani_naive = timpaniNaive;
export const kalimba_naive = kalimbaNaive;
export const synth_lead_naive = synthLeadNaive;
export const synth_pad_naive = synthPadNaive;
export const effect_naive = effectNaive;

const PRESET_FACTORIES: ReadonlyMap<string, () => InstrumentProfile> = new Map([
  ["sine", sine],
  ["silence", silence],
  ["flute_naive", fluteNaive],
  ["clarinet_naive", clarinetNaive],
  ["violin_naive", violinNaive],
  ["piano_naive", pianoNaive],
  ["pluck_naive", pluckNaive],
  ["brass_naive", brassNaive],
  ["organ_naive", organNaive],
  ["mallet_naive", malletNaive],
  ["celesta_naive", celestaNaive],
  ["glockenspiel_naive", glockenspielNaive],
  ["vibraphone_naive", vibraphoneNaive],
  ["marimba_naive", marimbaNaive],
  ["xylophone_naive", xylophoneNaive],
  ["tubular_bells_naive", tubularBellsNaive],
  ["timpani_naive", timpaniNaive],
  ["kalimba_naive", kalimbaNaive],
  ["synth_lead_naive", synthLeadNaive],
  ["synth_pad_naive", synthPadNaive],
  ["effect_naive", effectNaive],
]);

export function getInstrument(id: string): InstrumentProfile {
  if (typeof id !== "string") {
    throw new TypeError(`Instrument id must be a string, got ${typeof id}`);
  }
  const factory = PRESET_FACTORIES.get(id.trim());
  if (!factory) {
    throw new RangeError(`Unknown instrument preset id: "${id}"`);
  }
  return factory();
}

export function allInstruments(): readonly InstrumentProfile[] {
  return Array.from(PRESET_FACTORIES.values()).map((fn) => fn());
}
