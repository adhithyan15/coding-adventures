// ---------------------------------------------------------------------------
// syllabary.ts — introducing an abugida's syllables SLOWLY, one consonant at a time.
//
// The Dravidian syllabaries (Telugu / Kannada / Malayalam) carry ~350 syllables
// each: 35 consonants, each across ten vowels (ka kā ki kī ku kū ke kē ko kō).
// Dropping all 350 on a beginner is the opposite of learning to read — the whole
// point (the user's words) is to "build pattern recognition slowly … ka, ki, ku …
// kha, khi, khu". So the drill starts with ONE consonant's vowel row and unlocks
// the next consonant only once the current row is mastered.
//
// This module is the pure gate. It knows nothing about the DOM or the drill; it
// takes the script's syllables (in the generated consonant-major order) plus the
// SRS state, and answers "which syllables are unlocked right now?". Deterministic
// and unit-tested, with a control that keeps a later consonant locked until the
// current row is mastered.
// ---------------------------------------------------------------------------

import { MAX_BOX } from "./scheduler.ts";

/** The minimal syllable shape this module needs — a base syllable has ONE piece
 *  (the bare consonant), a signed syllable has two (consonant + vowel sign). */
interface Syllable {
  role: string;
  components: string[];
}

/** The minimal SRS shape — the Leitner box is all that "mastered" depends on. */
interface Boxed {
  box: number;
}

/** How mastered a row must be before the next consonant unlocks (Leitner box). */
export const ROW_MASTERY_BOX = 3;

/** Is this script one of the generated syllabaries (every letter a syllable)? */
export function isSyllabary(letters: { role: string }[]): boolean {
  return letters.length > 0 && letters.every((l) => l.role === "syllable");
}

/**
 * Segment a consonant-major syllabary into one index-group per consonant.
 *
 * The generator emits, for each consonant, its bare form (inherent “a”, a single
 * component) followed by its signed syllables (two components). So a new group
 * begins at every single-component syllable — a grounded boundary that needs no
 * separate marker. `[[0,1,2,…], [10,11,…], …]`.
 */
export function consonantGroups(letters: Syllable[]): number[][] {
  const groups: number[][] = [];
  letters.forEach((l, i) => {
    if (l.components.length <= 1 || groups.length === 0) groups.push([i]);
    else groups[groups.length - 1]!.push(i);
  });
  return groups;
}

/** Is every syllable in this group's index list mastered (box ≥ ROW_MASTERY_BOX)? */
function rowMastered(group: number[], states: Boxed[]): boolean {
  return group.every((i) => {
    const s = states[i];
    return s !== undefined && s.box >= Math.min(ROW_MASTERY_BOX, MAX_BOX);
  });
}

/**
 * How many consonants are unlocked given the current SRS state.
 *
 * Always at least one (you start on `ka`). Each additional consonant unlocks only
 * when EVERY earlier consonant's row is mastered — so a gap (an un-mastered early
 * row) holds the rest locked, exactly the "don't run ahead" the slow build wants.
 * Capped at the number of consonants.
 */
export function unlockedConsonantCount(groups: number[][], states: Boxed[]): number {
  let unlocked = 1;
  for (let g = 0; g < groups.length; g++) {
    if (!rowMastered(groups[g]!, states)) break;
    unlocked = g + 2; // this row is done → the next consonant is open
  }
  return Math.min(unlocked, groups.length);
}

/** The flat list of syllable indices the learner may currently be drilled on. */
export function unlockedLetterIndices(groups: number[][], unlockedCount: number): number[] {
  const n = Math.max(1, Math.min(unlockedCount, groups.length));
  return groups.slice(0, n).flat();
}
