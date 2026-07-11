use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use crate::model::{
    AppState, Card, CardFlag, CardProgress, CardState, Deck, ExternalSourceRecord,
    ExternalSourceTarget, Note, NoteType, Rating, Review,
};
use crate::queue::{is_new_progress_overlay, is_reviewable};
use crate::sm2::ONE_DAY_MS as MS_PER_DAY;
// Engram's search matching runs entirely on the zero-dependency `regex_engine`
// (Pike VM, linear-time): the boolean uses — user `re:` patterns, whole-word,
// and `*`/`_` globs — plus the one place a match *extent* is needed, the
// media-tag `replace_all` below. No third-party regex crate remains in the
// runtime dependency graph (`regex` is now a dev-dependency, used only by the
// `html_scan` cross-check test).
use regex_engine::{Regex, RegexBuilder};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use serde_json::Value;
use unicode_normalize::{char::is_combining_mark, UnicodeNormalize};

static DUPLICATE_HTML_MEDIA_TAGS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?is)<(?:img|audio|video|object|source)[^>]*(?:src|data)\s*=\s*(?:"([^"]+)"|'([^']+)'|([^ >]+))[^>]*>"#,
    )
    .expect("duplicate media tag regex should compile")
});

const FSRS5_DEFAULT_DECAY: f64 = 0.5;
const SECONDS_PER_DAY: i64 = 86_400;
const ANKI_TYPE_NEW: i64 = 0;
const ANKI_TYPE_LEARN: i64 = 1;
const ANKI_TYPE_REVIEW: i64 = 2;
const ANKI_TYPE_RELEARN: i64 = 3;
const ANKI_QUEUE_SCHED_BURIED: i64 = -3;
const ANKI_QUEUE_USER_BURIED: i64 = -2;
const ANKI_QUEUE_SUSPENDED: i64 = -1;
const ANKI_QUEUE_LEARN: i64 = 1;
const ANKI_QUEUE_REVIEW: i64 = 2;
const ANKI_QUEUE_DAY_LEARN: i64 = 3;
const ANKI_QUEUE_PREVIEW_REPEAT: i64 = 4;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CardSearchResult {
    pub card: Card,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub progress: Option<CardProgress>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SearchError {
    pub message: String,
    pub token: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SearchContext<'a> {
    pub current_deck_id: Option<&'a str>,
    pub day_start_offset_ms: Option<u64>,
    pub learn_ahead_secs: Option<u32>,
}

#[derive(Clone, Debug)]
struct SearchClause {
    kind: SearchClauseKind,
    negated: bool,
}

#[derive(Clone, Debug)]
enum SearchClauseKind {
    Text(TextFilter),
    Field(FieldFilter),
    CardId(IdFilter),
    NoteId(IdFilter),
    DeckId(IdFilter),
    NoteTypeId(IdFilter),
    Duplicate(DuplicateFilter),
    CardTemplate(String),
    Deck(String),
    CurrentDeck,
    Preset(String),
    NoteType(String),
    Tag(TagFilter),
    State(CardSearchState),
    Flag(FlagFilter),
    Marked(bool),
    Property(CardPropertyFilter),
    CustomDataKey(String),
    CustomDataNumeric(CardCustomDataNumericFilter),
    CustomDataString(CardCustomDataStringFilter),
    Added(RecentDaysFilter),
    Edited(RecentDaysFilter),
    Introduced(RecentDaysFilter),
    Rated(RatedFilter),
    Rescheduled(RecentDaysFilter),
}

#[derive(Clone, Debug)]
enum SearchExpr {
    Clause(SearchClause),
    And(Vec<SearchExpr>),
    Or(Vec<SearchExpr>),
    Not(Box<SearchExpr>),
}

#[derive(Clone, Debug)]
struct TextFilter {
    pattern: String,
    mode: TextMatchMode,
    regex: Option<Regex>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextMatchMode {
    Contains,
    WholeWord,
    NoCombining,
    StripCloze,
    Regex,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CardSearchState {
    New,
    Learning,
    Review,
    Relearning,
    Due,
    Suspended,
    Buried,
    BuriedManually,
    BuriedSibling,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FlagFilter {
    Any,
    None,
    Color(CardFlag),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ComparisonOperator {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CardProperty {
    Interval,
    Due,
    Repetitions,
    Lapses,
    Ease,
    Rated,
    Rescheduled,
    Position,
    Stability,
    Difficulty,
    Retrievability,
}

#[derive(Clone, Debug, PartialEq)]
struct CardPropertyFilter {
    property: CardProperty,
    operator: ComparisonOperator,
    value: f64,
    rating: Option<Rating>,
}

#[derive(Clone, Debug, PartialEq)]
struct CardCustomDataNumericFilter {
    key: String,
    operator: ComparisonOperator,
    value: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CardCustomDataStringFilter {
    key: String,
    operator: ComparisonOperator,
    value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DuplicateFilter {
    note_type_id: IdFilter,
    first_field: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RecentDaysFilter {
    days: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RatedFilter {
    days: u32,
    rating: Option<Rating>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IdFilter {
    values: Vec<String>,
}

#[derive(Clone, Debug)]
struct FieldFilter {
    name_pattern: String,
    value_pattern: FieldValuePattern,
}

#[derive(Clone, Debug)]
enum FieldValuePattern {
    Any,
    NonEmpty,
    Exact(String),
    Wildcard(String),
    Text(TextFilter),
}

#[derive(Clone, Debug)]
enum TagFilter {
    Hierarchical(String),
    NoCombining(String),
    Regex(TextFilter),
}

#[derive(Default)]
struct SearchMetadata<'a> {
    current_deck_id: Option<String>,
    filtered_deck_ids: HashSet<&'a str>,
    card_sources_by_id: HashMap<&'a str, Vec<&'a ExternalSourceRecord>>,
    note_sources_by_id: HashMap<&'a str, Vec<&'a ExternalSourceRecord>>,
    review_sources_by_id: HashMap<&'a str, Vec<&'a ExternalSourceRecord>>,
    decks_by_id: HashMap<&'a str, &'a Deck>,
    deck_original_ids_by_id: HashMap<&'a str, Vec<&'a str>>,
    decks_by_original_id: HashMap<&'a str, Vec<&'a Deck>>,
    note_type_original_ids_by_id: HashMap<&'a str, Vec<&'a str>>,
    excluded_field_ids_by_note_type_id: HashMap<&'a str, HashSet<String>>,
    deck_preset_names_by_id: HashMap<&'a str, Vec<String>>,
    deck_option_deck_ids: HashSet<&'a str>,
    collection_created_at_days: Option<i64>,
    day_start_offset_ms: u64,
    learn_ahead_secs: i64,
}

pub fn search_cards(
    state: &AppState,
    query: &str,
    now: u64,
) -> Result<Vec<CardSearchResult>, SearchError> {
    search_cards_with_context(state, query, now, SearchContext::default())
}

pub fn search_cards_with_context(
    state: &AppState,
    query: &str,
    now: u64,
    context: SearchContext<'_>,
) -> Result<Vec<CardSearchResult>, SearchError> {
    let expression = parse_query(query)?;
    let progress_by_card: HashMap<&str, &CardProgress> = state
        .card_progress
        .iter()
        .map(|progress| (progress.card_id.as_str(), progress))
        .collect();
    let decks_by_id: HashMap<&str, &Deck> = state
        .decks
        .iter()
        .map(|deck| (deck.id.as_str(), deck))
        .collect();
    let notes_by_id: HashMap<&str, &Note> = state
        .notes
        .iter()
        .map(|note| (note.id.as_str(), note))
        .collect();
    let note_types_by_id: HashMap<&str, &NoteType> = state
        .note_types
        .iter()
        .map(|note_type| (note_type.id.as_str(), note_type))
        .collect();
    let metadata = SearchMetadata::from_state_with_context(state, context);
    let mut reviews_by_card: HashMap<&str, Vec<&Review>> = HashMap::new();
    for review in &state.reviews {
        reviews_by_card
            .entry(review.card_id.as_str())
            .or_default()
            .push(review);
    }

    let results = state
        .cards
        .iter()
        .filter(|card| {
            let progress = progress_by_card.get(card.id.as_str()).copied();
            let deck = decks_by_id.get(card.deck_id.as_str()).copied();
            let note = note_for_card(card, &notes_by_id);
            let note_type = note
                .and_then(|note| note_types_by_id.get(note.note_type_id.as_str()))
                .copied();
            let reviews = reviews_by_card
                .get(card.id.as_str())
                .map_or(&[] as &[&Review], Vec::as_slice);
            expression_matches(
                &expression,
                card,
                progress,
                deck,
                note,
                note_type,
                &metadata,
                reviews,
                now,
            )
        })
        .map(|card| CardSearchResult {
            card: card.clone(),
            progress: progress_by_card
                .get(card.id.as_str())
                .map(|item| (*item).clone()),
        })
        .collect();

    Ok(results)
}

impl<'a> SearchMetadata<'a> {
    fn from_state_with_context(state: &'a AppState, context: SearchContext<'_>) -> Self {
        let deck_config_names = anki_deck_config_names_by_id(state);
        let decks_by_id: HashMap<&str, &Deck> = state
            .decks
            .iter()
            .map(|deck| (deck.id.as_str(), deck))
            .collect();
        let current_deck_id = context
            .current_deck_id
            .filter(|deck_id| !deck_id.is_empty())
            .map(str::to_string)
            .or_else(|| {
                state
                    .active_session
                    .as_ref()
                    .map(|active| active.deck_id.clone())
            });
        let day_start_offset_ms = context.day_start_offset_ms.unwrap_or_default() % MS_PER_DAY;
        let mut metadata = Self {
            current_deck_id,
            decks_by_id: decks_by_id.clone(),
            day_start_offset_ms,
            learn_ahead_secs: i64::from(context.learn_ahead_secs.unwrap_or_default()),
            deck_option_deck_ids: state
                .deck_options
                .iter()
                .map(|preset| preset.deck_id.as_str())
                .collect(),
            ..Self::default()
        };

        for source in &state.external_sources {
            match source.target {
                ExternalSourceTarget::Collection => {
                    metadata.collection_created_at_days =
                        source_i64_from_data(source, "createdAtDays");
                }
                ExternalSourceTarget::Deck => {
                    if let Some(original_id) = source.original_id.as_deref() {
                        metadata
                            .deck_original_ids_by_id
                            .entry(source.target_id.as_str())
                            .or_default()
                            .push(original_id);
                        if let Some(deck) = decks_by_id.get(source.target_id.as_str()) {
                            metadata
                                .decks_by_original_id
                                .entry(original_id)
                                .or_default()
                                .push(*deck);
                        }
                    }

                    if source
                        .data
                        .get("dyn")
                        .is_some_and(|value| value.parse::<i64>().unwrap_or(0) != 0)
                    {
                        metadata.filtered_deck_ids.insert(source.target_id.as_str());
                    }

                    let names = anki_deck_preset_names(source, &deck_config_names);
                    if !names.is_empty() {
                        metadata
                            .deck_preset_names_by_id
                            .entry(source.target_id.as_str())
                            .or_default()
                            .extend(names);
                    }
                }
                ExternalSourceTarget::NoteType => {
                    if let Some(original_id) = source.original_id.as_deref() {
                        metadata
                            .note_type_original_ids_by_id
                            .entry(source.target_id.as_str())
                            .or_default()
                            .push(original_id);
                    }
                    let excluded = anki_excluded_field_ids(source);
                    if !excluded.is_empty() {
                        metadata
                            .excluded_field_ids_by_note_type_id
                            .entry(source.target_id.as_str())
                            .or_default()
                            .extend(excluded);
                    }
                }
                ExternalSourceTarget::Card => {
                    metadata
                        .card_sources_by_id
                        .entry(source.target_id.as_str())
                        .or_default()
                        .push(source);
                }
                ExternalSourceTarget::Note => {
                    metadata
                        .note_sources_by_id
                        .entry(source.target_id.as_str())
                        .or_default()
                        .push(source);
                }
                ExternalSourceTarget::Review => {
                    metadata
                        .review_sources_by_id
                        .entry(source.target_id.as_str())
                        .or_default()
                        .push(source);
                }
                _ => {}
            }
        }

        metadata
    }
}

fn parse_query(query: &str) -> Result<SearchExpr, SearchError> {
    let tokens = tokenize(query)?;
    let mut parser = SearchParser::new(tokens);
    let expression = parser.parse_or()?;

    if let Some(token) = parser.peek() {
        return Err(SearchError {
            message: if token == ")" {
                "unexpected closing parenthesis".to_string()
            } else {
                "unexpected search token".to_string()
            },
            token: token.to_string(),
        });
    }

    Ok(expression)
}

fn tokenize(query: &str) -> Result<Vec<String>, SearchError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escaping = false;
    let mut quote_start_len = 0usize;

    for ch in query.chars() {
        if escaping {
            current.push(ch);
            escaping = false;
            continue;
        }

        match ch {
            '\\' => {
                current.push(ch);
                escaping = true;
            }
            '"' => {
                if in_quotes {
                    if current.len() == quote_start_len {
                        return Err(SearchError {
                            message: "empty quoted string".to_string(),
                            token: query.to_string(),
                        });
                    }
                    in_quotes = false;
                } else {
                    in_quotes = true;
                    quote_start_len = current.len();
                }
            }
            '(' | ')' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(current);
                    current = String::new();
                }
                tokens.push(ch.to_string());
            }
            ch if ch.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(current);
                    current = String::new();
                }
            }
            _ => current.push(ch),
        }
    }

    if in_quotes {
        return Err(SearchError {
            message: "unterminated quoted string".to_string(),
            token: query.to_string(),
        });
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

struct SearchParser {
    tokens: Vec<String>,
    position: usize,
}

impl SearchParser {
    fn new(tokens: Vec<String>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    fn parse_or(&mut self) -> Result<SearchExpr, SearchError> {
        let left = self.parse_and()?;
        if self
            .peek()
            .is_some_and(|token| token.eq_ignore_ascii_case("or"))
            && expression_is_empty(&left)
        {
            let token = self.next().expect("peeked token exists");
            return Err(SearchError {
                message: "OR operator is missing a left-hand clause".to_string(),
                token,
            });
        }

        let mut expressions = vec![left];
        while self
            .peek()
            .is_some_and(|token| token.eq_ignore_ascii_case("or"))
        {
            let operator = self.next().expect("peeked token exists");
            let right = self.parse_and()?;
            if expression_is_empty(&right) {
                return Err(SearchError {
                    message: "OR operator is missing a right-hand clause".to_string(),
                    token: operator,
                });
            }
            expressions.push(right);
        }

        Ok(fold_or(expressions))
    }

    fn parse_and(&mut self) -> Result<SearchExpr, SearchError> {
        let mut expressions = Vec::new();

        loop {
            let Some(token) = self.peek() else {
                break;
            };
            if token == ")" || token.eq_ignore_ascii_case("or") {
                break;
            }
            if token.eq_ignore_ascii_case("and") {
                let operator = self.next().expect("peeked token exists");
                if expressions.is_empty() {
                    return Err(SearchError {
                        message: "AND operator is missing a left-hand clause".to_string(),
                        token: operator,
                    });
                }
                if self.peek().is_none_or(|token| {
                    token == ")"
                        || token.eq_ignore_ascii_case("or")
                        || token.eq_ignore_ascii_case("and")
                }) {
                    return Err(SearchError {
                        message: "AND operator is missing a right-hand clause".to_string(),
                        token: "AND".to_string(),
                    });
                }
                continue;
            }

            expressions.push(self.parse_unary()?);
        }

        Ok(fold_and(expressions))
    }

    fn parse_unary(&mut self) -> Result<SearchExpr, SearchError> {
        if self.peek().is_some_and(|token| token == "-") {
            let operator = self.next().expect("peeked token exists");
            if self.peek().is_none_or(|token| {
                token == ")"
                    || token.eq_ignore_ascii_case("or")
                    || token.eq_ignore_ascii_case("and")
            }) {
                return Err(SearchError {
                    message: "negation is missing a clause".to_string(),
                    token: operator,
                });
            }
            return Ok(SearchExpr::Not(Box::new(self.parse_unary()?)));
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<SearchExpr, SearchError> {
        let token = self.next().ok_or_else(|| SearchError {
            message: "search expression is missing a clause".to_string(),
            token: String::new(),
        })?;

        if token == "(" {
            let expression = self.parse_or()?;
            if expression_is_empty(&expression) {
                return Err(SearchError {
                    message: "parenthesized search expression is empty".to_string(),
                    token,
                });
            }
            match self.next() {
                Some(closing) if closing == ")" => Ok(expression),
                Some(unexpected) => Err(SearchError {
                    message: "expected closing parenthesis".to_string(),
                    token: unexpected,
                }),
                None => Err(SearchError {
                    message: "missing closing parenthesis".to_string(),
                    token,
                }),
            }
        } else if token == ")" {
            Err(SearchError {
                message: "unexpected closing parenthesis".to_string(),
                token,
            })
        } else {
            parse_clause(&token).map(SearchExpr::Clause)
        }
    }

    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.position).map(String::as_str)
    }

    fn next(&mut self) -> Option<String> {
        let token = self.tokens.get(self.position).cloned()?;
        self.position += 1;
        Some(token)
    }
}

fn fold_and(expressions: Vec<SearchExpr>) -> SearchExpr {
    match expressions.len() {
        0 => SearchExpr::And(expressions),
        1 => expressions
            .into_iter()
            .next()
            .expect("one expression exists"),
        _ => SearchExpr::And(expressions),
    }
}

fn fold_or(expressions: Vec<SearchExpr>) -> SearchExpr {
    match expressions.len() {
        0 => SearchExpr::Or(expressions),
        1 => expressions
            .into_iter()
            .next()
            .expect("one expression exists"),
        _ => SearchExpr::Or(expressions),
    }
}

fn expression_is_empty(expression: &SearchExpr) -> bool {
    matches!(expression, SearchExpr::And(items) if items.is_empty())
}

fn parse_clause(token: &str) -> Result<SearchClause, SearchError> {
    let (negated, raw) = token
        .strip_prefix('-')
        .filter(|stripped| !stripped.is_empty())
        .map_or((false, token), |stripped| (true, stripped));

    let kind = match split_once_unescaped(raw, ':') {
        Some((key, value)) => parse_keyed_clause(raw, key, value)?,
        None => SearchClauseKind::Text(text_filter_contains(&unescape_search_pattern(token, raw)?)),
    };

    Ok(SearchClause { kind, negated })
}

fn split_once_unescaped(value: &str, delimiter: char) -> Option<(&str, &str)> {
    let mut escaped = false;
    for (index, ch) in value.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == delimiter {
            return Some((&value[..index], &value[index + ch.len_utf8()..]));
        }
    }
    None
}

fn parse_keyed_clause(
    token: &str,
    key: &str,
    value: &str,
) -> Result<SearchClauseKind, SearchError> {
    let key = unescape_search_pattern(token, key)?.to_ascii_lowercase();
    if is_field_search_key(&key) {
        return parse_field_filter(token, &key, value).map(SearchClauseKind::Field);
    }

    match key.as_str() {
        "w" | "nc" | "sc" | "re" => {
            parse_text_filter(token, &key, value).map(SearchClauseKind::Text)
        }
        "cid" | "cardid" | "card_id" | "card-id" => {
            let value = standard_search_fold(value);
            parse_id_filter(token, &value).map(SearchClauseKind::CardId)
        }
        "nid" | "noteid" | "note_id" | "note-id" => {
            let value = standard_search_fold(value);
            parse_id_filter(token, &value).map(SearchClauseKind::NoteId)
        }
        "did" | "deckid" | "deck_id" | "deck-id" => {
            let value = standard_search_fold(value);
            parse_id_filter(token, &value).map(SearchClauseKind::DeckId)
        }
        "mid" | "notetypeid" | "note_type_id" | "note-type-id" => {
            let value = standard_search_fold(value);
            parse_id_filter(token, &value).map(SearchClauseKind::NoteTypeId)
        }
        "dupe" => parse_duplicate_filter(token, value).map(SearchClauseKind::Duplicate),
        "card" | "template" | "cardtemplate" | "card_template" | "card-template" => {
            require_keyed_filter_value(token, value)?;
            Ok(SearchClauseKind::CardTemplate(standard_search_fold(
                &unescape_search_pattern(token, value)?,
            )))
        }
        "deck" => {
            require_keyed_filter_value(token, value)?;
            let term = standard_search_fold(&unescape_search_pattern(token, value)?);
            if term == "current" {
                Ok(SearchClauseKind::CurrentDeck)
            } else {
                Ok(SearchClauseKind::Deck(term))
            }
        }
        "preset" => {
            require_keyed_filter_value(token, value)?;
            Ok(SearchClauseKind::Preset(standard_search_fold(
                &unescape_search_pattern(token, value)?,
            )))
        }
        "note" | "notetype" | "note_type" | "note-type" => {
            require_keyed_filter_value(token, value)?;
            Ok(SearchClauseKind::NoteType(standard_search_fold(
                &unescape_search_pattern(token, value)?,
            )))
        }
        "tag" => {
            require_keyed_filter_value(token, value)?;
            parse_tag_filter(token, value).map(SearchClauseKind::Tag)
        }
        "state" => parse_state_filter(token, &value.to_lowercase()).map(SearchClauseKind::State),
        "is" => parse_is_filter(token, &value.to_lowercase()),
        "flag" => parse_flag_filter(token, &value.to_lowercase()).map(SearchClauseKind::Flag),
        "marked" => parse_bool_filter(token, &value.to_lowercase()).map(SearchClauseKind::Marked),
        "has-cd" | "has_cd" | "hascd" => {
            require_keyed_filter_value(token, value)?;
            Ok(SearchClauseKind::CustomDataKey(unescape_search_pattern(
                token, value,
            )?))
        }
        "prop" => parse_property_filter(token, value),
        "added" => {
            parse_recent_days_filter(token, &value.to_lowercase()).map(SearchClauseKind::Added)
        }
        "edited" => {
            parse_recent_days_filter(token, &value.to_lowercase()).map(SearchClauseKind::Edited)
        }
        "introduced" => {
            parse_recent_days_filter(token, &value.to_lowercase()).map(SearchClauseKind::Introduced)
        }
        "rated" => parse_rated_filter(token, &value.to_lowercase()).map(SearchClauseKind::Rated),
        "resched" | "rescheduled" => parse_recent_days_filter(token, &value.to_lowercase())
            .map(SearchClauseKind::Rescheduled),
        _ => parse_field_filter(token, &key, value).map(SearchClauseKind::Field),
    }
}

fn is_field_search_key(key: &str) -> bool {
    matches!(key, "front" | "back") || contains_search_wildcard(key) || key.contains(' ')
}

fn require_keyed_filter_value(token: &str, value: &str) -> Result<(), SearchError> {
    if value.is_empty() {
        return Err(SearchError {
            message: "search filter is missing a value".to_string(),
            token: token.to_string(),
        });
    }

    Ok(())
}

fn parse_field_filter(token: &str, key: &str, value: &str) -> Result<FieldFilter, SearchError> {
    let value_pattern = if let Some((mode, pattern)) = split_text_modifier(value) {
        FieldValuePattern::Text(parse_text_filter(token, mode, pattern)?)
    } else {
        parse_exact_field_value_filter(token, value)?
    };

    Ok(FieldFilter {
        name_pattern: key.to_string(),
        value_pattern,
    })
}

fn parse_exact_field_value_filter(
    token: &str,
    value: &str,
) -> Result<FieldValuePattern, SearchError> {
    let value = standard_search_fold(&unescape_search_pattern(token, value)?);
    match value.as_str() {
        "*" if contains_search_wildcard(&value) => Ok(FieldValuePattern::Any),
        "_*" if contains_search_wildcard(&value) => Ok(FieldValuePattern::NonEmpty),
        _ if contains_search_wildcard(&value) => Ok(FieldValuePattern::Wildcard(value)),
        _ => Ok(FieldValuePattern::Exact(search_literal_text(&value))),
    }
}

fn parse_tag_filter(token: &str, value: &str) -> Result<TagFilter, SearchError> {
    match split_text_modifier(value) {
        Some(("re", pattern)) => parse_text_filter(token, "re", pattern).map(TagFilter::Regex),
        Some(("nc", pattern)) => Ok(TagFilter::NoCombining(unescape_search_pattern(
            token, pattern,
        )?)),
        _ => Ok(TagFilter::Hierarchical(standard_search_fold(
            &unescape_search_pattern(token, value)?,
        ))),
    }
}

fn text_filter_contains(pattern: &str) -> TextFilter {
    TextFilter {
        pattern: pattern.to_string(),
        mode: TextMatchMode::Contains,
        regex: None,
    }
}

fn standard_search_fold(value: &str) -> String {
    value.to_ascii_lowercase()
}

fn parse_text_filter(token: &str, mode: &str, pattern: &str) -> Result<TextFilter, SearchError> {
    if pattern.is_empty() {
        return Err(SearchError {
            message: "text search modifier is missing a value".to_string(),
            token: token.to_string(),
        });
    }

    let mode = match mode {
        "w" => TextMatchMode::WholeWord,
        "nc" => TextMatchMode::NoCombining,
        "sc" => TextMatchMode::StripCloze,
        "re" => TextMatchMode::Regex,
        _ => unreachable!("validated search modifier"),
    };
    let pattern = match mode {
        TextMatchMode::Regex => pattern.to_string(),
        _ => unescape_search_pattern(token, pattern)?,
    };
    let regex = match mode {
        TextMatchMode::WholeWord => Some(build_whole_word_regex(token, &pattern)?),
        TextMatchMode::Regex => Some(build_search_regex(token, &pattern)?),
        TextMatchMode::Contains | TextMatchMode::NoCombining | TextMatchMode::StripCloze => None,
    };

    Ok(TextFilter {
        pattern,
        mode,
        regex,
    })
}

fn split_text_modifier(value: &str) -> Option<(&'static str, &str)> {
    let (mode, pattern) = split_once_unescaped(value, ':')?;
    let mode = match mode.to_ascii_lowercase().as_str() {
        "w" => "w",
        "nc" => "nc",
        "sc" => "sc",
        "re" => "re",
        _ => return None,
    };
    Some((mode, pattern))
}

fn build_search_regex(token: &str, pattern: &str) -> Result<Regex, SearchError> {
    RegexBuilder::new(pattern)
        .case_insensitive(true)
        .build()
        .map_err(|error| SearchError {
            message: format!("invalid regular expression: {error}"),
            token: token.to_string(),
        })
}

fn build_whole_word_regex(token: &str, pattern: &str) -> Result<Regex, SearchError> {
    let source = format!(
        "(?u)(?:^|[^\\p{{Alphabetic}}\\p{{Mark}}\\p{{Nd}}_]){}(?:$|[^\\p{{Alphabetic}}\\p{{Mark}}\\p{{Nd}}_])",
        search_pattern_regex_source(pattern, WildcardScope::Word)
    );
    RegexBuilder::new(&source)
        .case_insensitive(true)
        .build()
        .map_err(|error| SearchError {
            message: format!("invalid whole-word search pattern: {error}"),
            token: token.to_string(),
        })
}

#[derive(Clone, Copy)]
enum WildcardScope {
    Text,
    Word,
}

const ESCAPED_BACKSLASH: char = '\u{E000}';

fn search_pattern_regex_source(pattern: &str, scope: WildcardScope) -> String {
    let mut source = String::new();
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == ESCAPED_BACKSLASH {
            source.push_str(r"\\");
            continue;
        }
        if ch == '\\' {
            match chars.peek().copied() {
                Some('*' | '_') => {
                    let escaped = chars.next().expect("peeked wildcard exists");
                    source.push_str(&regex_engine::escape(&escaped.to_string()));
                }
                _ => source.push_str(&regex_engine::escape(&ch.to_string())),
            }
            continue;
        }

        match ch {
            '*' => match scope {
                WildcardScope::Text => source.push_str(".*"),
                WildcardScope::Word => source.push_str("[\\p{Alphabetic}\\p{Mark}\\p{Nd}_]*"),
            },
            '_' => match scope {
                WildcardScope::Text => source.push('.'),
                WildcardScope::Word => source.push_str("[\\p{Alphabetic}\\p{Mark}\\p{Nd}_]"),
            },
            _ => source.push_str(&regex_engine::escape(&ch.to_string())),
        }
    }
    source
}

fn parse_is_filter(token: &str, value: &str) -> Result<SearchClauseKind, SearchError> {
    match value {
        "marked" => Ok(SearchClauseKind::Marked(true)),
        "unmarked" => Ok(SearchClauseKind::Marked(false)),
        "flagged" => Ok(SearchClauseKind::Flag(FlagFilter::Any)),
        "unflagged" => Ok(SearchClauseKind::Flag(FlagFilter::None)),
        _ => parse_state_filter(token, value).map(SearchClauseKind::State),
    }
}

fn parse_state_filter(token: &str, value: &str) -> Result<CardSearchState, SearchError> {
    match value {
        "new" => Ok(CardSearchState::New),
        "learn" | "learning" => Ok(CardSearchState::Learning),
        "review" => Ok(CardSearchState::Review),
        "relearn" | "relearning" => Ok(CardSearchState::Relearning),
        "due" => Ok(CardSearchState::Due),
        "suspended" => Ok(CardSearchState::Suspended),
        "buried" => Ok(CardSearchState::Buried),
        "buried-manually" | "buried_manually" => Ok(CardSearchState::BuriedManually),
        "buried-sibling" | "buried_sibling" => Ok(CardSearchState::BuriedSibling),
        _ => Err(SearchError {
            message: "unknown card state filter".to_string(),
            token: token.to_string(),
        }),
    }
}

fn parse_flag_filter(token: &str, value: &str) -> Result<FlagFilter, SearchError> {
    match value {
        "any" | "flagged" => Ok(FlagFilter::Any),
        "0" | "none" | "unflagged" => Ok(FlagFilter::None),
        "1" | "red" => Ok(FlagFilter::Color(CardFlag::Red)),
        "2" | "orange" => Ok(FlagFilter::Color(CardFlag::Orange)),
        "3" | "green" => Ok(FlagFilter::Color(CardFlag::Green)),
        "4" | "blue" => Ok(FlagFilter::Color(CardFlag::Blue)),
        "5" | "pink" => Ok(FlagFilter::Color(CardFlag::Pink)),
        "6" | "turquoise" => Ok(FlagFilter::Color(CardFlag::Turquoise)),
        "7" | "purple" => Ok(FlagFilter::Color(CardFlag::Purple)),
        _ => Err(SearchError {
            message: "unknown card flag filter".to_string(),
            token: token.to_string(),
        }),
    }
}

fn parse_bool_filter(token: &str, value: &str) -> Result<bool, SearchError> {
    match value {
        "true" | "yes" | "1" => Ok(true),
        "false" | "no" | "0" => Ok(false),
        _ => Err(SearchError {
            message: "boolean search filter must be true or false".to_string(),
            token: token.to_string(),
        }),
    }
}

fn parse_property_filter(token: &str, value: &str) -> Result<SearchClauseKind, SearchError> {
    if let Some(filter) = parse_custom_data_numeric_filter(token, value)? {
        return Ok(SearchClauseKind::CustomDataNumeric(filter));
    }
    if let Some(filter) = parse_custom_data_string_filter(token, value)? {
        return Ok(SearchClauseKind::CustomDataString(filter));
    }

    let Some((property, operator, expected)) = split_property_comparison(value) else {
        return Err(SearchError {
            message: "property search must include a comparison operator".to_string(),
            token: token.to_string(),
        });
    };
    if property.is_empty() || expected.is_empty() {
        return Err(SearchError {
            message: "property search is missing a property or value".to_string(),
            token: token.to_string(),
        });
    }

    let property = match property.to_ascii_lowercase().as_str() {
        "ivl" | "interval" => CardProperty::Interval,
        "due" => CardProperty::Due,
        "reps" | "reviews" => CardProperty::Repetitions,
        "lapses" => CardProperty::Lapses,
        "ease" => CardProperty::Ease,
        "rated" => CardProperty::Rated,
        "resched" | "rescheduled" => CardProperty::Rescheduled,
        "pos" | "position" => CardProperty::Position,
        "s" | "stability" => CardProperty::Stability,
        "d" | "difficulty" => CardProperty::Difficulty,
        "r" | "retrievability" => CardProperty::Retrievability,
        _ => {
            return Err(SearchError {
                message: "unknown card property filter".to_string(),
                token: token.to_string(),
            });
        }
    };
    let (expected, rating) = parse_property_rating_suffix(token, property, expected)?;
    let value = expected.parse::<f64>().map_err(|_| SearchError {
        message: "property search value must be numeric".to_string(),
        token: token.to_string(),
    })?;

    Ok(SearchClauseKind::Property(CardPropertyFilter {
        property,
        operator,
        value,
        rating,
    }))
}

fn parse_property_rating_suffix<'a>(
    token: &str,
    property: CardProperty,
    expected: &'a str,
) -> Result<(&'a str, Option<Rating>), SearchError> {
    if property != CardProperty::Rated {
        return Ok((expected, None));
    }

    let Some((days, rating)) = expected.split_once(':') else {
        return Ok((expected, None));
    };
    if rating.is_empty() {
        return Err(SearchError {
            message: "rated property search rating is missing".to_string(),
            token: token.to_string(),
        });
    }

    Ok((days, Some(parse_rating_filter(token, rating)?)))
}

fn parse_custom_data_numeric_filter(
    token: &str,
    value: &str,
) -> Result<Option<CardCustomDataNumericFilter>, SearchError> {
    if !value
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("cdn:"))
    {
        return Ok(None);
    }

    let rest = &value[4..];
    let Some((key, operator, expected)) = split_property_comparison(rest) else {
        return Err(SearchError {
            message: "custom numeric data search must include a comparison operator".to_string(),
            token: token.to_string(),
        });
    };
    if key.is_empty() || expected.is_empty() {
        return Err(SearchError {
            message: "custom numeric data search is missing a key or value".to_string(),
            token: token.to_string(),
        });
    }
    let value = expected.parse::<f64>().map_err(|_| SearchError {
        message: "custom numeric data search value must be numeric".to_string(),
        token: token.to_string(),
    })?;

    Ok(Some(CardCustomDataNumericFilter {
        key: key.to_string(),
        operator,
        value,
    }))
}

fn parse_custom_data_string_filter(
    token: &str,
    value: &str,
) -> Result<Option<CardCustomDataStringFilter>, SearchError> {
    if !value
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("cds:"))
    {
        return Ok(None);
    }

    let rest = &value[4..];
    let Some((key, operator, expected)) = split_custom_data_string_comparison(rest) else {
        return Err(SearchError {
            message: "custom string data search must include = or !=".to_string(),
            token: token.to_string(),
        });
    };
    if key.is_empty() || expected.is_empty() {
        return Err(SearchError {
            message: "custom string data search is missing a key or value".to_string(),
            token: token.to_string(),
        });
    }

    Ok(Some(CardCustomDataStringFilter {
        key: key.to_string(),
        operator,
        value: expected.to_string(),
    }))
}

fn split_property_comparison(value: &str) -> Option<(&str, ComparisonOperator, &str)> {
    const OPERATORS: [(&str, ComparisonOperator); 6] = [
        (">=", ComparisonOperator::GreaterThanOrEqual),
        ("<=", ComparisonOperator::LessThanOrEqual),
        ("!=", ComparisonOperator::NotEqual),
        (">", ComparisonOperator::GreaterThan),
        ("<", ComparisonOperator::LessThan),
        ("=", ComparisonOperator::Equal),
    ];

    OPERATORS.iter().find_map(|(symbol, operator)| {
        value.find(symbol).map(|index| {
            let expected_start = index + symbol.len();
            (&value[..index], *operator, &value[expected_start..])
        })
    })
}

fn split_custom_data_string_comparison(value: &str) -> Option<(&str, ComparisonOperator, &str)> {
    const OPERATORS: [(&str, ComparisonOperator); 2] = [
        ("!=", ComparisonOperator::NotEqual),
        ("=", ComparisonOperator::Equal),
    ];

    OPERATORS.iter().find_map(|(symbol, operator)| {
        value.find(symbol).map(|index| {
            let expected_start = index + symbol.len();
            (&value[..index], *operator, &value[expected_start..])
        })
    })
}

fn parse_recent_days_filter(token: &str, value: &str) -> Result<RecentDaysFilter, SearchError> {
    let days = value.parse::<u32>().map_err(|_| SearchError {
        message: "recent-event search value must be a whole number of days".to_string(),
        token: token.to_string(),
    })?;

    Ok(RecentDaysFilter { days: days.max(1) })
}

fn parse_rated_filter(token: &str, value: &str) -> Result<RatedFilter, SearchError> {
    let mut parts = value.split(':');
    let days = parts
        .next()
        .expect("split always returns at least one item")
        .parse::<u32>()
        .map_err(|_| SearchError {
            message: "rated search value must start with a whole number of days".to_string(),
            token: token.to_string(),
        })?
        .max(1);
    let rating = match parts.next() {
        Some(raw_rating) if raw_rating.is_empty() => {
            return Err(SearchError {
                message: "rated search rating is missing".to_string(),
                token: token.to_string(),
            });
        }
        Some(raw_rating) => Some(parse_rating_filter(token, raw_rating)?),
        None => None,
    };
    if parts.next().is_some() {
        return Err(SearchError {
            message: "rated search has too many parts".to_string(),
            token: token.to_string(),
        });
    }

    Ok(RatedFilter { days, rating })
}

fn parse_id_filter(token: &str, value: &str) -> Result<IdFilter, SearchError> {
    let values = value
        .split(',')
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    if values.is_empty() {
        return Err(SearchError {
            message: "id search filter is missing a value".to_string(),
            token: token.to_string(),
        });
    }

    Ok(IdFilter { values })
}

fn parse_duplicate_filter(token: &str, value: &str) -> Result<DuplicateFilter, SearchError> {
    let Some((note_type_id, first_field)) = value.split_once(',') else {
        return Err(SearchError {
            message: "duplicate search must be dupe:notetype,text".to_string(),
            token: token.to_string(),
        });
    };
    let note_type_id = parse_id_filter(token, &standard_search_fold(note_type_id))?;

    Ok(DuplicateFilter {
        note_type_id,
        first_field: unescape_search_literal(first_field),
    })
}

fn unescape_search_literal(value: &str) -> String {
    let mut unescaped = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                unescaped.push(next);
            } else {
                unescaped.push(ch);
            }
        } else {
            unescaped.push(ch);
        }
    }
    unescaped
}

fn unescape_search_pattern(token: &str, value: &str) -> Result<String, SearchError> {
    let mut unescaped = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            unescaped.push(ch);
            continue;
        }

        let Some(next) = chars.next() else {
            return Err(SearchError {
                message: "unknown escape sequence".to_string(),
                token: token.to_string(),
            });
        };

        match next {
            '\\' => unescaped.push(ESCAPED_BACKSLASH),
            '"' | ':' | '(' | ')' | '-' => unescaped.push(next),
            '*' | '_' => {
                unescaped.push('\\');
                unescaped.push(next);
            }
            _ => {
                return Err(SearchError {
                    message: format!("unknown escape sequence: \\{next}"),
                    token: token.to_string(),
                });
            }
        }
    }
    Ok(unescaped)
}

fn search_literal_text(value: &str) -> String {
    let mut literal = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == ESCAPED_BACKSLASH {
            literal.push('\\');
            continue;
        }
        if ch == '\\' {
            match chars.next() {
                Some(next @ ('*' | '_')) => literal.push(next),
                Some(next) => {
                    literal.push('\\');
                    literal.push(next);
                }
                None => literal.push('\\'),
            }
            continue;
        }
        literal.push(ch);
    }
    literal
}

fn parse_rating_filter(token: &str, value: &str) -> Result<Rating, SearchError> {
    match value {
        "1" | "again" => Ok(Rating::Again),
        "2" | "hard" => Ok(Rating::Hard),
        "3" | "good" => Ok(Rating::Good),
        "4" | "easy" => Ok(Rating::Easy),
        _ => Err(SearchError {
            message: "rated search rating must be 1-4 or again/hard/good/easy".to_string(),
            token: token.to_string(),
        }),
    }
}

fn expression_matches(
    expression: &SearchExpr,
    card: &Card,
    progress: Option<&CardProgress>,
    deck: Option<&Deck>,
    note: Option<&Note>,
    note_type: Option<&NoteType>,
    metadata: &SearchMetadata<'_>,
    reviews: &[&Review],
    now: u64,
) -> bool {
    match expression {
        SearchExpr::Clause(clause) => clause_matches(
            clause, card, progress, deck, note, note_type, metadata, reviews, now,
        ),
        SearchExpr::And(expressions) => expressions.iter().all(|expression| {
            expression_matches(
                expression, card, progress, deck, note, note_type, metadata, reviews, now,
            )
        }),
        SearchExpr::Or(expressions) => expressions.iter().any(|expression| {
            expression_matches(
                expression, card, progress, deck, note, note_type, metadata, reviews, now,
            )
        }),
        SearchExpr::Not(expression) => !expression_matches(
            expression, card, progress, deck, note, note_type, metadata, reviews, now,
        ),
    }
}

fn clause_matches(
    clause: &SearchClause,
    card: &Card,
    progress: Option<&CardProgress>,
    deck: Option<&Deck>,
    note: Option<&Note>,
    note_type: Option<&NoteType>,
    metadata: &SearchMetadata<'_>,
    reviews: &[&Review],
    now: u64,
) -> bool {
    let card_sources = metadata
        .card_sources_by_id
        .get(card.id.as_str())
        .map_or(&[] as &[&ExternalSourceRecord], Vec::as_slice);
    let matched = match &clause.kind {
        SearchClauseKind::Text(filter) => text_matches(filter, card, note, note_type, metadata),
        SearchClauseKind::Field(filter) => field_matches(filter, card, note, note_type),
        SearchClauseKind::CardId(filter) => card_id_matches(filter, card, card_sources),
        SearchClauseKind::NoteId(filter) => note_id_matches(filter, card, note, metadata),
        SearchClauseKind::DeckId(filter) => deck_id_matches(filter, card, card_sources, metadata),
        SearchClauseKind::NoteTypeId(filter) => {
            note_type_id_matches(filter, note, note_type, metadata)
        }
        SearchClauseKind::Duplicate(filter) => duplicate_matches(filter, note, note_type, metadata),
        SearchClauseKind::CardTemplate(term) => card_template_matches(term, card, note_type),
        SearchClauseKind::Deck(term) => deck_matches(term, card, deck, card_sources, metadata),
        SearchClauseKind::CurrentDeck => current_deck_matches(card, deck, card_sources, metadata),
        SearchClauseKind::Preset(term) => preset_matches(term, card, deck, card_sources, metadata),
        SearchClauseKind::NoteType(term) => note_type_matches(term, note, note_type),
        SearchClauseKind::Tag(tag) => tag_matches(tag, note),
        SearchClauseKind::State(state) => {
            state_matches(*state, progress, card_sources, metadata, now)
        }
        SearchClauseKind::Flag(filter) => flag_matches(*filter, progress, card_sources),
        SearchClauseKind::Marked(expected) => marked_matches(*expected, progress, note),
        SearchClauseKind::Property(filter) => {
            property_matches(filter, progress, card_sources, metadata, reviews, now)
        }
        SearchClauseKind::CustomDataKey(key) => custom_data_key_matches(key, card_sources),
        SearchClauseKind::CustomDataNumeric(filter) => {
            custom_data_numeric_matches(filter, card_sources)
        }
        SearchClauseKind::CustomDataString(filter) => {
            custom_data_string_matches(filter, card_sources)
        }
        SearchClauseKind::Added(filter) => {
            added_matches(card, card_sources, filter.days, metadata, now)
        }
        SearchClauseKind::Edited(filter) => {
            note.is_some_and(|note| happened_recently(note.updated_at, filter.days, metadata, now))
        }
        SearchClauseKind::Introduced(filter) => {
            first_reviewed_within(reviews, filter.days, metadata, now)
        }
        SearchClauseKind::Rated(filter) => rated_matches(reviews, *filter, metadata, now),
        SearchClauseKind::Rescheduled(filter) => {
            rescheduled_matches(reviews, *filter, metadata, now)
        }
    };

    if clause.negated {
        !matched
    } else {
        matched
    }
}

fn text_matches(
    filter: &TextFilter,
    card: &Card,
    note: Option<&Note>,
    note_type: Option<&NoteType>,
    metadata: &SearchMetadata<'_>,
) -> bool {
    if let Some(note) = note {
        if let Some(note_type) = note_type {
            let excluded = metadata
                .excluded_field_ids_by_note_type_id
                .get(note_type.id.as_str());
            let matches_field = note_type.fields.iter().any(|field| {
                if excluded.is_some_and(|fields| fields.contains(&field.id)) {
                    return false;
                }
                let value = note
                    .fields
                    .iter()
                    .find(|value| value.field_id == field.id)
                    .map_or("", |value| value.value.as_str());
                text_filter_matches(filter, value)
            });
            return matches_field || preserved_anki_sort_field_matches(filter, note, metadata);
        }
        note.fields
            .iter()
            .any(|field| text_filter_matches(filter, &field.value))
            || preserved_anki_sort_field_matches(filter, note, metadata)
    } else {
        text_filter_matches(filter, &card.front) || text_filter_matches(filter, &card.back)
    }
}

fn field_matches(
    filter: &FieldFilter,
    card: &Card,
    note: Option<&Note>,
    note_type: Option<&NoteType>,
) -> bool {
    if let (Some(note), Some(note_type)) = (note, note_type) {
        for field in &note_type.fields {
            if !field_name_matches(&filter.name_pattern, &field.name) {
                continue;
            }
            let value = note
                .fields
                .iter()
                .find(|value| value.field_id == field.id)
                .map_or("", |value| value.value.as_str());
            if field_value_matches(&filter.value_pattern, value) {
                return true;
            }
        }
    }

    match filter.name_pattern.as_str() {
        "front" => field_value_matches(&filter.value_pattern, &card.front),
        "back" => field_value_matches(&filter.value_pattern, &card.back),
        _ => false,
    }
}

fn field_name_matches(pattern: &str, candidate: &str) -> bool {
    let candidate = standard_search_fold(candidate);
    if contains_search_wildcard(pattern) {
        search_pattern_matches(pattern, &candidate)
    } else {
        candidate == search_literal_text(pattern)
    }
}

fn field_value_matches(pattern: &FieldValuePattern, candidate: &str) -> bool {
    match pattern {
        FieldValuePattern::Any => true,
        FieldValuePattern::NonEmpty => !candidate.trim().is_empty(),
        FieldValuePattern::Exact(expected) => {
            let candidate = standard_search_fold(candidate);
            candidate == expected.as_str()
                || decoded_html_entity_pattern(expected)
                    .is_some_and(|decoded| candidate == standard_search_fold(&decoded))
        }
        FieldValuePattern::Wildcard(expected) => {
            let candidate = standard_search_fold(candidate);
            search_pattern_matches(expected, &candidate)
                || decoded_html_entity_pattern(expected).is_some_and(|decoded| {
                    search_pattern_matches(&standard_search_fold(&decoded), &candidate)
                })
        }
        FieldValuePattern::Text(filter) => text_filter_matches(filter, candidate),
    }
}

fn text_filter_matches(filter: &TextFilter, candidate: &str) -> bool {
    match filter.mode {
        TextMatchMode::Contains => {
            let pattern = standard_search_fold(&filter.pattern);
            let candidate = standard_search_fold(candidate);
            contains_search_pattern(&pattern, &candidate, WildcardScope::Text)
                || decoded_html_entity_pattern(&filter.pattern).is_some_and(|decoded| {
                    contains_search_pattern(
                        &standard_search_fold(&decoded),
                        &candidate,
                        WildcardScope::Text,
                    )
                })
        }
        TextMatchMode::WholeWord => {
            filter
                .regex
                .as_ref()
                .is_some_and(|regex| regex.is_match(candidate))
                || decoded_html_entity_pattern(&filter.pattern)
                    .is_some_and(|decoded| whole_word_pattern_matches(&decoded, candidate))
        }
        TextMatchMode::Regex => filter
            .regex
            .as_ref()
            .is_some_and(|regex| regex.is_match(candidate)),
        TextMatchMode::NoCombining => {
            contains_search_pattern(
                &normalize_no_combining(&filter.pattern),
                &normalize_no_combining(candidate),
                WildcardScope::Text,
            ) || decoded_html_entity_pattern(&filter.pattern).is_some_and(|decoded| {
                contains_search_pattern(
                    &normalize_no_combining(&decoded),
                    &normalize_no_combining(candidate),
                    WildcardScope::Text,
                )
            })
        }
        TextMatchMode::StripCloze => {
            let pattern = standard_search_fold(&filter.pattern);
            let candidate = standard_search_fold(&strip_cloze_markup(candidate));
            contains_search_pattern(&pattern, &candidate, WildcardScope::Text)
                || decoded_html_entity_pattern(&filter.pattern).is_some_and(|decoded| {
                    contains_search_pattern(
                        &standard_search_fold(&decoded),
                        &candidate,
                        WildcardScope::Text,
                    )
                })
        }
    }
}

fn whole_word_pattern_matches(pattern: &str, candidate: &str) -> bool {
    let source = format!(
        "(?u)(?:^|[^\\p{{Alphabetic}}\\p{{Mark}}\\p{{Nd}}_]){}(?:$|[^\\p{{Alphabetic}}\\p{{Mark}}\\p{{Nd}}_])",
        search_pattern_regex_source(pattern, WildcardScope::Word)
    );
    RegexBuilder::new(&source)
        .case_insensitive(true)
        .build()
        .is_ok_and(|regex| regex.is_match(candidate))
}

fn contains_search_pattern(pattern: &str, candidate: &str, scope: WildcardScope) -> bool {
    if contains_search_wildcard(pattern) {
        Regex::new(&search_pattern_regex_source(pattern, scope))
            .is_ok_and(|regex| regex.is_match(candidate))
    } else {
        candidate.contains(&search_literal_text(pattern))
    }
}

fn contains_search_wildcard(value: &str) -> bool {
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if matches!(ch, '*' | '_') {
            return true;
        }
    }
    false
}

fn normalize_no_combining(value: &str) -> String {
    let mut normalized = String::new();
    for ch in value.nfd() {
        if is_combining_mark(ch) {
            continue;
        }
        match ch {
            'ß' | 'ẞ' => normalized.push('s'),
            _ => normalized.extend(ch.to_lowercase()),
        }
    }
    normalized
}

fn strip_cloze_markup(value: &str) -> String {
    let mut stripped = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(start) = rest.find("{{c") {
        let (prefix, candidate) = rest.split_at(start);
        stripped.push_str(prefix);

        if let Some(marker) = parse_search_cloze_marker(candidate) {
            stripped.push_str(&strip_cloze_markup(marker.hidden));
            rest = &candidate[marker.consumed..];
        } else {
            stripped.push_str("{{c");
            rest = &candidate[3..];
        }
    }

    stripped.push_str(rest);
    stripped
}

struct SearchClozeMarker<'a> {
    hidden: &'a str,
    consumed: usize,
}

fn parse_search_cloze_marker(candidate: &str) -> Option<SearchClozeMarker<'_>> {
    if !candidate.starts_with("{{c") {
        return None;
    }

    let after_prefix = &candidate[3..];
    let digit_len = after_prefix
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(after_prefix.len());
    if digit_len == 0 || after_prefix[..digit_len].parse::<u32>().ok()? == 0 {
        return None;
    }

    let after_digits = &after_prefix[digit_len..];
    if !after_digits.starts_with("::") {
        return None;
    }

    let content_start = 3 + digit_len + 2;
    let after_content_start = &candidate[content_start..];
    let content_len = after_content_start.find("}}")?;
    let content = &after_content_start[..content_len];
    let consumed = content_start + content_len + 2;
    let hidden = content
        .split_once("::")
        .map_or(content, |(hidden, _hint)| hidden);

    Some(SearchClozeMarker { hidden, consumed })
}

fn anki_deck_config_names_by_id(state: &AppState) -> HashMap<String, String> {
    let mut names = HashMap::new();

    for source in state
        .external_sources
        .iter()
        .filter(|source| source.target == ExternalSourceTarget::Collection)
    {
        let Some(raw_config) = source.data.get("deckConfigJson") else {
            continue;
        };
        let Ok(Value::Object(configs)) = serde_json::from_str::<Value>(raw_config) else {
            continue;
        };

        for (config_id, config) in configs {
            if let Some(name) = config.get("name").and_then(Value::as_str) {
                names.entry(config_id).or_insert_with(|| name.to_string());
            }
        }
    }

    names
}

fn anki_deck_preset_names(
    source: &ExternalSourceRecord,
    deck_config_names: &HashMap<String, String>,
) -> Vec<String> {
    let mut names = Vec::new();

    for key in ["configName", "presetName"] {
        if let Some(name) = source.data.get(key) {
            push_unique_non_empty(&mut names, name);
        }
    }

    let config_id = source_i64_from_data(source, "configId")
        .or_else(|| source_i64_from_data(source, "conf"))
        .or_else(|| source_i64_from_raw_json(source, "conf"))
        .or_else(|| source.data.contains_key("rawJson").then_some(1));
    if let Some(config_id) = config_id {
        if let Some(name) = deck_config_names.get(&config_id.to_string()) {
            push_unique_non_empty(&mut names, name);
        }
    }

    names
}

fn push_unique_non_empty(values: &mut Vec<String>, value: &str) {
    let value = value.trim();
    if value.is_empty()
        || values
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(value))
    {
        return;
    }
    values.push(value.to_string());
}

fn source_i64_from_data(source: &ExternalSourceRecord, key: &str) -> Option<i64> {
    source
        .data
        .get(key)
        .and_then(|value| value.trim().parse::<i64>().ok())
}

fn source_i64_from_raw_json(source: &ExternalSourceRecord, key: &str) -> Option<i64> {
    let raw = source.data.get("rawJson")?;
    let value = serde_json::from_str::<Value>(raw).ok()?;
    value.get(key)?.as_i64()
}

fn anki_excluded_field_ids(source: &ExternalSourceRecord) -> Vec<String> {
    let raw = source.data.get("rawJson");
    let Some(Value::Array(fields)) = raw
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .and_then(|value| value.get("flds").cloned())
    else {
        return Vec::new();
    };

    fields
        .iter()
        .enumerate()
        .filter(|&(_index, field)| anki_field_excluded_from_search(field)).map(|(index, field)| {
                let ordinal = field
                    .get("ord")
                    .and_then(Value::as_i64)
                    .unwrap_or(index as i64);
                format!("{}:field:{ordinal}", source.target_id)
            })
        .collect()
}

fn anki_field_excluded_from_search(field: &Value) -> bool {
    ["exclude_from_search", "excludeFromSearch"]
        .into_iter()
        .any(|key| field.get(key).and_then(Value::as_bool).unwrap_or(false))
        || field.get("config").is_some_and(|config| {
            ["exclude_from_search", "excludeFromSearch"]
                .into_iter()
                .any(|key| config.get(key).and_then(Value::as_bool).unwrap_or(false))
        })
}

fn deck_matches(
    term: &str,
    card: &Card,
    deck: Option<&Deck>,
    card_sources: &[&ExternalSourceRecord],
    metadata: &SearchMetadata<'_>,
) -> bool {
    if term == "filtered" {
        return metadata.filtered_deck_ids.contains(card.deck_id.as_str())
            || imported_anki_original_deck_id(card_sources).is_some();
    }

    anki_hierarchical_name_matches(term, &card.deck_id)
        || deck.is_some_and(|deck| {
            anki_hierarchical_name_matches(term, &deck.id)
                || anki_hierarchical_name_matches(term, &deck.name)
        })
        || imported_original_deck_matches(term, card_sources, metadata)
}

fn current_deck_matches(
    card: &Card,
    deck: Option<&Deck>,
    card_sources: &[&ExternalSourceRecord],
    metadata: &SearchMetadata<'_>,
) -> bool {
    let Some(current_deck_id) = metadata.current_deck_id.as_deref() else {
        return false;
    };

    let current_deck_id_term = standard_search_fold(current_deck_id);
    deck_matches(&current_deck_id_term, card, deck, card_sources, metadata)
        || metadata
            .decks_by_id
            .get(current_deck_id)
            .is_some_and(|current_deck| {
                deck_matches(
                    &standard_search_fold(&current_deck.name),
                    card,
                    deck,
                    card_sources,
                    metadata,
                )
            })
}

fn imported_original_deck_matches(
    term: &str,
    card_sources: &[&ExternalSourceRecord],
    metadata: &SearchMetadata<'_>,
) -> bool {
    let Some(original_deck_id) = imported_anki_original_deck_id(card_sources) else {
        return false;
    };
    metadata
        .decks_by_original_id
        .get(original_deck_id)
        .is_some_and(|decks| {
            decks.iter().any(|deck| {
                anki_hierarchical_name_matches(term, &deck.id)
                    || anki_hierarchical_name_matches(term, &deck.name)
            })
        })
        || metadata
            .decks_by_id
            .get(original_deck_id)
            .is_some_and(|deck| {
                anki_hierarchical_name_matches(term, &deck.id)
                    || anki_hierarchical_name_matches(term, &deck.name)
            })
}

fn imported_anki_original_deck_id<'a>(
    card_sources: &'a [&'a ExternalSourceRecord],
) -> Option<&'a str> {
    card_sources.iter().find_map(|source| {
        source
            .data
            .get("originalDeckId")
            .map(|deck_id| deck_id.trim())
            .filter(|deck_id| !deck_id.is_empty() && *deck_id != "0")
    })
}

fn preset_matches(
    term: &str,
    card: &Card,
    deck: Option<&Deck>,
    card_sources: &[&ExternalSourceRecord],
    metadata: &SearchMetadata<'_>,
) -> bool {
    metadata
        .deck_preset_names_by_id
        .get(card.deck_id.as_str())
        .is_some_and(|names| {
            names
                .iter()
                .any(|name| preset_candidate_matches(term, name))
        })
        || (metadata
            .deck_option_deck_ids
            .contains(card.deck_id.as_str())
            && (preset_candidate_matches(term, &card.deck_id)
                || deck.is_some_and(|deck| {
                    preset_candidate_matches(term, &deck.id)
                        || preset_candidate_matches(term, &deck.name)
                })))
        || imported_original_deck_preset_matches(term, card_sources, metadata)
}

fn imported_original_deck_preset_matches(
    term: &str,
    card_sources: &[&ExternalSourceRecord],
    metadata: &SearchMetadata<'_>,
) -> bool {
    let Some(original_deck_id) = imported_anki_original_deck_id(card_sources) else {
        return false;
    };
    metadata
        .decks_by_original_id
        .get(original_deck_id)
        .is_some_and(|decks| {
            decks.iter().any(|deck| {
                metadata
                    .deck_preset_names_by_id
                    .get(deck.id.as_str())
                    .is_some_and(|names| {
                        names
                            .iter()
                            .any(|name| preset_candidate_matches(term, name))
                    })
                    || (metadata.deck_option_deck_ids.contains(deck.id.as_str())
                        && (preset_candidate_matches(term, &deck.id)
                            || preset_candidate_matches(term, &deck.name)))
            })
        })
        || metadata
            .decks_by_id
            .get(original_deck_id)
            .is_some_and(|deck| {
                metadata
                    .deck_preset_names_by_id
                    .get(deck.id.as_str())
                    .is_some_and(|names| {
                        names
                            .iter()
                            .any(|name| preset_candidate_matches(term, name))
                    })
                    || (metadata.deck_option_deck_ids.contains(deck.id.as_str())
                        && (preset_candidate_matches(term, &deck.id)
                            || preset_candidate_matches(term, &deck.name)))
            })
}

fn preset_candidate_matches(term: &str, candidate: &str) -> bool {
    let candidate = standard_search_fold(candidate);
    if contains_search_wildcard(term) {
        search_pattern_matches(term, &candidate)
    } else {
        candidate == search_literal_text(term)
    }
}

fn tag_matches(filter: &TagFilter, note: Option<&Note>) -> bool {
    match filter {
        TagFilter::Hierarchical(tag) if tag == "*" => true,
        TagFilter::Hierarchical(tag) if tag == "none" => {
            note.is_none_or(|note| note.tags.is_empty())
        }
        TagFilter::Hierarchical(tag) => note.is_some_and(|note| {
            note.tags
                .iter()
                .any(|candidate| anki_hierarchical_name_matches(tag, candidate))
        }),
        TagFilter::NoCombining(pattern) => {
            let pattern = normalize_no_combining(pattern);
            if pattern == "*" {
                return true;
            }
            if pattern == "none" {
                return note.is_none_or(|note| note.tags.is_empty());
            }
            note.is_some_and(|note| {
                note.tags.iter().any(|candidate| {
                    let candidate = normalize_no_combining(candidate);
                    anki_hierarchical_name_matches(&pattern, &candidate)
                })
            })
        }
        TagFilter::Regex(regex) => note.is_some_and(|note| {
            note.tags
                .iter()
                .any(|candidate| text_filter_matches(regex, candidate))
        }),
    }
}

fn anki_hierarchical_name_matches(pattern: &str, candidate: &str) -> bool {
    let candidate = standard_search_fold(candidate);
    if contains_search_wildcard(pattern) {
        return search_pattern_matches(pattern, &candidate);
    }

    let pattern = search_literal_text(pattern);
    candidate == pattern || candidate.starts_with(&format!("{pattern}::"))
}

fn search_pattern_matches(pattern: &str, candidate: &str) -> bool {
    if !contains_search_wildcard(pattern) {
        return candidate == search_literal_text(pattern);
    }

    Regex::new(&format!(
        "^{}$",
        search_pattern_regex_source(pattern, WildcardScope::Text)
    ))
    .is_ok_and(|regex| regex.is_match(candidate))
}

fn note_type_matches(term: &str, note: Option<&Note>, note_type: Option<&NoteType>) -> bool {
    note.is_some_and(|note| anki_name_candidate_matches(term, &note.note_type_id))
        || note_type.is_some_and(|note_type| {
            anki_name_candidate_matches(term, &note_type.id)
                || anki_name_candidate_matches(term, &note_type.name)
        })
}

fn id_filter_matches(filter: &IdFilter, value: &str) -> bool {
    filter
        .values
        .iter()
        .any(|expected| value.eq_ignore_ascii_case(expected))
}

fn card_id_matches(filter: &IdFilter, card: &Card, card_sources: &[&ExternalSourceRecord]) -> bool {
    id_filter_matches(filter, &card.id)
        || card_sources
            .iter()
            .filter_map(|source| source.original_id.as_deref())
            .any(|original_id| id_filter_matches(filter, original_id))
}

fn note_id_matches(
    filter: &IdFilter,
    card: &Card,
    note: Option<&Note>,
    metadata: &SearchMetadata<'_>,
) -> bool {
    card.lineage
        .as_ref()
        .is_some_and(|lineage| id_filter_matches(filter, &lineage.note_id))
        || note.is_some_and(|note| id_filter_matches(filter, &note.id))
        || note
            .map(|note| note.id.as_str())
            .or_else(|| {
                card.lineage
                    .as_ref()
                    .map(|lineage| lineage.note_id.as_str())
            })
            .and_then(|note_id| metadata.note_sources_by_id.get(note_id))
            .is_some_and(|sources| {
                sources
                    .iter()
                    .filter_map(|source| source.original_id.as_deref())
                    .any(|original_id| id_filter_matches(filter, original_id))
            })
}

fn deck_id_matches(
    filter: &IdFilter,
    card: &Card,
    card_sources: &[&ExternalSourceRecord],
    metadata: &SearchMetadata<'_>,
) -> bool {
    id_filter_matches(filter, &card.deck_id)
        || metadata
            .deck_original_ids_by_id
            .get(card.deck_id.as_str())
            .is_some_and(|original_ids| {
                original_ids
                    .iter()
                    .any(|original_id| id_filter_matches(filter, original_id))
            })
        || card_sources.iter().any(|source| {
            source_i64_from_data(source, "deckId")
                .is_some_and(|deck_id| id_filter_matches(filter, &deck_id.to_string()))
                || source_i64_from_data(source, "originalDeckId").is_some_and(|deck_id| {
                    deck_id != 0 && id_filter_matches(filter, &deck_id.to_string())
                })
                || source
                    .data
                    .get("originalDeckId")
                    .map(|deck_id| deck_id.trim())
                    .is_some_and(|deck_id| {
                        !deck_id.is_empty() && deck_id != "0" && id_filter_matches(filter, deck_id)
                    })
        })
}

fn note_type_id_matches(
    filter: &IdFilter,
    note: Option<&Note>,
    note_type: Option<&NoteType>,
    metadata: &SearchMetadata<'_>,
) -> bool {
    note.is_some_and(|note| note_type_id_candidate_matches(filter, &note.note_type_id, metadata))
        || note_type.is_some_and(|note_type| {
            note_type_id_candidate_matches(filter, &note_type.id, metadata)
        })
}

fn note_type_id_candidate_matches(
    filter: &IdFilter,
    note_type_id: &str,
    metadata: &SearchMetadata<'_>,
) -> bool {
    id_filter_matches(filter, note_type_id)
        || metadata
            .note_type_original_ids_by_id
            .get(note_type_id)
            .is_some_and(|original_ids| {
                original_ids
                    .iter()
                    .any(|original_id| id_filter_matches(filter, original_id))
            })
}

fn duplicate_matches(
    filter: &DuplicateFilter,
    note: Option<&Note>,
    note_type: Option<&NoteType>,
    metadata: &SearchMetadata<'_>,
) -> bool {
    let Some(note) = note else {
        return false;
    };
    if !note_type_id_matches(&filter.note_type_id, Some(note), note_type, metadata) {
        return false;
    }

    duplicate_sort_field(note, note_type, metadata)
        .is_some_and(|candidate| duplicate_text_matches(&filter.first_field, &candidate))
}

fn duplicate_sort_field<'a>(
    note: &'a Note,
    note_type: Option<&'a NoteType>,
    metadata: &SearchMetadata<'a>,
) -> Option<Cow<'a, str>> {
    if let Some(sort_field) =
        metadata
            .note_sources_by_id
            .get(note.id.as_str())
            .and_then(|sources| {
                sources
                    .iter()
                    .find_map(|source| source.data.get("sortField").map(String::as_str))
            })
    {
        return Some(Cow::Borrowed(sort_field));
    }

    let first_field_id = note_type.and_then(|note_type| {
        note_type
            .fields
            .iter()
            .min_by_key(|field| field.ordinal)
            .map(|field| field.id.as_str())
    });
    let value = first_field_id
        .and_then(|field_id| {
            note.fields
                .iter()
                .find(|field| field.field_id == field_id)
                .map(|field| field.value.as_str())
        })
        .or_else(|| note.fields.first().map(|field| field.value.as_str()))?;

    Some(Cow::Borrowed(value))
}

fn duplicate_text_matches(expected: &str, candidate: &str) -> bool {
    duplicate_search_text(expected) == duplicate_search_text(candidate)
}

fn duplicate_search_text(value: &str) -> String {
    rendered_search_text(value).chars().nfc().collect()
}

fn preserved_anki_sort_field_matches(
    filter: &TextFilter,
    note: &Note,
    metadata: &SearchMetadata<'_>,
) -> bool {
    preserved_anki_sort_field(note, metadata)
        .is_some_and(|sort_field| text_filter_matches(filter, sort_field))
}

fn preserved_anki_sort_field<'a>(note: &Note, metadata: &SearchMetadata<'a>) -> Option<&'a str> {
    metadata
        .note_sources_by_id
        .get(note.id.as_str())?
        .iter()
        .find_map(|source| source.data.get("sortField").map(String::as_str))
}

fn rendered_search_text(value: &str) -> Cow<'_, str> {
    if !value.contains('<') && !value.contains('&') {
        return Cow::Borrowed(value);
    }

    let with_media =
        DUPLICATE_HTML_MEDIA_TAGS.replace_all(value, |captures: &regex_engine::Captures| {
            let filename = captures
                .get(1)
                .or_else(|| captures.get(2))
                .or_else(|| captures.get(3))
                .map_or("", |capture| capture.as_str());
            format!(" {filename} ")
        });
    let without_tags = crate::html_scan::strip_tags(&with_media);
    Cow::Owned(decode_search_html_entities(&without_tags))
}

fn decoded_html_entity_pattern(pattern: &str) -> Option<String> {
    if !pattern.contains('&') {
        return None;
    }
    let decoded = decode_search_html_entities(pattern);
    (decoded != pattern).then_some(decoded)
}

fn decode_search_html_entities(value: &str) -> String {
    if !value.contains('&') {
        return value.to_string();
    }

    let mut decoded = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('&') {
        decoded.push_str(&rest[..start]);
        let after_amp = &rest[start + 1..];
        let Some(end) = after_amp.find(';') else {
            decoded.push('&');
            rest = after_amp;
            continue;
        };
        let entity = &after_amp[..end];
        if let Some(ch) = decode_search_html_entity(entity) {
            decoded.push(ch);
            rest = &after_amp[end + 1..];
        } else {
            decoded.push('&');
            rest = after_amp;
        }
    }
    decoded.push_str(rest);
    decoded
}

fn decode_search_html_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some(' '),
        _ => {
            let codepoint = entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| {
                    entity
                        .strip_prefix('#')
                        .and_then(|decimal| decimal.parse::<u32>().ok())
                })?;
            char::from_u32(codepoint)
        }
    }
}

fn card_template_matches(term: &str, card: &Card, note_type: Option<&NoteType>) -> bool {
    let lineage = card.lineage.as_ref();
    let template_id = lineage
        .map(|lineage| lineage.template_id.as_str())
        .or_else(|| card.id.split_once("::").map(|(_, template_id)| template_id));
    let template_ordinal = lineage.map(|lineage| lineage.ordinal);
    let requested_ordinal = term
        .parse::<u32>()
        .ok()
        .map(|ordinal| ordinal.saturating_sub(1));

    if requested_ordinal.is_some_and(|ordinal| template_ordinal == Some(ordinal)) {
        return true;
    }

    template_id.is_some_and(|template_id| anki_name_candidate_matches(term, template_id))
        || note_type.is_some_and(|note_type| {
            note_type.templates.iter().any(|template| {
                let is_current_template = template_id
                    .is_some_and(|template_id| template.id.eq_ignore_ascii_case(template_id))
                    || template_ordinal.is_some_and(|ordinal| template.ordinal == ordinal);
                is_current_template
                    && (anki_name_candidate_matches(term, &template.id)
                        || anki_name_candidate_matches(term, &template.name)
                        || requested_ordinal.is_some_and(|ordinal| template.ordinal == ordinal))
            })
        })
}

fn anki_name_candidate_matches(term: &str, candidate: &str) -> bool {
    let candidate = standard_search_fold(candidate);
    if contains_search_wildcard(term) {
        search_pattern_matches(term, &candidate)
    } else {
        candidate == search_literal_text(term)
    }
}

fn state_matches(
    state: CardSearchState,
    progress: Option<&CardProgress>,
    card_sources: &[&ExternalSourceRecord],
    metadata: &SearchMetadata<'_>,
    now: u64,
) -> bool {
    match state {
        CardSearchState::New => imported_anki_state_matches(state, card_sources, metadata, now)
            .unwrap_or_else(|| progress.is_none_or(is_new_progress_overlay)),
        CardSearchState::Due => imported_anki_card_is_due(card_sources, metadata, now)
            .unwrap_or_else(|| progress.is_some_and(|progress| is_reviewable(progress, now))),
        CardSearchState::Learning => {
            imported_anki_state_matches(state, card_sources, metadata, now).unwrap_or_else(|| {
                progress.is_some_and(|progress| {
                    matches!(progress.state, CardState::Learning | CardState::Relearning)
                })
            })
        }
        CardSearchState::Review => imported_anki_state_matches(state, card_sources, metadata, now)
            .unwrap_or_else(|| {
                progress.is_some_and(|progress| {
                    matches!(progress.state, CardState::Review | CardState::Relearning)
                        && !is_new_progress_overlay(progress)
                        && progress.suspended_at.is_none()
                        && !is_buried(progress, now)
                })
            }),
        CardSearchState::Relearning => {
            progress.is_some_and(|progress| progress.state == CardState::Relearning)
        }
        CardSearchState::Suspended => {
            imported_anki_state_matches(state, card_sources, metadata, now)
                .unwrap_or_else(|| progress.is_some_and(is_suspended))
        }
        CardSearchState::Buried => imported_anki_state_matches(state, card_sources, metadata, now)
            .unwrap_or_else(|| progress.is_some_and(|progress| is_buried(progress, now))),
        CardSearchState::BuriedManually => {
            imported_anki_state_matches(state, card_sources, metadata, now).unwrap_or_else(|| {
                progress.is_some_and(|progress| is_buried(progress, now))
                    && anki_card_queue_matches(card_sources, ANKI_QUEUE_USER_BURIED)
            })
        }
        CardSearchState::BuriedSibling => {
            imported_anki_state_matches(state, card_sources, metadata, now).unwrap_or_else(|| {
                progress.is_some_and(|progress| is_buried(progress, now))
                    && anki_card_queue_matches(card_sources, ANKI_QUEUE_SCHED_BURIED)
            })
        }
    }
}

fn imported_anki_state_matches(
    state: CardSearchState,
    card_sources: &[&ExternalSourceRecord],
    metadata: &SearchMetadata<'_>,
    now: u64,
) -> Option<bool> {
    card_sources.iter().find_map(|source| match state {
        CardSearchState::New => Some(source_i64_from_data(source, "kind")? == ANKI_TYPE_NEW),
        CardSearchState::Learning => {
            let kind = source_i64_from_data(source, "kind")?;
            Some(matches!(kind, ANKI_TYPE_LEARN | ANKI_TYPE_RELEARN))
        }
        CardSearchState::Review => {
            let kind = source_i64_from_data(source, "kind")?;
            Some(matches!(kind, ANKI_TYPE_REVIEW | ANKI_TYPE_RELEARN))
        }
        CardSearchState::Due => imported_anki_card_is_due(&[*source], metadata, now),
        CardSearchState::Suspended => {
            Some(source_i64_from_data(source, "queue")? == ANKI_QUEUE_SUSPENDED)
        }
        CardSearchState::Buried => {
            let queue = source_i64_from_data(source, "queue")?;
            Some(matches!(
                queue,
                ANKI_QUEUE_USER_BURIED | ANKI_QUEUE_SCHED_BURIED
            ))
        }
        CardSearchState::BuriedManually => {
            Some(source_i64_from_data(source, "queue")? == ANKI_QUEUE_USER_BURIED)
        }
        CardSearchState::BuriedSibling => {
            Some(source_i64_from_data(source, "queue")? == ANKI_QUEUE_SCHED_BURIED)
        }
        CardSearchState::Relearning => None,
    })
}

fn flag_matches(
    filter: FlagFilter,
    progress: Option<&CardProgress>,
    card_sources: &[&ExternalSourceRecord],
) -> bool {
    if let Some(flag) = imported_anki_card_flag(card_sources) {
        return flag_filter_matches(filter, flag);
    }

    flag_filter_matches(filter, progress.and_then(|progress| progress.flag))
}

fn flag_filter_matches(filter: FlagFilter, flag: Option<CardFlag>) -> bool {
    match filter {
        FlagFilter::Any => flag.is_some(),
        FlagFilter::None => flag.is_none(),
        FlagFilter::Color(expected) => flag == Some(expected),
    }
}

fn imported_anki_card_flag(card_sources: &[&ExternalSourceRecord]) -> Option<Option<CardFlag>> {
    card_sources
        .iter()
        .find_map(|source| source_i64_from_data(source, "flags").map(anki_card_flag))
}

fn anki_card_flag(flags: i64) -> Option<CardFlag> {
    match flags & 0b111 {
        1 => Some(CardFlag::Red),
        2 => Some(CardFlag::Orange),
        3 => Some(CardFlag::Green),
        4 => Some(CardFlag::Blue),
        5 => Some(CardFlag::Pink),
        6 => Some(CardFlag::Turquoise),
        7 => Some(CardFlag::Purple),
        _ => None,
    }
}

fn marked_matches(expected: bool, progress: Option<&CardProgress>, note: Option<&Note>) -> bool {
    let marked = progress.is_some_and(|progress| progress.marked_at.is_some())
        || note.is_some_and(|note| {
            note.tags
                .iter()
                .any(|tag| tag.eq_ignore_ascii_case("marked"))
        });
    marked == expected
}

fn property_matches(
    filter: &CardPropertyFilter,
    progress: Option<&CardProgress>,
    card_sources: &[&ExternalSourceRecord],
    metadata: &SearchMetadata<'_>,
    reviews: &[&Review],
    now: u64,
) -> bool {
    match filter.property {
        CardProperty::Interval => imported_anki_card_property(card_sources, "interval")
            .map_or_else(
                || {
                    compare_number(
                        progress.map_or(0.0, |progress| f64::from(progress.interval)),
                        filter.operator,
                        filter.value,
                    )
                },
                |actual| compare_number(actual, filter.operator, filter.value),
            ),
        CardProperty::Due => imported_anki_due_relative_days(card_sources, metadata, now)
            .map_or_else(
                || {
                    progress.is_some_and(|progress| {
                        compare_number(
                            f64::from(relative_day_bucket(
                                progress.next_due_at,
                                now,
                                metadata.day_start_offset_ms,
                            )),
                            filter.operator,
                            filter.value,
                        )
                    })
                },
                |days| compare_number(days as f64, filter.operator, filter.value),
            ),
        CardProperty::Repetitions => imported_anki_card_property(card_sources, "repetitions")
            .map_or_else(
                || {
                    compare_number(
                        progress.map_or(0.0, |progress| f64::from(progress.times_seen)),
                        filter.operator,
                        filter.value,
                    )
                },
                |actual| compare_number(actual, filter.operator, filter.value),
            ),
        CardProperty::Lapses => imported_anki_card_property(card_sources, "lapses").map_or_else(
            || {
                compare_number(
                    progress.map_or(0.0, |progress| f64::from(progress.times_incorrect)),
                    filter.operator,
                    filter.value,
                )
            },
            |actual| compare_number(actual, filter.operator, filter.value),
        ),
        CardProperty::Ease => imported_anki_card_factor(card_sources).map_or_else(
            || {
                progress.is_some_and(|progress| {
                    compare_number(progress.ease_factor, filter.operator, filter.value)
                })
            },
            |actual| compare_number(actual, filter.operator, filter.value),
        ),
        CardProperty::Rated => reviews.iter().any(|review| {
            !anki_review_is_manual_reschedule(review, metadata)
                && filter.rating.is_none_or(|rating| review.rating == rating)
                && compare_number(
                    f64::from(relative_day_bucket(
                        review.reviewed_at,
                        now,
                        metadata.day_start_offset_ms,
                    )),
                    filter.operator,
                    filter.value,
                )
        }),
        CardProperty::Rescheduled => reviews.iter().any(|review| {
            anki_review_is_manual_reschedule(review, metadata)
                && compare_number(
                    f64::from(relative_day_bucket(
                        review.reviewed_at,
                        now,
                        metadata.day_start_offset_ms,
                    )),
                    filter.operator,
                    filter.value,
                )
        }),
        CardProperty::Position => imported_new_card_position(progress, card_sources)
            .is_some_and(|position| compare_number(position as f64, filter.operator, filter.value)),
        CardProperty::Stability => progress
            .and_then(|progress| progress.fsrs_stability)
            .or_else(|| imported_fsrs_variable(card_sources, "s"))
            .is_some_and(|actual| compare_number(actual, filter.operator, filter.value)),
        CardProperty::Difficulty => progress
            .and_then(|progress| progress.fsrs_difficulty)
            .or_else(|| imported_fsrs_variable(card_sources, "d"))
            .is_some_and(|actual| {
                compare_number(actual, filter.operator, filter.value * 9.0 + 1.0)
            }),
        CardProperty::Retrievability => native_fsrs_retrievability(progress, now)
            .or_else(|| imported_fsrs_retrievability(progress, card_sources, metadata, now))
            .is_some_and(|actual| compare_number(actual, filter.operator, filter.value)),
    }
}

fn imported_anki_card_property(card_sources: &[&ExternalSourceRecord], key: &str) -> Option<f64> {
    card_sources
        .iter()
        .find_map(|source| source_i64_from_data(source, key).map(|value| value as f64))
}

fn imported_anki_card_factor(card_sources: &[&ExternalSourceRecord]) -> Option<f64> {
    imported_anki_card_property(card_sources, "factor").map(|factor| factor / 1000.0)
}

fn imported_fsrs_variable(card_sources: &[&ExternalSourceRecord], key: &str) -> Option<f64> {
    card_sources
        .iter()
        .find_map(|source| card_data_number(source, key))
}

fn native_fsrs_retrievability(progress: Option<&CardProgress>, now: u64) -> Option<f64> {
    let progress = progress?;
    if is_new_progress_overlay(progress) {
        return None;
    }
    let stability = progress.fsrs_stability?;
    let difficulty = progress.fsrs_difficulty?;
    if !(stability.is_finite() && stability > 0.0 && difficulty.is_finite()) {
        return None;
    }
    let elapsed_days = now.saturating_sub(progress.last_seen_at) as f32 / MS_PER_DAY as f32;
    Some(f64::from(fsrs::current_retrievability(
        fsrs::MemoryState {
            stability: stability as f32,
            difficulty: difficulty as f32,
        },
        elapsed_days,
        fsrs::FSRS6_DEFAULT_DECAY,
    )))
}

fn imported_fsrs_retrievability(
    progress: Option<&CardProgress>,
    card_sources: &[&ExternalSourceRecord],
    metadata: &SearchMetadata<'_>,
    now: u64,
) -> Option<f64> {
    if progress.is_none_or(is_new_progress_overlay) {
        return None;
    }

    card_sources.iter().find_map(|source| {
        if source_i64_from_data(source, "kind") == Some(0) {
            return None;
        }
        let stability = card_data_number(source, "s")?;
        let _difficulty = card_data_number(source, "d")?;
        let elapsed_seconds = imported_fsrs_elapsed_seconds(source, metadata, now)?;
        let decay = card_data_number(source, "decay").unwrap_or(FSRS5_DEFAULT_DECAY);
        Some(fsrs_retrievability(stability, elapsed_seconds, decay))
    })
}

fn imported_fsrs_elapsed_seconds(
    source: &ExternalSourceRecord,
    metadata: &SearchMetadata<'_>,
    now: u64,
) -> Option<u64> {
    let now_secs = i64::try_from(now / 1000).ok()?;
    if let Some(last_review_time) = card_data_number(source, "lrt").map(|value| value as i64) {
        return Some(nonnegative_seconds_between(now_secs, last_review_time));
    }

    let due = source_i64_from_data(source, "due")?;
    let interval = source_i64_from_data(source, "interval")?;
    if due > 365_000 {
        return Some(nonnegative_seconds_between(
            now_secs,
            due.saturating_sub(interval),
        ));
    }

    let today = now_secs
        .div_euclid(SECONDS_PER_DAY)
        .saturating_sub(metadata.collection_created_at_days?);
    let review_day = due.saturating_sub(interval);
    Some(nonnegative_days_between(today, review_day).saturating_mul(SECONDS_PER_DAY as u64))
}

fn nonnegative_seconds_between(later: i64, earlier: i64) -> u64 {
    later.saturating_sub(earlier).max(0) as u64
}

fn nonnegative_days_between(later: i64, earlier: i64) -> u64 {
    later.saturating_sub(earlier).max(0) as u64
}

fn fsrs_retrievability(stability: f64, elapsed_seconds: u64, decay: f64) -> f64 {
    if stability <= 0.0 || decay <= 0.0 {
        return 0.0;
    }
    let elapsed_days = elapsed_seconds as f64 / SECONDS_PER_DAY as f64;
    let factor = 0.9_f64.powf(1.0 / -decay) - 1.0;
    (1.0 + factor * elapsed_days / stability).powf(-decay)
}

fn imported_anki_card_is_due(
    card_sources: &[&ExternalSourceRecord],
    metadata: &SearchMetadata<'_>,
    now: u64,
) -> Option<bool> {
    card_sources.iter().find_map(|source| {
        let queue = source_i64_from_data(source, "queue")?;
        let due = imported_anki_due_value(source)?;
        match queue {
            ANKI_QUEUE_REVIEW | ANKI_QUEUE_DAY_LEARN => {
                Some(due <= imported_anki_today(metadata, now)?)
            }
            ANKI_QUEUE_LEARN | ANKI_QUEUE_PREVIEW_REPEAT => {
                let now_secs = i64::try_from(now / 1000).ok()?;
                Some(due <= now_secs.saturating_add(metadata.learn_ahead_secs))
            }
            _ => None,
        }
    })
}

fn imported_anki_due_relative_days(
    card_sources: &[&ExternalSourceRecord],
    metadata: &SearchMetadata<'_>,
    now: u64,
) -> Option<i64> {
    card_sources.iter().find_map(|source| {
        let queue = source_i64_from_data(source, "queue")?;
        match queue {
            ANKI_QUEUE_REVIEW | ANKI_QUEUE_DAY_LEARN => {
                Some(imported_anki_due_value(source)? - imported_anki_today(metadata, now)?)
            }
            ANKI_QUEUE_LEARN | ANKI_QUEUE_PREVIEW_REPEAT => {
                let next_day_at = imported_anki_next_day_at(metadata, now)?;
                Some(imported_anki_due_value(source)?.saturating_sub(next_day_at) / SECONDS_PER_DAY)
            }
            _ => None,
        }
    })
}

fn imported_anki_due_value(source: &ExternalSourceRecord) -> Option<i64> {
    source_i64_from_data(source, "originalDue")
        .filter(|due| *due != 0)
        .or_else(|| source_i64_from_data(source, "due"))
}

fn imported_anki_today(metadata: &SearchMetadata<'_>, now: u64) -> Option<i64> {
    let today = i64::try_from(scheduler_day_index(now, metadata.day_start_offset_ms)).ok()?;
    Some(today.saturating_sub(metadata.collection_created_at_days?))
}

fn imported_anki_next_day_at(metadata: &SearchMetadata<'_>, now: u64) -> Option<i64> {
    let next_day_start = scheduler_day_start_ms(
        scheduler_day_index(now, metadata.day_start_offset_ms).saturating_add(1),
        metadata.day_start_offset_ms,
    ) / 1000;
    i64::try_from(next_day_start).ok()
}

fn imported_new_card_position(
    progress: Option<&CardProgress>,
    card_sources: &[&ExternalSourceRecord],
) -> Option<i64> {
    if !progress.is_none_or(is_new_progress_overlay) {
        return None;
    }

    card_sources.iter().find_map(|source| {
        let due = source_i64_from_data(source, "due")?;
        if source_i64_from_data(source, "kind").is_some_and(|kind| kind != 0)
            || source_i64_from_data(source, "queue").is_some_and(|queue| queue != 0)
        {
            return None;
        }
        Some(due)
    })
}

fn custom_data_key_matches(key: &str, card_sources: &[&ExternalSourceRecord]) -> bool {
    card_sources
        .iter()
        .any(|source| card_custom_data_value(source, key).is_some())
}

fn custom_data_numeric_matches(
    filter: &CardCustomDataNumericFilter,
    card_sources: &[&ExternalSourceRecord],
) -> bool {
    card_sources.iter().any(|source| {
        card_custom_data_value(source, &filter.key)
            .as_ref()
            .and_then(custom_data_number)
            .is_some_and(|actual| compare_number(actual, filter.operator, filter.value))
    })
}

fn custom_data_string_matches(
    filter: &CardCustomDataStringFilter,
    card_sources: &[&ExternalSourceRecord],
) -> bool {
    card_sources.iter().any(|source| {
        let Some(actual) = card_custom_data_value(source, &filter.key)
            .and_then(|value| custom_data_string(&value))
        else {
            return false;
        };
        match filter.operator {
            ComparisonOperator::Equal => actual == filter.value,
            ComparisonOperator::NotEqual => actual != filter.value,
            _ => false,
        }
    })
}

fn card_custom_data_value(source: &ExternalSourceRecord, key: &str) -> Option<Value> {
    let data = card_data_object(source)?;
    if let Some(custom_data) = data.get("cd") {
        return anki_custom_data_value(custom_data, key);
    }
    data.get(key).cloned()
}

fn anki_custom_data_value(custom_data: &Value, key: &str) -> Option<Value> {
    match custom_data {
        Value::Object(data) => data.get(key).cloned(),
        Value::String(raw) => {
            let Value::Object(data) = serde_json::from_str::<Value>(raw).ok()? else {
                return None;
            };
            data.get(key).cloned()
        }
        _ => None,
    }
}

fn card_data_value(source: &ExternalSourceRecord, key: &str) -> Option<Value> {
    card_data_object(source)?.get(key).cloned()
}

fn card_data_object(source: &ExternalSourceRecord) -> Option<serde_json::Map<String, Value>> {
    let raw = source.data.get("data")?;
    let Value::Object(data) = serde_json::from_str::<Value>(raw).ok()? else {
        return None;
    };
    Some(data)
}

fn card_data_number(source: &ExternalSourceRecord, key: &str) -> Option<f64> {
    card_data_value(source, key)
        .as_ref()
        .and_then(custom_data_number)
}

fn custom_data_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(value) => value.parse::<f64>().ok(),
        _ => None,
    }
}

fn custom_data_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Null => Some("null".to_string()),
        Value::Array(_) | Value::Object(_) => None,
    }
}

fn added_matches(
    card: &Card,
    card_sources: &[&ExternalSourceRecord],
    days: u32,
    metadata: &SearchMetadata<'_>,
    now: u64,
) -> bool {
    let timestamp = imported_anki_card_added_at(card_sources).unwrap_or(card.created_at);
    happened_recently(timestamp, days, metadata, now)
}

fn imported_anki_card_added_at(card_sources: &[&ExternalSourceRecord]) -> Option<u64> {
    card_sources
        .iter()
        .find_map(|source| source.original_id.as_deref()?.parse::<u64>().ok())
}

fn rated_matches(
    reviews: &[&Review],
    filter: RatedFilter,
    metadata: &SearchMetadata<'_>,
    now: u64,
) -> bool {
    reviews.iter().any(|review| {
        !anki_review_is_manual_reschedule(review, metadata)
            && filter.rating.is_none_or(|rating| review.rating == rating)
            && happened_recently(review.reviewed_at, filter.days, metadata, now)
    })
}

fn rescheduled_matches(
    reviews: &[&Review],
    filter: RecentDaysFilter,
    metadata: &SearchMetadata<'_>,
    now: u64,
) -> bool {
    reviews.iter().any(|review| {
        anki_review_is_manual_reschedule(review, metadata)
            && happened_recently(review.reviewed_at, filter.days, metadata, now)
    })
}

fn anki_review_is_manual_reschedule(review: &Review, metadata: &SearchMetadata<'_>) -> bool {
    metadata
        .review_sources_by_id
        .get(review.id.as_str())
        .is_some_and(|sources| {
            sources
                .iter()
                .any(|source| source_i64_from_data(source, "ease") == Some(0))
        })
}

fn first_reviewed_within(
    reviews: &[&Review],
    days: u32,
    metadata: &SearchMetadata<'_>,
    now: u64,
) -> bool {
    reviews
        .iter()
        .filter(|review| !anki_review_is_manual_reschedule(review, metadata))
        .map(|review| review.reviewed_at)
        .min()
        .is_some_and(|reviewed_at| happened_recently(reviewed_at, days, metadata, now))
}

fn happened_recently(timestamp: u64, days: u32, metadata: &SearchMetadata<'_>, now: u64) -> bool {
    timestamp <= now && timestamp >= recent_day_cutoff_ms(days, now, metadata.day_start_offset_ms)
}

fn recent_day_cutoff_ms(days: u32, now: u64, day_start_offset_ms: u64) -> u64 {
    let next_day_start = scheduler_day_start_ms(
        scheduler_day_index(now, day_start_offset_ms).saturating_add(1),
        day_start_offset_ms,
    );
    next_day_start.saturating_sub(u64::from(days.max(1)).saturating_mul(MS_PER_DAY))
}

fn relative_day_bucket(timestamp: u64, now: u64, day_start_offset_ms: u64) -> i32 {
    let timestamp_day = scheduler_day_index(timestamp, day_start_offset_ms);
    let now_day = scheduler_day_index(now, day_start_offset_ms);
    let diff = timestamp_day as i128 - now_day as i128;
    diff.clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32
}

fn scheduler_day_index(timestamp: u64, day_start_offset_ms: u64) -> u64 {
    timestamp.saturating_sub(day_start_offset_ms) / MS_PER_DAY
}

fn scheduler_day_start_ms(day_index: u64, day_start_offset_ms: u64) -> u64 {
    day_index
        .saturating_mul(MS_PER_DAY)
        .saturating_add(day_start_offset_ms)
}

fn compare_number(actual: f64, operator: ComparisonOperator, expected: f64) -> bool {
    match operator {
        ComparisonOperator::Equal => actual == expected,
        ComparisonOperator::NotEqual => actual != expected,
        ComparisonOperator::LessThan => actual < expected,
        ComparisonOperator::LessThanOrEqual => actual <= expected,
        ComparisonOperator::GreaterThan => actual > expected,
        ComparisonOperator::GreaterThanOrEqual => actual >= expected,
    }
}

fn is_suspended(progress: &CardProgress) -> bool {
    progress.suspended_at.is_some() || progress.state == CardState::Suspended
}

fn is_buried(progress: &CardProgress, now: u64) -> bool {
    progress
        .buried_until
        .is_some_and(|buried_until| buried_until > now)
        || progress.state == CardState::Buried
}

fn anki_card_queue_matches(card_sources: &[&ExternalSourceRecord], expected: i64) -> bool {
    card_sources
        .iter()
        .any(|source| source_i64_from_data(source, "queue") == Some(expected))
}

fn note_for_card<'a>(card: &Card, notes_by_id: &'a HashMap<&str, &Note>) -> Option<&'a Note> {
    if let Some(lineage) = &card.lineage {
        return notes_by_id.get(lineage.note_id.as_str()).copied();
    }

    let (note_id, _) = card.id.split_once("::")?;
    notes_by_id
        .get(note_id)
        .copied()
        .filter(|note| note.deck_id == card.deck_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ActiveSessionState, CardLineage, CardTemplate, Deck, DeckOptions, DeckOptionsPreset,
        ExternalSourceRecord, ExternalSourceTarget, FieldDef, NoteFieldValue, NoteType, Review,
        TemplateRequirementMode,
    };
    use crate::sm2::{INITIAL_EASE_FACTOR, ONE_DAY_MS};
    use std::collections::BTreeMap;

    const NOW: u64 = 1_700_000_000_000;

    fn deck(id: &str, name: &str) -> Deck {
        Deck {
            id: id.to_string(),
            name: name.to_string(),
            description: match id {
                "tamil" => "script and vocabulary",
                _ => "romance language vocabulary",
            }
            .to_string(),
            created_at: NOW,
        }
    }

    fn card(id: &str, deck_id: &str, front: &str, back: &str) -> Card {
        Card {
            id: id.to_string(),
            deck_id: deck_id.to_string(),
            front: front.to_string(),
            back: back.to_string(),
            created_at: NOW,
            lineage: None,
        }
    }

    fn progress(card_id: &str, state: CardState, next_due_at: u64) -> CardProgress {
        CardProgress {
            card_id: card_id.to_string(),
            state,
            interval: 1,
            ease_factor: INITIAL_EASE_FACTOR,
            next_due_at,
            learning_step_index: None,
            buried_until: None,
            suspended_at: None,
            times_seen: 1,
            times_correct: 1,
            times_incorrect: 0,
            last_seen_at: NOW - ONE_DAY_MS,
            fsrs_stability: None,
            fsrs_difficulty: None,
            flag: None,
            marked_at: None,
        }
    }

    fn metadata_overlay(card_id: &str) -> CardProgress {
        CardProgress {
            card_id: card_id.to_string(),
            state: CardState::Review,
            interval: 0,
            ease_factor: INITIAL_EASE_FACTOR,
            next_due_at: NOW,
            learning_step_index: None,
            buried_until: None,
            suspended_at: None,
            times_seen: 0,
            times_correct: 0,
            times_incorrect: 0,
            last_seen_at: NOW,
            fsrs_stability: None,
            fsrs_difficulty: None,
            flag: Some(CardFlag::Red),
            marked_at: Some(NOW),
        }
    }

    fn review(id: &str, card_id: &str, rating: Rating, reviewed_at: u64) -> Review {
        Review {
            id: id.to_string(),
            session_id: "session".to_string(),
            card_id: card_id.to_string(),
            rating,
            reviewed_at,
            answer_time_ms: None,
            leech_event: None,
            previous_progress: None,
            resulting_progress: None,
            previous_active_session: None,
            sibling_progress_snapshots: Vec::new(),
        }
    }

    fn note_type() -> NoteType {
        NoteType {
            id: "basic".to_string(),
            name: "Basic".to_string(),
            fields: vec![FieldDef {
                id: "front".to_string(),
                name: "Front".to_string(),
                required: true,
                ordinal: 0,
            }],
            templates: vec![CardTemplate {
                id: "forward".to_string(),
                name: "Forward".to_string(),
                front_template: "{{Front}}".to_string(),
                back_template: "Answer".to_string(),
                deck_id: None,
                required_field_names: vec!["Front".to_string()],
                requirement_mode: TemplateRequirementMode::All,
                ordinal: 0,
            }],
            stylesheet: None,
            created_at: NOW,
            updated_at: NOW,
        }
    }

    fn tagged_note() -> Note {
        Note {
            id: "note".to_string(),
            note_type_id: "basic".to_string(),
            deck_id: "tamil".to_string(),
            fields: vec![NoteFieldValue {
                field_id: "front".to_string(),
                value: "uyir letter".to_string(),
            }],
            tags: vec!["script".to_string(), "tamil".to_string()],
            created_at: NOW,
            updated_at: NOW,
        }
    }

    fn state() -> AppState {
        AppState {
            decks: vec![deck("tamil", "Tamil"), deck("spanish", "Spanish")],
            note_types: vec![note_type()],
            notes: vec![tagged_note()],
            cards: vec![
                card("note::forward", "tamil", "letter-a", "a"),
                card("due", "tamil", "vanakkam", "hello"),
                card("future", "tamil", "nandri", "thanks"),
                card("suspended", "spanish", "hola", "hello"),
                card("buried", "spanish", "gracias", "thanks"),
                card("new", "spanish", "adios", "goodbye"),
            ],
            card_progress: vec![
                progress("due", CardState::Review, NOW - 1),
                progress("future", CardState::Review, NOW + ONE_DAY_MS),
                {
                    let mut progress = progress("suspended", CardState::Review, NOW - 1);
                    progress.suspended_at = Some(NOW - 10);
                    progress
                },
                {
                    let mut progress = progress("buried", CardState::Review, NOW - 1);
                    progress.buried_until = Some(NOW + ONE_DAY_MS);
                    progress
                },
            ],
            sessions: Vec::new(),
            reviews: Vec::new(),
            deck_options: Vec::new(),
            external_sources: Vec::new(),
            media_assets: Vec::new(),
            active_session: None,
        }
    }

    fn ids_for(query: &str) -> Vec<String> {
        search_cards(&state(), query, NOW)
            .unwrap()
            .into_iter()
            .map(|result| result.card.id)
            .collect()
    }

    #[test]
    fn plain_text_search_matches_standalone_cards_and_note_fields() {
        assert_eq!(ids_for("vanakkam"), vec!["due"]);
        assert_eq!(ids_for("uyir"), vec!["note::forward"]);
        assert_eq!(ids_for("\"script and vocabulary\""), Vec::<String>::new());
        assert_eq!(ids_for("script"), Vec::<String>::new());
        assert_eq!(ids_for("tag:script"), vec!["note::forward"]);
        assert_eq!(
            ids_for("deck:tamil"),
            vec!["note::forward", "due", "future"]
        );
    }

    #[test]
    fn anki_browser_text_modifiers_match_words_combining_marks_clozes_and_regex() {
        let note_type = NoteType {
            id: "basic".to_string(),
            name: "Basic".to_string(),
            fields: vec![FieldDef {
                id: "front".to_string(),
                name: "Front".to_string(),
                required: true,
                ordinal: 0,
            }],
            templates: vec![CardTemplate {
                id: "forward".to_string(),
                name: "Forward".to_string(),
                front_template: "{{Front}}".to_string(),
                back_template: "Answer".to_string(),
                deck_id: None,
                required_field_names: vec!["Front".to_string()],
                requirement_mode: TemplateRequirementMode::All,
                ordinal: 0,
            }],
            stylesheet: None,
            created_at: NOW,
            updated_at: NOW,
        };
        let note = |id: &str, front: &str| Note {
            id: id.to_string(),
            note_type_id: "basic".to_string(),
            deck_id: "languages".to_string(),
            fields: vec![NoteFieldValue {
                field_id: "front".to_string(),
                value: front.to_string(),
            }],
            tags: Vec::new(),
            created_at: NOW,
            updated_at: NOW,
        };
        let card_for_note = |note_id: &str| {
            let mut card = card(&format!("{note_id}::forward"), "languages", "front", "back");
            card.lineage = Some(CardLineage {
                note_id: note_id.to_string(),
                note_type_id: "basic".to_string(),
                template_id: "forward".to_string(),
                ordinal: 0,
                cloze_ordinal: None,
            });
            card
        };
        let state = AppState {
            decks: vec![deck("languages", "Languages")],
            note_types: vec![note_type],
            notes: vec![
                note("accent-note", "über café heißen ば"),
                note(
                    "cloze-note",
                    "The {{c1::capital}} of {{c2::France}} carries a mnemonic.",
                ),
            ],
            cards: vec![
                card("dog-word", "languages", "a dog", "canis"),
                card("doggy", "languages", "doggy", "canine"),
                card("underdog", "languages", "underdog", "outsider"),
                card("digits", "languages", "lesson 123", "numbers"),
                card_for_note("accent-note"),
                card_for_note("cloze-note"),
            ],
            ..AppState::default()
        };
        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("w:dog"), vec!["dog-word"]);
        assert_eq!(ids_for("w:dog*"), vec!["dog-word", "doggy"]);
        assert_eq!(ids_for("w:*dog"), vec!["dog-word", "underdog"]);
        assert_eq!(ids_for("nc:uber"), vec!["accent-note::forward"]);
        assert_eq!(ids_for("nc:cafe"), vec!["accent-note::forward"]);
        assert_eq!(ids_for("nc:は"), vec!["accent-note::forward"]);
        assert_eq!(ids_for("nc:heisen"), vec!["accent-note::forward"]);
        assert_eq!(ids_for("\"nc:heißen\""), vec!["accent-note::forward"]);
        assert_eq!(
            ids_for("\"sc:capital of France\""),
            vec!["cloze-note::forward"]
        );
        assert_eq!(ids_for("\"re:^A DOG$\""), vec!["dog-word"]);
        assert_eq!(ids_for("re:\\d{3}"), vec!["digits"]);
    }

    #[test]
    fn standard_text_search_folds_ascii_case_only() {
        let note_type = NoteType {
            id: "basic".to_string(),
            name: "Basic".to_string(),
            fields: vec![FieldDef {
                id: "front".to_string(),
                name: "Front".to_string(),
                required: true,
                ordinal: 0,
            }],
            templates: vec![CardTemplate {
                id: "forward".to_string(),
                name: "Forward".to_string(),
                front_template: "{{Front}}".to_string(),
                back_template: "Answer".to_string(),
                deck_id: None,
                required_field_names: vec!["Front".to_string()],
                requirement_mode: TemplateRequirementMode::All,
                ordinal: 0,
            }],
            stylesheet: None,
            created_at: NOW,
            updated_at: NOW,
        };
        let note = |id: &str, front: &str, tags: Vec<&str>| Note {
            id: id.to_string(),
            note_type_id: "basic".to_string(),
            deck_id: "languages".to_string(),
            fields: vec![NoteFieldValue {
                field_id: "front".to_string(),
                value: front.to_string(),
            }],
            tags: tags.into_iter().map(str::to_string).collect(),
            created_at: NOW,
            updated_at: NOW,
        };
        let card_for_note = |note_id: &str| {
            let mut card = card(&format!("{note_id}::forward"), "languages", "front", "back");
            card.lineage = Some(CardLineage {
                note_id: note_id.to_string(),
                note_type_id: "basic".to_string(),
                template_id: "forward".to_string(),
                ordinal: 0,
                cloze_ordinal: None,
            });
            card
        };
        let state = AppState {
            decks: vec![deck("languages", "Languages")],
            note_types: vec![note_type],
            notes: vec![
                note("ascii-note", "Latin Root", vec!["CaseTag"]),
                note(
                    "greek-note",
                    "\u{03c3}\u{03c5}\u{03c3}\u{03c4}\u{03b7}\u{03bc}\u{03b1}",
                    vec!["\u{03b5}\u{03c4}\u{03b9}"],
                ),
                note(
                    "cyrillic-note",
                    "\u{043c}\u{0438}\u{0440}",
                    vec!["\u{044f}\u{0437}\u{044b}\u{043a}"],
                ),
            ],
            cards: vec![
                card_for_note("ascii-note"),
                card_for_note("greek-note"),
                card_for_note("cyrillic-note"),
            ],
            ..AppState::default()
        };
        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("latin"), vec!["ascii-note::forward"]);
        assert_eq!(ids_for("LATIN"), vec!["ascii-note::forward"]);
        assert_eq!(ids_for("tag:casetag"), vec!["ascii-note::forward"]);
        assert!(ids_for("\u{03a3}\u{03c5}\u{03c3}").is_empty());
        assert_eq!(
            ids_for("\u{03c3}\u{03c5}\u{03c3}"),
            vec!["greek-note::forward"]
        );
        assert!(
            ids_for("front:\u{03a3}\u{03c5}\u{03c3}\u{03c4}\u{03b7}\u{03bc}\u{03b1}").is_empty()
        );
        assert!(ids_for("front:\u{03a3}*").is_empty());
        assert_eq!(
            ids_for("w:\u{03a3}\u{03c5}\u{03c3}*"),
            vec!["greek-note::forward"]
        );
        assert_eq!(
            ids_for("re:\u{03a3}\u{03c5}\u{03c3}"),
            vec!["greek-note::forward"]
        );
        assert!(ids_for("\u{041c}\u{0438}\u{0440}").is_empty());
        assert_eq!(
            ids_for("re:\u{041c}\u{0438}\u{0440}"),
            vec!["cyrillic-note::forward"]
        );
        assert!(ids_for("tag:\u{042f}\u{0437}*").is_empty());
        assert_eq!(
            ids_for("tag:\u{044f}\u{0437}*"),
            vec!["cyrillic-note::forward"]
        );
    }

    #[test]
    fn anki_browser_text_search_uses_raw_html_and_preserved_sort_field() {
        let note_type = NoteType {
            id: "basic".to_string(),
            name: "Basic".to_string(),
            fields: vec![FieldDef {
                id: "front".to_string(),
                name: "Front".to_string(),
                required: true,
                ordinal: 0,
            }],
            templates: vec![CardTemplate {
                id: "forward".to_string(),
                name: "Forward".to_string(),
                front_template: "{{Front}}".to_string(),
                back_template: "Answer".to_string(),
                deck_id: None,
                required_field_names: vec!["Front".to_string()],
                requirement_mode: TemplateRequirementMode::All,
                ordinal: 0,
            }],
            stylesheet: None,
            created_at: NOW,
            updated_at: NOW,
        };
        let note = |id: &str, front: &str| Note {
            id: id.to_string(),
            note_type_id: "basic".to_string(),
            deck_id: "languages".to_string(),
            fields: vec![NoteFieldValue {
                field_id: "front".to_string(),
                value: front.to_string(),
            }],
            tags: Vec::new(),
            created_at: NOW,
            updated_at: NOW,
        };
        let card_for_note = |note_id: &str| {
            let mut card = card(&format!("{note_id}::forward"), "languages", "front", "back");
            card.lineage = Some(CardLineage {
                note_id: note_id.to_string(),
                note_type_id: "basic".to_string(),
                template_id: "forward".to_string(),
                ordinal: 0,
                cloze_ordinal: None,
            });
            card
        };
        let state = AppState {
            decks: vec![deck("languages", "Languages")],
            note_types: vec![note_type],
            notes: vec![
                note("entity-note", "Tamil &amp; Sanskrit"),
                note("literal-amp-note", "&text"),
                note("split-note", "proto-<b>dravidian</b> root"),
                note("media-note", "<img src=\"amma.mp3\">"),
                note("cloze-note", "{{c1::<b>hidden</b>}} clue"),
            ],
            cards: vec![
                card_for_note("entity-note"),
                card_for_note("literal-amp-note"),
                card_for_note("split-note"),
                card_for_note("media-note"),
                card_for_note("cloze-note"),
            ],
            external_sources: vec![ExternalSourceRecord {
                target: ExternalSourceTarget::Note,
                target_id: "split-note".to_string(),
                source: "anki-v11".to_string(),
                original_id: Some("split-note".to_string()),
                data: BTreeMap::from([(
                    "sortField".to_string(),
                    "proto-dravidian root".to_string(),
                )]),
            }],
            ..AppState::default()
        };
        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert!(ids_for(r#""Tamil & Sanskrit""#).is_empty());
        assert_eq!(
            ids_for(r#""Tamil &amp; Sanskrit""#),
            vec!["entity-note::forward"]
        );
        assert_eq!(ids_for("&amp;text"), vec!["literal-amp-note::forward"]);
        assert_eq!(
            ids_for(r#"front:"&amp;text""#),
            vec!["literal-amp-note::forward"]
        );
        assert_eq!(
            ids_for("proto-dravidian"),
            vec!["split-note::forward"],
            "preserved Anki sfld should allow formatted sort-field text to match"
        );
        assert_eq!(ids_for("w:dravidian"), vec!["split-note::forward"]);
        assert_eq!(ids_for("amma.mp3"), vec!["media-note::forward"]);
        assert_eq!(ids_for("sc:hidden"), vec!["cloze-note::forward"]);
        assert!(ids_for(r#"front:"Tamil & Sanskrit""#).is_empty());
        assert_eq!(
            ids_for(r#"front:"Tamil &amp; Sanskrit""#),
            vec!["entity-note::forward"]
        );
        assert!(ids_for(r#"front:"proto-dravidian root""#).is_empty());
        assert_eq!(ids_for("front:*dravidian*"), vec!["split-note::forward"]);
        assert_eq!(ids_for("front:re:&amp;"), vec!["entity-note::forward"]);
        assert_eq!(ids_for("re:<b>dravidian</b>"), vec!["split-note::forward"]);
        assert!(ids_for(r#""re:Tamil & Sanskrit""#).is_empty());
    }

    #[test]
    fn anki_browser_field_filters_use_exact_and_wildcard_matching() {
        let note_type = NoteType {
            id: "basic".to_string(),
            name: "Basic".to_string(),
            fields: vec![
                FieldDef {
                    id: "front".to_string(),
                    name: "Front".to_string(),
                    required: true,
                    ordinal: 0,
                },
                FieldDef {
                    id: "back".to_string(),
                    name: "Back".to_string(),
                    required: false,
                    ordinal: 1,
                },
                FieldDef {
                    id: "extra".to_string(),
                    name: "Extra".to_string(),
                    required: false,
                    ordinal: 2,
                },
            ],
            templates: vec![CardTemplate {
                id: "forward".to_string(),
                name: "Forward".to_string(),
                front_template: "{{Front}}".to_string(),
                back_template: "{{Back}}".to_string(),
                deck_id: None,
                required_field_names: vec!["Front".to_string()],
                requirement_mode: TemplateRequirementMode::All,
                ordinal: 0,
            }],
            stylesheet: None,
            created_at: NOW,
            updated_at: NOW,
        };
        let note = |id: &str, front: &str, back: &str, extra: &str| Note {
            id: id.to_string(),
            note_type_id: "basic".to_string(),
            deck_id: "tamil".to_string(),
            fields: vec![
                NoteFieldValue {
                    field_id: "front".to_string(),
                    value: front.to_string(),
                },
                NoteFieldValue {
                    field_id: "back".to_string(),
                    value: back.to_string(),
                },
                NoteFieldValue {
                    field_id: "extra".to_string(),
                    value: extra.to_string(),
                },
            ],
            tags: Vec::new(),
            created_at: NOW,
            updated_at: NOW,
        };
        let card_for_note = |note_id: &str, front: &str, back: &str| {
            let mut card = card(&format!("{note_id}::forward"), "tamil", front, back);
            card.lineage = Some(CardLineage {
                note_id: note_id.to_string(),
                note_type_id: "basic".to_string(),
                template_id: "forward".to_string(),
                ordinal: 0,
                cloze_ordinal: None,
            });
            card
        };
        let state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            note_types: vec![note_type],
            notes: vec![
                note("dog-note", "a dog", "", "Latin root"),
                note("cat-note", "cat", "tail", ""),
            ],
            cards: vec![
                card_for_note("dog-note", "a dog", ""),
                card_for_note("cat-note", "cat", "tail"),
                card("star-card", "tamil", "foo*bar", " "),
                card("underscore-card", "tamil", "foo_bar", " "),
                card("x-card", "tamil", "fooxbar", " "),
                card("standalone", "tamil", "hola", "hello"),
            ],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert!(ids_for("front:dog").is_empty());
        assert_eq!(ids_for("front:\"a dog\""), vec!["dog-note::forward"]);
        assert_eq!(ids_for("front:*dog*"), vec!["dog-note::forward"]);
        assert_eq!(ids_for("front:a_dog"), vec!["dog-note::forward"]);
        assert_eq!(ids_for("front:foo\\*bar"), vec!["star-card"]);
        assert_eq!(ids_for("front:foo\\_bar"), vec!["underscore-card"]);
        assert_eq!(
            ids_for("front:foo_bar"),
            vec!["star-card", "underscore-card", "x-card"]
        );
        assert_eq!(ids_for("fr*:\"a dog\""), vec!["dog-note::forward"]);
        assert_eq!(ids_for("back:"), vec!["dog-note::forward"]);
        assert_eq!(ids_for("back:_*"), vec!["cat-note::forward", "standalone"]);
        assert_eq!(ids_for("front:hola"), vec!["standalone"]);
        assert_eq!(ids_for("Extra:\"Latin root\""), vec!["dog-note::forward"]);
        assert_eq!(ids_for("Extra:"), vec!["cat-note::forward"]);
        assert_eq!(ids_for("Extra:_*"), vec!["dog-note::forward"]);
    }

    #[test]
    fn anki_browser_field_and_tag_regex_modifiers_search_targeted_values() {
        let note_type = NoteType {
            id: "basic".to_string(),
            name: "Basic".to_string(),
            fields: vec![FieldDef {
                id: "front".to_string(),
                name: "Front".to_string(),
                required: true,
                ordinal: 0,
            }],
            templates: vec![CardTemplate {
                id: "forward".to_string(),
                name: "Forward".to_string(),
                front_template: "{{Front}}".to_string(),
                back_template: "Answer".to_string(),
                deck_id: None,
                required_field_names: vec!["Front".to_string()],
                requirement_mode: TemplateRequirementMode::All,
                ordinal: 0,
            }],
            stylesheet: None,
            created_at: NOW,
            updated_at: NOW,
        };
        let note = |id: &str, front: &str, tags: Vec<&str>| Note {
            id: id.to_string(),
            note_type_id: "basic".to_string(),
            deck_id: "languages".to_string(),
            fields: vec![NoteFieldValue {
                field_id: "front".to_string(),
                value: front.to_string(),
            }],
            tags: tags.into_iter().map(str::to_string).collect(),
            created_at: NOW,
            updated_at: NOW,
        };
        let card_for_note = |note_id: &str| {
            let mut card = card(&format!("{note_id}::forward"), "languages", "front", "back");
            card.lineage = Some(CardLineage {
                note_id: note_id.to_string(),
                note_type_id: "basic".to_string(),
                template_id: "forward".to_string(),
                ordinal: 0,
                cloze_ordinal: None,
            });
            card
        };
        let state = AppState {
            decks: vec![deck("languages", "Languages")],
            note_types: vec![note_type],
            notes: vec![
                note("range-note", "B1 grammar", vec!["lesson-17", "parent"]),
                note(
                    "child-note",
                    "cafe au lait",
                    vec!["lesson-09", "parent::child"],
                ),
                note("dog-note", "a dog", Vec::new()),
            ],
            cards: vec![
                card_for_note("range-note"),
                card_for_note("child-note"),
                card_for_note("dog-note"),
            ],
            ..AppState::default()
        };
        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("front:re:^[a-c]1"), vec!["range-note::forward"]);
        assert!(ids_for("front:re:^[a-c]1$").is_empty());
        assert_eq!(ids_for("front:w:dog"), vec!["dog-note::forward"]);
        assert_eq!(ids_for("front:nc:café"), vec!["child-note::forward"]);
        assert_eq!(ids_for("tag:re:^parent$"), vec!["range-note::forward"]);
        assert_eq!(
            ids_for("\"tag:re:lesson-(1[7-9]|2[0-5])\""),
            vec!["range-note::forward"]
        );
    }

    #[test]
    fn state_filters_find_new_due_suspended_and_buried_cards() {
        assert_eq!(ids_for("state:new"), vec!["note::forward", "new"]);
        assert_eq!(ids_for("is:due"), vec!["due"]);
        assert_eq!(ids_for("is:suspended"), vec!["suspended"]);
        assert_eq!(ids_for("is:buried"), vec!["buried"]);

        let mut state = state();
        state
            .card_progress
            .push(progress("learning", CardState::Learning, NOW - 1));
        state
            .cards
            .push(card("learning", "tamil", "learn", "learn"));
        state
            .card_progress
            .push(progress("relearning", CardState::Relearning, NOW - 1));
        state
            .cards
            .push(card("relearning", "tamil", "relearn", "relearn"));
        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids_for("is:learn"), vec!["learning", "relearning"]);
        assert_eq!(ids_for("is:review"), vec!["due", "future", "relearning"]);
        assert_eq!(ids_for("is:learn -is:review"), vec!["learning"]);
        assert_eq!(ids_for("is:learn is:review"), vec!["relearning"]);
        assert_eq!(ids_for("state:relearn"), vec!["relearning"]);
    }

    #[test]
    fn imported_anki_due_filters_use_queue_and_due_days() {
        let today = 200_i64;
        let created_at_days = (NOW / 1000) as i64 / SECONDS_PER_DAY - today;
        let collection_source = ExternalSourceRecord {
            target: ExternalSourceTarget::Collection,
            target_id: "collection".to_string(),
            source: "anki-v11".to_string(),
            original_id: Some("1".to_string()),
            data: BTreeMap::from([("createdAtDays".to_string(), created_at_days.to_string())]),
        };
        let card_source = |card_id: &str, kind: i64, queue: i64, due: i64, original_due: i64| {
            ExternalSourceRecord {
                target: ExternalSourceTarget::Card,
                target_id: card_id.to_string(),
                source: "anki-v11".to_string(),
                original_id: Some(card_id.to_string()),
                data: BTreeMap::from([
                    ("kind".to_string(), kind.to_string()),
                    ("queue".to_string(), queue.to_string()),
                    ("due".to_string(), due.to_string()),
                    ("originalDue".to_string(), original_due.to_string()),
                ]),
            }
        };
        let now_secs = (NOW / 1000) as i64;
        let next_day_at =
            imported_anki_next_day_at(&SearchMetadata::default(), NOW).expect("next day cutoff");
        let state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            cards: vec![
                card("review-overdue", "tamil", "review", "overdue"),
                card("review-today", "tamil", "review", "today"),
                card("review-future", "tamil", "review", "future"),
                card("day-learn", "tamil", "day", "learn"),
                card("learn-due", "tamil", "learn", "due"),
                card("learn-future", "tamil", "learn", "future"),
                card("learn-tomorrow", "tamil", "learn", "tomorrow"),
                card("filtered-original", "tamil", "filtered", "original"),
            ],
            external_sources: vec![
                collection_source,
                card_source(
                    "review-overdue",
                    ANKI_TYPE_REVIEW,
                    ANKI_QUEUE_REVIEW,
                    today - 2,
                    0,
                ),
                card_source(
                    "review-today",
                    ANKI_TYPE_REVIEW,
                    ANKI_QUEUE_REVIEW,
                    today,
                    0,
                ),
                card_source(
                    "review-future",
                    ANKI_TYPE_REVIEW,
                    ANKI_QUEUE_REVIEW,
                    today + 3,
                    0,
                ),
                card_source("day-learn", ANKI_TYPE_LEARN, ANKI_QUEUE_DAY_LEARN, today, 0),
                card_source(
                    "learn-due",
                    ANKI_TYPE_LEARN,
                    ANKI_QUEUE_LEARN,
                    now_secs - 1,
                    0,
                ),
                card_source(
                    "learn-future",
                    ANKI_TYPE_LEARN,
                    ANKI_QUEUE_LEARN,
                    now_secs + 600,
                    0,
                ),
                card_source(
                    "learn-tomorrow",
                    ANKI_TYPE_LEARN,
                    ANKI_QUEUE_LEARN,
                    next_day_at + SECONDS_PER_DAY,
                    0,
                ),
                card_source(
                    "filtered-original",
                    ANKI_TYPE_REVIEW,
                    ANKI_QUEUE_REVIEW,
                    today + 7,
                    today - 1,
                ),
            ],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };
        let ids_for_context = |query: &str, context: SearchContext<'_>| {
            search_cards_with_context(&state, query, NOW, context)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(
            ids_for("is:due"),
            vec![
                "review-overdue",
                "review-today",
                "day-learn",
                "learn-due",
                "filtered-original"
            ]
        );
        assert_eq!(
            ids_for_context(
                "is:due",
                SearchContext {
                    learn_ahead_secs: Some(600),
                    ..SearchContext::default()
                },
            ),
            vec![
                "review-overdue",
                "review-today",
                "day-learn",
                "learn-due",
                "learn-future",
                "filtered-original"
            ]
        );
        assert!(ids_for("is:new").is_empty());
        assert_eq!(
            ids_for("is:review"),
            vec![
                "review-overdue",
                "review-today",
                "review-future",
                "filtered-original"
            ]
        );
        assert_eq!(
            ids_for("is:learn"),
            vec!["day-learn", "learn-due", "learn-future", "learn-tomorrow"]
        );
        assert_eq!(
            ids_for("prop:due<0"),
            vec!["review-overdue", "filtered-original"]
        );
        assert_eq!(
            ids_for("prop:due=0"),
            vec!["review-today", "day-learn", "learn-due", "learn-future"]
        );
        assert_eq!(
            ids_for("prop:due>0"),
            vec!["review-future", "learn-tomorrow"]
        );
    }

    #[test]
    fn imported_anki_due_filters_use_scheduler_day_context() {
        let day_start_offset_ms = 4 * 60 * 60 * 1000;
        let now = scheduler_day_start_ms(scheduler_day_index(NOW, 0), 0) + 2 * 60 * 60 * 1000;
        let today = 200_i64;
        let created_at_days =
            i64::try_from(scheduler_day_index(now, day_start_offset_ms)).unwrap() - today;
        let collection_source = ExternalSourceRecord {
            target: ExternalSourceTarget::Collection,
            target_id: "collection".to_string(),
            source: "anki-v11".to_string(),
            original_id: Some("1".to_string()),
            data: BTreeMap::from([("createdAtDays".to_string(), created_at_days.to_string())]),
        };
        let card_source = |card_id: &str, due: i64| ExternalSourceRecord {
            target: ExternalSourceTarget::Card,
            target_id: card_id.to_string(),
            source: "anki-v11".to_string(),
            original_id: Some(card_id.to_string()),
            data: BTreeMap::from([
                ("kind".to_string(), ANKI_TYPE_REVIEW.to_string()),
                ("queue".to_string(), ANKI_QUEUE_REVIEW.to_string()),
                ("due".to_string(), due.to_string()),
            ]),
        };
        let state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            cards: vec![
                card("today-review", "tamil", "today", "review"),
                card("tomorrow-review", "tamil", "tomorrow", "review"),
            ],
            external_sources: vec![
                collection_source,
                card_source("today-review", today),
                card_source("tomorrow-review", today + 1),
            ],
            ..AppState::default()
        };
        let ids_for = |query: &str| {
            search_cards_with_context(
                &state,
                query,
                now,
                SearchContext {
                    day_start_offset_ms: Some(day_start_offset_ms),
                    ..SearchContext::default()
                },
            )
            .unwrap()
            .into_iter()
            .map(|result| result.card.id)
            .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("is:due"), vec!["today-review"]);
        assert_eq!(ids_for("prop:due=0"), vec!["today-review"]);
        assert_eq!(ids_for("prop:due=1"), vec!["tomorrow-review"]);
    }

    #[test]
    fn buried_reason_filters_use_imported_anki_queue_metadata() {
        let anki_card_queue = |card_id: &str, queue: i64| ExternalSourceRecord {
            target: ExternalSourceTarget::Card,
            target_id: card_id.to_string(),
            source: "anki-v11".to_string(),
            original_id: Some(card_id.to_string()),
            data: BTreeMap::from([("queue".to_string(), queue.to_string())]),
        };
        let buried_progress = |card_id: &str| {
            let mut progress = progress(card_id, CardState::Buried, NOW + ONE_DAY_MS);
            progress.buried_until = Some(NOW + ONE_DAY_MS);
            progress
        };
        let state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            cards: vec![
                card("manual", "tamil", "manual", "manual"),
                card("sibling", "tamil", "sibling", "sibling"),
                card("generic", "tamil", "generic", "generic"),
            ],
            card_progress: vec![
                buried_progress("manual"),
                buried_progress("sibling"),
                buried_progress("generic"),
            ],
            external_sources: vec![
                anki_card_queue("manual", -2),
                anki_card_queue("sibling", -3),
            ],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("is:buried"), vec!["manual", "sibling", "generic"]);
        assert_eq!(ids_for("is:buried-manually"), vec!["manual"]);
        assert_eq!(ids_for("is:buried-sibling"), vec!["sibling"]);
    }

    #[test]
    fn metadata_only_progress_overlay_searches_as_new_and_flagged() {
        let state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            note_types: Vec::new(),
            notes: Vec::new(),
            cards: vec![card("flagged-new", "tamil", "amma", "mother")],
            card_progress: vec![metadata_overlay("flagged-new")],
            sessions: Vec::new(),
            reviews: Vec::new(),
            deck_options: Vec::new(),
            external_sources: Vec::new(),
            media_assets: Vec::new(),
            active_session: None,
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("is:new"), vec!["flagged-new"]);
        assert_eq!(ids_for("flag:red"), vec!["flagged-new"]);
        assert_eq!(ids_for("flag:1"), vec!["flagged-new"]);
        assert!(ids_for("is:due").is_empty());
        assert!(ids_for("is:review").is_empty());
    }

    #[test]
    fn marked_filters_use_anki_marked_note_tag() {
        let note = |id: &str, tags: Vec<&str>| Note {
            id: id.to_string(),
            note_type_id: "basic".to_string(),
            deck_id: "tamil".to_string(),
            fields: vec![NoteFieldValue {
                field_id: "front".to_string(),
                value: id.to_string(),
            }],
            tags: tags.into_iter().map(str::to_string).collect(),
            created_at: NOW,
            updated_at: NOW,
        };
        let card_for_note = |note_id: &str| {
            let mut card = card(&format!("{note_id}::forward"), "tamil", note_id, note_id);
            card.lineage = Some(CardLineage {
                note_id: note_id.to_string(),
                note_type_id: "basic".to_string(),
                template_id: "forward".to_string(),
                ordinal: 0,
                cloze_ordinal: None,
            });
            card
        };
        let state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            note_types: vec![note_type()],
            notes: vec![
                note("marked-note", vec!["Marked"]),
                note("plain-note", vec![]),
            ],
            cards: vec![card_for_note("marked-note"), card_for_note("plain-note")],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("marked:true"), vec!["marked-note::forward"]);
        assert_eq!(ids_for("is:marked"), vec!["marked-note::forward"]);
        assert_eq!(ids_for("marked:false"), vec!["plain-note::forward"]);
        assert_eq!(ids_for("is:unmarked"), vec!["plain-note::forward"]);
    }

    #[test]
    fn imported_anki_flag_filters_use_preserved_card_flags() {
        let anki_card_flags = |card_id: &str, flags: i64| ExternalSourceRecord {
            target: ExternalSourceTarget::Card,
            target_id: card_id.to_string(),
            source: "anki-v11".to_string(),
            original_id: Some(card_id.to_string()),
            data: BTreeMap::from([("flags".to_string(), flags.to_string())]),
        };
        let state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            cards: vec![
                card("red", "tamil", "red", "red"),
                card("none", "tamil", "none", "none"),
                card("pink", "tamil", "pink", "pink"),
            ],
            external_sources: vec![
                anki_card_flags("red", 1),
                anki_card_flags("none", 0),
                anki_card_flags("pink", 5),
            ],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("flag:red"), vec!["red"]);
        assert_eq!(ids_for("flag:1"), vec!["red"]);
        assert_eq!(ids_for("flag:pink"), vec!["pink"]);
        assert_eq!(ids_for("flag:any"), vec!["red", "pink"]);
        assert_eq!(ids_for("is:flagged"), vec!["red", "pink"]);
        assert_eq!(ids_for("flag:0"), vec!["none"]);
        assert_eq!(ids_for("is:unflagged"), vec!["none"]);
    }

    #[test]
    fn deck_tag_flag_and_mark_filters_compose() {
        let mut state = state();
        state.card_progress[0].flag = Some(CardFlag::Blue);
        state.card_progress[0].marked_at = Some(NOW);

        let ids: Vec<_> = search_cards(&state, "deck:tamil flag:blue marked:true", NOW)
            .unwrap()
            .into_iter()
            .map(|result| result.card.id)
            .collect();
        assert_eq!(ids, vec!["due"]);

        assert_eq!(ids_for("tag:script"), vec!["note::forward"]);
        assert_eq!(ids_for("note:basic"), vec!["note::forward"]);
        assert_eq!(ids_for("note:bas*"), vec!["note::forward"]);
        assert_eq!(ids_for("noteType:basic"), vec!["note::forward"]);
        assert!(ids_for("note:asi").is_empty());
    }

    #[test]
    fn anki_browser_id_and_card_template_filters_match_current_card() {
        let mut state = state();
        state.note_types[0].templates.push(CardTemplate {
            id: "reverse".to_string(),
            name: "Reverse".to_string(),
            front_template: "{{Back}}".to_string(),
            back_template: "{{Front}}".to_string(),
            deck_id: None,
            required_field_names: vec!["Back".to_string()],
            requirement_mode: TemplateRequirementMode::All,
            ordinal: 1,
        });
        state.external_sources.extend([
            ExternalSourceRecord {
                target: ExternalSourceTarget::Card,
                target_id: "due".to_string(),
                source: "anki-v11".to_string(),
                original_id: Some("12345".to_string()),
                data: BTreeMap::new(),
            },
            ExternalSourceRecord {
                target: ExternalSourceTarget::Note,
                target_id: "note".to_string(),
                source: "anki-v11".to_string(),
                original_id: Some("67890".to_string()),
                data: BTreeMap::new(),
            },
        ]);

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(
            ids_for("cid:due,note::forward"),
            vec!["note::forward", "due"]
        );
        assert_eq!(ids_for("cid:12345"), vec!["due"]);
        assert_eq!(ids_for("cardId:DUE"), vec!["due"]);
        assert_eq!(ids_for("nid:note"), vec!["note::forward"]);
        assert_eq!(ids_for("nid:67890"), vec!["note::forward"]);
        assert_eq!(ids_for("note-id:NOTE"), vec!["note::forward"]);
        assert_eq!(ids_for("card:forward"), vec!["note::forward"]);
        assert_eq!(ids_for("card:for*"), vec!["note::forward"]);
        assert_eq!(ids_for("card:0"), vec!["note::forward"]);
        assert_eq!(ids_for("template:Forward"), vec!["note::forward"]);
        assert!(ids_for("card:ward").is_empty());
        assert!(ids_for("card:reverse").is_empty());
    }

    #[test]
    fn anki_browser_mid_and_did_filters_match_imported_ids() {
        let mut note_type = note_type();
        note_type.id = "local-basic".to_string();

        let note = |id: &str, deck_id: &str| Note {
            id: id.to_string(),
            note_type_id: "local-basic".to_string(),
            deck_id: deck_id.to_string(),
            fields: vec![NoteFieldValue {
                field_id: "front".to_string(),
                value: id.to_string(),
            }],
            tags: Vec::new(),
            created_at: NOW,
            updated_at: NOW,
        };
        let card_for_note = |card_id: &str, note_id: &str, deck_id: &str| {
            let mut card = card(card_id, deck_id, note_id, note_id);
            card.lineage = Some(CardLineage {
                note_id: note_id.to_string(),
                note_type_id: "local-basic".to_string(),
                template_id: "forward".to_string(),
                ordinal: 0,
                cloze_ordinal: None,
            });
            card
        };
        let source_record = |target, target_id: &str, original_id: &str| ExternalSourceRecord {
            target,
            target_id: target_id.to_string(),
            source: "anki-v11".to_string(),
            original_id: Some(original_id.to_string()),
            data: BTreeMap::new(),
        };
        let card_source =
            |card_id: &str, deck_id: i64, original_deck_id: i64| ExternalSourceRecord {
                target: ExternalSourceTarget::Card,
                target_id: card_id.to_string(),
                source: "anki-v11".to_string(),
                original_id: Some(card_id.to_string()),
                data: BTreeMap::from([
                    ("deckId".to_string(), deck_id.to_string()),
                    ("originalDeckId".to_string(), original_deck_id.to_string()),
                ]),
            };
        let state = AppState {
            decks: vec![deck("native", "Native"), deck("filtered", "Filtered")],
            note_types: vec![note_type],
            notes: vec![
                note("native-note", "native"),
                note("filtered-note", "filtered"),
            ],
            cards: vec![
                card_for_note("native-card", "native-note", "native"),
                card_for_note("filtered-card", "filtered-note", "filtered"),
            ],
            external_sources: vec![
                source_record(ExternalSourceTarget::NoteType, "local-basic", "101"),
                source_record(ExternalSourceTarget::Deck, "native", "2"),
                ExternalSourceRecord {
                    target: ExternalSourceTarget::Deck,
                    target_id: "filtered".to_string(),
                    source: "anki-v11".to_string(),
                    original_id: Some("3".to_string()),
                    data: BTreeMap::from([("dyn".to_string(), "1".to_string())]),
                },
                card_source("filtered-card", 3, 2),
            ],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("did:2"), vec!["native-card", "filtered-card"]);
        assert_eq!(ids_for("did:3"), vec!["filtered-card"]);
        assert_eq!(ids_for("did:native"), vec!["native-card"]);
        assert_eq!(ids_for("deck:Native"), vec!["native-card", "filtered-card"]);
        assert_eq!(ids_for("deck:Filtered"), vec!["filtered-card"]);
        assert_eq!(ids_for("deck:filtered"), vec!["filtered-card"]);
        assert!(ids_for("did:4").is_empty());
        assert_eq!(ids_for("mid:101"), vec!["native-card", "filtered-card"]);
        assert_eq!(
            ids_for("mid:local-basic"),
            vec!["native-card", "filtered-card"]
        );
        assert!(ids_for("mid:102").is_empty());
    }

    #[test]
    fn deck_current_uses_search_context_or_active_session() {
        let mut state = AppState {
            decks: vec![
                deck("tamil", "Tamil"),
                deck("tamil::script", "Tamil::Script"),
                deck("filtered", "Filtered"),
                deck("spanish", "Spanish"),
            ],
            cards: vec![
                card("tamil-card", "tamil", "amma", "mother"),
                card("tamil-child-card", "tamil::script", "uyir", "vowel"),
                card("filtered-tamil-card", "filtered", "filtered", "original"),
                card("spanish-card", "spanish", "madre", "mother"),
            ],
            external_sources: vec![
                ExternalSourceRecord {
                    target: ExternalSourceTarget::Deck,
                    target_id: "tamil".to_string(),
                    source: "anki-v11".to_string(),
                    original_id: Some("100".to_string()),
                    data: BTreeMap::new(),
                },
                ExternalSourceRecord {
                    target: ExternalSourceTarget::Deck,
                    target_id: "filtered".to_string(),
                    source: "anki-v11".to_string(),
                    original_id: Some("200".to_string()),
                    data: BTreeMap::from([("dyn".to_string(), "1".to_string())]),
                },
                ExternalSourceRecord {
                    target: ExternalSourceTarget::Card,
                    target_id: "filtered-tamil-card".to_string(),
                    source: "anki-v11".to_string(),
                    original_id: Some("300".to_string()),
                    data: BTreeMap::from([
                        ("deckId".to_string(), "200".to_string()),
                        ("originalDeckId".to_string(), "100".to_string()),
                    ]),
                },
            ],
            ..AppState::default()
        };

        let ids_for = |state: &AppState, context: SearchContext<'_>| {
            search_cards_with_context(state, "deck:current", NOW, context)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert!(ids_for(&state, SearchContext::default()).is_empty());
        assert_eq!(
            ids_for(
                &state,
                SearchContext {
                    current_deck_id: Some("tamil"),
                    ..SearchContext::default()
                },
            ),
            vec!["tamil-card", "tamil-child-card", "filtered-tamil-card"]
        );

        state.active_session = Some(ActiveSessionState {
            session_id: "session".to_string(),
            deck_id: "spanish".to_string(),
            queue: Vec::new(),
            current_index: 0,
            current_card_started_at: None,
            revealed: false,
        });

        assert_eq!(
            search_cards(&state, "deck:current", NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>(),
            vec!["spanish-card"]
        );
        assert_eq!(
            ids_for(
                &state,
                SearchContext {
                    current_deck_id: Some("tamil"),
                    ..SearchContext::default()
                },
            ),
            vec!["tamil-card", "tamil-child-card", "filtered-tamil-card"],
            "explicit UI deck context should override active-session fallback"
        );
    }

    #[test]
    fn unqualified_search_skips_imported_anki_excluded_fields() {
        let note_type = NoteType {
            id: "100".to_string(),
            name: "Imported Basic".to_string(),
            fields: vec![
                FieldDef {
                    id: "100:field:0".to_string(),
                    name: "Front".to_string(),
                    required: true,
                    ordinal: 0,
                },
                FieldDef {
                    id: "100:field:1".to_string(),
                    name: "Hidden".to_string(),
                    required: false,
                    ordinal: 1,
                },
            ],
            templates: vec![CardTemplate {
                id: "100:template:0".to_string(),
                name: "Forward".to_string(),
                front_template: "{{Front}}".to_string(),
                back_template: "Answer".to_string(),
                deck_id: None,
                required_field_names: vec!["Front".to_string()],
                requirement_mode: TemplateRequirementMode::All,
                ordinal: 0,
            }],
            stylesheet: None,
            created_at: NOW,
            updated_at: NOW,
        };
        let note = Note {
            id: "hidden-note".to_string(),
            note_type_id: "100".to_string(),
            deck_id: "languages".to_string(),
            fields: vec![
                NoteFieldValue {
                    field_id: "100:field:0".to_string(),
                    value: "visible".to_string(),
                },
                NoteFieldValue {
                    field_id: "100:field:1".to_string(),
                    value: "secret etymology".to_string(),
                },
            ],
            tags: Vec::new(),
            created_at: NOW,
            updated_at: NOW,
        };
        let mut note_card = card("hidden-note::forward", "languages", "visible", "answer");
        note_card.lineage = Some(CardLineage {
            note_id: "hidden-note".to_string(),
            note_type_id: "100".to_string(),
            template_id: "100:template:0".to_string(),
            ordinal: 0,
            cloze_ordinal: None,
        });
        let state = AppState {
            decks: vec![deck("languages", "Languages")],
            note_types: vec![note_type],
            notes: vec![note],
            cards: vec![note_card],
            external_sources: vec![ExternalSourceRecord {
                target: ExternalSourceTarget::NoteType,
                target_id: "100".to_string(),
                source: "anki-v11".to_string(),
                original_id: Some("100".to_string()),
                data: BTreeMap::from([(
                    "rawJson".to_string(),
                    r#"{"flds":[{"ord":0,"name":"Front"},{"ord":1,"name":"Hidden","exclude_from_search":true}]}"#
                        .to_string(),
                )]),
            }],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("visible"), vec!["hidden-note::forward"]);
        assert!(ids_for("secret").is_empty());
        assert!(ids_for("re:secret").is_empty());
        assert_eq!(ids_for("Hidden:*secret*"), vec!["hidden-note::forward"]);
    }

    #[test]
    fn anki_browser_duplicate_filter_matches_first_field_text() {
        let mut note_type = note_type();
        note_type.id = "local-basic".to_string();

        let note = |id: &str, first_field: &str, note_type_id: &str| Note {
            id: id.to_string(),
            note_type_id: note_type_id.to_string(),
            deck_id: "tamil".to_string(),
            fields: vec![NoteFieldValue {
                field_id: "front".to_string(),
                value: first_field.to_string(),
            }],
            tags: Vec::new(),
            created_at: NOW,
            updated_at: NOW,
        };
        let card_for_note = |note_id: &str| {
            let mut card = card(&format!("{note_id}::forward"), "tamil", note_id, note_id);
            card.lineage = Some(CardLineage {
                note_id: note_id.to_string(),
                note_type_id: "local-basic".to_string(),
                template_id: "forward".to_string(),
                ordinal: 0,
                cloze_ordinal: None,
            });
            card
        };
        let note_source = |note_id: &str, sort_field: &str| ExternalSourceRecord {
            target: ExternalSourceTarget::Note,
            target_id: note_id.to_string(),
            source: "anki-v11".to_string(),
            original_id: Some(note_id.to_string()),
            data: BTreeMap::from([("sortField".to_string(), sort_field.to_string())]),
        };
        let state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            note_types: vec![
                note_type,
                NoteType {
                    id: "other".to_string(),
                    name: "Other".to_string(),
                    fields: vec![FieldDef {
                        id: "front".to_string(),
                        name: "Front".to_string(),
                        required: true,
                        ordinal: 0,
                    }],
                    templates: Vec::new(),
                    stylesheet: None,
                    created_at: NOW,
                    updated_at: NOW,
                },
            ],
            notes: vec![
                note("plain-note", "Latin root", "local-basic"),
                note("html-note", "<b>Latin root</b>", "local-basic"),
                note("media-note", "<img src='foo.jpg'>", "local-basic"),
                note("entity-note", "Latin &amp; root", "local-basic"),
                note("preserved-note", "changed value", "local-basic"),
                note("other-note", "Latin root", "other"),
            ],
            cards: vec![
                card_for_note("plain-note"),
                card_for_note("html-note"),
                card_for_note("media-note"),
                card_for_note("entity-note"),
                card_for_note("preserved-note"),
                card_for_note("other-note"),
            ],
            external_sources: vec![
                ExternalSourceRecord {
                    target: ExternalSourceTarget::NoteType,
                    target_id: "local-basic".to_string(),
                    source: "anki-v11".to_string(),
                    original_id: Some("101".to_string()),
                    data: BTreeMap::new(),
                },
                note_source("preserved-note", "Preserved root"),
            ],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(
            ids_for("\"dupe:101,Latin root\""),
            vec!["plain-note::forward", "html-note::forward"]
        );
        assert_eq!(
            ids_for("\"dupe:101,<img src='foo.jpg'>\""),
            vec!["media-note::forward"]
        );
        assert_eq!(
            ids_for("\"dupe:101,Latin & root\""),
            vec!["entity-note::forward"]
        );
        assert_eq!(
            ids_for("\"dupe:101,Preserved root\""),
            vec!["preserved-note::forward"]
        );
        assert_eq!(
            ids_for("\"dupe:other,Latin root\""),
            vec!["other-note::forward"]
        );
        assert!(ids_for("dupe:101,missing").is_empty());
    }

    #[test]
    fn anki_browser_none_filtered_and_card_number_filters_match() {
        let mut state = AppState {
            decks: vec![
                deck("regular", "Regular"),
                deck("filtered-deck", "Cram Session"),
            ],
            note_types: vec![note_type()],
            notes: vec![
                Note {
                    id: "tagged-note".to_string(),
                    note_type_id: "basic".to_string(),
                    deck_id: "regular".to_string(),
                    fields: vec![NoteFieldValue {
                        field_id: "front".to_string(),
                        value: "tagged".to_string(),
                    }],
                    tags: vec!["script".to_string()],
                    created_at: NOW,
                    updated_at: NOW,
                },
                Note {
                    id: "untagged-note".to_string(),
                    note_type_id: "basic".to_string(),
                    deck_id: "filtered-deck".to_string(),
                    fields: vec![NoteFieldValue {
                        field_id: "front".to_string(),
                        value: "untagged".to_string(),
                    }],
                    tags: Vec::new(),
                    created_at: NOW,
                    updated_at: NOW,
                },
            ],
            cards: vec![
                {
                    let mut card = card("tagged-note::forward", "regular", "front", "back");
                    card.lineage = Some(CardLineage {
                        note_id: "tagged-note".to_string(),
                        note_type_id: "basic".to_string(),
                        template_id: "forward".to_string(),
                        ordinal: 0,
                        cloze_ordinal: None,
                    });
                    card
                },
                {
                    let mut card = card("tagged-note::reverse", "regular", "back", "front");
                    card.lineage = Some(CardLineage {
                        note_id: "tagged-note".to_string(),
                        note_type_id: "basic".to_string(),
                        template_id: "reverse".to_string(),
                        ordinal: 1,
                        cloze_ordinal: None,
                    });
                    card
                },
                {
                    let mut card = card("untagged-note::forward", "filtered-deck", "front", "back");
                    card.lineage = Some(CardLineage {
                        note_id: "untagged-note".to_string(),
                        note_type_id: "basic".to_string(),
                        template_id: "forward".to_string(),
                        ordinal: 0,
                        cloze_ordinal: None,
                    });
                    card
                },
            ],
            card_progress: vec![{
                let mut progress = progress("tagged-note::forward", CardState::Review, NOW);
                progress.flag = Some(CardFlag::Red);
                progress
            }],
            external_sources: vec![ExternalSourceRecord {
                target: ExternalSourceTarget::Deck,
                target_id: "filtered-deck".to_string(),
                source: "anki".to_string(),
                original_id: Some("3".to_string()),
                data: BTreeMap::from([("dyn".to_string(), "1".to_string())]),
            }],
            ..AppState::default()
        };
        state.note_types[0].templates.push(CardTemplate {
            id: "reverse".to_string(),
            name: "Reverse".to_string(),
            front_template: "{{Back}}".to_string(),
            back_template: "{{Front}}".to_string(),
            deck_id: None,
            required_field_names: vec!["Back".to_string()],
            requirement_mode: TemplateRequirementMode::All,
            ordinal: 1,
        });

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(
            ids_for("tag:*"),
            vec![
                "tagged-note::forward",
                "tagged-note::reverse",
                "untagged-note::forward"
            ]
        );
        assert_eq!(ids_for("tag:none"), vec!["untagged-note::forward"]);
        assert_eq!(ids_for("deck:filtered"), vec!["untagged-note::forward"]);
        assert_eq!(
            ids_for("card:1"),
            vec!["tagged-note::forward", "untagged-note::forward"]
        );
        assert_eq!(ids_for("card:2"), vec!["tagged-note::reverse"]);
        assert_eq!(ids_for("flag:0 card:2"), vec!["tagged-note::reverse"]);
    }

    #[test]
    fn native_filtered_deck_original_key_matches_deck_and_did_filters() {
        let state = AppState {
            decks: vec![
                deck("regular", "Regular"),
                deck("filtered-deck", "Cram Session"),
            ],
            cards: vec![card("card", "filtered-deck", "front", "back")],
            external_sources: vec![
                ExternalSourceRecord {
                    target: ExternalSourceTarget::Deck,
                    target_id: "filtered-deck".to_string(),
                    source: "anki-v11".to_string(),
                    original_id: None,
                    data: BTreeMap::from([("dyn".to_string(), "1".to_string())]),
                },
                ExternalSourceRecord {
                    target: ExternalSourceTarget::Card,
                    target_id: "card".to_string(),
                    source: "anki-v11".to_string(),
                    original_id: None,
                    data: BTreeMap::from([("originalDeckId".to_string(), "regular".to_string())]),
                },
            ],
            ..AppState::default()
        };
        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("deck:filtered"), vec!["card"]);
        assert_eq!(ids_for("deck:Regular"), vec!["card"]);
        assert_eq!(ids_for("did:regular"), vec!["card"]);
    }

    #[test]
    fn anki_browser_tag_and_deck_hierarchy_filters_match() {
        let note = |id: &str, deck_id: &str, tags: Vec<&str>| Note {
            id: id.to_string(),
            note_type_id: "basic".to_string(),
            deck_id: deck_id.to_string(),
            fields: vec![NoteFieldValue {
                field_id: "front".to_string(),
                value: id.to_string(),
            }],
            tags: tags.into_iter().map(str::to_string).collect(),
            created_at: NOW,
            updated_at: NOW,
        };
        let card_for_note = |note_id: &str, deck_id: &str| {
            let mut card = card(&format!("{note_id}::forward"), deck_id, note_id, note_id);
            card.lineage = Some(CardLineage {
                note_id: note_id.to_string(),
                note_type_id: "basic".to_string(),
                template_id: "forward".to_string(),
                ordinal: 0,
                cloze_ordinal: None,
            });
            card
        };
        let state = AppState {
            decks: vec![
                deck("french", "French"),
                deck("french-verbs", "French::Verbs"),
                deck("languages-french", "Languages::French"),
            ],
            note_types: vec![note_type()],
            notes: vec![
                note("animal-note", "french", vec!["animal::mammal"]),
                note("accent-note", "french", vec!["U\u{0308}ber::Noun"]),
                note("verb-note", "french-verbs", vec!["grammar"]),
                note("language-note", "languages-french", vec!["language"]),
            ],
            cards: vec![
                card_for_note("animal-note", "french"),
                card_for_note("accent-note", "french"),
                card_for_note("verb-note", "french-verbs"),
                card_for_note("language-note", "languages-french"),
            ],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("tag:animal"), vec!["animal-note::forward"]);
        assert_eq!(ids_for("tag:animal::*"), vec!["animal-note::forward"]);
        assert!(ids_for("tag:uber").is_empty());
        assert_eq!(ids_for("tag:nc:uber"), vec!["accent-note::forward"]);
        assert_eq!(ids_for("tag:nc:uber::*"), vec!["accent-note::forward"]);
        assert_eq!(
            ids_for("deck:french"),
            vec![
                "animal-note::forward",
                "accent-note::forward",
                "verb-note::forward"
            ]
        );
        assert_eq!(ids_for("deck:french::*"), vec!["verb-note::forward"]);
        assert_eq!(
            ids_for("deck:*french"),
            vec![
                "animal-note::forward",
                "accent-note::forward",
                "language-note::forward"
            ]
        );
    }

    #[test]
    fn anki_browser_preset_filter_matches_imported_and_native_deck_options() {
        let state = AppState {
            decks: vec![
                deck("story", "Spanish::Latin"),
                deck("filtered", "Filtered"),
                deck("defaulted", "Tamil"),
                deck("native", "Native Deck"),
            ],
            cards: vec![
                card("story-card", "story", "aqua", "water"),
                card("filtered-story-card", "filtered", "cram", "story"),
                card("default-card", "defaulted", "vanakkam", "hello"),
                card("native-card", "native", "custom", "options"),
            ],
            deck_options: vec![DeckOptionsPreset {
                deck_id: "native".to_string(),
                options: DeckOptions::default(),
            }],
            external_sources: vec![
                ExternalSourceRecord {
                    target: ExternalSourceTarget::Collection,
                    target_id: "collection".to_string(),
                    source: "anki-v11".to_string(),
                    original_id: Some("1".to_string()),
                    data: BTreeMap::from([(
                        "deckConfigJson".to_string(),
                        r#"{"1":{"id":1,"name":"Default"},"7":{"id":7,"name":"Story defaults"}}"#
                            .to_string(),
                    )]),
                },
                ExternalSourceRecord {
                    target: ExternalSourceTarget::Deck,
                    target_id: "story".to_string(),
                    source: "anki-v11".to_string(),
                    original_id: Some("2".to_string()),
                    data: BTreeMap::from([(
                        "rawJson".to_string(),
                        r#"{"id":2,"name":"Spanish::Latin","conf":7}"#.to_string(),
                    )]),
                },
                ExternalSourceRecord {
                    target: ExternalSourceTarget::Deck,
                    target_id: "defaulted".to_string(),
                    source: "anki-v11".to_string(),
                    original_id: Some("3".to_string()),
                    data: BTreeMap::from([("configId".to_string(), "1".to_string())]),
                },
                ExternalSourceRecord {
                    target: ExternalSourceTarget::Deck,
                    target_id: "filtered".to_string(),
                    source: "anki-v11".to_string(),
                    original_id: Some("4".to_string()),
                    data: BTreeMap::from([("dyn".to_string(), "1".to_string())]),
                },
                ExternalSourceRecord {
                    target: ExternalSourceTarget::Card,
                    target_id: "filtered-story-card".to_string(),
                    source: "anki-v11".to_string(),
                    original_id: Some("5".to_string()),
                    data: BTreeMap::from([
                        ("deckId".to_string(), "4".to_string()),
                        ("originalDeckId".to_string(), "2".to_string()),
                    ]),
                },
            ],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(
            ids_for(r#"preset:"Story defaults""#),
            vec!["story-card", "filtered-story-card"]
        );
        assert_eq!(
            ids_for("preset:Story*"),
            vec!["story-card", "filtered-story-card"]
        );
        assert_eq!(ids_for("preset:Default"), vec!["default-card"]);
        assert_eq!(ids_for(r#"preset:"Native Deck""#), vec!["native-card"]);
        assert!(ids_for("preset:Spanish").is_empty());
    }

    #[test]
    fn property_filters_match_progress_metrics() {
        let mut state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            cards: vec![
                card("new", "tamil", "new", "new"),
                card("due", "tamil", "due", "due"),
                card("lapsed", "tamil", "lapsed", "lapsed"),
                card("low-ease", "tamil", "ease", "ease"),
            ],
            card_progress: vec![
                {
                    let mut progress = progress("due", CardState::Review, NOW - 2 * ONE_DAY_MS);
                    progress.interval = 12;
                    progress.times_seen = 5;
                    progress
                },
                {
                    let mut progress = progress("lapsed", CardState::Review, NOW + ONE_DAY_MS);
                    progress.interval = 3;
                    progress.times_seen = 8;
                    progress.times_incorrect = 2;
                    progress
                },
                {
                    let mut progress = progress("low-ease", CardState::Review, NOW);
                    progress.interval = 1;
                    progress.ease_factor = 2.1;
                    progress
                },
            ],
            ..AppState::default()
        };

        let ids_for = |state: &AppState, query: &str| {
            search_cards(state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for(&state, "prop:ivl>=10"), vec!["due"]);
        assert_eq!(ids_for(&state, "prop:due<0"), vec!["due"]);
        assert_eq!(ids_for(&state, "prop:reps=0"), vec!["new"]);
        assert_eq!(ids_for(&state, "prop:lapses>1"), vec!["lapsed"]);
        assert_eq!(ids_for(&state, "prop:ease<2.5"), vec!["low-ease"]);

        state.reviews = vec![
            review("recent", "due", Rating::Good, NOW - ONE_DAY_MS / 2),
            review("older", "lapsed", Rating::Again, NOW - 3 * ONE_DAY_MS),
        ];
        assert_eq!(ids_for(&state, "prop:rated=0"), vec!["due"]);
        assert_eq!(ids_for(&state, "prop:rated<-1"), vec!["lapsed"]);
    }

    #[test]
    fn imported_anki_property_filters_use_card_row_metrics() {
        let anki_card_metrics =
            |card_id: &str, interval: i64, repetitions: i64, lapses: i64, factor: i64| {
                ExternalSourceRecord {
                    target: ExternalSourceTarget::Card,
                    target_id: card_id.to_string(),
                    source: "anki-v11".to_string(),
                    original_id: Some(card_id.to_string()),
                    data: BTreeMap::from([
                        ("interval".to_string(), interval.to_string()),
                        ("repetitions".to_string(), repetitions.to_string()),
                        ("lapses".to_string(), lapses.to_string()),
                        ("factor".to_string(), factor.to_string()),
                    ]),
                }
            };
        let state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            cards: vec![
                card("fresh", "tamil", "fresh", "fresh"),
                card("mature", "tamil", "mature", "mature"),
                card("easy", "tamil", "easy", "easy"),
            ],
            external_sources: vec![
                anki_card_metrics("fresh", 0, 0, 0, 0),
                anki_card_metrics("mature", 21, 6, 1, 2300),
                anki_card_metrics("easy", 5, 2, 0, 2800),
            ],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("prop:ivl>=20"), vec!["mature"]);
        assert_eq!(ids_for("prop:interval=0"), vec!["fresh"]);
        assert_eq!(ids_for("prop:reps=0"), vec!["fresh"]);
        assert_eq!(ids_for("prop:reviews>=6"), vec!["mature"]);
        assert_eq!(ids_for("prop:lapses=1"), vec!["mature"]);
        assert_eq!(ids_for("prop:ease=2.3"), vec!["mature"]);
        assert_eq!(ids_for("prop:ease>2.5"), vec!["easy"]);
    }

    #[test]
    fn fsrs_property_filters_match_imported_anki_card_data() {
        let anki_card_data = |card_id: &str, data: &str| ExternalSourceRecord {
            target: ExternalSourceTarget::Card,
            target_id: card_id.to_string(),
            source: "anki-v11".to_string(),
            original_id: Some(card_id.to_string()),
            data: BTreeMap::from([("data".to_string(), data.to_string())]),
        };
        let mut native_progress = progress("native", CardState::Review, NOW);
        native_progress.fsrs_stability = Some(6.0);
        native_progress.fsrs_difficulty = Some(7.3);
        native_progress.last_seen_at = NOW - ONE_DAY_MS;
        let state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            cards: vec![
                card("stable", "tamil", "stable", "stable"),
                card("difficult", "tamil", "difficult", "difficult"),
                card("native", "tamil", "native", "native"),
            ],
            card_progress: vec![
                progress("stable", CardState::Review, NOW),
                progress("difficult", CardState::Review, NOW),
                native_progress,
            ],
            external_sources: vec![
                anki_card_data(
                    "stable",
                    &format!(r#"{{"s":12.5,"d":5.5,"lrt":{}}}"#, NOW / 1000 - 86_400),
                ),
                anki_card_data(
                    "difficult",
                    &format!(r#"{{"s":2.0,"d":8.2,"lrt":{}}}"#, NOW / 1000 - 864_000),
                ),
            ],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("prop:s>10"), vec!["stable"]);
        assert_eq!(ids_for("prop:stability<=2"), vec!["difficult"]);
        assert_eq!(ids_for("prop:stability=6"), vec!["native"]);
        assert_eq!(ids_for("prop:d=0.5"), vec!["stable"]);
        assert_eq!(ids_for("prop:difficulty=0.7"), vec!["native"]);
        assert_eq!(ids_for("prop:difficulty>0.7"), vec!["difficult"]);
        assert_eq!(ids_for("prop:r>0.98"), vec!["stable"]);
        assert_eq!(
            ids_for("prop:retrievability>0.95"),
            vec!["stable", "native"]
        );
        assert_eq!(ids_for("prop:retrievability<0.91"), vec!["difficult"]);
        assert!(ids_for("prop:s=0").is_empty());
        assert!(ids_for("prop:r=1").is_empty());
    }

    #[test]
    fn position_property_matches_imported_anki_new_card_positions() {
        let anki_card_source =
            |card_id: &str, kind: i64, queue: i64, due: i64| ExternalSourceRecord {
                target: ExternalSourceTarget::Card,
                target_id: card_id.to_string(),
                source: "anki-v11".to_string(),
                original_id: Some(card_id.to_string()),
                data: BTreeMap::from([
                    ("kind".to_string(), kind.to_string()),
                    ("queue".to_string(), queue.to_string()),
                    ("due".to_string(), due.to_string()),
                ]),
            };
        let state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            cards: vec![
                card("new-early", "tamil", "early", "position"),
                card("new-late", "tamil", "late", "position"),
                card("review", "tamil", "review", "position"),
                card("native-new", "tamil", "native", "new"),
            ],
            card_progress: vec![progress("review", CardState::Review, NOW)],
            external_sources: vec![
                anki_card_source("new-early", 0, 0, 42),
                anki_card_source("new-late", 0, 0, 175),
                anki_card_source("review", 2, 2, 20),
            ],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("prop:pos<=100"), vec!["new-early"]);
        assert_eq!(ids_for("prop:position>100"), vec!["new-late"]);
        assert!(ids_for("prop:pos=20").is_empty());
        assert!(ids_for("prop:pos=0").is_empty());
    }

    #[test]
    fn custom_data_filters_match_imported_anki_card_data() {
        let anki_card_data = |card_id: &str, data: &str| ExternalSourceRecord {
            target: ExternalSourceTarget::Card,
            target_id: card_id.to_string(),
            source: "anki-v11".to_string(),
            original_id: Some(card_id.to_string()),
            data: BTreeMap::from([("data".to_string(), data.to_string())]),
        };
        let state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            cards: vec![
                card("rescheduled", "tamil", "one", "one"),
                card("manual", "tamil", "two", "two"),
                card("nested", "tamil", "two and a half", "two and a half"),
                card("invalid", "tamil", "three", "three"),
                card("plain", "tamil", "four", "four"),
            ],
            external_sources: vec![
                anki_card_data(
                    "rescheduled",
                    r#"{"v":"reschedule","d":6.25,"n":"9","enabled":true,"empty":null}"#,
                ),
                anki_card_data("manual", r#"{"v":"manual","d":4}"#),
                anki_card_data(
                    "nested",
                    r#"{"s":30,"d":6.5,"cd":"{\"v\":\"nested\",\"n\":11,\"enabled\":false}"}"#,
                ),
                anki_card_data("invalid", "not-json"),
            ],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("has-cd:v"), vec!["rescheduled", "manual", "nested"]);
        assert_eq!(ids_for("has-cd:empty"), vec!["rescheduled"]);
        assert_eq!(ids_for("prop:cdn:d>5"), vec!["rescheduled"]);
        assert_eq!(ids_for("prop:cdn:n=9"), vec!["rescheduled"]);
        assert_eq!(ids_for("prop:cdn:n=11"), vec!["nested"]);
        assert_eq!(ids_for("prop:cds:v=reschedule"), vec!["rescheduled"]);
        assert_eq!(ids_for("prop:cds:v=nested"), vec!["nested"]);
        assert_eq!(ids_for("prop:cds:v!=reschedule"), vec!["manual", "nested"]);
        assert_eq!(ids_for("prop:cds:enabled=true"), vec!["rescheduled"]);
        assert_eq!(ids_for("prop:cds:enabled=false"), vec!["nested"]);
        assert!(ids_for("has-cd:cd").is_empty());
        assert!(ids_for("has-cd:missing").is_empty());
    }

    #[test]
    fn imported_anki_added_filter_uses_original_card_id_timestamp() {
        let anki_card_source = |card_id: &str, original_id: u64| ExternalSourceRecord {
            target: ExternalSourceTarget::Card,
            target_id: card_id.to_string(),
            source: "anki-v11".to_string(),
            original_id: Some(original_id.to_string()),
            data: BTreeMap::new(),
        };
        let state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            cards: vec![
                card("recent-import", "tamil", "recent", "import"),
                card("older-import", "tamil", "older", "import"),
                card("native", "tamil", "native", "card"),
            ],
            external_sources: vec![
                anki_card_source("recent-import", NOW - ONE_DAY_MS / 2),
                anki_card_source("older-import", NOW - 2 * ONE_DAY_MS),
            ],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("added:1"), vec!["recent-import", "native"]);
        assert_eq!(
            ids_for("added:3"),
            vec!["recent-import", "older-import", "native"]
        );
    }

    #[test]
    fn recent_event_filters_match_added_edited_introduced_and_rated_cards() {
        let mut state = state();
        state.cards[1].created_at = NOW - 3 * ONE_DAY_MS;
        state.notes[0].updated_at = NOW - ONE_DAY_MS / 2;
        state.reviews = vec![
            review("recent-good", "due", Rating::Good, NOW - ONE_DAY_MS / 2),
            review("older-again", "future", Rating::Again, NOW - 3 * ONE_DAY_MS),
            review("older-easy", "buried", Rating::Easy, NOW - 2 * ONE_DAY_MS),
        ];

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(
            ids_for("added:1"),
            vec!["note::forward", "future", "suspended", "buried", "new"]
        );
        assert_eq!(ids_for("added:0"), ids_for("added:1"));
        assert_eq!(ids_for("edited:1"), vec!["note::forward"]);
        assert_eq!(ids_for("edited:0"), ids_for("edited:1"));
        assert_eq!(ids_for("introduced:1"), vec!["due"]);
        assert_eq!(ids_for("introduced:0"), ids_for("introduced:1"));
        assert_eq!(ids_for("rated:0"), ids_for("rated:1"));
        assert_eq!(ids_for("rated:1:3"), vec!["due"]);
        assert_eq!(ids_for("rated:4:again"), vec!["future"]);
        assert_eq!(ids_for("prop:rated=0:good"), vec!["due"]);
        assert_eq!(ids_for("prop:rated<-1:again"), vec!["future"]);
    }

    #[test]
    fn recent_event_filters_use_anki_day_boundaries() {
        let day_start = (NOW / ONE_DAY_MS) * ONE_DAY_MS;
        let now = day_start + ONE_DAY_MS / 2;
        let just_before_today = day_start - 1;

        let mut today_note = tagged_note();
        today_note.id = "today-note".to_string();
        today_note.updated_at = day_start;
        let mut yesterday_note = tagged_note();
        yesterday_note.id = "yesterday-note".to_string();
        yesterday_note.updated_at = just_before_today;

        let card_for_note = |note_id: &str| {
            let mut card = card(&format!("{note_id}::forward"), "tamil", note_id, "answer");
            card.created_at = just_before_today;
            card.lineage = Some(CardLineage {
                note_id: note_id.to_string(),
                note_type_id: "basic".to_string(),
                template_id: "forward".to_string(),
                ordinal: 0,
                cloze_ordinal: None,
            });
            card
        };
        let mut today_added = card("today-added", "tamil", "today", "added");
        today_added.created_at = day_start;
        let mut yesterday_added = card("yesterday-added", "tamil", "yesterday", "added");
        yesterday_added.created_at = just_before_today;
        let mut today_reviewed = card("today-reviewed", "tamil", "today", "reviewed");
        today_reviewed.created_at = just_before_today;
        let mut yesterday_reviewed = card("yesterday-reviewed", "tamil", "yesterday", "reviewed");
        yesterday_reviewed.created_at = just_before_today;

        let state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            note_types: vec![note_type()],
            notes: vec![today_note, yesterday_note],
            cards: vec![
                today_added,
                yesterday_added,
                card_for_note("today-note"),
                card_for_note("yesterday-note"),
                today_reviewed,
                yesterday_reviewed,
            ],
            reviews: vec![
                review("today-good", "today-reviewed", Rating::Good, day_start),
                review(
                    "yesterday-good",
                    "yesterday-reviewed",
                    Rating::Good,
                    just_before_today,
                ),
            ],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, now)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("added:1"), vec!["today-added"]);
        assert_eq!(ids_for("edited:1"), vec!["today-note::forward"]);
        assert_eq!(ids_for("rated:1:good"), vec!["today-reviewed"]);
        assert_eq!(ids_for("introduced:1"), vec!["today-reviewed"]);
        assert_eq!(ids_for("prop:rated=0:good"), vec!["today-reviewed"]);
        assert_eq!(ids_for("prop:rated=-1:good"), vec!["yesterday-reviewed"]);
        assert_eq!(
            ids_for("rated:2:good"),
            vec!["today-reviewed", "yesterday-reviewed"]
        );
    }

    #[test]
    fn recent_event_filters_use_scheduler_day_cutoff_context() {
        let cutoff_offset = 4 * 60 * 60 * 1000;
        let day_start = (NOW / ONE_DAY_MS) * ONE_DAY_MS;
        let now = day_start + 3 * 60 * 60 * 1000;
        let scheduler_today_start = day_start + cutoff_offset - ONE_DAY_MS;
        let just_before_scheduler_today = scheduler_today_start - 1;

        let mut inside_note = tagged_note();
        inside_note.id = "inside-note".to_string();
        inside_note.updated_at = scheduler_today_start;
        let mut outside_note = tagged_note();
        outside_note.id = "outside-note".to_string();
        outside_note.updated_at = just_before_scheduler_today;

        let card_for_note = |note_id: &str| {
            let mut card = card(&format!("{note_id}::forward"), "tamil", note_id, "answer");
            card.created_at = just_before_scheduler_today;
            card.lineage = Some(CardLineage {
                note_id: note_id.to_string(),
                note_type_id: "basic".to_string(),
                template_id: "forward".to_string(),
                ordinal: 0,
                cloze_ordinal: None,
            });
            card
        };
        let mut inside_added = card("inside-added", "tamil", "inside", "added");
        inside_added.created_at = scheduler_today_start;
        let mut outside_added = card("outside-added", "tamil", "outside", "added");
        outside_added.created_at = just_before_scheduler_today;
        let old_card = |id: &str| {
            let mut card = card(id, "tamil", id, id);
            card.created_at = just_before_scheduler_today;
            card
        };

        let state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            note_types: vec![note_type()],
            notes: vec![inside_note, outside_note],
            cards: vec![
                inside_added,
                outside_added,
                card_for_note("inside-note"),
                card_for_note("outside-note"),
                old_card("inside-reviewed"),
                old_card("outside-reviewed"),
                old_card("manual-rescheduled"),
            ],
            reviews: vec![
                review(
                    "inside-good",
                    "inside-reviewed",
                    Rating::Good,
                    scheduler_today_start,
                ),
                review(
                    "outside-good",
                    "outside-reviewed",
                    Rating::Good,
                    just_before_scheduler_today,
                ),
                review(
                    "manual-rescheduled-review",
                    "manual-rescheduled",
                    Rating::Good,
                    scheduler_today_start,
                ),
            ],
            external_sources: vec![ExternalSourceRecord {
                target: ExternalSourceTarget::Review,
                target_id: "manual-rescheduled-review".to_string(),
                source: "anki-v11".to_string(),
                original_id: Some("manual-rescheduled-review".to_string()),
                data: BTreeMap::from([("ease".to_string(), "0".to_string())]),
            }],
            ..AppState::default()
        };
        let ids_for = |query: &str| {
            search_cards_with_context(
                &state,
                query,
                now,
                SearchContext {
                    day_start_offset_ms: Some(cutoff_offset),
                    ..SearchContext::default()
                },
            )
            .unwrap()
            .into_iter()
            .map(|result| result.card.id)
            .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("added:1"), vec!["inside-added"]);
        assert_eq!(ids_for("edited:1"), vec!["inside-note::forward"]);
        assert_eq!(ids_for("rated:1:good"), vec!["inside-reviewed"]);
        assert_eq!(ids_for("introduced:1"), vec!["inside-reviewed"]);
        assert_eq!(ids_for("prop:rated=0:good"), vec!["inside-reviewed"]);
        assert_eq!(ids_for("prop:rated=-1:good"), vec!["outside-reviewed"]);
        assert_eq!(ids_for("resched:1"), vec!["manual-rescheduled"]);
        assert_eq!(ids_for("prop:resched=0"), vec!["manual-rescheduled"]);
    }

    #[test]
    fn rescheduled_filters_use_imported_anki_manual_reschedule_reviews() {
        let anki_review_source = |review_id: &str, ease: i64| ExternalSourceRecord {
            target: ExternalSourceTarget::Review,
            target_id: review_id.to_string(),
            source: "anki-v11".to_string(),
            original_id: Some(review_id.to_string()),
            data: BTreeMap::from([("ease".to_string(), ease.to_string())]),
        };
        let state = AppState {
            decks: vec![deck("tamil", "Tamil")],
            cards: vec![
                card("manual", "tamil", "manual", "rescheduled"),
                card("answered", "tamil", "answered", "normally"),
                card("old-manual", "tamil", "old", "rescheduled"),
                card("native", "tamil", "native", "review"),
            ],
            reviews: vec![
                review(
                    "manual-recent",
                    "manual",
                    Rating::Good,
                    NOW - ONE_DAY_MS / 2,
                ),
                review(
                    "answered-recent",
                    "answered",
                    Rating::Good,
                    NOW - ONE_DAY_MS / 2,
                ),
                review(
                    "manual-old",
                    "old-manual",
                    Rating::Good,
                    NOW - 3 * ONE_DAY_MS,
                ),
                review("native-good", "native", Rating::Good, NOW - ONE_DAY_MS / 2),
            ],
            external_sources: vec![
                anki_review_source("manual-recent", 0),
                anki_review_source("answered-recent", 3),
                anki_review_source("manual-old", 0),
            ],
            ..AppState::default()
        };

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("resched:1"), vec!["manual"]);
        assert_eq!(ids_for("resched:0"), ids_for("resched:1"));
        assert_eq!(ids_for("introduced:1"), vec!["answered", "native"]);
        assert_eq!(ids_for("prop:resched=0"), vec!["manual"]);
        assert_eq!(ids_for("prop:rescheduled<-1"), vec!["old-manual"]);
        assert_eq!(ids_for("rated:1"), vec!["answered", "native"]);
        assert_eq!(ids_for("rated:1:good"), vec!["answered", "native"]);
        assert_eq!(ids_for("prop:rated=0"), vec!["answered", "native"]);
        assert_eq!(ids_for("prop:rated=0:good"), vec!["answered", "native"]);
    }

    #[test]
    fn imported_anki_cards_search_through_lineage_notes() {
        let mut state = state();
        let mut imported = card("2000", "spanish", "hola", "hello");
        imported.lineage = Some(CardLineage {
            note_id: "note".to_string(),
            note_type_id: "basic".to_string(),
            template_id: "forward".to_string(),
            ordinal: 0,
            cloze_ordinal: None,
        });
        state.cards = vec![imported];

        let ids_for = |query: &str| {
            search_cards(&state, query, NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>()
        };

        assert_eq!(ids_for("tag:script"), vec!["2000"]);
        assert_eq!(ids_for("note:basic"), vec!["2000"]);
        assert_eq!(ids_for("nid:note"), vec!["2000"]);
        assert_eq!(ids_for("card:forward"), vec!["2000"]);
        assert_eq!(ids_for("uyir"), vec!["2000"]);
    }

    #[test]
    fn negated_clauses_exclude_matches() {
        assert_eq!(ids_for("deck:spanish -is:buried"), vec!["suspended", "new"]);
    }

    #[test]
    fn or_groups_match_either_side_in_source_order() {
        assert_eq!(
            ids_for("deck:tamil OR is:buried"),
            vec!["note::forward", "due", "future", "buried"]
        );
        assert_eq!(
            ids_for("front:vanakkam OR back:goodbye"),
            vec!["due", "new"]
        );
    }

    #[test]
    fn parenthesized_groups_compose_with_implicit_and_and_negation() {
        assert_eq!(
            ids_for("deck:spanish (front:hola OR front:adios)"),
            vec!["suspended", "new"]
        );
        assert_eq!(
            ids_for("deck:tamil -(front:nandri OR is:due)"),
            vec!["note::forward"]
        );
        assert_eq!(
            ids_for("(deck:tamil tag:script) OR (deck:spanish is:new)"),
            vec!["note::forward", "new"]
        );
    }

    #[test]
    fn parser_reports_syntax_errors_and_treats_unknown_keys_as_fields() {
        assert!(search_cards(&state(), "kind:review", NOW)
            .unwrap()
            .is_empty());

        let mut escaped_state = state();
        escaped_state.decks.push(deck("punct", ":"));
        escaped_state
            .cards
            .push(card("colon-deck", "punct", "literal", "deck"));
        escaped_state
            .cards
            .push(card("dash-front", "tamil", "-lead", "dash"));
        assert_eq!(
            search_cards(&escaped_state, "deck:\\:", NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>(),
            vec!["colon-deck"]
        );
        assert_eq!(
            search_cards(&escaped_state, "\\-lead", NOW)
                .unwrap()
                .into_iter()
                .map(|result| result.card.id)
                .collect::<Vec<_>>(),
            vec!["dash-front"]
        );

        let error = search_cards(&state(), "\"vanakkam", NOW).unwrap_err();
        assert_eq!(error.message, "unterminated quoted string");

        let error = search_cards(&state(), "\"\"", NOW).unwrap_err();
        assert_eq!(error.message, "empty quoted string");

        let error = search_cards(&state(), "\\%", NOW).unwrap_err();
        assert_eq!(error.message, "unknown escape sequence: \\%");

        let error = search_cards(&state(), "OR deck:tamil", NOW).unwrap_err();
        assert_eq!(error.message, "OR operator is missing a left-hand clause");

        let error = search_cards(&state(), "deck:tamil OR", NOW).unwrap_err();
        assert_eq!(error.message, "OR operator is missing a right-hand clause");

        let error = search_cards(&state(), "(deck:tamil OR is:due", NOW).unwrap_err();
        assert_eq!(error.message, "missing closing parenthesis");

        let error = search_cards(&state(), "deck:tamil)", NOW).unwrap_err();
        assert_eq!(error.message, "unexpected closing parenthesis");

        let error = search_cards(&state(), "dupe:101", NOW).unwrap_err();
        assert_eq!(error.token, "dupe:101");
    }
}
