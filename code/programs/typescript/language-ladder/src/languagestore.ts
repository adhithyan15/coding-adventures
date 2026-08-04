// Persist the set of languages a learner wants to mix. Values are normalized
// against the current registry, so removed tracks cannot poison saved state and
// newly added tracks are available from the picker immediately.

export const LANGUAGE_STORAGE_KEY = "language-ladder.languages.v1";

export interface LanguageStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}
export function normalizeLanguageSelection(
  candidate: Iterable<string>,
  available: readonly string[],
): string[] {
  const wanted = new Set(candidate);
  const normalized = available.filter((id) => wanted.has(id));
  return normalized.length > 0 ? normalized : [...available];
}

export function loadLanguages(
  storage: LanguageStorage | null,
  available: readonly string[],
): string[] {
  if (!storage) return [...available];
  try {
    const raw = storage.getItem(LANGUAGE_STORAGE_KEY);
    if (!raw) return [...available];
    const parsed: unknown = JSON.parse(raw);
    return normalizeLanguageSelection(Array.isArray(parsed) ? parsed.filter((x): x is string => typeof x === "string") : [], available);
  } catch {
    return [...available];
  }
}

export function saveLanguages(
  storage: LanguageStorage | null,
  selection: Iterable<string>,
  available: readonly string[],
): string[] {
  const normalized = normalizeLanguageSelection(selection, available);
  try {
    storage?.setItem(LANGUAGE_STORAGE_KEY, JSON.stringify(normalized));
  } catch {
    // Persistence is an enhancement; an unavailable store must not block study.
  }
  return normalized;
}
