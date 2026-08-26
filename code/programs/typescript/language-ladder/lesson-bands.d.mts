// Types for lesson-bands.mjs, which is plain JavaScript so that both the
// TypeScript Vite config and a bare `node scripts/*.mjs` can import it.
export declare const LESSON_BAND_CHAPTERS: number;
export declare const MAX_CHAPTER_DIGITS: number;

export interface LessonBand {
  track: string;
  series: string;
  band: number;
}

export declare function lessonBand(track: string, filename: string): LessonBand | null;
export declare function bandChunkName(band: LessonBand): string;
export declare function bandChunkNameForModuleId(moduleId: string): string | null;
