declare module "virtual:human-language-ledgers" {
  export const spine: unknown;
  export const curriculumLoaders: Record<string, () => Promise<unknown>>;
  export const chapterLoaders: Record<string, () => Promise<unknown>>;
}
