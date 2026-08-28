export interface LandingLink {
  readonly label: string;
  readonly href: string;
}

export interface LandingStat {
  readonly value: string;
  readonly label: string;
}

export interface LandingPath {
  readonly number: string;
  readonly title: string;
  readonly description: string;
  readonly links: readonly LandingLink[];
}

export interface LandingLab {
  readonly kicker: string;
  readonly title: string;
  readonly href: string;
  readonly description: string;
  readonly tags: readonly string[];
  readonly featured: boolean;
}

export interface LandingWorkshopItem {
  readonly kicker: string;
  readonly title: string;
  readonly href: string;
  readonly description: string;
}

export interface LandingModel {
  readonly schemaVersion: 1;
  readonly site: {
    readonly title: string;
    readonly description: string;
    readonly shortDescription: string;
    readonly canonicalUrl: string;
    readonly repositoryUrl: string;
    readonly ogImage: string;
    readonly ogImageWidth: number;
    readonly ogImageHeight: number;
    readonly updated: string;
  };
  readonly hero: {
    readonly eyebrow: string;
    readonly title: string;
    readonly accent: string;
    readonly intro: string;
    readonly primaryAction: LandingLink;
    readonly secondaryAction: LandingLink;
    readonly stack: readonly { readonly label: string; readonly detail: string }[];
  };
  readonly snapshot: {
    readonly items: readonly LandingStat[];
    readonly note: string;
  };
  readonly paths: readonly LandingPath[];
  readonly labs: readonly LandingLab[];
  readonly forme: {
    readonly eyebrow: string;
    readonly title: string;
    readonly description: string;
    readonly actions: readonly LandingLink[];
    readonly pipeline: readonly { readonly label: string; readonly detail: string }[];
    readonly status: readonly LandingStat[];
  };
  readonly workshop: readonly LandingWorkshopItem[];
  readonly footer: string;
}

type RecordValue = Readonly<Record<string, unknown>>;

export function parseLandingModel(value: unknown): LandingModel {
  const root = record(value, "landing document");
  if (root.schemaVersion !== 1) fail("schemaVersion must equal 1");
  const site = record(root.site, "site");
  const hero = record(root.hero, "hero");
  const snapshot = record(root.snapshot, "snapshot");
  const forme = record(root.forme, "forme");

  return {
    schemaVersion: 1,
    site: {
      title: text(site, "title"),
      description: text(site, "description"),
      shortDescription: text(site, "shortDescription"),
      canonicalUrl: absoluteUrl(site, "canonicalUrl"),
      repositoryUrl: absoluteUrl(site, "repositoryUrl"),
      ogImage: relativePath(site, "ogImage"),
      ogImageWidth: positiveInteger(site, "ogImageWidth"),
      ogImageHeight: positiveInteger(site, "ogImageHeight"),
      updated: date(site, "updated"),
    },
    hero: {
      eyebrow: text(hero, "eyebrow"),
      title: text(hero, "title"),
      accent: text(hero, "accent"),
      intro: text(hero, "intro"),
      primaryAction: link(hero.primaryAction, "hero.primaryAction"),
      secondaryAction: link(hero.secondaryAction, "hero.secondaryAction"),
      stack: array(hero.stack, "hero.stack", (item, index) => {
        const entry = record(item, `hero.stack[${index}]`);
        return { label: text(entry, "label"), detail: text(entry, "detail") };
      }),
    },
    snapshot: {
      items: array(snapshot.items, "snapshot.items", stat),
      note: text(snapshot, "note"),
    },
    paths: array(root.paths, "paths", path),
    labs: array(root.labs, "labs", lab),
    forme: {
      eyebrow: text(forme, "eyebrow"),
      title: text(forme, "title"),
      description: text(forme, "description"),
      actions: array(forme.actions, "forme.actions", link),
      pipeline: array(forme.pipeline, "forme.pipeline", (item, index) => {
        const entry = record(item, `forme.pipeline[${index}]`);
        return { label: text(entry, "label"), detail: text(entry, "detail") };
      }),
      status: array(forme.status, "forme.status", stat),
    },
    workshop: array(root.workshop, "workshop", workshopItem),
    footer: text(root, "footer"),
  };
}

function path(value: unknown, index: number): LandingPath {
  const entry = record(value, `paths[${index}]`);
  return {
    number: text(entry, "number"),
    title: text(entry, "title"),
    description: text(entry, "description"),
    links: array(entry.links, `paths[${index}].links`, link),
  };
}

function lab(value: unknown, index: number): LandingLab {
  const entry = record(value, `labs[${index}]`);
  return {
    kicker: text(entry, "kicker"),
    title: text(entry, "title"),
    href: href(entry, "href"),
    description: text(entry, "description"),
    tags: array(entry.tags, `labs[${index}].tags`, (tag, tagIndex) =>
      nonEmptyString(tag, `labs[${index}].tags[${tagIndex}]`)),
    featured: entry.featured === undefined ? false : boolean(entry.featured, `labs[${index}].featured`),
  };
}

function workshopItem(value: unknown, index: number): LandingWorkshopItem {
  const entry = record(value, `workshop[${index}]`);
  return {
    kicker: text(entry, "kicker"),
    title: text(entry, "title"),
    href: href(entry, "href"),
    description: text(entry, "description"),
  };
}

function link(value: unknown, indexOrName: number | string): LandingLink {
  const name = typeof indexOrName === "number" ? `links[${indexOrName}]` : indexOrName;
  const entry = record(value, name);
  return { label: text(entry, "label"), href: href(entry, "href") };
}

function stat(value: unknown, index: number): LandingStat {
  const entry = record(value, `stats[${index}]`);
  return { value: text(entry, "value"), label: text(entry, "label") };
}

function record(value: unknown, name: string): RecordValue {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    fail(`${name} must be an object`);
  }
  return value as RecordValue;
}

function array<T>(
  value: unknown,
  name: string,
  parse: (item: unknown, index: number) => T,
): readonly T[] {
  if (!Array.isArray(value) || value.length === 0) fail(`${name} must be a non-empty array`);
  return value.map(parse);
}

function text(value: RecordValue, key: string): string {
  return nonEmptyString(value[key], key);
}

function nonEmptyString(value: unknown, name: string): string {
  if (typeof value !== "string" || value.trim().length === 0) fail(`${name} must be a non-empty string`);
  return value;
}

function href(value: RecordValue, key: string): string {
  const result = text(value, key);
  const rootRelative = result.startsWith("/") && !result.startsWith("//") && !result.includes("\\");
  if (!(result.startsWith("#") || rootRelative || validHttpsUrl(result))) {
    fail(`${key} must be a hash, root-relative path, or HTTPS URL`);
  }
  return result;
}

function validHttpsUrl(value: string): boolean {
  try {
    return new URL(value).protocol === "https:";
  } catch {
    return false;
  }
}

function absoluteUrl(value: RecordValue, key: string): string {
  const result = text(value, key);
  let parsed: URL;
  try { parsed = new URL(result); } catch { fail(`${key} must be an absolute URL`); }
  if (parsed.protocol !== "https:") fail(`${key} must use HTTPS`);
  return result;
}

function relativePath(value: RecordValue, key: string): string {
  const result = text(value, key);
  if (result.startsWith("/") || result.includes("\\") || result.split("/").some(part => part === "..")) {
    fail(`${key} must be a portable relative path`);
  }
  return result;
}

function positiveInteger(value: RecordValue, key: string): number {
  const result = value[key];
  if (typeof result !== "number" || !Number.isInteger(result) || result <= 0) {
    fail(`${key} must be a positive integer`);
  }
  return result;
}

function date(value: RecordValue, key: string): string {
  const result = text(value, key);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(result)) fail(`${key} must be YYYY-MM-DD`);
  return result;
}

function boolean(value: unknown, name: string): boolean {
  if (typeof value !== "boolean") fail(`${name} must be boolean`);
  return value;
}

function fail(message: string): never {
  throw new Error(`landing model: ${message}`);
}
