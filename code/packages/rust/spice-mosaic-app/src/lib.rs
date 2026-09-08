//! State and host protocol for the Berkeley SPICE Mosaic workbench.
//!
//! The UI is intentionally a thin editor and result viewer. Parsing and
//! simulation remain in `spice-netlist-parser`, which keeps every host on the
//! same Berkeley-v1 execution contract as the command-line interface.

use std::{collections::BTreeSet, error::Error, fmt};

use mosaic_app_runtime::{
    Announcement, AppUpdate, ColorScheme, Event, MosaicApp, Politeness, Snapshot, StartContext,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use spice_netlist_parser::{inspect_netlist_json, parse_berkeley_app_deck, run_netlist_json};

const SNAPSHOT_SCHEMA: &str = "spice-mosaic-app/state";
const SNAPSHOT_VERSION: u32 = 1;
const DEFAULT_DECK: &str = "* Berkeley SPICE Mosaic workbench\nV1 in 0 DC 1 AC 1\nR1 in out 1k\nR2 out 0 1k\nC1 out 0 1u IC=0\n.options method=trap\n.op\n.dc V1 0 1 1\n.ac dec 1 1k 1k\n.tran 1m 1m\n.tf V(out) V1\n.save V(out)\n.end\n";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct AnalysisRow {
    index: u64,
    kind: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ResultTable {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedState {
    deck: String,
    selected_analysis_row: usize,
}

/// A deliberately small host state. No parser or engine state crosses the
/// snapshot boundary; it is reconstructed from the saved deck.
pub struct SpiceMosaicApp {
    deck: String,
    analyses: Vec<AnalysisRow>,
    selected_analysis_row: usize,
    result_table: ResultTable,
    result_text: String,
    diagnostic_rows: Vec<String>,
    diagnostics: String,
    mode: &'static str,
    dark: bool,
}

impl Default for SpiceMosaicApp {
    fn default() -> Self {
        Self {
            deck: DEFAULT_DECK.to_owned(),
            analyses: Vec::new(),
            selected_analysis_row: 0,
            result_table: ResultTable::default(),
            result_text: String::new(),
            diagnostic_rows: Vec::new(),
            diagnostics: "Edit a deck, then inspect its runnable analyses or run it.".to_owned(),
            mode: "Draft",
            dark: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpiceMosaicError(pub String);

impl fmt::Display for SpiceMosaicError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for SpiceMosaicError {}

fn invalid(message: impl Into<String>) -> SpiceMosaicError {
    SpiceMosaicError(message.into())
}

fn event_name(name: &str) -> String {
    if let Some(rest) = name.strip_prefix("on") {
        let mut characters = rest.chars();
        if let Some(first) = characters.next().filter(char::is_ascii_uppercase) {
            return first.to_ascii_lowercase().to_string() + characters.as_str();
        }
    }
    name.to_owned()
}

fn analysis_rows(deck: &str) -> Result<(String, Vec<AnalysisRow>), SpiceMosaicError> {
    let inspection = inspect_netlist_json(deck).map_err(|error| invalid(error.to_string()))?;
    let payload: Value =
        serde_json::from_str(&inspection).map_err(|error| invalid(error.to_string()))?;
    let analyses = payload["analyses"]
        .as_array()
        .ok_or_else(|| invalid("inspection payload is missing analyses"))?
        .iter()
        .map(|analysis| {
            let index = analysis["index"]
                .as_u64()
                .ok_or_else(|| invalid("inspection analysis is missing index"))?;
            let kind = analysis["kind"]
                .as_str()
                .ok_or_else(|| invalid("inspection analysis is missing kind"))?
                .to_owned();
            Ok(AnalysisRow { index, kind })
        })
        .collect::<Result<Vec<_>, SpiceMosaicError>>()?;
    Ok((inspection, analyses))
}

fn diagnostic_rows(deck: &str) -> Vec<String> {
    parse_berkeley_app_deck(deck)
        .diagnostics
        .into_iter()
        .map(|diagnostic| {
            let severity = match diagnostic.severity {
                spice_netlist_parser::BerkeleyDiagnosticSeverity::Error => "error",
                spice_netlist_parser::BerkeleyDiagnosticSeverity::Warning => "warning",
                spice_netlist_parser::BerkeleyDiagnosticSeverity::Note => "note",
            };
            let location = diagnostic.span.map_or_else(
                || "deck".to_owned(),
                |span| format!("line {}, column {}", span.start_line, span.start_column),
            );
            format!(
                "[{severity}] {} at {location}: {}",
                diagnostic.code, diagnostic.message
            )
        })
        .collect()
}

fn result_cell(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}

fn selected_result_table(
    result: &str,
    selected_index: usize,
) -> Result<ResultTable, SpiceMosaicError> {
    let payload: Value =
        serde_json::from_str(result).map_err(|error| invalid(error.to_string()))?;
    let analyses = payload["analyses"]
        .as_array()
        .ok_or_else(|| invalid("result payload is missing analyses"))?;
    let Some(analysis) = analyses.get(selected_index) else {
        return Ok(ResultTable::default());
    };
    let records = analysis["records"]
        .as_array()
        .ok_or_else(|| invalid("result analysis is missing records"))?;
    let columns = records
        .iter()
        .filter_map(Value::as_object)
        .flat_map(|record| record.keys().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let rows = records
        .iter()
        .map(|record| {
            let record = record
                .as_object()
                .ok_or_else(|| invalid("result record must be an object"))?;
            Ok(columns
                .iter()
                .map(|column| record.get(column).map_or_else(String::new, result_cell))
                .collect())
        })
        .collect::<Result<Vec<Vec<String>>, SpiceMosaicError>>()?;
    Ok(ResultTable { columns, rows })
}

impl SpiceMosaicApp {
    fn update(&self) -> AppUpdate {
        let selected_label = self
            .analyses
            .get(self.selected_analysis_row)
            .map(|analysis| format!(".{} (analysis {})", analysis.kind, analysis.index))
            .unwrap_or_else(|| "No analysis selected".to_owned());
        AppUpdate::new(json!({
            "workbench-title": "Berkeley SPICE Workbench",
            "mode-label": self.mode,
            "netlist-label": "Netlist",
            "netlist-text": self.deck,
            "netlist-placeholder": "Paste a Berkeley SPICE deck",
            "inspect-label": "Inspect",
            "run-label": "Run",
            "diagnostics-label": "Status",
            "diagnostics": self.diagnostics,
            "diagnostic-rows": self.diagnostic_rows,
            "analysis-label": "Runnable analyses",
            "analysis-rows": self.analyses.iter().map(|analysis| vec![
                format!(".{} (analysis {})", analysis.kind, analysis.index),
            ]).collect::<Vec<_>>(),
            "selected-analysis-label": selected_label,
            "result-label": "Selected result records",
            "result-columns": self.result_table.columns,
            "result-rows": self.result_table.rows,
            "raw-result-label": "Raw result JSON",
            "result-text": self.result_text,
            "dark-theme": self.dark,
        }))
    }

    fn announced(&self, message: impl Into<String>) -> AppUpdate {
        let mut update = self.update();
        update.announcements.push(Announcement {
            politeness: Politeness::Polite,
            message: message.into(),
        });
        update
    }

    fn inspect(&mut self) -> Result<AppUpdate, SpiceMosaicError> {
        let (inspection, analyses) = analysis_rows(&self.deck)?;
        let diagnostic_rows = diagnostic_rows(&self.deck);
        self.selected_analysis_row = self
            .selected_analysis_row
            .min(analyses.len().saturating_sub(1));
        self.diagnostics = format!("Found {} runnable analyses.", analyses.len());
        self.result_text = inspection;
        self.result_table = ResultTable::default();
        self.diagnostic_rows = diagnostic_rows;
        self.analyses = analyses;
        self.mode = "Inspection";
        Ok(self.announced(self.diagnostics.clone()))
    }

    fn run(&mut self) -> Result<AppUpdate, SpiceMosaicError> {
        // Build every fallible value first so rejected runs leave the current
        // deck and visible result unchanged for a host retry.
        let (_, analyses) = analysis_rows(&self.deck)?;
        let result = run_netlist_json(&self.deck).map_err(|error| invalid(error.to_string()))?;
        let selected_analysis_row = self
            .selected_analysis_row
            .min(analyses.len().saturating_sub(1));
        let result_table = selected_result_table(&result, selected_analysis_row)?;
        let diagnostic_rows = diagnostic_rows(&self.deck);
        self.selected_analysis_row = selected_analysis_row;
        self.diagnostics = format!("Executed {} analyses.", analyses.len());
        self.result_text = result;
        self.result_table = result_table;
        self.diagnostic_rows = diagnostic_rows;
        self.analyses = analyses;
        self.mode = "Results";
        Ok(self.announced(self.diagnostics.clone()))
    }
}

impl MosaicApp for SpiceMosaicApp {
    type Error = SpiceMosaicError;

    fn start(&mut self, context: StartContext) -> Result<AppUpdate, Self::Error> {
        if let Some(snapshot) = context.restored_snapshot {
            return self.restore(snapshot);
        }
        self.dark = context.color_scheme == ColorScheme::Dark;
        self.inspect()
    }

    fn dispatch(&mut self, event: Event) -> Result<AppUpdate, Self::Error> {
        if !event.payload.is_object() {
            return Err(invalid("event payload must be an object"));
        }
        match event_name(&event.name).as_str() {
            "netlistChange" => {
                let deck = event.payload["value"]
                    .as_str()
                    .ok_or_else(|| invalid("netlistChange requires text value"))?;
                self.deck = deck.to_owned();
                self.analyses.clear();
                self.selected_analysis_row = 0;
                self.result_table = ResultTable::default();
                self.result_text.clear();
                self.diagnostic_rows = diagnostic_rows(deck);
                self.diagnostics = "Deck changed. Inspect before running.".to_owned();
                self.mode = "Draft";
                Ok(self.update())
            }
            "inspect" => self.inspect(),
            "run" => self.run(),
            "selectAnalysis" => {
                let index = event.payload["index"]
                    .as_u64()
                    .ok_or_else(|| invalid("selectAnalysis requires non-negative index"))?
                    as usize;
                if index >= self.analyses.len() {
                    return Err(invalid("selectAnalysis index is out of range"));
                }
                let result_table = if self.mode == "Results" {
                    selected_result_table(&self.result_text, index)?
                } else {
                    ResultTable::default()
                };
                self.selected_analysis_row = index;
                self.result_table = result_table;
                Ok(self.announced(format!("Selected .{} analysis.", self.analyses[index].kind)))
            }
            _ => Err(invalid(format!(
                "unknown SPICE workbench event: {}",
                event.name
            ))),
        }
    }

    fn snapshot(&self) -> Result<Option<Snapshot>, Self::Error> {
        let bytes = serde_json::to_vec(&SavedState {
            deck: self.deck.clone(),
            selected_analysis_row: self.selected_analysis_row,
        })
        .map_err(|error| invalid(error.to_string()))?;
        Ok(Some(Snapshot {
            schema: SNAPSHOT_SCHEMA.into(),
            version: SNAPSHOT_VERSION,
            bytes,
        }))
    }

    fn restore(&mut self, snapshot: Snapshot) -> Result<AppUpdate, Self::Error> {
        if snapshot.schema != SNAPSHOT_SCHEMA || snapshot.version != SNAPSHOT_VERSION {
            return Err(invalid("unsupported SPICE workbench snapshot"));
        }
        let saved: SavedState =
            serde_json::from_slice(&snapshot.bytes).map_err(|error| invalid(error.to_string()))?;
        let (_, analyses) = analysis_rows(&saved.deck)?;
        if saved.selected_analysis_row >= analyses.len() && !analyses.is_empty() {
            return Err(invalid("snapshot analysis selection is out of range"));
        }
        self.deck = saved.deck;
        self.analyses = analyses;
        self.selected_analysis_row = saved.selected_analysis_row;
        self.result_table = ResultTable::default();
        self.result_text.clear();
        self.diagnostic_rows = diagnostic_rows(&self.deck);
        self.diagnostics = "Workbench restored. Inspect or run the saved deck.".to_owned();
        self.mode = "Draft";
        Ok(self.announced(self.diagnostics.clone()))
    }
}

mosaic_app_capi::export_mosaic_app!(SpiceMosaicApp, SpiceMosaicApp::default());
mosaic_app_wasm::export_mosaic_wasm!(SpiceMosaicApp, SpiceMosaicApp::default());

#[cfg(test)]
mod tests {
    use super::*;
    use mosaic_app_runtime::{MosaicRuntime, Platform};

    fn dispatch(app: &mut SpiceMosaicApp, name: &str, payload: Value) -> AppUpdate {
        app.dispatch(Event::new(1, name, payload)).unwrap()
    }

    #[test]
    fn starts_with_the_complete_berkeley_analysis_plan() {
        let mut app = SpiceMosaicApp::default();
        let update = app
            .start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        assert_eq!(update.props["mode-label"], "Inspection");
        assert_eq!(update.props["analysis-rows"].as_array().unwrap().len(), 5);
        assert_eq!(update.props["analysis-rows"][4][0], ".tf (analysis 5)");
    }

    #[test]
    fn editable_deck_inspects_runs_and_selects_an_analysis() {
        let mut app = SpiceMosaicApp::default();
        app.start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        let changed = dispatch(&mut app, "onNetlistChange", json!({"value": DEFAULT_DECK}));
        assert_eq!(changed.props["mode-label"], "Draft");
        let inspected = dispatch(&mut app, "onInspect", json!({}));
        assert_eq!(
            inspected.props["analysis-rows"].as_array().unwrap().len(),
            5
        );
        let selected = dispatch(&mut app, "onSelectAnalysis", json!({"index": 4}));
        assert_eq!(
            selected.props["selected-analysis-label"],
            ".tf (analysis 5)"
        );
        let run = dispatch(&mut app, "onRun", json!({}));
        assert_eq!(run.props["mode-label"], "Results");
        assert!(!run.props["result-columns"].as_array().unwrap().is_empty());
        assert!(!run.props["result-rows"].as_array().unwrap().is_empty());
        assert!(run.props["result-text"]
            .as_str()
            .unwrap()
            .contains("schemaVersion"));
    }

    #[test]
    fn rejected_inspection_keeps_the_last_visible_state_for_retry() {
        let mut app = SpiceMosaicApp::default();
        app.start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        dispatch(&mut app, "netlistChange", json!({"value": "+ orphan\n"}));
        let before = app.snapshot().unwrap();
        assert!(app.dispatch(Event::new(1, "inspect", json!({}))).is_err());
        assert_eq!(app.snapshot().unwrap(), before);
    }

    #[test]
    fn deck_edits_expose_parser_diagnostics_with_source_spans() {
        let mut app = SpiceMosaicApp::default();
        app.start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        let update = dispatch(&mut app, "netlistChange", json!({"value": "+ orphan\n"}));
        let diagnostics = update.props["diagnostic-rows"].as_array().unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0]
            .as_str()
            .unwrap()
            .contains("SPICE_SYNTAX_CONTINUATION_WITHOUT_CARD"));
        assert!(diagnostics[0]
            .as_str()
            .unwrap()
            .contains("line 1, column 1"));
    }

    #[test]
    fn runtime_retries_a_rejected_event_without_consuming_its_sequence() {
        let mut runtime = MosaicRuntime::new(SpiceMosaicApp::default());
        runtime
            .start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        assert!(runtime
            .dispatch(Event::new(1, "selectAnalysis", json!({"index": 99})))
            .is_err());
        let update = runtime
            .dispatch(Event::new(1, "selectAnalysis", json!({"index": 0})))
            .unwrap();
        assert_eq!(update.revision, 2);
        assert_eq!(update.props["selected-analysis-label"], ".op (analysis 1)");
    }
}
