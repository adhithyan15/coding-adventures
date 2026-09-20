import type {
  CurriculumExtensionNode,
  CurriculumPathSegment,
  LanguageCurriculum,
} from "./types.js";

/** One extension edge owned beside a lesson, including its exact legacy order. */
export interface CurriculumExtensionMembership {
  id: string;
  order: number;
}

/**
 * The complete curriculum placement of one lesson.
 *
 * This is deliberately one owner per lesson. Adding a lesson therefore adds one
 * file instead of appending the same id to shared path and extension arrays.
 */
export interface CurriculumLessonMembership {
  id: string;
  pathSegment: string;
  pathOrder: number;
  extensions: CurriculumExtensionMembership[];
}

export type AuthoredCurriculumPathSegment = Omit<CurriculumPathSegment, "lessons">;
export type AuthoredCurriculumExtensionNode = Omit<CurriculumExtensionNode, "lessons">;

/** The canonical shard projection before direct lesson owners are attached. */
export interface AuthoredLanguageCurriculum
  extends Omit<LanguageCurriculum, "path" | "extensions"> {
  path: AuthoredCurriculumPathSegment[];
  extensions: AuthoredCurriculumExtensionNode[];
}

function exactDenseOrder(
  rows: readonly { id: string; order: number }[],
  label: string,
): string[] {
  const ordered = [...rows].sort(
    (left, right) => left.order - right.order || left.id.localeCompare(right.id),
  );
  for (let index = 0; index < ordered.length; index += 1) {
    const row = ordered[index]!;
    if (!Number.isInteger(row.order) || row.order !== index) {
      throw new Error(
        `${label}: lesson '${row.id}' has order ${String(row.order)}, expected ${index}`,
      );
    }
  }
  return ordered.map((row) => row.id);
}

function withLessonsAfter(
  owner: Record<string, unknown>,
  after: string,
  lessons: readonly string[],
): Record<string, unknown> {
  const hydrated: Record<string, unknown> = {};
  let inserted = false;
  for (const [key, value] of Object.entries(owner)) {
    hydrated[key] = value;
    if (key === after) {
      hydrated.lessons = [...lessons];
      inserted = true;
    }
  }
  if (!inserted) hydrated.lessons = [...lessons];
  return hydrated;
}

/**
 * Reconstruct the historical public curriculum shape from direct lesson owners.
 *
 * The aggregate arrays remain available to every existing consumer, but no
 * canonical shard stores them. Every edge has exactly one durable owner.
 */
export function attachCurriculumLessonMemberships(
  curriculum: AuthoredLanguageCurriculum,
  owners: readonly CurriculumLessonMembership[],
): LanguageCurriculum {
  const paths = new Map<string, AuthoredCurriculumPathSegment>();
  for (const path of curriculum.path) {
    const record = path as unknown as Record<string, unknown>;
    if (Object.hasOwn(record, "lessons")) {
      throw new Error(
        `curriculum path '${path.id}': must not store derived 'lessons'; ` +
          "membership is owned by direct lesson owners",
      );
    }
    if (paths.has(path.id)) throw new Error(`curriculum path: duplicate segment id '${path.id}'`);
    paths.set(path.id, path);
  }

  const extensions = new Map<string, AuthoredCurriculumExtensionNode>();
  for (const extension of curriculum.extensions) {
    const record = extension as unknown as Record<string, unknown>;
    if (Object.hasOwn(record, "lessons")) {
      throw new Error(
        `curriculum extension '${extension.id}': must not store derived 'lessons'; ` +
          "membership is owned by direct lesson owners",
      );
    }
    if (extensions.has(extension.id)) {
      throw new Error(`curriculum extensions: duplicate id '${extension.id}'`);
    }
    extensions.set(extension.id, extension);
  }

  const pathRows = new Map<string, Array<{ id: string; order: number }>>(
    curriculum.path.map((path) => [path.id, []]),
  );
  const extensionRows = new Map<string, Array<{ id: string; order: number }>>(
    curriculum.extensions.map((extension) => [extension.id, []]),
  );
  const seenLessons = new Set<string>();

  for (const owner of owners) {
    if (seenLessons.has(owner.id)) {
      throw new Error(`curriculum lesson membership: duplicate owner '${owner.id}'`);
    }
    seenLessons.add(owner.id);
    const path = paths.get(owner.pathSegment);
    if (path === undefined) {
      throw new Error(
        `curriculum lesson '${owner.id}': path segment '${owner.pathSegment}' has no owner`,
      );
    }
    pathRows.get(owner.pathSegment)!.push({ id: owner.id, order: owner.pathOrder });

    const attached = new Set([...path.before, ...path.inline, ...path.after]);
    const seenExtensions = new Set<string>();
    for (const membership of owner.extensions) {
      if (seenExtensions.has(membership.id)) {
        throw new Error(
          `curriculum lesson '${owner.id}': repeats extension '${membership.id}'`,
        );
      }
      seenExtensions.add(membership.id);
      if (!extensions.has(membership.id)) {
        throw new Error(
          `curriculum lesson '${owner.id}': extension '${membership.id}' has no owner`,
        );
      }
      if (!attached.has(membership.id)) {
        throw new Error(
          `curriculum lesson '${owner.id}': extension '${membership.id}' is not attached ` +
            `to path segment '${owner.pathSegment}'`,
        );
      }
      extensionRows.get(membership.id)!.push({ id: owner.id, order: membership.order });
    }
  }

  const hydratedPaths = curriculum.path.map((path) =>
    withLessonsAfter(
      path as unknown as Record<string, unknown>,
      "spine_node",
      exactDenseOrder(pathRows.get(path.id)!, `curriculum path '${path.id}'`),
    ) as unknown as CurriculumPathSegment,
  );
  const hydratedExtensions = curriculum.extensions.map((extension) =>
    withLessonsAfter(
      extension as unknown as Record<string, unknown>,
      "prerequisites",
      exactDenseOrder(
        extensionRows.get(extension.id)!,
        `curriculum extension '${extension.id}'`,
      ),
    ) as unknown as CurriculumExtensionNode,
  );

  return {
    ...curriculum,
    path: hydratedPaths,
    extensions: hydratedExtensions,
  };
}
