#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct LanguageCollection {
    pub languages: Vec<Language>,
    pub scripts: Vec<Script>,
    pub graphemes: Vec<Grapheme>,
    pub phonemes: Vec<Phoneme>,
    pub lexemes: Vec<Lexeme>,
    pub lexeme_links: Vec<LexemeLink>,
    pub lesson_nodes: Vec<LessonNode>,
    pub exercises: Vec<Exercise>,
    pub review_bindings: Vec<ReviewBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Language {
    pub id: String,
    pub name: String,
    pub autonym: String,
    pub iso_639_3: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum WritingDirection {
    LeftToRight,
    RightToLeft,
    TopToBottom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Script {
    pub id: String,
    pub language_id: String,
    pub name: String,
    pub direction: WritingDirection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Grapheme {
    pub id: String,
    pub script_id: String,
    pub symbol: String,
    pub transliteration: String,
    pub phoneme_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Phoneme {
    pub id: String,
    pub language_id: String,
    pub ipa: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Lexeme {
    pub id: String,
    pub language_id: String,
    pub lemma: String,
    pub transliteration: Option<String>,
    pub glosses: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum LexemeRelation {
    InheritedFrom,
    BorrowedFrom,
    DerivedFrom,
    CognateWith,
    RelatedTo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct LexemeLink {
    pub source_lexeme_id: String,
    pub target_lexeme_id: String,
    pub relation: LexemeRelation,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct LessonNode {
    pub id: String,
    pub language_id: String,
    pub title: String,
    pub concept_ids: Vec<String>,
    pub exercise_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum ExerciseKind {
    ScriptReading,
    Transliteration,
    VocabularyRecall,
    GrammarPattern,
    EtymologyStory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Exercise {
    pub id: String,
    pub language_id: String,
    pub kind: ExerciseKind,
    pub prompt: String,
    pub answer: String,
    pub item_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ReviewBinding {
    pub item_id: String,
    pub engram_card_id: String,
}

pub fn etymology_path(collection: &LanguageCollection, lexeme_id: &str) -> Vec<Lexeme> {
    let mut path = Vec::new();
    let mut current_id = lexeme_id.to_string();
    let mut seen = Vec::new();

    while !seen.contains(&current_id) {
        seen.push(current_id.clone());
        let Some(current) = find_lexeme(collection, &current_id) else {
            break;
        };
        path.push(current.clone());

        let Some(link) = collection.lexeme_links.iter().find(|link| {
            link.source_lexeme_id == current_id
                && matches!(
                    link.relation,
                    LexemeRelation::InheritedFrom
                        | LexemeRelation::BorrowedFrom
                        | LexemeRelation::DerivedFrom
                )
        }) else {
            break;
        };
        current_id = link.target_lexeme_id.clone();
    }

    path
}

pub fn lexemes_with_shared_ancestor(
    collection: &LanguageCollection,
    ancestor_lexeme_id: &str,
) -> Vec<Lexeme> {
    collection
        .lexeme_links
        .iter()
        .filter(|link| {
            link.target_lexeme_id == ancestor_lexeme_id
                && matches!(
                    link.relation,
                    LexemeRelation::InheritedFrom
                        | LexemeRelation::BorrowedFrom
                        | LexemeRelation::DerivedFrom
                )
        })
        .filter_map(|link| find_lexeme(collection, &link.source_lexeme_id).cloned())
        .collect()
}

pub fn review_card_ids_for_lesson(collection: &LanguageCollection, lesson_id: &str) -> Vec<String> {
    let Some(lesson) = collection
        .lesson_nodes
        .iter()
        .find(|lesson| lesson.id == lesson_id)
    else {
        return Vec::new();
    };

    collection
        .review_bindings
        .iter()
        .filter(|binding| lesson.concept_ids.contains(&binding.item_id))
        .map(|binding| binding.engram_card_id.clone())
        .collect()
}

fn find_lexeme<'a>(collection: &'a LanguageCollection, id: &str) -> Option<&'a Lexeme> {
    collection.lexemes.iter().find(|lexeme| lexeme.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lexeme(id: &str, language_id: &str, lemma: &str, gloss: &str) -> Lexeme {
        Lexeme {
            id: id.to_string(),
            language_id: language_id.to_string(),
            lemma: lemma.to_string(),
            transliteration: None,
            glosses: vec![gloss.to_string()],
            tags: Vec::new(),
        }
    }

    fn collection() -> LanguageCollection {
        LanguageCollection {
            languages: vec![
                Language {
                    id: "la".to_string(),
                    name: "Latin".to_string(),
                    autonym: "Latina".to_string(),
                    iso_639_3: Some("lat".to_string()),
                },
                Language {
                    id: "es".to_string(),
                    name: "Spanish".to_string(),
                    autonym: "Espanol".to_string(),
                    iso_639_3: Some("spa".to_string()),
                },
                Language {
                    id: "fr".to_string(),
                    name: "French".to_string(),
                    autonym: "francais".to_string(),
                    iso_639_3: Some("fra".to_string()),
                },
                Language {
                    id: "en".to_string(),
                    name: "English".to_string(),
                    autonym: "English".to_string(),
                    iso_639_3: Some("eng".to_string()),
                },
            ],
            scripts: vec![Script {
                id: "tamil-script".to_string(),
                language_id: "ta".to_string(),
                name: "Tamil".to_string(),
                direction: WritingDirection::LeftToRight,
            }],
            graphemes: vec![Grapheme {
                id: "tamil-a".to_string(),
                script_id: "tamil-script".to_string(),
                symbol: "a".to_string(),
                transliteration: "a".to_string(),
                phoneme_ids: vec!["ta-a".to_string()],
            }],
            phonemes: vec![Phoneme {
                id: "ta-a".to_string(),
                language_id: "ta".to_string(),
                ipa: "a".to_string(),
                description: "open front vowel".to_string(),
            }],
            lexemes: vec![
                lexeme("la-schola", "la", "schola", "school"),
                lexeme("es-escuela", "es", "escuela", "school"),
                lexeme("fr-ecole", "fr", "ecole", "school"),
                lexeme("en-school", "en", "school", "school"),
            ],
            lexeme_links: vec![
                LexemeLink {
                    source_lexeme_id: "es-escuela".to_string(),
                    target_lexeme_id: "la-schola".to_string(),
                    relation: LexemeRelation::InheritedFrom,
                    note: "Spanish inherited the word through Vulgar Latin.".to_string(),
                },
                LexemeLink {
                    source_lexeme_id: "fr-ecole".to_string(),
                    target_lexeme_id: "la-schola".to_string(),
                    relation: LexemeRelation::InheritedFrom,
                    note: "French inherited the word through Old French.".to_string(),
                },
                LexemeLink {
                    source_lexeme_id: "en-school".to_string(),
                    target_lexeme_id: "la-schola".to_string(),
                    relation: LexemeRelation::BorrowedFrom,
                    note: "English borrowed the word through Germanic and Latin contact."
                        .to_string(),
                },
            ],
            lesson_nodes: vec![LessonNode {
                id: "spanish-school-story".to_string(),
                language_id: "es".to_string(),
                title: "Escuela and its cousins".to_string(),
                concept_ids: vec!["es-escuela".to_string(), "la-schola".to_string()],
                exercise_ids: vec!["exercise-1".to_string()],
            }],
            exercises: vec![Exercise {
                id: "exercise-1".to_string(),
                language_id: "es".to_string(),
                kind: ExerciseKind::EtymologyStory,
                prompt: "Trace escuela back to Latin.".to_string(),
                answer: "escuela comes from Latin schola.".to_string(),
                item_ids: vec!["es-escuela".to_string(), "la-schola".to_string()],
            }],
            review_bindings: vec![
                ReviewBinding {
                    item_id: "es-escuela".to_string(),
                    engram_card_id: "card-es-escuela".to_string(),
                },
                ReviewBinding {
                    item_id: "la-schola".to_string(),
                    engram_card_id: "card-la-schola".to_string(),
                },
            ],
        }
    }

    #[test]
    fn etymology_path_traces_to_ancestor() {
        let path = etymology_path(&collection(), "es-escuela");
        let ids: Vec<_> = path.iter().map(|lexeme| lexeme.id.as_str()).collect();

        assert_eq!(ids, vec!["es-escuela", "la-schola"]);
    }

    #[test]
    fn shared_ancestor_finds_cognate_story_candidates() {
        let cousins = lexemes_with_shared_ancestor(&collection(), "la-schola");
        let ids: Vec<_> = cousins.iter().map(|lexeme| lexeme.id.as_str()).collect();

        assert_eq!(ids, vec!["es-escuela", "fr-ecole", "en-school"]);
    }

    #[test]
    fn lesson_review_bindings_return_engram_cards() {
        let cards = review_card_ids_for_lesson(&collection(), "spanish-school-story");

        assert_eq!(cards, vec!["card-es-escuela", "card-la-schola"]);
    }
}
