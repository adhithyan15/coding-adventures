// Compatibility facade for the historical figure-target import path.
// Stable algorithms and writing-system policy now have separate owners.

export * from "./figure-target-core.js";
export {
  APART_CLUSTER_SOURCES,
  DERIVED_FILMSTRIP_SCRIPTS,
  FUSED_LETTER_SEQUENCE_SOURCES,
  FUSED_SIGN_PAIRS,
  FUSED_SIGN_PAIR_SOURCES,
  HEADLINE_WORD_SCRIPTS,
  HEADLINE_WORD_SIGNS,
  HEADLINE_WORD_SPLIT_LETTER_SOURCES,
  SEPARATE_LETTER_SCRIPTS,
  WRITTEN_SIGN_SIDES,
  type ApartClusterSource,
  type FusedLetterSequenceSource,
  type FusedSignPairSource,
  type SplitLetterSource,
} from "./figure-target-policy.js";
