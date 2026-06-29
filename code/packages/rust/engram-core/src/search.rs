use std::collections::HashMap;

use crate::model::{
    AppState, Card, CardFlag, CardProgress, CardState, Deck, Note, NoteType, Rating, Review,
};
use crate::queue::{is_new_progress_overlay, is_reviewable};
use crate::sm2::ONE_DAY_MS as MS_PER_DAY;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, PartialEq)]
struct SearchClause {
    kind: SearchClauseKind,
    negated: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum SearchClauseKind {
    Text(String),
    Front(String),
    Back(String),
    CardId(IdFilter),
    NoteId(IdFilter),
    CardTemplate(String),
    Deck(String),
    NoteType(String),
    Tag(String),
    State(CardSearchState),
    Flag(FlagFilter),
    Marked(bool),
    Property(CardPropertyFilter),
    Added(RecentDaysFilter),
    Edited(RecentDaysFilter),
    Introduced(RecentDaysFilter),
    Rated(RatedFilter),
}

#[derive(Clone, Debug, PartialEq)]
enum SearchExpr {
    Clause(SearchClause),
    And(Vec<SearchExpr>),
    Or(Vec<SearchExpr>),
    Not(Box<SearchExpr>),
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
}

#[derive(Clone, Debug, PartialEq)]
struct CardPropertyFilter {
    property: CardProperty,
    operator: ComparisonOperator,
    value: f64,
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

pub fn search_cards(
    state: &AppState,
    query: &str,
    now: u64,
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

    for ch in query.chars() {
        if escaping {
            current.push(ch);
            escaping = false;
            continue;
        }

        match ch {
            '\\' if in_quotes => escaping = true,
            '"' => in_quotes = !in_quotes,
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

    if escaping {
        current.push('\\');
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

    let kind = match raw.split_once(':') {
        Some((key, value)) => parse_keyed_clause(raw, key, value)?,
        None => SearchClauseKind::Text(raw.to_lowercase()),
    };

    Ok(SearchClause { kind, negated })
}

fn parse_keyed_clause(
    token: &str,
    key: &str,
    value: &str,
) -> Result<SearchClauseKind, SearchError> {
    if value.is_empty() {
        return Err(SearchError {
            message: "search filter is missing a value".to_string(),
            token: token.to_string(),
        });
    }

    let value = value.to_lowercase();
    match key.to_ascii_lowercase().as_str() {
        "front" => Ok(SearchClauseKind::Front(value)),
        "back" => Ok(SearchClauseKind::Back(value)),
        "cid" | "cardid" | "card_id" | "card-id" => {
            parse_id_filter(token, &value).map(SearchClauseKind::CardId)
        }
        "nid" | "noteid" | "note_id" | "note-id" => {
            parse_id_filter(token, &value).map(SearchClauseKind::NoteId)
        }
        "card" | "template" | "cardtemplate" | "card_template" | "card-template" => {
            Ok(SearchClauseKind::CardTemplate(value))
        }
        "deck" => Ok(SearchClauseKind::Deck(value)),
        "note" | "notetype" | "note_type" | "note-type" => Ok(SearchClauseKind::NoteType(value)),
        "tag" => Ok(SearchClauseKind::Tag(value)),
        "state" => parse_state_filter(token, &value).map(SearchClauseKind::State),
        "is" => parse_is_filter(token, &value),
        "flag" => parse_flag_filter(token, &value).map(SearchClauseKind::Flag),
        "marked" => parse_bool_filter(token, &value).map(SearchClauseKind::Marked),
        "prop" => parse_property_filter(token, &value).map(SearchClauseKind::Property),
        "added" => parse_recent_days_filter(token, &value).map(SearchClauseKind::Added),
        "edited" => parse_recent_days_filter(token, &value).map(SearchClauseKind::Edited),
        "introduced" => parse_recent_days_filter(token, &value).map(SearchClauseKind::Introduced),
        "rated" => parse_rated_filter(token, &value).map(SearchClauseKind::Rated),
        _ => Err(SearchError {
            message: "unknown search filter".to_string(),
            token: token.to_string(),
        }),
    }
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
        _ => Err(SearchError {
            message: "unknown card state filter".to_string(),
            token: token.to_string(),
        }),
    }
}

fn parse_flag_filter(token: &str, value: &str) -> Result<FlagFilter, SearchError> {
    match value {
        "any" | "flagged" => Ok(FlagFilter::Any),
        "none" | "unflagged" => Ok(FlagFilter::None),
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

fn parse_property_filter(token: &str, value: &str) -> Result<CardPropertyFilter, SearchError> {
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

    let property = match property {
        "ivl" | "interval" => CardProperty::Interval,
        "due" => CardProperty::Due,
        "reps" | "reviews" => CardProperty::Repetitions,
        "lapses" => CardProperty::Lapses,
        "ease" => CardProperty::Ease,
        "rated" => CardProperty::Rated,
        _ => {
            return Err(SearchError {
                message: "unknown card property filter".to_string(),
                token: token.to_string(),
            });
        }
    };
    let value = expected.parse::<f64>().map_err(|_| SearchError {
        message: "property search value must be numeric".to_string(),
        token: token.to_string(),
    })?;

    Ok(CardPropertyFilter {
        property,
        operator,
        value,
    })
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

fn parse_recent_days_filter(token: &str, value: &str) -> Result<RecentDaysFilter, SearchError> {
    let days = value.parse::<u32>().map_err(|_| SearchError {
        message: "recent-event search value must be a whole number of days".to_string(),
        token: token.to_string(),
    })?;

    Ok(RecentDaysFilter { days })
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
        })?;
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
    reviews: &[&Review],
    now: u64,
) -> bool {
    match expression {
        SearchExpr::Clause(clause) => {
            clause_matches(clause, card, progress, deck, note, note_type, reviews, now)
        }
        SearchExpr::And(expressions) => expressions.iter().all(|expression| {
            expression_matches(
                expression, card, progress, deck, note, note_type, reviews, now,
            )
        }),
        SearchExpr::Or(expressions) => expressions.iter().any(|expression| {
            expression_matches(
                expression, card, progress, deck, note, note_type, reviews, now,
            )
        }),
        SearchExpr::Not(expression) => !expression_matches(
            expression, card, progress, deck, note, note_type, reviews, now,
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
    reviews: &[&Review],
    now: u64,
) -> bool {
    let matched = match &clause.kind {
        SearchClauseKind::Text(term) => text_matches(term, card, deck, note),
        SearchClauseKind::Front(term) => contains_case_insensitive(&card.front, term),
        SearchClauseKind::Back(term) => contains_case_insensitive(&card.back, term),
        SearchClauseKind::CardId(filter) => id_filter_matches(filter, &card.id),
        SearchClauseKind::NoteId(filter) => note_id_matches(filter, card, note),
        SearchClauseKind::CardTemplate(term) => card_template_matches(term, card, note_type),
        SearchClauseKind::Deck(term) => deck_matches(term, card, deck),
        SearchClauseKind::NoteType(term) => note_type_matches(term, note, note_type),
        SearchClauseKind::Tag(tag) => note.is_some_and(|note| {
            note.tags
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(tag))
        }),
        SearchClauseKind::State(state) => state_matches(*state, progress, now),
        SearchClauseKind::Flag(filter) => flag_matches(*filter, progress),
        SearchClauseKind::Marked(expected) => {
            progress.is_some_and(|progress| progress.marked_at.is_some()) == *expected
        }
        SearchClauseKind::Property(filter) => property_matches(filter, progress, reviews, now),
        SearchClauseKind::Added(filter) => happened_recently(card.created_at, filter.days, now),
        SearchClauseKind::Edited(filter) => {
            note.is_some_and(|note| happened_recently(note.updated_at, filter.days, now))
        }
        SearchClauseKind::Introduced(filter) => first_reviewed_within(reviews, filter.days, now),
        SearchClauseKind::Rated(filter) => rated_matches(reviews, *filter, now),
    };

    if clause.negated {
        !matched
    } else {
        matched
    }
}

fn text_matches(term: &str, card: &Card, deck: Option<&Deck>, note: Option<&Note>) -> bool {
    contains_case_insensitive(&card.front, term)
        || contains_case_insensitive(&card.back, term)
        || deck.is_some_and(|deck| {
            contains_case_insensitive(&deck.name, term)
                || contains_case_insensitive(&deck.description, term)
        })
        || note.is_some_and(|note| {
            note.tags
                .iter()
                .any(|tag| contains_case_insensitive(tag, term))
                || note
                    .fields
                    .iter()
                    .any(|field| contains_case_insensitive(&field.value, term))
        })
}

fn deck_matches(term: &str, card: &Card, deck: Option<&Deck>) -> bool {
    contains_case_insensitive(&card.deck_id, term)
        || deck.is_some_and(|deck| {
            contains_case_insensitive(&deck.id, term) || contains_case_insensitive(&deck.name, term)
        })
}

fn note_type_matches(term: &str, note: Option<&Note>, note_type: Option<&NoteType>) -> bool {
    note.is_some_and(|note| contains_case_insensitive(&note.note_type_id, term))
        || note_type.is_some_and(|note_type| {
            contains_case_insensitive(&note_type.id, term)
                || contains_case_insensitive(&note_type.name, term)
        })
}

fn id_filter_matches(filter: &IdFilter, value: &str) -> bool {
    filter
        .values
        .iter()
        .any(|expected| value.eq_ignore_ascii_case(expected))
}

fn note_id_matches(filter: &IdFilter, card: &Card, note: Option<&Note>) -> bool {
    card.lineage
        .as_ref()
        .is_some_and(|lineage| id_filter_matches(filter, &lineage.note_id))
        || note.is_some_and(|note| id_filter_matches(filter, &note.id))
}

fn card_template_matches(term: &str, card: &Card, note_type: Option<&NoteType>) -> bool {
    let lineage = card.lineage.as_ref();
    let template_id = lineage
        .map(|lineage| lineage.template_id.as_str())
        .or_else(|| card.id.split_once("::").map(|(_, template_id)| template_id));
    let template_ordinal = lineage.map(|lineage| lineage.ordinal);

    template_id.is_some_and(|template_id| contains_case_insensitive(template_id, term))
        || note_type.is_some_and(|note_type| {
            note_type.templates.iter().any(|template| {
                let is_current_template = template_id
                    .is_some_and(|template_id| template.id.eq_ignore_ascii_case(template_id))
                    || template_ordinal.is_some_and(|ordinal| template.ordinal == ordinal);
                is_current_template
                    && (contains_case_insensitive(&template.id, term)
                        || contains_case_insensitive(&template.name, term))
            })
        })
}

fn state_matches(state: CardSearchState, progress: Option<&CardProgress>, now: u64) -> bool {
    match state {
        CardSearchState::New => progress.map_or(true, is_new_progress_overlay),
        CardSearchState::Due => progress.is_some_and(|progress| is_reviewable(progress, now)),
        CardSearchState::Learning => {
            progress.is_some_and(|progress| progress.state == CardState::Learning)
        }
        CardSearchState::Review => progress.is_some_and(|progress| {
            progress.state == CardState::Review
                && !is_new_progress_overlay(progress)
                && progress.suspended_at.is_none()
                && !is_buried(progress, now)
        }),
        CardSearchState::Relearning => {
            progress.is_some_and(|progress| progress.state == CardState::Relearning)
        }
        CardSearchState::Suspended => progress.is_some_and(is_suspended),
        CardSearchState::Buried => progress.is_some_and(|progress| is_buried(progress, now)),
    }
}

fn flag_matches(filter: FlagFilter, progress: Option<&CardProgress>) -> bool {
    match filter {
        FlagFilter::Any => progress.is_some_and(|progress| progress.flag.is_some()),
        FlagFilter::None => progress.map_or(true, |progress| progress.flag.is_none()),
        FlagFilter::Color(flag) => progress.is_some_and(|progress| progress.flag == Some(flag)),
    }
}

fn property_matches(
    filter: &CardPropertyFilter,
    progress: Option<&CardProgress>,
    reviews: &[&Review],
    now: u64,
) -> bool {
    match filter.property {
        CardProperty::Interval => compare_number(
            progress.map_or(0.0, |progress| f64::from(progress.interval)),
            filter.operator,
            filter.value,
        ),
        CardProperty::Due => progress.is_some_and(|progress| {
            compare_number(
                f64::from(relative_day_bucket(progress.next_due_at, now)),
                filter.operator,
                filter.value,
            )
        }),
        CardProperty::Repetitions => compare_number(
            progress.map_or(0.0, |progress| f64::from(progress.times_seen)),
            filter.operator,
            filter.value,
        ),
        CardProperty::Lapses => compare_number(
            progress.map_or(0.0, |progress| f64::from(progress.times_incorrect)),
            filter.operator,
            filter.value,
        ),
        CardProperty::Ease => progress.is_some_and(|progress| {
            compare_number(progress.ease_factor, filter.operator, filter.value)
        }),
        CardProperty::Rated => reviews.iter().any(|review| {
            compare_number(
                f64::from(relative_day_bucket(review.reviewed_at, now)),
                filter.operator,
                filter.value,
            )
        }),
    }
}

fn rated_matches(reviews: &[&Review], filter: RatedFilter, now: u64) -> bool {
    reviews.iter().any(|review| {
        filter.rating.map_or(true, |rating| review.rating == rating)
            && happened_recently(review.reviewed_at, filter.days, now)
    })
}

fn first_reviewed_within(reviews: &[&Review], days: u32, now: u64) -> bool {
    reviews
        .iter()
        .map(|review| review.reviewed_at)
        .min()
        .is_some_and(|reviewed_at| happened_recently(reviewed_at, days, now))
}

fn happened_recently(timestamp: u64, days: u32, now: u64) -> bool {
    timestamp <= now && now.saturating_sub(timestamp) <= recent_window_ms(days)
}

fn recent_window_ms(days: u32) -> u64 {
    u64::from(days).saturating_mul(MS_PER_DAY)
}

fn relative_day_bucket(timestamp: u64, now: u64) -> i32 {
    let diff = timestamp as i128 - now as i128;
    (diff / i128::from(MS_PER_DAY)) as i32
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

fn contains_case_insensitive(value: &str, term: &str) -> bool {
    value.to_lowercase().contains(term)
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
        CardLineage, CardTemplate, Deck, FieldDef, NoteFieldValue, NoteType, Review,
    };
    use crate::sm2::{INITIAL_EASE_FACTOR, ONE_DAY_MS};

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
                required_field_names: vec!["Front".to_string()],
                ordinal: 0,
            }],
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
    fn plain_text_search_matches_cards_decks_and_note_fields() {
        assert_eq!(ids_for("vanakkam"), vec!["due"]);
        assert_eq!(
            ids_for("\"script and vocabulary\""),
            vec!["note::forward", "due", "future"]
        );
        assert_eq!(ids_for("uyir"), vec!["note::forward"]);
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
        assert_eq!(ids_for("is:learn"), vec!["learning"]);
        assert_eq!(ids_for("state:relearn"), vec!["relearning"]);
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
        assert_eq!(ids_for("noteType:basic"), vec!["note::forward"]);
    }

    #[test]
    fn anki_browser_id_and_card_template_filters_match_current_card() {
        let mut state = state();
        state.note_types[0].templates.push(CardTemplate {
            id: "reverse".to_string(),
            name: "Reverse".to_string(),
            front_template: "{{Back}}".to_string(),
            back_template: "{{Front}}".to_string(),
            required_field_names: vec!["Back".to_string()],
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
            ids_for("cid:due,note::forward"),
            vec!["note::forward", "due"]
        );
        assert_eq!(ids_for("cardId:DUE"), vec!["due"]);
        assert_eq!(ids_for("nid:note"), vec!["note::forward"]);
        assert_eq!(ids_for("note-id:NOTE"), vec!["note::forward"]);
        assert_eq!(ids_for("card:forward"), vec!["note::forward"]);
        assert_eq!(ids_for("template:Forward"), vec!["note::forward"]);
        assert!(ids_for("card:reverse").is_empty());
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
        assert_eq!(ids_for("edited:1"), vec!["note::forward"]);
        assert_eq!(ids_for("introduced:1"), vec!["due"]);
        assert_eq!(ids_for("rated:1:3"), vec!["due"]);
        assert_eq!(ids_for("rated:4:again"), vec!["future"]);
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
    fn parser_reports_unknown_filters_and_unclosed_quotes() {
        let error = search_cards(&state(), "kind:review", NOW).unwrap_err();
        assert_eq!(error.token, "kind:review");

        let error = search_cards(&state(), "\"vanakkam", NOW).unwrap_err();
        assert_eq!(error.message, "unterminated quoted string");

        let error = search_cards(&state(), "OR deck:tamil", NOW).unwrap_err();
        assert_eq!(error.message, "OR operator is missing a left-hand clause");

        let error = search_cards(&state(), "deck:tamil OR", NOW).unwrap_err();
        assert_eq!(error.message, "OR operator is missing a right-hand clause");

        let error = search_cards(&state(), "(deck:tamil OR is:due", NOW).unwrap_err();
        assert_eq!(error.message, "missing closing parenthesis");

        let error = search_cards(&state(), "deck:tamil)", NOW).unwrap_err();
        assert_eq!(error.message, "unexpected closing parenthesis");
    }
}
