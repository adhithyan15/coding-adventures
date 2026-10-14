// Stable types shared by direct writing-system figure-target policy owners.

export interface FusedSignPairSource {
  readonly citation: string;
  readonly url: string;
  readonly pairs: readonly string[];
}

export interface FusedLetterSequenceSource {
  readonly citation: string;
  readonly url: string;
  readonly sequences: readonly string[];
}

export interface ApartClusterSource {
  readonly citation: string;
  readonly url: string;
  readonly clusters: readonly string[];
}

export interface SplitLetterSource {
  readonly citation: string;
  readonly url: string;
  readonly letters: readonly string[];
}

export interface HeadlineWordPolicy {
  readonly letter: RegExp;
  readonly signs?: ReadonlySet<string>;
  readonly consonant?: RegExp;
  readonly splitLetterSources?: readonly SplitLetterSource[];
}

/**
 * One direct owner for the facts that permit target derivation in a writing
 * system. Optional fields are permissions: absence always means refusal.
 */
export interface FigureTargetPolicy {
  readonly script: string;
  readonly tracks: readonly string[];
  readonly separateLetters?: true;
  readonly writtenSignSides?: Readonly<Record<string, "before" | "after">>;
  readonly fusedSignPairSources?: readonly FusedSignPairSource[];
  readonly fusedLetterSequenceSources?: readonly FusedLetterSequenceSource[];
  readonly apartClusterSources?: readonly ApartClusterSource[];
  readonly headlineWord?: HeadlineWordPolicy;
}
