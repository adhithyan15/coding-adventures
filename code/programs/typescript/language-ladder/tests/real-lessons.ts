// Shared asynchronous fixture for tests that exercise the complete authored
// curriculum. Production keeps the corpus lazy; the test suite opts in here.
import { loadBundledLessons } from "../src/lessons.ts";

export const REAL_LESSONS = await loadBundledLessons();
