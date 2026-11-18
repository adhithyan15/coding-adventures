//! Public-API conformance against the canonical Hamurabi v1 corpus.
//!
//! The JSON parser's syntax tree keeps each numeric token's original text. We
//! convert that tree into a small typed fixture view instead of passing the
//! `u64` RNG words through an `i64`/`f64` JSON value representation.

use coding_adventures_json_parser::try_parse_json;
use coding_adventures_bounded_json::{parse as parse_bounded_json, JsonValue as BoundedJson};
use hamurabi::{
    format_report, new_game, step_year, validate_order, GameStatus, HamurabiError, HamurabiState,
    Rating, TurnOrder,
};
use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::collections::{BTreeMap, BTreeSet};

const CORPUS: &str = include_str!("../../../../specs/fixtures/hamurabi-v1/cases.json");

const EXPECTED_IDS: &[&str] = &[
    "full_10_year_charlemagne_win",
    "full_10_year_acre_win",
    "full_10_year_nero_rating",
    "full_10_year_disaster_rating",
    "turn_with_plague_struck",
    "turn_with_rat_infestation",
    "turn_with_land_purchase",
    "turn_with_land_sale",
    "immediate_impeachment_100_percent_starved",
    "borderline_impeachment_46_percent_starved",
    "borderline_non_impeachment_45_percent_starved",
    "refusal_game_already_ended_impeached",
    "refusal_invalid_order_bounds_revenue_overflow",
    "refusal_insufficient_bushels_for_land",
    "refusal_insufficient_acres_to_sell",
    "refusal_insufficient_bushels_to_feed",
    "refusal_insufficient_acres_to_plant",
    "refusal_insufficient_population_to_plant",
    "refusal_insufficient_grain_for_seed",
    "boundary_sell_exact_owned_acres",
    "boundary_feed_exact_available_bushels",
    "boundary_plant_exact_labor_limit",
    "boundary_plant_exact_seed_limit",
    "boundary_plant_odd_acres_ceil_seed",
    "boundary_buy_max_affordable_acres",
    "refusal_overflow_u32_max_bushels_to_feed",
    "refusal_overflow_u32_max_acres_to_plant",
    "refusal_boundary_one_bushel_over_available",
    "refusal_boundary_one_acre_over_owned",
    "refusal_boundary_one_bushel_short_seed",
    "refusal_boundary_one_acre_over_labor",
];

#[derive(Debug, Clone, PartialEq, Eq)]
enum Json {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    fn object(&self, at: &str) -> Result<&BTreeMap<String, Json>, String> {
        match self {
            Self::Object(value) => Ok(value),
            _ => Err(format!("{at}: expected object")),
        }
    }

    fn array(&self, at: &str) -> Result<&[Json], String> {
        match self {
            Self::Array(value) => Ok(value),
            _ => Err(format!("{at}: expected array")),
        }
    }

    fn string(&self, at: &str) -> Result<&str, String> {
        match self {
            Self::String(value) => Ok(value),
            _ => Err(format!("{at}: expected string")),
        }
    }

    fn field<'a>(&'a self, key: &str, at: &str) -> Result<&'a Json, String> {
        self.object(at)?
            .get(key)
            .ok_or_else(|| format!("{at}: missing field {key:?}"))
    }

    fn number_text(&self, at: &str) -> Result<&str, String> {
        match self {
            Self::Number(value) => Ok(value),
            _ => Err(format!("{at}: expected JSON integer")),
        }
    }

    fn u64(&self, at: &str) -> Result<u64, String> {
        self.number_text(at)?
            .parse()
            .map_err(|_| format!("{at}: expected u64 integer"))
    }

    fn u32(&self, at: &str) -> Result<u32, String> {
        self.number_text(at)?
            .parse()
            .map_err(|_| format!("{at}: expected u32 integer"))
    }

    fn u8(&self, at: &str) -> Result<u8, String> {
        self.number_text(at)?
            .parse()
            .map_err(|_| format!("{at}: expected u8 integer"))
    }

    fn i32(&self, at: &str) -> Result<i32, String> {
        self.number_text(at)?
            .parse()
            .map_err(|_| format!("{at}: expected i32 integer"))
    }

    fn boolean(&self, at: &str) -> Result<bool, String> {
        match self {
            Self::Bool(value) => Ok(*value),
            _ => Err(format!("{at}: expected boolean")),
        }
    }

    fn exact_keys(&self, expected: &[&str], at: &str) -> Result<(), String> {
        let object = self.object(at)?;
        let actual: BTreeSet<_> = object.keys().map(String::as_str).collect();
        let expected: BTreeSet<_> = expected.iter().copied().collect();
        if actual != expected {
            return Err(format!(
                "{at}: object fields differ (missing: {:?}, unknown: {:?})",
                expected.difference(&actual).collect::<Vec<_>>(),
                actual.difference(&expected).collect::<Vec<_>>()
            ));
        }
        Ok(())
    }
}

/// Convert the JSON grammar tree while retaining integer lexemes exactly.
fn json_value(node: &GrammarASTNode, at: &str) -> Result<Json, String> {
    if node.rule_name != "value" {
        return Err(format!("{at}: expected JSON value node, got {}", node.rule_name));
    }
    let mut result = None;
    for child in &node.children {
        let value = match child {
            ASTNodeOrToken::Node(child) => match child.rule_name.as_str() {
                "object" => json_object(child, at)?,
                "array" => json_array(child, at)?,
                other => return Err(format!("{at}: unknown JSON value rule {other:?}")),
            },
            ASTNodeOrToken::Token(token) => json_scalar(token, at)?,
        };
        if result.replace(value).is_some() {
            return Err(format!("{at}: JSON value contains multiple alternatives"));
        }
    }
    result.ok_or_else(|| format!("{at}: empty JSON value"))
}

fn json_scalar(token: &Token, at: &str) -> Result<Json, String> {
    match token.type_name.as_deref() {
        Some("TRUE") => Ok(Json::Bool(true)),
        Some("FALSE") => Ok(Json::Bool(false)),
        Some("NULL") => Ok(Json::Null),
        Some(other) => Err(format!("{at}: unknown scalar token {other:?}")),
        None => match token.type_ {
            TokenType::String => Ok(Json::String(decode_string_token(&token.value, at)?)),
            TokenType::Number => Ok(Json::Number(token.value.clone())),
            _ => Err(format!("{at}: unexpected scalar token {:?}", token.type_)),
        },
    }
}

/// Decode one JSON string token without routing any number through this parser.
fn decode_string_token(raw: &str, at: &str) -> Result<String, String> {
    match parse_bounded_json(&format!("\"{raw}\""))
        .map_err(|error| format!("{at}: invalid JSON string token: {error}"))?
    {
        BoundedJson::String(value) => Ok(value),
        _ => Err(format!("{at}: JSON string token did not decode as a string")),
    }
}

fn json_object(node: &GrammarASTNode, at: &str) -> Result<Json, String> {
    if node.rule_name != "object" {
        return Err(format!("{at}: expected object node"));
    }
    let mut fields = BTreeMap::new();
    for child in &node.children {
        let ASTNodeOrToken::Node(pair) = child else {
            continue;
        };
        if pair.rule_name != "pair" {
            return Err(format!("{at}: unexpected object child {}", pair.rule_name));
        }
        let key = match pair.children.first() {
            Some(ASTNodeOrToken::Token(token)) if token.type_ == TokenType::String => {
                decode_string_token(&token.value, &format!("{at}.key"))?
            }
            _ => return Err(format!("{at}: object pair has no string key")),
        };
        let value_node = pair.children.iter().find_map(|child| match child {
            ASTNodeOrToken::Node(value) if value.rule_name == "value" => Some(value),
            _ => None,
        });
        let value = json_value(
            value_node.ok_or_else(|| format!("{at}.{key}: object pair has no value"))?,
            &format!("{at}.{key}"),
        )?;
        if fields.insert(key.clone(), value).is_some() {
            return Err(format!("{at}: duplicate object key {key:?}"));
        }
    }
    Ok(Json::Object(fields))
}

fn json_array(node: &GrammarASTNode, at: &str) -> Result<Json, String> {
    if node.rule_name != "array" {
        return Err(format!("{at}: expected array node"));
    }
    let mut values = Vec::new();
    for child in &node.children {
        match child {
            ASTNodeOrToken::Node(value) if value.rule_name == "value" => {
                values.push(json_value(value, &format!("{at}[{}]", values.len()))?);
            }
            ASTNodeOrToken::Node(other) => {
                return Err(format!("{at}: unknown array child {}", other.rule_name));
            }
            ASTNodeOrToken::Token(token)
                if matches!(token.type_, TokenType::LBracket | TokenType::RBracket | TokenType::Comma) => {}
            ASTNodeOrToken::Token(token) => {
                return Err(format!("{at}: unexpected array token {:?}", token.type_));
            }
        }
    }
    Ok(Json::Array(values))
}

fn parse_document(source: &str) -> Result<Json, String> {
    let ast = try_parse_json(source).map_err(|error| format!("JSON parse: {error}"))?;
    json_value(&ast, "document")
}

#[derive(Debug)]
struct TrajectoryTurn {
    number: u8,
    order: TurnOrder,
    expected_state: HamurabiState,
    expected_report: String,
}

#[derive(Debug)]
struct Refusal {
    order: TurnOrder,
    expected: HamurabiError,
}

#[derive(Debug)]
struct CorpusCase {
    id: String,
    seed: u64,
    initial: HamurabiState,
    turns: Option<Vec<TrajectoryTurn>>,
    refusal: Option<Refusal>,
}

fn read_order(value: &Json, at: &str) -> Result<TurnOrder, String> {
    value.exact_keys(
        &["acres_delta", "bushels_to_feed", "acres_to_plant"],
        at,
    )?;
    Ok(TurnOrder {
        acres_delta: value.field("acres_delta", at)?.i32(&format!("{at}.acres_delta"))?,
        bushels_to_feed: value
            .field("bushels_to_feed", at)?
            .u32(&format!("{at}.bushels_to_feed"))?,
        acres_to_plant: value
            .field("acres_to_plant", at)?
            .u32(&format!("{at}.acres_to_plant"))?,
    })
}

fn read_status(value: &Json, at: &str) -> Result<GameStatus, String> {
    let text = value.string(at)?;
    match text {
        "Playing" => Ok(GameStatus::Playing),
        "GameOverExtinction" => Ok(GameStatus::GameOverExtinction),
        _ if text.starts_with("Won(") && text.ends_with(')') => {
            let rating = match &text[4..text.len() - 1] {
                "Charlemagne" => Rating::Charlemagne,
                "Acre" => Rating::Acre,
                "Nero" => Rating::Nero,
                "Disaster" => Rating::Disaster,
                other => return Err(format!("{at}: unknown rating {other:?}")),
            };
            Ok(GameStatus::Won { rating })
        }
        _ if text.starts_with("Impeached(starved=") && text.ends_with(')') => {
            let payload = &text[18..text.len() - 1];
            let (starved, percent) = payload
                .split_once(", percent=")
                .ok_or_else(|| format!("{at}: malformed impeachment status"))?;
            Ok(GameStatus::Impeached {
                starved_this_turn: starved
                    .parse()
                    .map_err(|_| format!("{at}: invalid starved payload"))?,
                percent: percent
                    .parse()
                    .map_err(|_| format!("{at}: invalid percent payload"))?,
            })
        }
        _ => Err(format!("{at}: unknown status tag {text:?}")),
    }
}

fn read_state(value: &Json, at: &str) -> Result<HamurabiState, String> {
    const KEYS: &[&str] = &[
        "year",
        "population",
        "bushels",
        "acres",
        "land_price",
        "harvest_per_acre",
        "rats_eaten",
        "starved_last_year",
        "starved_total",
        "percent_starved_sum",
        "immigrants_last_year",
        "plague_struck",
        "status",
        "rng_state",
        "rng_inc",
    ];
    value.exact_keys(KEYS, at)?;
    let field = |key: &str| value.field(key, at);
    macro_rules! u32_field {
        ($key:literal) => {
            field($key)?.u32(&format!("{at}.{}", $key))?
        };
    }
    Ok(HamurabiState {
        year: field("year")?.u8(&format!("{at}.year"))?,
        population: u32_field!("population"),
        bushels: u32_field!("bushels"),
        acres: u32_field!("acres"),
        land_price: u32_field!("land_price"),
        harvest_per_acre: u32_field!("harvest_per_acre"),
        rats_eaten: u32_field!("rats_eaten"),
        starved_last_year: u32_field!("starved_last_year"),
        starved_total: u32_field!("starved_total"),
        percent_starved_sum: u32_field!("percent_starved_sum"),
        immigrants_last_year: u32_field!("immigrants_last_year"),
        plague_struck: field("plague_struck")?.boolean(&format!("{at}.plague_struck"))?,
        status: read_status(field("status")?, &format!("{at}.status"))?,
        rng_state: field("rng_state")?.u64(&format!("{at}.rng_state"))?,
        rng_inc: field("rng_inc")?.u64(&format!("{at}.rng_inc"))?,
    })
}

fn read_error(value: &Json, at: &str) -> Result<HamurabiError, String> {
    match value.string(at)? {
        "GameAlreadyEnded" => Ok(HamurabiError::GameAlreadyEnded),
        "InvalidOrderBounds" => Ok(HamurabiError::InvalidOrderBounds),
        "InsufficientBushelsForLand" => Ok(HamurabiError::InsufficientBushelsForLand),
        "InsufficientAcresToSell" => Ok(HamurabiError::InsufficientAcresToSell),
        "InsufficientBushelsToFeed" => Ok(HamurabiError::InsufficientBushelsToFeed),
        "InsufficientAcresToPlant" => Ok(HamurabiError::InsufficientAcresToPlant),
        "InsufficientPopulationToPlant" => Ok(HamurabiError::InsufficientPopulationToPlant),
        "InsufficientGrainForSeed" => Ok(HamurabiError::InsufficientGrainForSeed),
        other => Err(format!("{at}: unknown error variant {other:?}")),
    }
}

fn read_case(value: &Json, index: usize) -> Result<CorpusCase, String> {
    let at = format!("cases[{index}]");
    let object = value.object(&at)?;
    let id = value.field("id", &at)?.string(&format!("{at}.id"))?.to_owned();
    let seed = value.field("seed", &at)?.u64(&format!("{at}.seed"))?;
    let initial = read_state(value.field("initial_state", &at)?, &format!("{at}.initial_state"))?;
    if let Some(expected_error) = object.get("expected_error") {
        value.exact_keys(
            &["id", "description", "seed", "initial_state", "order", "expected_error"],
            &at,
        )?;
        Ok(CorpusCase {
            id,
            seed,
            initial,
            turns: None,
            refusal: Some(Refusal {
                order: read_order(value.field("order", &at)?, &format!("{at}.order"))?,
                expected: read_error(expected_error, &format!("{at}.expected_error"))?,
            }),
        })
    } else if let Some(turns) = object.get("turns") {
        value.exact_keys(&["id", "description", "seed", "initial_state", "turns"], &at)?;
        let mut parsed_turns = Vec::new();
        for (turn_index, turn) in turns.array(&format!("{at}.turns"))?.iter().enumerate() {
            let turn_at = format!("{at}.turns[{turn_index}]");
            turn.exact_keys(
                &["turn", "order", "expected_state", "expected_report"],
                &turn_at,
            )?;
            parsed_turns.push(TrajectoryTurn {
                number: turn.field("turn", &turn_at)?.u8(&format!("{turn_at}.turn"))?,
                order: read_order(turn.field("order", &turn_at)?, &format!("{turn_at}.order"))?,
                expected_state: read_state(
                    turn.field("expected_state", &turn_at)?,
                    &format!("{turn_at}.expected_state"),
                )?,
                expected_report: turn
                    .field("expected_report", &turn_at)?
                    .string(&format!("{turn_at}.expected_report"))?
                    .to_owned(),
            });
        }
        Ok(CorpusCase {
            id,
            seed,
            initial,
            turns: Some(parsed_turns),
            refusal: None,
        })
    } else {
        Err(format!("{at}: unknown or skipped case operation"))
    }
}

fn parse_cases(source: &str) -> Result<Vec<CorpusCase>, String> {
    let document = parse_document(source)?;
    document.exact_keys(&["version", "suite", "cases"], "document")?;
    if document.field("version", "document")?.u8("document.version")? != 1 {
        return Err("document.version: unsupported corpus version".into());
    }
    if document.field("suite", "document")?.string("document.suite")? != "hamurabi_deterministic_reign" {
        return Err("document.suite: unexpected suite".into());
    }
    let values = document.field("cases", "document")?.array("document.cases")?;
    if values.len() != EXPECTED_IDS.len() {
        return Err(format!("document.cases: expected {} cases, found {}", EXPECTED_IDS.len(), values.len()));
    }
    let mut cases = Vec::with_capacity(values.len());
    let mut seen = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let case = read_case(value, index)?;
        if !seen.insert(case.id.clone()) {
            return Err(format!("cases[{index}]: duplicate case id {:?}", case.id));
        }
        if !EXPECTED_IDS.contains(&case.id.as_str()) {
            return Err(format!("cases[{index}]: unknown case id {:?}", case.id));
        }
        cases.push(case);
    }
    let expected: BTreeSet<_> = EXPECTED_IDS.iter().copied().collect();
    let missing: Vec<_> = expected.difference(&seen.iter().map(String::as_str).collect()).copied().collect();
    if !missing.is_empty() {
        return Err(format!("document.cases: missing case ids {missing:?}"));
    }
    Ok(cases)
}

trait Adapter {
    fn new_game(&self, seed: u64) -> HamurabiState;
    fn validate_order(&self, state: &HamurabiState, order: TurnOrder) -> Result<(), HamurabiError>;
    fn step_year(&self, state: &HamurabiState, order: TurnOrder) -> Result<HamurabiState, HamurabiError>;
    fn format_report(&self, state: &HamurabiState) -> String;
}

struct PublicApi;

impl Adapter for PublicApi {
    fn new_game(&self, seed: u64) -> HamurabiState {
        new_game(seed)
    }
    fn validate_order(&self, state: &HamurabiState, order: TurnOrder) -> Result<(), HamurabiError> {
        validate_order(state, &order)
    }
    fn step_year(&self, state: &HamurabiState, order: TurnOrder) -> Result<HamurabiState, HamurabiError> {
        step_year(state, &order)
    }
    fn format_report(&self, state: &HamurabiState) -> String {
        format_report(state)
    }
}

struct AlwaysError;

impl Adapter for AlwaysError {
    fn new_game(&self, seed: u64) -> HamurabiState {
        new_game(seed)
    }
    fn validate_order(&self, _state: &HamurabiState, _order: TurnOrder) -> Result<(), HamurabiError> {
        Err(HamurabiError::GameAlreadyEnded)
    }
    fn step_year(&self, _state: &HamurabiState, _order: TurnOrder) -> Result<HamurabiState, HamurabiError> {
        Err(HamurabiError::GameAlreadyEnded)
    }
    fn format_report(&self, state: &HamurabiState) -> String {
        format_report(state)
    }
}

fn state_matches(actual: &HamurabiState, expected: &HamurabiState, at: &str) -> Result<(), String> {
    macro_rules! check {
        ($field:ident) => {
            if actual.$field != expected.$field {
                return Err(format!("{at}.{}: actual {:?}, expected {:?}", stringify!($field), actual.$field, expected.$field));
            }
        };
    }
    check!(year);
    check!(population);
    check!(bushels);
    check!(acres);
    check!(land_price);
    check!(harvest_per_acre);
    check!(rats_eaten);
    check!(starved_last_year);
    check!(starved_total);
    check!(percent_starved_sum);
    check!(immigrants_last_year);
    check!(plague_struck);
    check!(status);
    check!(rng_state);
    check!(rng_inc);
    Ok(())
}

fn run_corpus(source: &str, api: &dyn Adapter) -> Result<(), String> {
    let cases = parse_cases(source)?;
    let expected_fresh_starts = [
        "full_10_year_charlemagne_win",
        "full_10_year_acre_win",
        "full_10_year_nero_rating",
        "full_10_year_disaster_rating",
    ];
    let mut total_turns = 0;
    let mut refusal_count = 0;
    let mut errors = BTreeSet::new();
    for case in cases {
        let case_at = format!("case {}", case.id);
        if expected_fresh_starts.contains(&case.id.as_str()) {
            state_matches(&api.new_game(case.seed), &case.initial, &format!("{case_at}.initial_state/new_game"))?;
        }
        match (case.turns, case.refusal) {
            (Some(turns), None) => {
                let mut actual = case.initial;
                for turn in turns {
                    let at = format!("{case_at} turn {}", turn.number);
                    if actual.year != turn.number {
                        return Err(format!("{at}: current state year is {}", actual.year));
                    }
                    api.validate_order(&actual, turn.order)
                        .map_err(|error| format!("{at}: validate_order unexpectedly returned {error:?}"))?;
                    actual = api
                        .step_year(&actual, turn.order)
                        .map_err(|error| format!("{at}: step_year unexpectedly returned {error:?}"))?;
                    state_matches(&actual, &turn.expected_state, &format!("{at}.expected_state"))?;
                    let report = api.format_report(&actual);
                    if report != turn.expected_report {
                        return Err(format!("{at}.expected_report: complete text differs (actual {report:?}, expected {:?})", turn.expected_report));
                    }
                    total_turns += 1;
                }
            }
            (None, Some(refusal)) => {
                refusal_count += 1;
                errors.insert(format!("{:?}", refusal.expected));
                let validation = api.validate_order(&case.initial, refusal.order);
                if validation != Err(refusal.expected) {
                    return Err(format!("{case_at}: validate_order returned {validation:?}, expected Err({:?})", refusal.expected));
                }
                let step = api.step_year(&case.initial, refusal.order);
                if step != Err(refusal.expected) {
                    return Err(format!("{case_at}: step_year returned {step:?}, expected Err({:?})", refusal.expected));
                }
            }
            _ => return Err(format!("{case_at}: unknown or skipped operation")),
        }
    }
    if total_turns != 53 {
        return Err(format!("corpus: expected 53 trajectory turns, observed {total_turns}"));
    }
    if refusal_count != 14 {
        return Err(format!("corpus: expected 14 refusal cases, observed {refusal_count}"));
    }
    if errors.len() != 8 {
        return Err(format!("corpus: expected all 8 refusal variants, observed {}: {errors:?}", errors.len()));
    }
    Ok(())
}

#[test]
fn consumes_every_canonical_case_through_public_api() {
    run_corpus(CORPUS, &PublicApi).expect("canonical Hamurabi corpus must conform");
}

#[test]
fn same_harness_rejects_nonconforming_adapters_and_fixture_mutations() {
    let always_error = run_corpus(CORPUS, &AlwaysError).unwrap_err();
    assert!(always_error.contains("full_10_year_charlemagne_win turn 1"), "{always_error}");
    assert!(always_error.contains("validate_order unexpectedly returned"), "{always_error}");

    let original_rng = "15877481753564384925";
    let changed_rng = "15877481753564384926";
    let altered_rng = CORPUS.replacen(original_rng, changed_rng, 1);
    assert_ne!(altered_rng, CORPUS, "RNG mutation must hit the expected initial state's >2^53 word");
    let rng_error = run_corpus(&altered_rng, &PublicApi).unwrap_err();
    assert!(rng_error.contains("full_10_year_charlemagne_win"), "{rng_error}");
    assert!(rng_error.contains("initial_state/new_game.rng_state"), "{rng_error}");

    let report_line = "LAND IS TRADING AT 23 BUSHELS PER ACRE.\\n\"";
    let altered_report = CORPUS.replacen(report_line, "LAND IS TRADING AT 23 BUSHELS PER ACRE.\"", 1);
    assert_ne!(altered_report, CORPUS, "report mutation must remove the canonical final newline");
    let report_error = run_corpus(&altered_report, &PublicApi).unwrap_err();
    assert!(report_error.contains("full_10_year_charlemagne_win turn 1"), "{report_error}");
    assert!(report_error.contains("expected_report"), "{report_error}");

    let altered_error = CORPUS.replacen("\"expected_error\": \"GameAlreadyEnded\"", "\"expected_error\": \"InvalidOrderBounds\"", 1);
    assert_ne!(altered_error, CORPUS, "error mutation must alter a canonical refusal");
    let error_mismatch = run_corpus(&altered_error, &PublicApi).unwrap_err();
    assert!(error_mismatch.contains("refusal_game_already_ended_impeached"), "{error_mismatch}");
    assert!(error_mismatch.contains("validate_order returned"), "{error_mismatch}");

    let missing_and_duplicate = CORPUS.replacen(
        "\"id\": \"refusal_boundary_one_acre_over_labor\"",
        "\"id\": \"full_10_year_charlemagne_win\"",
        1,
    );
    let missing_error = parse_cases(&missing_and_duplicate).unwrap_err();
    assert!(missing_error.contains("duplicate case id") || missing_error.contains("missing case ids"), "{missing_error}");
}

#[test]
fn duplicate_json_keys_fail_closed() {
    let duplicate = CORPUS.replacen(
        "\"year\": 1,\n        \"population\": 100",
        "\"year\": 1,\n        \"year\": 1,\n        \"population\": 100",
        1,
    );
    assert_ne!(duplicate, CORPUS);
    let error = parse_cases(&duplicate).unwrap_err();
    assert!(error.contains("duplicate object key \"year\""), "{error}");

    let escaped_alias = CORPUS.replacen(
        "\"year\": 1,\n        \"population\": 100",
        "\"year\": 1,\n        \"\\u0079ear\": 1,\n        \"population\": 100",
        1,
    );
    assert_ne!(escaped_alias, CORPUS);
    let error = parse_cases(&escaped_alias).unwrap_err();
    assert!(error.contains("duplicate object key \"year\""), "{error}");
}

#[test]
fn json_string_escapes_decode_to_distinct_byte_sequences() {
    let newline = decode_string_token(r"\n", "newline").unwrap();
    let literal_backslash_n = decode_string_token(r"\\n", "literal backslash n").unwrap();
    assert_eq!(newline, "\n");
    assert_eq!(literal_backslash_n, r"\n");
    assert_ne!(newline.as_bytes(), literal_backslash_n.as_bytes());
}

#[test]
fn unknown_or_missing_case_operation_fails_closed() {
    let skipped = CORPUS.replacen(
        "\"turns\": [",
        "\"operation\": \"mystery\",\n      \"turns\": [",
        1,
    );
    // Unknown fields and operation tags cannot be silently ignored.
    let error = parse_cases(&skipped).unwrap_err();
    assert!(error.contains("unknown") || error.contains("fields differ"), "{error}");

    let no_operation = CORPUS.replacen(
        ",\n      \"expected_error\": \"GameAlreadyEnded\"",
        "",
        1,
    );
    let error = parse_cases(&no_operation).unwrap_err();
    assert!(error.contains("unknown or skipped case operation") || error.contains("fields differ"), "{error}");
}
