/**
 * The authored vocabulary for lesson `sounds:` frontmatter.
 *
 * This is deliberately independent of the lesson corpus. Deriving the allowed
 * set from the lessons being checked would bless every typo at the moment it
 * was introduced and turn the validator into a check that cannot fail.
 */
export interface SoundTagRegistry {
  version: 1;
  tracks: Record<string, readonly string[]>;
}

const TRACK_ID = /^[a-z][a-z0-9-]*$/;
const SOUND_TAG = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

function record(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`sound-tag registry: ${label} must be an object`);
  }
  return value as Record<string, unknown>;
}

/** Parse and shape-check `core/sound-tags.json` at the filesystem boundary. */
export function parseSoundTagRegistry(value: unknown): SoundTagRegistry {
  const root = record(value, "root");
  if (root.version !== 1) {
    throw new Error("sound-tag registry: version must be 1");
  }
  const rawTracks = record(root.tracks, "tracks");
  const trackNames = Object.keys(rawTracks);
  const sortedTrackNames = [...trackNames].sort();
  if (trackNames.some((name, index) => name !== sortedTrackNames[index])) {
    throw new Error("sound-tag registry: tracks must be sorted by id");
  }

  const tracks: Record<string, readonly string[]> = Object.create(null);
  for (const track of trackNames) {
    if (!TRACK_ID.test(track)) {
      throw new Error(`sound-tag registry: unsafe track id '${track}'`);
    }
    const rawTags = rawTracks[track];
    if (!Array.isArray(rawTags)) {
      throw new Error(`sound-tag registry: '${track}' must contain an array`);
    }
    const tags: string[] = [];
    for (const [index, rawTag] of rawTags.entries()) {
      if (typeof rawTag !== "string" || !SOUND_TAG.test(rawTag)) {
        throw new Error(
          `sound-tag registry: ${track}[${index}] must be a lowercase hyphenated tag`,
        );
      }
      if (index > 0 && tags[index - 1]! >= rawTag) {
        throw new Error(
          `sound-tag registry: '${track}' tags must be sorted and unique near '${rawTag}'`,
        );
      }
      tags.push(rawTag);
    }
    tracks[track] = tags;
  }
  return { version: 1, tracks };
}
