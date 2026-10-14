// Stable composition boundary for direct writing-system figure-target owners.
// Existing-script changes belong in one file under figure-target-policies/;
// this module changes only when a writing system itself is added or removed.

import {
  arabicFigureTargetPolicy,
  persoArabicFigureTargetPolicy,
  urduNastaliqFigureTargetPolicy,
} from "./figure-target-policies/arabic-family.js";
import { bengaliFigureTargetPolicy } from "./figure-target-policies/bengali.js";
import { chineseFigureTargetPolicy } from "./figure-target-policies/chinese.js";
import { cyrillicFigureTargetPolicy } from "./figure-target-policies/cyrillic.js";
import { devanagariFigureTargetPolicy } from "./figure-target-policies/devanagari.js";
import { gujaratiFigureTargetPolicy } from "./figure-target-policies/gujarati.js";
import { gurmukhiFigureTargetPolicy } from "./figure-target-policies/gurmukhi.js";
import { japaneseFigureTargetPolicy } from "./figure-target-policies/japanese.js";
import { kannadaFigureTargetPolicy } from "./figure-target-policies/kannada.js";
import { latinFigureTargetPolicy } from "./figure-target-policies/latin.js";
import { malayalamFigureTargetPolicy } from "./figure-target-policies/malayalam.js";
import { tamilFigureTargetPolicy } from "./figure-target-policies/tamil.js";
import { teluguFigureTargetPolicy } from "./figure-target-policies/telugu.js";
import type {
  ApartClusterSource,
  FigureTargetPolicy,
  FusedLetterSequenceSource,
  FusedSignPairSource,
  SplitLetterSource,
} from "./figure-target-policy-types.js";

const policies: readonly FigureTargetPolicy[] = [
  tamilFigureTargetPolicy,
  devanagariFigureTargetPolicy,
  gujaratiFigureTargetPolicy,
  arabicFigureTargetPolicy,
  persoArabicFigureTargetPolicy,
  urduNastaliqFigureTargetPolicy,
  cyrillicFigureTargetPolicy,
  chineseFigureTargetPolicy,
  japaneseFigureTargetPolicy,
  kannadaFigureTargetPolicy,
  malayalamFigureTargetPolicy,
  teluguFigureTargetPolicy,
  bengaliFigureTargetPolicy,
  gurmukhiFigureTargetPolicy,
  latinFigureTargetPolicy,
];

const SAFE_ID = /^[a-z][a-z0-9-]*$/;

function assertUnique(values: readonly string[], label: string): void {
  const seen = new Set<string>();
  for (const value of values) {
    if (seen.has(value)) throw new Error(`duplicate figure-target ${label} '${value}'`);
    seen.add(value);
  }
}

function assertSources(
  script: string,
  kind: string,
  sources: readonly { citation: string; url: string; rows: readonly string[] }[],
): void {
  for (const source of sources) {
    if (source.citation.trim().length < 20) {
      throw new Error(`${script} ${kind} source needs a specific citation`);
    }
    if (!/^https:\/\/\S+$/.test(source.url)) {
      throw new Error(`${script} ${kind} source URL must be HTTPS`);
    }
    if (source.rows.length === 0) throw new Error(`${script} ${kind} source has no rows`);
    assertUnique(source.rows, `${script} ${kind} row`);
    for (const row of source.rows) {
      if (row === "" || row.normalize("NFD") !== row) {
        throw new Error(`${script} ${kind} rows must be non-empty NFD strings`);
      }
    }
  }
}

function validatedPolicies(input: readonly FigureTargetPolicy[]): readonly FigureTargetPolicy[] {
  assertUnique(input.map((policy) => policy.script), "script");
  assertUnique(input.flatMap((policy) => policy.tracks), "track");
  for (const policy of input) {
    if (!SAFE_ID.test(policy.script)) throw new Error(`unsafe figure-target script '${policy.script}'`);
    if (policy.tracks.length === 0 || policy.tracks.some((track) => !SAFE_ID.test(track))) {
      throw new Error(`${policy.script} must own at least one safe track`);
    }
    for (const sign of Object.keys(policy.writtenSignSides ?? {})) {
      if (sign.normalize("NFD") !== sign || !/^\p{M}+$/u.test(sign)) {
        throw new Error(`${policy.script} written-sign rows must be NFD marks`);
      }
    }
    assertSources(
      policy.script,
      "fused-pair",
      (policy.fusedSignPairSources ?? []).map((source) => ({ ...source, rows: source.pairs })),
    );
    assertSources(
      policy.script,
      "fused-sequence",
      (policy.fusedLetterSequenceSources ?? []).map((source) => ({ ...source, rows: source.sequences })),
    );
    assertSources(
      policy.script,
      "apart-cluster",
      (policy.apartClusterSources ?? []).map((source) => ({ ...source, rows: source.clusters })),
    );
    assertSources(
      policy.script,
      "split-letter",
      (policy.headlineWord?.splitLetterSources ?? []).map((source) => ({ ...source, rows: source.letters })),
    );
  }
  return input;
}

const validated = validatedPolicies(policies);
const byScript = new Map(validated.map((policy) => [policy.script, policy]));

export function figureTargetPolicy(script: string): FigureTargetPolicy | undefined {
  return byScript.get(script);
}

export const DERIVED_FILMSTRIP_SCRIPTS: Readonly<Record<string, string>> = Object.fromEntries(
  validated.flatMap((policy) => policy.tracks.map((track) => [track, policy.script])),
);

export const SEPARATE_LETTER_SCRIPTS: ReadonlySet<string> = new Set(
  validated.filter((policy) => policy.separateLetters).map((policy) => policy.script),
);

export const WRITTEN_SIGN_SIDES: Readonly<
  Record<string, Readonly<Record<string, "before" | "after">>>
> = Object.fromEntries(
  validated
    .filter((policy) => policy.writtenSignSides !== undefined)
    .map((policy) => [policy.script, policy.writtenSignSides!]),
);

export const FUSED_SIGN_PAIR_SOURCES: Readonly<Record<string, readonly FusedSignPairSource[]>> =
  Object.fromEntries(
    validated
      .filter((policy) => policy.fusedSignPairSources !== undefined)
      .map((policy) => [policy.script, policy.fusedSignPairSources!]),
  );

export const FUSED_SIGN_PAIRS: Readonly<Record<string, ReadonlySet<string>>> = Object.fromEntries(
  Object.entries(FUSED_SIGN_PAIR_SOURCES).map(([script, sources]) => [
    script,
    new Set(sources.flatMap((source) => source.pairs)),
  ]),
);

export const FUSED_LETTER_SEQUENCE_SOURCES: Readonly<
  Record<string, readonly FusedLetterSequenceSource[]>
> = Object.fromEntries(
  validated
    .filter((policy) => policy.fusedLetterSequenceSources !== undefined)
    .map((policy) => [policy.script, policy.fusedLetterSequenceSources!]),
);

export const APART_CLUSTER_SOURCES: Readonly<Record<string, readonly ApartClusterSource[]>> =
  Object.fromEntries(
    validated
      .filter((policy) => policy.apartClusterSources !== undefined)
      .map((policy) => [policy.script, policy.apartClusterSources!]),
  );

export const HEADLINE_WORD_SCRIPTS: Readonly<Record<string, RegExp>> = Object.fromEntries(
  validated
    .filter((policy) => policy.headlineWord !== undefined)
    .map((policy) => [policy.script, policy.headlineWord!.letter]),
);

export const HEADLINE_WORD_SPLIT_LETTER_SOURCES: Readonly<
  Record<string, readonly SplitLetterSource[]>
> = Object.fromEntries(
  validated
    .filter((policy) => policy.headlineWord?.splitLetterSources !== undefined)
    .map((policy) => [policy.script, policy.headlineWord!.splitLetterSources!]),
);

export const HEADLINE_WORD_SIGNS: Readonly<Record<string, ReadonlySet<string>>> = Object.fromEntries(
  validated
    .filter((policy) => policy.headlineWord?.signs !== undefined)
    .map((policy) => [policy.script, policy.headlineWord!.signs!]),
);

export type {
  ApartClusterSource,
  FusedLetterSequenceSource,
  FusedSignPairSource,
  SplitLetterSource,
} from "./figure-target-policy-types.js";
