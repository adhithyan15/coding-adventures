// @coding-adventures/human-language-data
//
// The machine-readable bridge from the Human Languages curriculum (Markdown
// lessons + concept taxonomy) to the cross-language dataset the Engram deck
// generator and companion app consume. See code/specs/HL01-*.

export * from "./types.js";
export * from "./constants.js";
export { splitFrontmatter, type Frontmatter } from "./frontmatter.js";
export { parseLesson, buildDataset, type ParsedLesson } from "./parse.js";
export {
  allConcepts,
  conceptsByLanguage,
  languagesForConcept,
  coverageByLanguage,
} from "./queries.js";
export { validate, hasErrors, summarize, type ValidateInput } from "./validate.js";
export {
  validateCurriculum,
  type CurriculumValidationInput,
} from "./curriculum.js";
export {
  defaultCurriculumRoot,
  trackScript,
  loadTaxonomy,
  loadLanguageRegistry,
  loadCurriculumSpine,
  loadBookCorpus,
  loadLessons,
  loadScripts,
  loadEverything,
} from "./loader.js";
export { runValidate } from "./cli.js";
