#![recursion_limit = "512"]

//! State and host protocol for the Berkeley SPICE Mosaic workbench.
//!
//! The UI is intentionally a thin editor and result viewer. Parsing and
//! simulation remain in `spice-netlist-parser`, which keeps every host on the
//! same Berkeley-v1 execution contract as the command-line interface.

use std::{collections::BTreeSet, error::Error, fmt};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use mosaic_app_runtime::{
    Announcement, AppUpdate, ColorScheme, Delivery, Effect, EffectCompletionError, EffectId,
    EffectResult, Event, MosaicApp, Politeness, Snapshot, StartContext, EFFECT_PROTOCOL_VERSION,
    MAX_EFFECT_ID, PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use spice_netlist_parser::{inspect_netlist_json, parse_berkeley_app_deck, run_netlist_json};

mod schematic;

pub use schematic::{
    SchematicAnalysis, SchematicAnalysisCard, SchematicAnalysisSettings, SchematicComponent,
    SchematicComponentKind, SchematicDocument, SchematicError, SchematicNetLabel, SchematicPoint,
    SchematicWire,
};

const SNAPSHOT_SCHEMA: &str = "spice-mosaic-app/state";
const SNAPSHOT_VERSION: u32 = 5;
const SCHEMATIC_FILE_SCHEMA: &str = "spice-mosaic/schematic";
const SCHEMATIC_FILE_VERSION: u32 = 1;
const MAX_SCHEMATIC_FILE_BYTES: usize = 16 * 1024 * 1024;
const MAX_SCHEMATIC_HISTORY_ENTRIES: usize = 100;
const DEFAULT_DECK: &str = "* Berkeley SPICE Mosaic workbench\nV1 in 0 DC 1 AC 1\nR1 in out 1k\nR2 out 0 1k\nC1 out 0 1u IC=0\n.options method=trap\n.op\n.dc V1 0 1 1\n.ac dec 1 1k 1k\n.tran 1m 3m\n.tf V(out) V1\n.save V(out)\n.end\n";

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

#[derive(Clone, Debug, Default, PartialEq)]
struct WaveformPlot {
    labels: Vec<String>,
    selected_label: String,
    axis_label: String,
    segments: Vec<[f64; 4]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedState {
    deck: String,
    selected_analysis_row: usize,
    #[serde(default)]
    schematic: Option<SchematicDocument>,
    #[serde(default)]
    selected_schematic_component: Option<String>,
    #[serde(default)]
    selected_schematic_terminal: usize,
    #[serde(default)]
    selected_schematic_route_target: Option<String>,
    #[serde(default)]
    selected_schematic_analysis_card: usize,
    #[serde(default = "default_next_effect_id")]
    next_schematic_file_effect_id: EffectId,
}

fn default_next_effect_id() -> EffectId {
    1
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SchematicFile {
    schema: String,
    version: u32,
    document: SchematicDocument,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SchematicFileOperation {
    Open,
    Save,
}

impl SchematicFileOperation {
    fn effect_kind(self) -> &'static str {
        match self {
            Self::Open => "file.open",
            Self::Save => "file.save",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Open => "Schematic import",
            Self::Save => "Schematic export",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingSchematicFileEffect {
    id: EffectId,
    operation: SchematicFileOperation,
}

/// One in-memory schematic checkpoint. History deliberately stays outside the
/// persisted snapshot contract so restored sessions never resurrect stale edits.
#[derive(Clone, Debug, Eq, PartialEq)]
struct SchematicHistoryEntry {
    document: Option<SchematicDocument>,
    selected_component: Option<String>,
    selected_terminal: usize,
    selected_route_target: Option<String>,
    selected_analysis_card: usize,
}

/// A deliberately small host state. No parser or engine state crosses the
/// snapshot boundary; it is reconstructed from the saved deck.
pub struct SpiceMosaicApp {
    deck: String,
    analyses: Vec<AnalysisRow>,
    selected_analysis_row: usize,
    result_table: ResultTable,
    waveform_plot: WaveformPlot,
    result_text: String,
    diagnostic_rows: Vec<String>,
    diagnostics: String,
    schematic: Option<SchematicDocument>,
    selected_schematic_component: Option<String>,
    selected_schematic_terminal: usize,
    selected_schematic_route_target: Option<String>,
    selected_schematic_analysis_card: usize,
    protocol_version: u32,
    next_schematic_file_effect_id: EffectId,
    pending_schematic_file_effect: Option<PendingSchematicFileEffect>,
    schematic_undo: Vec<SchematicHistoryEntry>,
    schematic_redo: Vec<SchematicHistoryEntry>,
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
            waveform_plot: WaveformPlot::default(),
            result_text: String::new(),
            diagnostic_rows: Vec::new(),
            diagnostics: "Edit a deck, then inspect its runnable analyses or run it.".to_owned(),
            schematic: None,
            selected_schematic_component: None,
            selected_schematic_terminal: 0,
            selected_schematic_route_target: None,
            selected_schematic_analysis_card: 0,
            protocol_version: PROTOCOL_VERSION,
            next_schematic_file_effect_id: default_next_effect_id(),
            pending_schematic_file_effect: None,
            schematic_undo: Vec::new(),
            schematic_redo: Vec::new(),
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

fn normalize(value: f64, low: f64, high: f64, start: f64, end: f64) -> f64 {
    if high <= low {
        return (start + end) / 2.0;
    }
    start + ((value - low) / (high - low)) * (end - start)
}

fn waveform_plot(
    deck: &str,
    selected_analysis_row: usize,
    selected_waveform_row: usize,
) -> Result<WaveformPlot, SpiceMosaicError> {
    let app = parse_berkeley_app_deck(deck);
    let execution = app
        .run_artifacts()
        .map_err(|error| invalid(error.to_string()))?;
    let Some(analysis) = execution.analyses.get(selected_analysis_row) else {
        return Ok(WaveformPlot::default());
    };
    let labels = analysis
        .waveform_series
        .iter()
        .map(|series| series.name.clone())
        .collect::<Vec<_>>();
    let Some(series) = analysis
        .waveform_series
        .get(selected_waveform_row.min(analysis.waveform_series.len().saturating_sub(1)))
    else {
        return Ok(WaveformPlot {
            labels,
            ..WaveformPlot::default()
        });
    };
    let points = series
        .points
        .iter()
        .filter(|point| point.x.is_finite() && point.y.is_finite())
        .collect::<Vec<_>>();
    let x_low = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let x_high = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let y_low = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let y_high = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let segments = points
        .windows(2)
        .map(|pair| {
            let first = pair[0];
            let second = pair[1];
            [
                normalize(first.x, x_low, x_high, 28.0, 344.0),
                normalize(first.y, y_low, y_high, 188.0, 20.0),
                normalize(second.x, x_low, x_high, 28.0, 344.0),
                normalize(second.y, y_low, y_high, 188.0, 20.0),
            ]
        })
        .collect();

    Ok(WaveformPlot {
        labels,
        selected_label: series.name.clone(),
        axis_label: format!("{} versus {}", series.y_column, series.x_column),
        segments,
    })
}

impl SpiceMosaicApp {
    fn schematic_rows(&self) -> Vec<String> {
        self.schematic
            .as_ref()
            .map(|document| {
                document
                    .components
                    .iter()
                    .map(|component| component.reference.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn selected_schematic_component(&self) -> Option<&SchematicComponent> {
        let reference = self.selected_schematic_component.as_deref()?;
        self.schematic
            .as_ref()?
            .components
            .iter()
            .find(|component| component.reference == reference)
    }

    fn selected_schematic_terminal(&self) -> Option<SchematicPoint> {
        self.selected_schematic_component()?
            .terminals
            .get(self.selected_schematic_terminal)
            .copied()
    }

    fn schematic_history_entry(&self) -> SchematicHistoryEntry {
        SchematicHistoryEntry {
            document: self.schematic.clone(),
            selected_component: self.selected_schematic_component.clone(),
            selected_terminal: self.selected_schematic_terminal,
            selected_route_target: self.selected_schematic_route_target.clone(),
            selected_analysis_card: self.selected_schematic_analysis_card,
        }
    }

    fn record_schematic_edit(&mut self, before: SchematicHistoryEntry) {
        if before == self.schematic_history_entry() {
            return;
        }
        if self.schematic_undo.len() == MAX_SCHEMATIC_HISTORY_ENTRIES {
            self.schematic_undo.remove(0);
        }
        self.schematic_undo.push(before);
        self.schematic_redo.clear();
    }

    fn restore_schematic_history_entry(
        &mut self,
        entry: SchematicHistoryEntry,
    ) -> Result<(), SpiceMosaicError> {
        if let Some(document) = &entry.document {
            if let Some(reference) = &entry.selected_component {
                if !document
                    .components
                    .iter()
                    .any(|component| component.reference == *reference)
                {
                    return Err(invalid("schematic history selection is unknown"));
                }
            }
            if entry.selected_analysis_card >= document.analysis_card_count() {
                return Err(invalid(
                    "schematic history analysis selection is out of range",
                ));
            }
            if let Some(reference) = &entry.selected_component {
                let component = document
                    .components
                    .iter()
                    .find(|component| component.reference == *reference)
                    .expect("the component selection was checked above");
                if entry.selected_terminal >= component.terminals.len() {
                    return Err(invalid(
                        "schematic history terminal selection is out of range",
                    ));
                }
            } else if entry.selected_terminal != 0 {
                return Err(invalid(
                    "schematic history without a component has a terminal selection",
                ));
            }
            if let Some(target) = &entry.selected_route_target {
                let Some(selected) = &entry.selected_component else {
                    return Err(invalid(
                        "schematic history route target requires a component selection",
                    ));
                };
                if selected == target {
                    return Err(invalid(
                        "schematic history route target matches the selected component",
                    ));
                }
                if !document
                    .components
                    .iter()
                    .any(|component| component.reference == *target)
                {
                    return Err(invalid("schematic history route target is unknown"));
                }
            }
        } else if entry.selected_component.is_some()
            || entry.selected_terminal != 0
            || entry.selected_route_target.is_some()
            || entry.selected_analysis_card != 0
        {
            return Err(invalid(
                "schematic history without a document has a selection",
            ));
        }
        self.schematic = entry.document;
        self.selected_schematic_component = entry.selected_component;
        self.selected_schematic_terminal = entry.selected_terminal;
        self.selected_schematic_route_target = entry.selected_route_target;
        self.selected_schematic_analysis_card = entry.selected_analysis_card;
        Ok(())
    }

    fn undo_schematic_edit(&mut self) -> Result<(), SpiceMosaicError> {
        let previous = self
            .schematic_undo
            .pop()
            .ok_or_else(|| invalid("undoSchematic requires a prior schematic edit"))?;
        let current = self.schematic_history_entry();
        self.restore_schematic_history_entry(previous)?;
        self.schematic_redo.push(current);
        Ok(())
    }

    fn redo_schematic_edit(&mut self) -> Result<(), SpiceMosaicError> {
        let next = self
            .schematic_redo
            .pop()
            .ok_or_else(|| invalid("redoSchematic requires an undone schematic edit"))?;
        let current = self.schematic_history_entry();
        self.restore_schematic_history_entry(next)?;
        if self.schematic_undo.len() == MAX_SCHEMATIC_HISTORY_ENTRIES {
            self.schematic_undo.remove(0);
        }
        self.schematic_undo.push(current);
        Ok(())
    }

    fn schematic_grid_lines() -> Vec<[f64; 4]> {
        (0..=8)
            .flat_map(|index| {
                let x = 20.0 + f64::from(index) * 40.0;
                let y = 20.0 + f64::from(index) * 25.0;
                [[x, 20.0, x, 220.0], [20.0, y, 340.0, y]]
            })
            .collect()
    }

    fn schematic_geometry(&self) -> (Vec<[f64; 4]>, Vec<[f64; 2]>) {
        let Some(document) = &self.schematic else {
            return (Vec::new(), Vec::new());
        };
        let points = document
            .components
            .iter()
            .flat_map(|component| component.terminals.iter().copied())
            .chain(
                document
                    .wires
                    .iter()
                    .flat_map(|wire| [wire.start, wire.end]),
            )
            .collect::<BTreeSet<_>>();
        let Some(minimum) = points.first().copied() else {
            return (Vec::new(), Vec::new());
        };
        let maximum = points.last().copied().unwrap_or(minimum);
        let project = |point: SchematicPoint| {
            let x_span = (maximum.x - minimum.x).max(1) as f64;
            let y_span = (maximum.y - minimum.y).max(1) as f64;
            [
                20.0 + f64::from(point.x - minimum.x) / x_span * 320.0,
                20.0 + f64::from(point.y - minimum.y) / y_span * 200.0,
            ]
        };
        let segments = document
            .wires
            .iter()
            .flat_map(|wire| {
                let start = project(wire.start);
                let end = project(wire.end);
                if wire.start.x == wire.end.x || wire.start.y == wire.end.y {
                    vec![[start[0], start[1], end[0], end[1]]]
                } else {
                    vec![
                        [start[0], start[1], end[0], start[1]],
                        [end[0], start[1], end[0], end[1]],
                    ]
                }
            })
            .collect();
        let terminals = points.into_iter().map(project).collect();
        (segments, terminals)
    }

    fn update(&self) -> AppUpdate {
        let (schematic_wire_segments, schematic_terminal_points) = self.schematic_geometry();
        let selected_schematic_component = self.selected_schematic_component();
        let selected_schematic_terminal = selected_schematic_component.and_then(|component| {
            component
                .terminals
                .get(self.selected_schematic_terminal)
                .copied()
        });
        let schematic_terminal_controls = selected_schematic_component
            .map(|component| {
                component
                    .terminals
                    .iter()
                    .enumerate()
                    .map(|(index, _)| format!("Terminal {}", index + 1))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let selected_schematic_terminal_label = selected_schematic_terminal
            .map(|_| format!("Terminal {}", self.selected_schematic_terminal + 1))
            .unwrap_or_else(|| "Select a component".to_owned());
        let schematic_net_label = selected_schematic_terminal
            .and_then(|point| self.schematic.as_ref()?.net_label_at(point))
            .unwrap_or("");
        let schematic_route_targets = self
            .schematic
            .as_ref()
            .map(|document| {
                document
                    .components
                    .iter()
                    .filter(|component| {
                        Some(component.reference.as_str())
                            != self.selected_schematic_component.as_deref()
                    })
                    .map(|component| component.reference.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let selected_schematic_route_target = self
            .selected_schematic_route_target
            .as_deref()
            .and_then(|reference| {
                self.schematic
                    .as_ref()?
                    .components
                    .iter()
                    .find(|component| component.reference == reference)
            });
        let schematic_route_terminal_controls = selected_schematic_route_target
            .map(|component| {
                (0..component.terminals.len())
                    .map(|index| format!("Terminal {}", index + 1))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let schematic_value_disabled = selected_schematic_component
            .is_none_or(|component| component.kind == SchematicComponentKind::Ground);
        let selected_schematic_analysis_card = self
            .schematic
            .as_ref()
            .map(|document| {
                self.selected_schematic_analysis_card
                    .min(document.analysis_card_count().saturating_sub(1))
            })
            .unwrap_or(0);
        let schematic_analysis_card_count = self
            .schematic
            .as_ref()
            .map(SchematicDocument::analysis_card_count)
            .unwrap_or(0);
        let (
            schematic_analysis_source_options,
            selected_schematic_analysis_source_label,
            schematic_analysis_parameter_labels,
            schematic_analysis_parameter_values,
            schematic_analysis_parameter_disabled,
        ) = self
            .schematic
            .as_ref()
            .map(|document| {
                let card = document
                    .analysis_card(selected_schematic_analysis_card)
                    .expect("a schematic always exposes one effective analysis card");
                let labels = document
                    .analysis_card_parameter_labels(selected_schematic_analysis_card)
                    .expect("the selected card is in range")
                    .map(str::to_owned);
                let values = document
                    .analysis_card_parameter_values(selected_schematic_analysis_card)
                    .expect("the selected card is in range")
                    .map(str::to_owned);
                let source_options = if matches!(
                    card.analysis,
                    SchematicAnalysis::DcSweep | SchematicAnalysis::TransferFunction
                ) {
                    document
                        .independent_source_references()
                        .into_iter()
                        .map(str::to_owned)
                        .collect()
                } else {
                    Vec::new()
                };
                let selected_source = if matches!(
                    card.analysis,
                    SchematicAnalysis::DcSweep | SchematicAnalysis::TransferFunction
                ) && source_options.iter().any(|source| {
                    source
                        == match card.analysis {
                            SchematicAnalysis::DcSweep => &card.settings.dc_source,
                            SchematicAnalysis::TransferFunction => &card.settings.tf_source,
                            _ => unreachable!("source availability was checked above"),
                        }
                }) {
                    match card.analysis {
                        SchematicAnalysis::DcSweep => card.settings.dc_source,
                        SchematicAnalysis::TransferFunction => card.settings.tf_source,
                        _ => unreachable!("source availability was checked above"),
                    }
                } else if card.analysis == SchematicAnalysis::DcSweep {
                    "Select a DC source".to_owned()
                } else if card.analysis == SchematicAnalysis::TransferFunction {
                    "Select a transfer-function source".to_owned()
                } else {
                    "No DC source required".to_owned()
                };
                let disabled = labels.each_ref().map(|label| label.is_empty());
                (source_options, selected_source, labels, values, disabled)
            })
            .unwrap_or_else(|| {
                (
                    Vec::new(),
                    "Select a schematic analysis".to_owned(),
                    [String::new(), String::new(), String::new()],
                    [String::new(), String::new(), String::new()],
                    [true, true, true],
                )
            });
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
            "waveform-label": "Waveforms",
            "waveform-rows": self.waveform_plot.labels,
            "selected-waveform-label": self.waveform_plot.selected_label,
            "waveform-axis-label": self.waveform_plot.axis_label,
            "waveform-segments": self.waveform_plot.segments,
            "raw-result-label": "Raw result JSON",
            "result-text": self.result_text,
            "schematic-label": "Schematic",
            "schematic-title-label": "Schematic title",
            "schematic-title": self.schematic.as_ref().map(|document| document.title.as_str()).unwrap_or("No schematic loaded"),
            "schematic-title-disabled": self.schematic.is_none(),
            "open-schematic-label": "Open schematic",
            "save-schematic-label": "Save schematic",
            "undo-schematic-label": "Undo",
            "undo-schematic-disabled": self.schematic_undo.is_empty(),
            "redo-schematic-label": "Redo",
            "redo-schematic-disabled": self.schematic_redo.is_empty(),
            "schematic-rows": self.schematic_rows(),
            "schematic-palette": [
                SchematicComponentKind::Resistor.palette_label(),
                SchematicComponentKind::Capacitor.palette_label(),
                SchematicComponentKind::Inductor.palette_label(),
                SchematicComponentKind::DcVoltage.palette_label(),
                SchematicComponentKind::DcCurrent.palette_label(),
                SchematicComponentKind::AcVoltage.palette_label(),
                SchematicComponentKind::Ground.palette_label(),
            ],
            "schematic-analysis-label": "Add analysis card",
            "schematic-analysis-controls": [
                SchematicAnalysis::OperatingPoint.palette_label(),
                SchematicAnalysis::DcSweep.palette_label(),
                SchematicAnalysis::AcSweep.palette_label(),
                SchematicAnalysis::Transient.palette_label(),
                SchematicAnalysis::TransferFunction.palette_label(),
            ],
            "schematic-analysis-card-label": "Analysis plan",
            "schematic-analysis-card-rows": self.schematic.as_ref().map(|document| document.analysis_card_labels().into_iter().map(|label| vec![label]).collect::<Vec<_>>()).unwrap_or_default(),
            "selected-schematic-analysis-label": self.schematic.as_ref().and_then(|document| document.analysis_card(selected_schematic_analysis_card)).map(|card| card.analysis.palette_label()).unwrap_or(SchematicAnalysis::default().palette_label()),
            "schematic-analysis-kind-label": "Change selected card",
            "schematic-analysis-kind-disabled": self.schematic.is_none(),
            "schematic-analysis-kind-controls": [
                SchematicAnalysis::OperatingPoint.palette_label(),
                SchematicAnalysis::DcSweep.palette_label(),
                SchematicAnalysis::AcSweep.palette_label(),
                SchematicAnalysis::Transient.palette_label(),
                SchematicAnalysis::TransferFunction.palette_label(),
            ],
            "move-schematic-analysis-card-earlier-label": "Move earlier",
            "move-schematic-analysis-card-earlier-disabled": selected_schematic_analysis_card == 0,
            "move-schematic-analysis-card-later-label": "Move later",
            "move-schematic-analysis-card-later-disabled": schematic_analysis_card_count == 0 || selected_schematic_analysis_card + 1 >= schematic_analysis_card_count,
            "schematic-analysis-configuration-label": "Selected card configuration",
            "remove-schematic-analysis-card-label": "Remove selected card",
            "schematic-analysis-source-label": self.schematic.as_ref().and_then(|document| document.analysis_card(selected_schematic_analysis_card)).map(|card| match card.analysis {
                SchematicAnalysis::DcSweep => "DC sweep source",
                SchematicAnalysis::TransferFunction => "Transfer-function source",
                _ => "Independent source",
            }).unwrap_or("Independent source"),
            "schematic-analysis-source-options": schematic_analysis_source_options,
            "selected-schematic-analysis-source-label": selected_schematic_analysis_source_label,
            "schematic-analysis-parameter-one-label": schematic_analysis_parameter_labels[0],
            "schematic-analysis-parameter-one-value": schematic_analysis_parameter_values[0],
            "schematic-analysis-parameter-one-disabled": schematic_analysis_parameter_disabled[0],
            "schematic-analysis-parameter-two-label": schematic_analysis_parameter_labels[1],
            "schematic-analysis-parameter-two-value": schematic_analysis_parameter_values[1],
            "schematic-analysis-parameter-two-disabled": schematic_analysis_parameter_disabled[1],
            "schematic-analysis-parameter-three-label": schematic_analysis_parameter_labels[2],
            "schematic-analysis-parameter-three-value": schematic_analysis_parameter_values[2],
            "schematic-analysis-parameter-three-disabled": schematic_analysis_parameter_disabled[2],
            "schematic-grid-label": "Grid routing",
            "schematic-grid-lines": Self::schematic_grid_lines(),
            "schematic-wire-segments": schematic_wire_segments,
            "schematic-terminal-points": schematic_terminal_points,
            "schematic-wire-label": "Wires",
            "schematic-wire-rows": self.schematic.as_ref().map(|document| document.wires.iter().enumerate().map(|(index, _)| format!("Remove wire {}", index + 1)).collect::<Vec<_>>()).unwrap_or_default(),
            "selected-schematic-label": self.selected_schematic_component.as_deref().unwrap_or("No component selected"),
            "schematic-properties-label": "Component properties",
            "selected-schematic-kind-label": selected_schematic_component.map(|component| component.kind.palette_label()).unwrap_or("Select a component"),
            "schematic-terminal-label": "Terminal",
            "schematic-terminal-controls": schematic_terminal_controls,
            "selected-schematic-terminal-label": selected_schematic_terminal_label,
            "schematic-terminal-disabled": selected_schematic_component.is_none(),
            "schematic-net-label-label": "Net label",
            "schematic-net-label": schematic_net_label,
            "schematic-net-label-placeholder": "Name this Berkeley net",
            "schematic-net-label-disabled": selected_schematic_component.is_none(),
            "schematic-reference-label": "Reference",
            "schematic-reference": selected_schematic_component.map(|component| component.reference.as_str()).unwrap_or(""),
            "schematic-reference-placeholder": "Select a component",
            "schematic-reference-disabled": selected_schematic_component.is_none(),
            "schematic-value-label": "SPICE value",
            "schematic-value": selected_schematic_component.map(|component| component.value.as_str()).unwrap_or(""),
            "schematic-value-placeholder": "Select a non-ground component",
            "schematic-value-disabled": schematic_value_disabled,
            "remove-schematic-component-label": "Remove selected component",
            "remove-schematic-component-disabled": selected_schematic_component.is_none(),
            "route-schematic-label": "Route selected terminal to",
            "schematic-route-targets": schematic_route_targets,
            "selected-schematic-route-target-label": selected_schematic_route_target.map(|component| component.reference.as_str()).unwrap_or("Select a target component"),
            "schematic-route-terminal-controls": schematic_route_terminal_controls,
            "schematic-route-terminal-disabled": selected_schematic_component.is_none() || selected_schematic_route_target.is_none(),
            "synchronize-schematic-label": "Sync netlist",
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
        self.waveform_plot = WaveformPlot::default();
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
        let waveform_plot = waveform_plot(&self.deck, selected_analysis_row, 0)?;
        let diagnostic_rows = diagnostic_rows(&self.deck);
        self.selected_analysis_row = selected_analysis_row;
        self.diagnostics = format!("Executed {} analyses.", analyses.len());
        self.result_text = result;
        self.result_table = result_table;
        self.waveform_plot = waveform_plot;
        self.diagnostic_rows = diagnostic_rows;
        self.analyses = analyses;
        self.mode = "Results";
        Ok(self.announced(self.diagnostics.clone()))
    }

    fn load_schematic(
        &mut self,
        mut document: SchematicDocument,
    ) -> Result<AppUpdate, SpiceMosaicError> {
        let history = self.schematic_history_entry();
        document.migrate_legacy_analysis_cards();
        document
            .validate()
            .map_err(|error| invalid(error.to_string()))?;
        self.selected_schematic_component = None;
        self.selected_schematic_terminal = 0;
        self.selected_schematic_route_target = None;
        self.selected_schematic_analysis_card = 0;
        self.schematic = Some(document);
        self.record_schematic_edit(history);
        self.diagnostics = "Schematic loaded. Sync its canonical netlist when ready.".to_owned();
        self.mode = "Schematic";
        Ok(self.announced(self.diagnostics.clone()))
    }

    fn suggested_schematic_file_name(document: &SchematicDocument) -> String {
        let name = document
            .title
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect::<String>();
        let name = name.trim_matches('-');
        let name = if name.is_empty() { "schematic" } else { name };
        format!("{}.spice-mosaic.json", &name[..name.len().min(80)])
    }

    fn canonical_schematic_file(document: &SchematicDocument) -> Result<Vec<u8>, SpiceMosaicError> {
        let mut document = document.clone();
        document.migrate_legacy_analysis_cards();
        document
            .validate()
            .map_err(|error| invalid(error.to_string()))?;
        serde_json::to_vec(&SchematicFile {
            schema: SCHEMATIC_FILE_SCHEMA.to_owned(),
            version: SCHEMATIC_FILE_VERSION,
            document,
        })
        .map_err(|error| invalid(error.to_string()))
    }

    fn request_schematic_file(
        &mut self,
        operation: SchematicFileOperation,
        payload: Value,
    ) -> Result<AppUpdate, SpiceMosaicError> {
        if self.protocol_version < EFFECT_PROTOCOL_VERSION {
            return Err(invalid("schematic document I/O requires Mosaic protocol 2"));
        }
        if self.pending_schematic_file_effect.is_some() {
            return Err(invalid(
                "a schematic document operation is already in progress",
            ));
        }
        let id = self.next_schematic_file_effect_id;
        if id == 0 || id > MAX_EFFECT_ID {
            return Err(invalid(
                "schematic document effect identifiers are exhausted",
            ));
        }
        self.next_schematic_file_effect_id = id.saturating_add(1);
        self.pending_schematic_file_effect = Some(PendingSchematicFileEffect { id, operation });
        let mut update = self.announced(format!("{} requested.", operation.label()));
        update.effects.push(Effect {
            id,
            delivery: Delivery::Await,
            kind: operation.effect_kind().to_owned(),
            payload,
        });
        Ok(update)
    }

    fn request_schematic_open(&mut self) -> Result<AppUpdate, SpiceMosaicError> {
        self.request_schematic_file(
            SchematicFileOperation::Open,
            json!({"mimeType": "application/json", "extension": ".json"}),
        )
    }

    fn request_schematic_save(&mut self) -> Result<AppUpdate, SpiceMosaicError> {
        let document = self
            .schematic
            .as_ref()
            .ok_or_else(|| invalid("saveSchematic requires a loaded schematic"))?;
        let bytes = Self::canonical_schematic_file(document)?;
        self.request_schematic_file(
            SchematicFileOperation::Save,
            json!({
                "mimeType": "application/json",
                "extension": ".json",
                "suggestedName": Self::suggested_schematic_file_name(document),
                "bytes": BASE64.encode(bytes),
            }),
        )
    }

    fn complete_schematic_file(
        &mut self,
        pending: PendingSchematicFileEffect,
        result: EffectResult,
    ) -> Result<AppUpdate, SpiceMosaicError> {
        match result {
            EffectResult::Cancelled(_) => {
                self.pending_schematic_file_effect = None;
                self.diagnostics = format!("{} cancelled.", pending.operation.label());
                Ok(self.announced(self.diagnostics.clone()))
            }
            EffectResult::Failed(failure) => {
                self.pending_schematic_file_effect = None;
                self.diagnostics =
                    format!("{} failed: {}", pending.operation.label(), failure.message);
                Ok(self.announced(self.diagnostics.clone()))
            }
            EffectResult::Ok(_) if pending.operation == SchematicFileOperation::Save => {
                self.pending_schematic_file_effect = None;
                self.diagnostics = "Schematic exported.".to_owned();
                Ok(self.announced(self.diagnostics.clone()))
            }
            EffectResult::Ok(payload) => {
                let encoded = payload["bytes"]
                    .as_str()
                    .ok_or_else(|| invalid("schematic import result is missing base64 bytes"))?;
                let bytes = BASE64.decode(encoded).map_err(|error| {
                    invalid(format!("schematic import bytes are invalid: {error}"))
                })?;
                if bytes.len() > MAX_SCHEMATIC_FILE_BYTES {
                    return Err(invalid("schematic import exceeds 16 MiB"));
                }
                let mut file: SchematicFile = serde_json::from_slice(&bytes).map_err(|error| {
                    invalid(format!("schematic import is invalid JSON: {error}"))
                })?;
                if file.schema != SCHEMATIC_FILE_SCHEMA || file.version != SCHEMATIC_FILE_VERSION {
                    return Err(invalid("unsupported schematic document format"));
                }
                file.document.migrate_legacy_analysis_cards();
                file.document
                    .validate()
                    .map_err(|error| invalid(error.to_string()))?;
                self.pending_schematic_file_effect = None;
                self.load_schematic(file.document)
            }
        }
    }

    fn synchronize_schematic(&mut self) -> Result<AppUpdate, SpiceMosaicError> {
        let deck = self
            .schematic
            .as_ref()
            .ok_or_else(|| invalid("synchronizeSchematic requires a loaded schematic"))?
            .to_berkeley_netlist()
            .map_err(|error| invalid(error.to_string()))?;
        self.deck = deck;
        self.inspect()
    }
}

impl MosaicApp for SpiceMosaicApp {
    type Error = SpiceMosaicError;

    fn start(&mut self, context: StartContext) -> Result<AppUpdate, Self::Error> {
        self.protocol_version = context.protocol_version;
        self.dark = context.color_scheme == ColorScheme::Dark;
        if let Some(snapshot) = context.restored_snapshot {
            return self.restore(snapshot);
        }
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
                self.waveform_plot = WaveformPlot::default();
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
                let waveform_plot = if self.mode == "Results" {
                    waveform_plot(&self.deck, index, 0)?
                } else {
                    WaveformPlot::default()
                };
                self.selected_analysis_row = index;
                self.result_table = result_table;
                self.waveform_plot = waveform_plot;
                Ok(self.announced(format!("Selected .{} analysis.", self.analyses[index].kind)))
            }
            "selectWaveform" => {
                if self.mode != "Results" {
                    return Err(invalid("selectWaveform requires a completed run"));
                }
                let index = event.payload["index"]
                    .as_u64()
                    .ok_or_else(|| invalid("selectWaveform requires non-negative index"))?
                    as usize;
                let waveform_plot = waveform_plot(&self.deck, self.selected_analysis_row, index)?;
                if index >= waveform_plot.labels.len() {
                    return Err(invalid("selectWaveform index is out of range"));
                }
                self.waveform_plot = waveform_plot;
                Ok(self.announced(format!(
                    "Selected {} waveform.",
                    self.waveform_plot.selected_label
                )))
            }
            "schematicLoad" => {
                let document =
                    serde_json::from_value(event.payload["document"].clone()).map_err(|error| {
                        invalid(format!("schematicLoad requires a document: {error}"))
                    })?;
                self.load_schematic(document)
            }
            "schematicTitleChange" => {
                let title = event.payload["value"]
                    .as_str()
                    .ok_or_else(|| invalid("schematicTitleChange requires text value"))?;
                let history = self.schematic_history_entry();
                let document = self
                    .schematic
                    .as_mut()
                    .ok_or_else(|| invalid("schematicTitleChange requires a loaded schematic"))?;
                document
                    .set_title(title)
                    .map_err(|error| invalid(error.to_string()))?;
                self.record_schematic_edit(history);
                self.diagnostics = "Updated schematic title. Save or sync when ready.".to_owned();
                Ok(self.announced(self.diagnostics.clone()))
            }
            "openSchematic" => self.request_schematic_open(),
            "saveSchematic" => self.request_schematic_save(),
            "placeSchematicComponent" => {
                let kind = event.payload["kind"]
                    .as_str()
                    .ok_or_else(|| invalid("placeSchematicComponent requires a palette kind"))?;
                let kind = SchematicComponentKind::from_palette_label(kind)
                    .map_err(|error| invalid(error.to_string()))?;
                let history = self.schematic_history_entry();
                let document = self.schematic.get_or_insert_with(|| SchematicDocument {
                    title: "Untitled schematic".to_owned(),
                    components: Vec::new(),
                    wires: Vec::new(),
                    net_labels: Vec::new(),
                    analysis: SchematicAnalysis::default(),
                    analysis_settings: SchematicAnalysisSettings::default(),
                    analysis_cards: Vec::new(),
                });
                let reference = document
                    .place_palette_component(kind)
                    .map_err(|error| invalid(error.to_string()))?;
                self.selected_schematic_component = Some(reference.clone());
                self.selected_schematic_terminal = 0;
                self.selected_schematic_route_target = None;
                self.record_schematic_edit(history);
                self.diagnostics = format!(
                    "Placed {reference} on the grid. Select a component and route it to a target."
                );
                self.mode = "Schematic";
                Ok(self.announced(self.diagnostics.clone()))
            }
            "addSchematicAnalysis" => {
                let analysis = event.payload["analysis"]
                    .as_str()
                    .ok_or_else(|| invalid("addSchematicAnalysis requires an analysis"))?;
                let analysis = SchematicAnalysis::from_palette_label(analysis)
                    .map_err(|error| invalid(error.to_string()))?;
                let history = self.schematic_history_entry();
                let document = self.schematic.get_or_insert_with(|| SchematicDocument {
                    title: "Untitled schematic".to_owned(),
                    components: Vec::new(),
                    wires: Vec::new(),
                    net_labels: Vec::new(),
                    analysis: SchematicAnalysis::default(),
                    analysis_settings: SchematicAnalysisSettings::default(),
                    analysis_cards: Vec::new(),
                });
                self.selected_schematic_analysis_card = document.add_analysis_card(analysis);
                self.record_schematic_edit(history);
                self.diagnostics = format!(
                    "Added {} to the canonical schematic plan.",
                    analysis.palette_label()
                );
                self.mode = "Schematic";
                Ok(self.announced(self.diagnostics.clone()))
            }
            "selectSchematicAnalysis" => {
                let analysis = event.payload["analysis"]
                    .as_str()
                    .ok_or_else(|| invalid("selectSchematicAnalysis requires an analysis"))?;
                let analysis = SchematicAnalysis::from_palette_label(analysis)
                    .map_err(|error| invalid(error.to_string()))?;
                let history = self.schematic_history_entry();
                let document = self.schematic.as_mut().ok_or_else(|| {
                    invalid("selectSchematicAnalysis requires a loaded schematic")
                })?;
                document
                    .set_analysis_card_kind(self.selected_schematic_analysis_card, analysis)
                    .map_err(|error| invalid(error.to_string()))?;
                self.record_schematic_edit(history);
                self.diagnostics = format!(
                    "Selected {} for the active schematic analysis card.",
                    analysis.palette_label()
                );
                Ok(self.announced(self.diagnostics.clone()))
            }
            "selectSchematicAnalysisCard" => {
                let index = event.payload["index"].as_u64().ok_or_else(|| {
                    invalid("selectSchematicAnalysisCard requires non-negative index")
                })? as usize;
                let document = self.schematic.as_ref().ok_or_else(|| {
                    invalid("selectSchematicAnalysisCard requires a loaded schematic")
                })?;
                if index >= document.analysis_card_count() {
                    return Err(invalid("selectSchematicAnalysisCard index is out of range"));
                }
                self.selected_schematic_analysis_card = index;
                Ok(self.announced(format!(
                    "Selected {}.",
                    document.analysis_card_labels()[index]
                )))
            }
            "moveSchematicAnalysisCardEarlier" | "moveSchematicAnalysisCardLater" => {
                let direction = event_name(&event.name);
                let card_count = self
                    .schematic
                    .as_ref()
                    .ok_or_else(|| {
                        invalid("moveSchematicAnalysisCard requires a loaded schematic")
                    })?
                    .analysis_card_count();
                let target = match direction.as_str() {
                    "moveSchematicAnalysisCardEarlier" => self
                        .selected_schematic_analysis_card
                        .checked_sub(1)
                        .ok_or_else(|| {
                            invalid("selected schematic analysis card is already first")
                        })?,
                    "moveSchematicAnalysisCardLater" => {
                        if self.selected_schematic_analysis_card + 1 >= card_count {
                            return Err(invalid(
                                "selected schematic analysis card is already last",
                            ));
                        }
                        self.selected_schematic_analysis_card + 1
                    }
                    _ => unreachable!("the match arm lists every analysis-card move event"),
                };
                let history = self.schematic_history_entry();
                self.schematic
                    .as_mut()
                    .expect("the schematic was checked before moving a card")
                    .move_analysis_card(self.selected_schematic_analysis_card, target)
                    .map_err(|error| invalid(error.to_string()))?;
                self.selected_schematic_analysis_card = target;
                self.record_schematic_edit(history);
                self.diagnostics = format!(
                    "Moved the selected schematic analysis card {}.",
                    if direction == "moveSchematicAnalysisCardEarlier" {
                        "earlier"
                    } else {
                        "later"
                    }
                );
                Ok(self.announced(self.diagnostics.clone()))
            }
            "removeSchematicAnalysisCard" => {
                let history = self.schematic_history_entry();
                let document = self.schematic.as_mut().ok_or_else(|| {
                    invalid("removeSchematicAnalysisCard requires a loaded schematic")
                })?;
                document
                    .remove_analysis_card(self.selected_schematic_analysis_card)
                    .map_err(|error| invalid(error.to_string()))?;
                self.selected_schematic_analysis_card = self
                    .selected_schematic_analysis_card
                    .min(document.analysis_card_count().saturating_sub(1));
                self.record_schematic_edit(history);
                self.diagnostics = "Removed the selected schematic analysis card.".to_owned();
                Ok(self.announced(self.diagnostics.clone()))
            }
            "selectSchematicAnalysisSource" => {
                let reference = event.payload["reference"].as_str().ok_or_else(|| {
                    invalid("selectSchematicAnalysisSource requires a source reference")
                })?;
                let history = self.schematic_history_entry();
                let document = self.schematic.as_mut().ok_or_else(|| {
                    invalid("selectSchematicAnalysisSource requires a loaded schematic")
                })?;
                document
                    .set_analysis_card_source(self.selected_schematic_analysis_card, reference)
                    .map_err(|error| invalid(error.to_string()))?;
                self.record_schematic_edit(history);
                self.diagnostics = format!(
                    "Selected {reference} as the canonical analysis source. Sync the netlist when ready."
                );
                Ok(self.announced(self.diagnostics.clone()))
            }
            "schematicAnalysisParameterOneChange"
            | "schematicAnalysisParameterTwoChange"
            | "schematicAnalysisParameterThreeChange" => {
                let value = event.payload["value"].as_str().ok_or_else(|| {
                    invalid("schematic analysis parameter change requires text value")
                })?;
                let index = match event_name(&event.name).as_str() {
                    "schematicAnalysisParameterOneChange" => 0,
                    "schematicAnalysisParameterTwoChange" => 1,
                    "schematicAnalysisParameterThreeChange" => 2,
                    _ => unreachable!("the match arm lists every analysis parameter event"),
                };
                let history = self.schematic_history_entry();
                let document = self.schematic.as_mut().ok_or_else(|| {
                    invalid("schematic analysis parameter change requires a loaded schematic")
                })?;
                document
                    .set_analysis_card_parameter(
                        self.selected_schematic_analysis_card,
                        index,
                        value,
                    )
                    .map_err(|error| invalid(error.to_string()))?;
                self.record_schematic_edit(history);
                self.diagnostics =
                    "Updated the canonical analysis card. Sync the netlist when ready.".to_owned();
                Ok(self.announced(self.diagnostics.clone()))
            }
            "schematicPlace" => {
                let component: SchematicComponent =
                    serde_json::from_value(event.payload["component"].clone()).map_err(
                        |error| invalid(format!("schematicPlace requires a component: {error}")),
                    )?;
                let history = self.schematic_history_entry();
                let document = self
                    .schematic
                    .as_mut()
                    .ok_or_else(|| invalid("schematicPlace requires a loaded schematic"))?;
                if document
                    .components
                    .iter()
                    .any(|existing| existing.reference == component.reference)
                {
                    return Err(invalid("schematicPlace component reference already exists"));
                }
                document.components.push(component);
                self.record_schematic_edit(history);
                self.diagnostics =
                    "Component placed. Connect endpoints, then sync the netlist.".to_owned();
                Ok(self.announced(self.diagnostics.clone()))
            }
            "schematicConnect" => {
                let wire: SchematicWire = serde_json::from_value(event.payload["wire"].clone())
                    .map_err(|error| {
                        invalid(format!("schematicConnect requires a wire: {error}"))
                    })?;
                let history = self.schematic_history_entry();
                let document = self
                    .schematic
                    .as_mut()
                    .ok_or_else(|| invalid("schematicConnect requires a loaded schematic"))?;
                document
                    .connect_wire(wire)
                    .map_err(|error| invalid(error.to_string()))?;
                self.record_schematic_edit(history);
                self.diagnostics = "Endpoints connected. Sync the netlist when ready.".to_owned();
                Ok(self.announced(self.diagnostics.clone()))
            }
            "selectSchematicComponent" => {
                let reference = event.payload["reference"]
                    .as_str()
                    .ok_or_else(|| invalid("selectSchematicComponent requires reference"))?;
                let document = self.schematic.as_ref().ok_or_else(|| {
                    invalid("selectSchematicComponent requires a loaded schematic")
                })?;
                if !document
                    .components
                    .iter()
                    .any(|component| component.reference == reference)
                {
                    return Err(invalid("selectSchematicComponent reference is unknown"));
                }
                self.selected_schematic_component = Some(reference.to_owned());
                self.selected_schematic_terminal = 0;
                self.selected_schematic_route_target = None;
                Ok(self.announced(format!("Selected {reference}.")))
            }
            "selectSchematicTerminal" => {
                let index = event.payload["index"].as_u64().ok_or_else(|| {
                    invalid("selectSchematicTerminal requires a non-negative index")
                })? as usize;
                let component = self.selected_schematic_component().ok_or_else(|| {
                    invalid("selectSchematicTerminal requires a selected component")
                })?;
                if index >= component.terminals.len() {
                    return Err(invalid("selectSchematicTerminal index is out of range"));
                }
                self.selected_schematic_terminal = index;
                Ok(self.announced(format!("Selected Terminal {}.", index + 1)))
            }
            "schematicNetLabelChange" => {
                let name = event.payload["value"]
                    .as_str()
                    .ok_or_else(|| invalid("schematicNetLabelChange requires text value"))?;
                let point = self.selected_schematic_terminal().ok_or_else(|| {
                    invalid("schematicNetLabelChange requires a selected component terminal")
                })?;
                let history = self.schematic_history_entry();
                let document = self.schematic.as_mut().ok_or_else(|| {
                    invalid("schematicNetLabelChange requires a loaded schematic")
                })?;
                document
                    .set_net_label(point, name)
                    .map_err(|error| invalid(error.to_string()))?;
                self.record_schematic_edit(history);
                self.diagnostics = if name.trim().is_empty() {
                    "Cleared the selected terminal net label. Sync the netlist when ready."
                        .to_owned()
                } else {
                    format!(
                        "Named the selected terminal net {}. Sync the netlist when ready.",
                        name.trim()
                    )
                };
                Ok(self.announced(self.diagnostics.clone()))
            }
            "schematicValueChange" => {
                let value = event.payload["value"]
                    .as_str()
                    .ok_or_else(|| invalid("schematicValueChange requires text value"))?;
                let reference = self
                    .selected_schematic_component
                    .clone()
                    .ok_or_else(|| invalid("schematicValueChange requires a selected component"))?;
                let history = self.schematic_history_entry();
                let document = self
                    .schematic
                    .as_mut()
                    .ok_or_else(|| invalid("schematicValueChange requires a loaded schematic"))?;
                document
                    .set_component_value(&reference, value)
                    .map_err(|error| invalid(error.to_string()))?;
                self.record_schematic_edit(history);
                self.diagnostics =
                    format!("Updated {reference} value. Sync the netlist when ready.");
                Ok(self.announced(self.diagnostics.clone()))
            }
            "schematicReferenceChange" => {
                let new_reference = event.payload["value"]
                    .as_str()
                    .ok_or_else(|| invalid("schematicReferenceChange requires text value"))?;
                let current_reference =
                    self.selected_schematic_component.clone().ok_or_else(|| {
                        invalid("schematicReferenceChange requires a selected component")
                    })?;
                let history = self.schematic_history_entry();
                let document = self.schematic.as_mut().ok_or_else(|| {
                    invalid("schematicReferenceChange requires a loaded schematic")
                })?;
                let updated_cards = document
                    .rename_component(&current_reference, new_reference)
                    .map_err(|error| invalid(error.to_string()))?;
                self.selected_schematic_component = Some(new_reference.to_owned());
                self.record_schematic_edit(history);
                self.diagnostics = format!(
                    "Renamed {current_reference} to {new_reference}; updated {updated_cards} DC sweep source binding(s)."
                );
                Ok(self.announced(self.diagnostics.clone()))
            }
            "removeSchematicComponent" => {
                let reference = self.selected_schematic_component.clone().ok_or_else(|| {
                    invalid("removeSchematicComponent requires a selected component")
                })?;
                let history = self.schematic_history_entry();
                let document = self.schematic.as_mut().ok_or_else(|| {
                    invalid("removeSchematicComponent requires a loaded schematic")
                })?;
                let removed_wires = document
                    .remove_component(&reference)
                    .map_err(|error| invalid(error.to_string()))?;
                self.selected_schematic_component = None;
                self.selected_schematic_terminal = 0;
                self.selected_schematic_route_target = None;
                self.record_schematic_edit(history);
                self.diagnostics = format!(
                    "Removed {reference} and {removed_wires} incident wire(s). Sync the netlist when ready."
                );
                Ok(self.announced(self.diagnostics.clone()))
            }
            "removeSchematicWire" => {
                let index = event.payload["index"].as_u64().ok_or_else(|| {
                    invalid("removeSchematicWire requires a non-negative wire index")
                })? as usize;
                let history = self.schematic_history_entry();
                let document = self
                    .schematic
                    .as_mut()
                    .ok_or_else(|| invalid("removeSchematicWire requires a loaded schematic"))?;
                document
                    .remove_wire(index)
                    .map_err(|error| invalid(error.to_string()))?;
                self.record_schematic_edit(history);
                self.diagnostics =
                    format!("Removed wire {}. Sync the netlist when ready.", index + 1);
                Ok(self.announced(self.diagnostics.clone()))
            }
            "selectSchematicRouteTarget" => {
                let target = event.payload["reference"]
                    .as_str()
                    .ok_or_else(|| invalid("selectSchematicRouteTarget requires reference"))?;
                let selected = self
                    .selected_schematic_component
                    .as_deref()
                    .ok_or_else(|| {
                        invalid("selectSchematicRouteTarget requires a selected component")
                    })?;
                if selected == target {
                    return Err(invalid("schematic route requires two different components"));
                }
                let document = self.schematic.as_ref().ok_or_else(|| {
                    invalid("selectSchematicRouteTarget requires a loaded schematic")
                })?;
                if !document
                    .components
                    .iter()
                    .any(|component| component.reference == target)
                {
                    return Err(invalid("selectSchematicRouteTarget reference is unknown"));
                }
                self.selected_schematic_route_target = Some(target.to_owned());
                Ok(self.announced(format!("Selected {target} as the route target.")))
            }
            "routeToSchematicTerminal" => {
                let target_terminal = event.payload["index"].as_u64().ok_or_else(|| {
                    invalid("routeToSchematicTerminal requires a non-negative terminal index")
                })? as usize;
                let selected = self.selected_schematic_component.clone().ok_or_else(|| {
                    invalid("routeToSchematicTerminal requires a selected component")
                })?;
                let target = self
                    .selected_schematic_route_target
                    .clone()
                    .ok_or_else(|| {
                        invalid("routeToSchematicTerminal requires a selected route target")
                    })?;
                let selected_terminal = self.selected_schematic_terminal;
                let history = self.schematic_history_entry();
                let document = self.schematic.as_mut().ok_or_else(|| {
                    invalid("routeToSchematicTerminal requires a loaded schematic")
                })?;
                document
                    .route_terminals(&selected, selected_terminal, &target, target_terminal)
                    .map_err(|error| invalid(error.to_string()))?;
                self.record_schematic_edit(history);
                self.diagnostics = format!(
                    "Routed {selected} Terminal {} to {target} Terminal {} on the schematic grid.",
                    selected_terminal + 1,
                    target_terminal + 1
                );
                Ok(self.announced(self.diagnostics.clone()))
            }
            "undoSchematic" => {
                self.undo_schematic_edit()?;
                self.diagnostics = "Undid the last schematic edit.".to_owned();
                Ok(self.announced(self.diagnostics.clone()))
            }
            "redoSchematic" => {
                self.redo_schematic_edit()?;
                self.diagnostics = "Redid the schematic edit.".to_owned();
                Ok(self.announced(self.diagnostics.clone()))
            }
            "synchronizeSchematic" => self.synchronize_schematic(),
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
            schematic: self.schematic.clone(),
            selected_schematic_component: self.selected_schematic_component.clone(),
            selected_schematic_terminal: self.selected_schematic_terminal,
            selected_schematic_route_target: self.selected_schematic_route_target.clone(),
            selected_schematic_analysis_card: self.selected_schematic_analysis_card,
            next_schematic_file_effect_id: self.next_schematic_file_effect_id,
        })
        .map_err(|error| invalid(error.to_string()))?;
        Ok(Some(Snapshot {
            schema: SNAPSHOT_SCHEMA.into(),
            version: SNAPSHOT_VERSION,
            bytes,
        }))
    }

    fn restore(&mut self, snapshot: Snapshot) -> Result<AppUpdate, Self::Error> {
        if snapshot.schema != SNAPSHOT_SCHEMA || !(1..=SNAPSHOT_VERSION).contains(&snapshot.version)
        {
            return Err(invalid("unsupported SPICE workbench snapshot"));
        }
        let saved: SavedState =
            serde_json::from_slice(&snapshot.bytes).map_err(|error| invalid(error.to_string()))?;
        let (_, analyses) = analysis_rows(&saved.deck)?;
        if saved.selected_analysis_row >= analyses.len() && !analyses.is_empty() {
            return Err(invalid("snapshot analysis selection is out of range"));
        }
        if saved.next_schematic_file_effect_id == 0
            || saved.next_schematic_file_effect_id > MAX_EFFECT_ID.saturating_add(1)
        {
            return Err(invalid(
                "snapshot schematic document effect identifier is invalid",
            ));
        }
        self.deck = saved.deck;
        self.analyses = analyses;
        self.selected_analysis_row = saved.selected_analysis_row;
        self.result_table = ResultTable::default();
        self.waveform_plot = WaveformPlot::default();
        self.result_text.clear();
        self.diagnostic_rows = diagnostic_rows(&self.deck);
        self.schematic = saved.schematic;
        self.selected_schematic_component = saved.selected_schematic_component;
        self.selected_schematic_terminal = saved.selected_schematic_terminal;
        self.selected_schematic_route_target = saved.selected_schematic_route_target;
        self.selected_schematic_analysis_card = saved.selected_schematic_analysis_card;
        self.next_schematic_file_effect_id = self
            .next_schematic_file_effect_id
            .max(saved.next_schematic_file_effect_id);
        self.pending_schematic_file_effect = None;
        self.schematic_undo.clear();
        self.schematic_redo.clear();
        if let Some(document) = &mut self.schematic {
            document.migrate_legacy_analysis_cards();
        }
        if let (Some(document), Some(reference)) =
            (&self.schematic, &self.selected_schematic_component)
        {
            let Some(component) = document
                .components
                .iter()
                .find(|component| &component.reference == reference)
            else {
                return Err(invalid("snapshot schematic selection is unknown"));
            };
            if self.selected_schematic_terminal >= component.terminals.len() {
                return Err(invalid(
                    "snapshot schematic terminal selection is out of range",
                ));
            }
        } else if self.selected_schematic_terminal != 0 {
            return Err(invalid(
                "snapshot schematic without a component has a terminal selection",
            ));
        }
        if let Some(document) = &self.schematic {
            if self.selected_schematic_analysis_card >= document.analysis_card_count() {
                return Err(invalid(
                    "snapshot schematic analysis selection is out of range",
                ));
            }
        }
        if let Some(target) = &self.selected_schematic_route_target {
            let Some(document) = &self.schematic else {
                return Err(invalid(
                    "snapshot schematic route target requires a loaded schematic",
                ));
            };
            let Some(selected) = &self.selected_schematic_component else {
                return Err(invalid(
                    "snapshot schematic route target requires a component selection",
                ));
            };
            if selected == target {
                return Err(invalid(
                    "snapshot schematic route target matches the selected component",
                ));
            }
            if !document
                .components
                .iter()
                .any(|component| component.reference == *target)
            {
                return Err(invalid("snapshot schematic route target is unknown"));
            }
        }
        self.diagnostics = "Workbench restored. Inspect or run the saved deck.".to_owned();
        self.mode = "Draft";
        Ok(self.announced(self.diagnostics.clone()))
    }

    fn complete_effect(
        &mut self,
        id: EffectId,
        result: EffectResult,
    ) -> Result<AppUpdate, EffectCompletionError<Self::Error>> {
        let pending = self
            .pending_schematic_file_effect
            .ok_or_else(|| invalid("no schematic document operation is pending"))?;
        if pending.id != id {
            return Err(invalid("schematic document effect identifier is unknown").into());
        }
        self.complete_schematic_file(pending, result)
            .map_err(Into::into)
    }
}

mosaic_app_capi::export_mosaic_app!(SpiceMosaicApp, SpiceMosaicApp::default());
mosaic_app_wasm::export_mosaic_wasm!(SpiceMosaicApp, SpiceMosaicApp::default());

#[cfg(test)]
mod tests {
    use super::*;
    use mosaic_app_runtime::{EmptyOutcome, MosaicRuntime, Platform, EFFECT_PROTOCOL_VERSION};

    fn dispatch(app: &mut SpiceMosaicApp, name: &str, payload: Value) -> AppUpdate {
        app.dispatch(Event::new(1, name, payload)).unwrap()
    }

    fn schematic_document(title: &str) -> Value {
        json!({
            "title": title,
            "components": [
                {"reference":"V1","kind":"DcVoltage","value":"5","terminals":[{"x":0,"y":20},{"x":0,"y":0}]},
                {"reference":"R1","kind":"Resistor","value":"1k","terminals":[{"x":0,"y":20},{"x":40,"y":20}]},
                {"reference":"G1","kind":"Ground","value":"","terminals":[{"x":0,"y":0}]}
            ],
            "wires": []
        })
    }

    fn protocol_two_context() -> StartContext {
        let mut context = StartContext::new("en-US", Platform::Web);
        context.protocol_version = EFFECT_PROTOCOL_VERSION;
        context
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
        assert_eq!(update.props["schematic-analysis-kind-disabled"], true);
    }

    #[test]
    fn editable_deck_inspects_runs_selects_an_analysis_and_projects_waveform_segments() {
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
        let selected = dispatch(&mut app, "onSelectAnalysis", json!({"index": 3}));
        assert_eq!(
            selected.props["selected-analysis-label"],
            ".tran (analysis 4)"
        );
        let run = dispatch(&mut app, "onRun", json!({}));
        assert_eq!(run.props["mode-label"], "Results");
        assert!(!run.props["result-columns"].as_array().unwrap().is_empty());
        assert!(!run.props["result-rows"].as_array().unwrap().is_empty());
        assert!(run.props["result-text"]
            .as_str()
            .unwrap()
            .contains("schemaVersion"));
        assert_eq!(run.props["waveform-rows"], json!(["V(out)"]));
        assert_eq!(run.props["selected-waveform-label"], "V(out)");
        assert_eq!(run.props["waveform-axis-label"], "V(out) versus Time");
        let segments = run.props["waveform-segments"].as_array().unwrap();
        assert!(!segments.is_empty());
        assert!(segments.iter().all(|segment| {
            segment.as_array().is_some_and(|values| {
                values.len() == 4
                    && values
                        .iter()
                        .all(|value| value.as_f64().is_some_and(f64::is_finite))
            })
        }));
        let waveform = dispatch(&mut app, "onSelectWaveform", json!({"index": 0}));
        assert!(waveform
            .announcements
            .iter()
            .any(|announcement| announcement.message == "Selected V(out) waveform."));
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

    #[test]
    fn schematic_host_events_select_and_sync_a_canonical_deck() {
        let mut app = SpiceMosaicApp::default();
        app.start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        let document = json!({
            "title": "Host RC",
            "components": [
                {"reference":"V1","kind":"DcVoltage","value":"5","terminals":[{"x":0,"y":20},{"x":0,"y":0}]},
                {"reference":"R1","kind":"Resistor","value":"1k","terminals":[{"x":0,"y":20},{"x":40,"y":20}]},
                {"reference":"G1","kind":"Ground","value":"","terminals":[{"x":0,"y":0}]}
            ],
            "wires": []
        });
        let loaded = dispatch(&mut app, "schematicLoad", json!({"document": document}));
        assert_eq!(loaded.props["mode-label"], "Schematic");
        assert_eq!(loaded.props["schematic-rows"], json!(["V1", "R1", "G1"]));
        let selected = dispatch(
            &mut app,
            "onSelectSchematicComponent",
            json!({"reference":"R1"}),
        );
        assert_eq!(selected.props["selected-schematic-label"], "R1");
        let synchronized = dispatch(&mut app, "onSynchronizeSchematic", json!({}));
        assert_eq!(synchronized.props["mode-label"], "Inspection");
        assert_eq!(
            synchronized.props["netlist-text"],
            "* Host RC\nR1 n1 n2 1k\nV1 n1 0 DC 5\n.op\n.end\n"
        );
    }

    #[test]
    fn schematic_document_io_exports_canonical_json_and_imports_atomically() {
        let mut app = SpiceMosaicApp::default();
        app.start(protocol_two_context()).unwrap();
        dispatch(
            &mut app,
            "schematicLoad",
            json!({"document": schematic_document("Current RC")}),
        );

        let save = dispatch(&mut app, "onSaveSchematic", json!({}));
        let effect = save.effects.first().unwrap();
        assert_eq!(effect.delivery, Delivery::Await);
        assert_eq!(effect.kind, "file.save");
        assert_eq!(effect.payload["mimeType"], "application/json");
        assert_eq!(effect.payload["extension"], ".json");
        assert_eq!(
            effect.payload["suggestedName"],
            "current-rc.spice-mosaic.json"
        );
        let exported = BASE64
            .decode(effect.payload["bytes"].as_str().unwrap())
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&exported).unwrap()["schema"],
            SCHEMATIC_FILE_SCHEMA
        );
        let saved = app
            .complete_effect(
                effect.id,
                EffectResult::Ok(json!({"name":"current-rc.spice-mosaic.json"})),
            )
            .unwrap();
        assert_eq!(saved.props["diagnostics"], "Schematic exported.");

        let open = dispatch(&mut app, "onOpenSchematic", json!({}));
        let effect = open.effects.first().unwrap();
        assert_eq!(effect.delivery, Delivery::Await);
        assert_eq!(effect.kind, "file.open");
        assert_eq!(
            effect.payload,
            json!({"mimeType":"application/json","extension":".json"})
        );
        let before = app.snapshot().unwrap();
        assert!(app
            .complete_effect(effect.id, EffectResult::Ok(json!({"bytes":"not base64"})))
            .is_err());
        assert_eq!(app.snapshot().unwrap(), before);

        let file = SchematicFile {
            schema: SCHEMATIC_FILE_SCHEMA.to_owned(),
            version: SCHEMATIC_FILE_VERSION,
            document: serde_json::from_value(schematic_document("Imported RC")).unwrap(),
        };
        let bytes = BASE64.encode(serde_json::to_vec(&file).unwrap());
        let imported = app
            .complete_effect(
                effect.id,
                EffectResult::Ok(json!({"name":"imported.json","bytes":bytes})),
            )
            .unwrap();
        assert_eq!(imported.props["schematic-title"], "Imported RC");
        assert_eq!(imported.props["mode-label"], "Schematic");

        let snapshot = app.snapshot().unwrap().unwrap();
        let cancelled = dispatch(&mut app, "onOpenSchematic", json!({}));
        let cancelled = app
            .complete_effect(
                cancelled.effects[0].id,
                EffectResult::Cancelled(EmptyOutcome {}),
            )
            .unwrap();
        assert_eq!(cancelled.props["schematic-title"], "Imported RC");
        assert_eq!(
            cancelled.props["diagnostics"],
            "Schematic import cancelled."
        );
        app.restore(snapshot).unwrap();
        let resumed = dispatch(&mut app, "onOpenSchematic", json!({}));
        assert_eq!(resumed.effects[0].id, 4);
    }

    #[test]
    fn schematic_document_io_requires_the_protocol_two_file_contract() {
        let mut app = SpiceMosaicApp::default();
        app.start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        assert_eq!(
            app.dispatch(Event::new(1, "openSchematic", json!({})))
                .unwrap_err()
                .to_string(),
            "schematic document I/O requires Mosaic protocol 2"
        );
    }

    #[test]
    fn schematic_analysis_configuration_edits_the_canonical_sweep_card_before_sync() {
        let mut app = SpiceMosaicApp::default();
        app.start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        let document = json!({
            "title": "Configurable DC sweep",
            "components": [
                {"reference":"V1","kind":"DcVoltage","value":"5","terminals":[{"x":0,"y":20},{"x":0,"y":0}]},
                {"reference":"I1","kind":"DcCurrent","value":"1m","terminals":[{"x":40,"y":20},{"x":40,"y":0}]},
                {"reference":"R1","kind":"Resistor","value":"1k","terminals":[{"x":0,"y":20},{"x":40,"y":20}]},
                {"reference":"G1","kind":"Ground","value":"","terminals":[{"x":0,"y":0}]}
            ],
            "wires": [{"start":{"x":40,"y":0},"end":{"x":0,"y":0}}]
        });
        dispatch(&mut app, "schematicLoad", json!({"document": document}));
        let selected = dispatch(
            &mut app,
            "onSelectSchematicAnalysis",
            json!({"analysis":"DC sweep"}),
        );
        assert_eq!(
            selected.props["schematic-analysis-source-options"],
            json!(["V1", "I1"])
        );
        assert_eq!(
            selected.props["schematic-analysis-parameter-one-label"],
            "Start"
        );
        dispatch(
            &mut app,
            "onSelectSchematicAnalysisSource",
            json!({"reference":"I1"}),
        );
        dispatch(
            &mut app,
            "onSchematicAnalysisParameterOneChange",
            json!({"value":"-2"}),
        );
        dispatch(
            &mut app,
            "onSchematicAnalysisParameterTwoChange",
            json!({"value":"3"}),
        );
        let configured = dispatch(
            &mut app,
            "onSchematicAnalysisParameterThreeChange",
            json!({"value":"0.5"}),
        );
        assert_eq!(
            configured.props["selected-schematic-analysis-source-label"],
            "I1"
        );
        assert_eq!(
            configured.props["schematic-analysis-parameter-three-value"],
            "0.5"
        );
        let snapshot = app.snapshot().unwrap().unwrap();
        assert_eq!(snapshot.version, SNAPSHOT_VERSION);
        let mut restored = SpiceMosaicApp::default();
        let mut context = StartContext::new("en-US", Platform::Web);
        context.restored_snapshot = Some(snapshot);
        let restored_update = restored.start(context).unwrap();
        assert_eq!(
            restored_update.props["selected-schematic-analysis-source-label"],
            "I1"
        );
        assert_eq!(
            restored_update.props["schematic-analysis-parameter-one-value"],
            "-2"
        );
        let synchronized = dispatch(&mut app, "onSynchronizeSchematic", json!({}));
        assert!(synchronized.props["netlist-text"]
            .as_str()
            .unwrap()
            .contains(".dc I1 -2 3 0.5"));
    }

    #[test]
    fn schematic_transfer_function_configuration_persists_and_syncs_a_voltage_probe() {
        let mut app = SpiceMosaicApp::default();
        app.start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        let document = json!({
            "title": "Transfer divider",
            "components": [
                {"reference":"V1","kind":"DcVoltage","value":"1","terminals":[{"x":0,"y":20},{"x":0,"y":0}]},
                {"reference":"I1","kind":"DcCurrent","value":"1m","terminals":[{"x":40,"y":20},{"x":40,"y":0}]},
                {"reference":"R1","kind":"Resistor","value":"1k","terminals":[{"x":0,"y":20},{"x":40,"y":20}]},
                {"reference":"G1","kind":"Ground","value":"","terminals":[{"x":0,"y":0}]}
            ],
            "wires": [{"start":{"x":40,"y":0},"end":{"x":0,"y":0}}],
            "net_labels": [{"point":{"x":40,"y":20},"name":"OUT"}]
        });
        dispatch(&mut app, "schematicLoad", json!({"document": document}));
        let selected = dispatch(
            &mut app,
            "onSelectSchematicAnalysis",
            json!({"analysis":"Transfer function"}),
        );
        assert_eq!(
            selected.props["schematic-analysis-source-label"],
            "Transfer-function source"
        );
        assert_eq!(
            selected.props["schematic-analysis-source-options"],
            json!(["V1", "I1"])
        );
        assert_eq!(
            selected.props["schematic-analysis-parameter-one-label"],
            "Output node"
        );
        dispatch(
            &mut app,
            "onSelectSchematicAnalysisSource",
            json!({"reference":"I1"}),
        );
        let configured = dispatch(
            &mut app,
            "onSchematicAnalysisParameterOneChange",
            json!({"value":"OUT"}),
        );
        assert_eq!(
            configured.props["selected-schematic-analysis-source-label"],
            "I1"
        );
        let snapshot = app.snapshot().unwrap().unwrap();
        let mut restored = SpiceMosaicApp::default();
        let mut context = StartContext::new("en-US", Platform::Web);
        context.restored_snapshot = Some(snapshot);
        let restored_update = restored.start(context).unwrap();
        assert_eq!(
            restored_update.props["schematic-analysis-parameter-one-value"],
            "OUT"
        );
        assert_eq!(
            restored_update.props["selected-schematic-analysis-source-label"],
            "I1"
        );
        let synchronized = dispatch(&mut app, "onSynchronizeSchematic", json!({}));
        assert!(synchronized.props["netlist-text"]
            .as_str()
            .unwrap()
            .contains(".tf V(OUT) I1"));
    }

    #[test]
    fn schematic_host_builds_and_restores_an_ordered_analysis_plan() {
        let mut app = SpiceMosaicApp::default();
        app.start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        let document = json!({
            "title": "Ordered plan",
            "components": [
                {"reference":"V1","kind":"DcVoltage","value":"5","terminals":[{"x":0,"y":20},{"x":0,"y":0}]},
                {"reference":"R1","kind":"Resistor","value":"1k","terminals":[{"x":0,"y":20},{"x":40,"y":20}]},
                {"reference":"G1","kind":"Ground","value":"","terminals":[{"x":0,"y":0}]}
            ],
            "wires": []
        });
        dispatch(&mut app, "schematicLoad", json!({"document": document}));
        let dc = dispatch(
            &mut app,
            "onAddSchematicAnalysis",
            json!({"analysis":"DC sweep"}),
        );
        assert_eq!(
            dc.props["schematic-analysis-card-rows"],
            json!([["1. Operating point"], ["2. DC sweep"]])
        );
        dispatch(
            &mut app,
            "onSchematicAnalysisParameterOneChange",
            json!({"value":"-1"}),
        );
        dispatch(
            &mut app,
            "onSchematicAnalysisParameterTwoChange",
            json!({"value":"2"}),
        );
        dispatch(
            &mut app,
            "onSchematicAnalysisParameterThreeChange",
            json!({"value":"0.5"}),
        );
        let ac = dispatch(
            &mut app,
            "onAddSchematicAnalysis",
            json!({"analysis":"AC sweep"}),
        );
        assert_eq!(ac.props["selected-schematic-analysis-label"], "AC sweep");
        dispatch(
            &mut app,
            "onSchematicAnalysisParameterOneChange",
            json!({"value":"20"}),
        );
        dispatch(
            &mut app,
            "onSchematicAnalysisParameterTwoChange",
            json!({"value":"1"}),
        );
        let configured = dispatch(
            &mut app,
            "onSchematicAnalysisParameterThreeChange",
            json!({"value":"1k"}),
        );
        assert_eq!(
            configured.props["schematic-analysis-card-rows"],
            json!([["1. Operating point"], ["2. DC sweep"], ["3. AC sweep"]])
        );
        let moved = dispatch(&mut app, "onMoveSchematicAnalysisCardEarlier", json!({}));
        assert_eq!(
            moved.props["schematic-analysis-card-rows"],
            json!([["1. Operating point"], ["2. AC sweep"], ["3. DC sweep"]])
        );
        assert_eq!(moved.props["selected-schematic-analysis-label"], "AC sweep");
        assert_eq!(moved.props["schematic-analysis-kind-disabled"], false);
        assert_eq!(
            moved.props["schematic-analysis-parameter-three-value"],
            "1k"
        );
        let converted = dispatch(
            &mut app,
            "onSelectSchematicAnalysis",
            json!({"analysis":"Transient"}),
        );
        assert_eq!(
            converted.props["selected-schematic-analysis-label"],
            "Transient"
        );
        assert_eq!(
            dispatch(&mut app, "undoSchematic", json!({})).props
                ["selected-schematic-analysis-label"],
            "AC sweep"
        );
        dispatch(&mut app, "redoSchematic", json!({}));
        dispatch(
            &mut app,
            "onSchematicAnalysisParameterOneChange",
            json!({"value":"2m"}),
        );
        dispatch(
            &mut app,
            "onSchematicAnalysisParameterTwoChange",
            json!({"value":"20m"}),
        );
        let snapshot = app.snapshot().unwrap().unwrap();
        let mut restored = SpiceMosaicApp::default();
        let mut context = StartContext::new("en-US", Platform::Web);
        context.restored_snapshot = Some(snapshot);
        let restored_update = restored.start(context).unwrap();
        assert_eq!(
            restored_update.props["selected-schematic-analysis-label"],
            "Transient"
        );
        let synchronized = dispatch(&mut app, "onSynchronizeSchematic", json!({}));
        assert!(synchronized.props["netlist-text"]
            .as_str()
            .unwrap()
            .contains(".op\n.tran 2m 20m\n.dc V1 -1 2 0.5\n.end"));
    }

    #[test]
    fn schematic_connect_rejects_dangling_and_duplicate_host_wires() {
        let mut app = SpiceMosaicApp::default();
        app.start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        let document = json!({
            "title": "Host wire validation",
            "components": [
                {"reference":"V1","kind":"DcVoltage","value":"5","terminals":[{"x":0,"y":20},{"x":0,"y":0}]},
                {"reference":"R1","kind":"Resistor","value":"1k","terminals":[{"x":0,"y":20},{"x":40,"y":20}]},
                {"reference":"G1","kind":"Ground","value":"","terminals":[{"x":0,"y":0}]}
            ],
            "wires": []
        });
        dispatch(&mut app, "schematicLoad", json!({"document": document}));

        let dangling = app
            .dispatch(Event::new(
                1,
                "schematicConnect",
                json!({"wire":{"start":{"x":0,"y":20},"end":{"x":99,"y":99}}}),
            ))
            .unwrap_err();
        assert_eq!(
            dangling.to_string(),
            "schematic wire endpoints must be component terminals"
        );

        dispatch(
            &mut app,
            "schematicConnect",
            json!({"wire":{"start":{"x":0,"y":20},"end":{"x":40,"y":20}}}),
        );
        let duplicate = app
            .dispatch(Event::new(
                1,
                "schematicConnect",
                json!({"wire":{"start":{"x":40,"y":20},"end":{"x":0,"y":20}}}),
            ))
            .unwrap_err();
        assert_eq!(duplicate.to_string(), "schematic wire is already connected");
    }

    #[test]
    fn schematic_net_label_inspector_names_terminals_and_restores_selection_history() {
        let mut app = SpiceMosaicApp::default();
        app.start(protocol_two_context()).unwrap();
        dispatch(
            &mut app,
            "schematicLoad",
            json!({"document": schematic_document("Named divider")}),
        );
        let selected = dispatch(
            &mut app,
            "onSelectSchematicComponent",
            json!({"reference":"R1"}),
        );
        assert_eq!(
            selected.props["schematic-terminal-controls"],
            json!(["Terminal 1", "Terminal 2"])
        );
        let input = dispatch(
            &mut app,
            "onSchematicNetLabelChange",
            json!({"value":"INPUT"}),
        );
        assert_eq!(input.props["schematic-net-label"], "INPUT");
        let selected_terminal = dispatch(&mut app, "onSelectSchematicTerminal", json!({"index":1}));
        assert_eq!(
            selected_terminal.props["selected-schematic-terminal-label"],
            "Terminal 2"
        );
        let output = dispatch(
            &mut app,
            "onSchematicNetLabelChange",
            json!({"value":"OUTPUT"}),
        );
        assert_eq!(output.props["schematic-net-label"], "OUTPUT");
        let synchronized = dispatch(&mut app, "onSynchronizeSchematic", json!({}));
        assert!(synchronized.props["netlist-text"]
            .as_str()
            .unwrap()
            .contains("R1 INPUT OUTPUT 1k\nV1 INPUT 0 DC 5"));

        dispatch(&mut app, "onUndoSchematic", json!({}));
        assert_eq!(app.update().props["schematic-net-label"], "");
        assert_eq!(
            app.update().props["selected-schematic-terminal-label"],
            "Terminal 2"
        );
        dispatch(&mut app, "onRedoSchematic", json!({}));
        let snapshot = app.snapshot().unwrap().unwrap();
        let mut restored = SpiceMosaicApp::default();
        let mut context = StartContext::new("en-US", Platform::Web);
        context.restored_snapshot = Some(snapshot);
        let restored_update = restored.start(context).unwrap();
        assert_eq!(restored_update.props["schematic-net-label"], "OUTPUT");
        assert_eq!(
            restored_update.props["selected-schematic-terminal-label"],
            "Terminal 2"
        );

        let conflict = app
            .dispatch(Event::new(
                1,
                "schematicConnect",
                json!({"wire":{"start":{"x":0,"y":20},"end":{"x":40,"y":20}}}),
            ))
            .unwrap_err();
        assert_eq!(conflict.to_string(), "schematic net has conflicting labels");
    }

    #[test]
    fn schematic_net_label_inspector_links_disconnected_terminals_by_name() {
        let mut app = SpiceMosaicApp::default();
        app.start(protocol_two_context()).unwrap();
        dispatch(
            &mut app,
            "schematicLoad",
            json!({
                "document": {
                    "title": "Remote label link",
                    "components": [
                        {"reference":"V1","kind":"DcVoltage","value":"5","terminals":[{"x":0,"y":20},{"x":0,"y":0}]},
                        {"reference":"R1","kind":"Resistor","value":"1k","terminals":[{"x":40,"y":20},{"x":40,"y":0}]},
                        {"reference":"G1","kind":"Ground","value":"","terminals":[{"x":0,"y":0}]}
                    ],
                    "wires": []
                }
            }),
        );
        dispatch(
            &mut app,
            "onSelectSchematicComponent",
            json!({"reference":"V1"}),
        );
        dispatch(
            &mut app,
            "onSchematicNetLabelChange",
            json!({"value":"SENSE"}),
        );
        dispatch(
            &mut app,
            "onSelectSchematicComponent",
            json!({"reference":"R1"}),
        );
        let linked = dispatch(
            &mut app,
            "onSchematicNetLabelChange",
            json!({"value":"SENSE"}),
        );
        assert_eq!(linked.props["schematic-net-label"], "SENSE");

        let synchronized = dispatch(&mut app, "onSynchronizeSchematic", json!({}));
        let deck = synchronized.props["netlist-text"].as_str().unwrap();
        assert!(deck.contains("R1 SENSE n1 1k\nV1 SENSE 0 DC 5"));

        dispatch(&mut app, "onUndoSchematic", json!({}));
        assert_eq!(app.update().props["schematic-net-label"], "");
        dispatch(&mut app, "onRedoSchematic", json!({}));
        assert_eq!(app.update().props["schematic-net-label"], "SENSE");
    }

    #[test]
    fn schematic_value_inspector_updates_the_canonical_component_before_sync() {
        let mut app = SpiceMosaicApp::default();
        app.start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        let document = json!({
            "title": "Editable host RC",
            "components": [
                {"reference":"V1","kind":"DcVoltage","value":"5","terminals":[{"x":0,"y":20},{"x":0,"y":0}]},
                {"reference":"R1","kind":"Resistor","value":"1k","terminals":[{"x":0,"y":20},{"x":40,"y":20}]},
                {"reference":"G1","kind":"Ground","value":"","terminals":[{"x":0,"y":0}]}
            ],
            "wires": []
        });
        dispatch(&mut app, "schematicLoad", json!({"document": document}));
        dispatch(
            &mut app,
            "onSelectSchematicComponent",
            json!({"reference":"R1"}),
        );
        let edited = dispatch(&mut app, "onSchematicValueChange", json!({"value":"2k"}));
        assert_eq!(edited.props["selected-schematic-kind-label"], "Resistor");
        assert_eq!(edited.props["schematic-value"], "2k");
        assert_eq!(edited.props["schematic-value-disabled"], false);
        let synchronized = dispatch(&mut app, "onSynchronizeSchematic", json!({}));
        assert!(synchronized.props["netlist-text"]
            .as_str()
            .unwrap()
            .contains("R1 n1 n2 2k"));

        dispatch(
            &mut app,
            "onSelectSchematicComponent",
            json!({"reference":"G1"}),
        );
        let ground = app
            .dispatch(Event::new(1, "schematicValueChange", json!({"value":"0"})))
            .unwrap_err();
        assert_eq!(
            ground.to_string(),
            "G1 ground symbol does not accept a SPICE value"
        );
    }

    #[test]
    fn schematic_edit_events_remove_selected_components_and_source_ordered_wires() {
        let mut app = SpiceMosaicApp::default();
        app.start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        let document = json!({
            "title": "Editable wiring",
            "components": [
                {"reference":"V1","kind":"DcVoltage","value":"5","terminals":[{"x":0,"y":20},{"x":0,"y":0}]},
                {"reference":"R1","kind":"Resistor","value":"1k","terminals":[{"x":0,"y":20},{"x":40,"y":20}]},
                {"reference":"G1","kind":"Ground","value":"","terminals":[{"x":0,"y":0}]}
            ],
            "wires": [
                {"start":{"x":0,"y":20},"end":{"x":40,"y":20}},
                {"start":{"x":40,"y":20},"end":{"x":0,"y":0}}
            ]
        });
        dispatch(&mut app, "schematicLoad", json!({"document": document}));
        let removed_wire = dispatch(&mut app, "onRemoveSchematicWire", json!({"index": 0}));
        assert_eq!(
            removed_wire.props["schematic-wire-rows"],
            json!(["Remove wire 1"])
        );
        dispatch(
            &mut app,
            "onSelectSchematicComponent",
            json!({"reference":"R1"}),
        );
        let removed_component = dispatch(&mut app, "onRemoveSchematicComponent", json!({}));
        assert_eq!(
            removed_component.props["schematic-rows"],
            json!(["V1", "G1"])
        );
        assert_eq!(removed_component.props["schematic-wire-rows"], json!([]));
        assert_eq!(
            removed_component.props["selected-schematic-label"],
            "No component selected"
        );
        assert_eq!(
            removed_component.props["remove-schematic-component-disabled"],
            true
        );
        let missing_selection = app
            .dispatch(Event::new(1, "removeSchematicComponent", json!({})))
            .unwrap_err();
        assert_eq!(
            missing_selection.to_string(),
            "removeSchematicComponent requires a selected component"
        );
    }

    #[test]
    fn schematic_metadata_events_rename_titles_and_preserve_dc_source_bindings() {
        let mut app = SpiceMosaicApp::default();
        app.start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        let document = json!({
            "title": "Initial title",
            "components": [
                {"reference":"V1","kind":"DcVoltage","value":"5","terminals":[{"x":0,"y":20},{"x":0,"y":0}]},
                {"reference":"R1","kind":"Resistor","value":"1k","terminals":[{"x":0,"y":20},{"x":40,"y":20}]},
                {"reference":"G1","kind":"Ground","value":"","terminals":[{"x":0,"y":0}]}
            ],
            "wires": [{"start":{"x":40,"y":20},"end":{"x":0,"y":0}}]
        });
        dispatch(&mut app, "schematicLoad", json!({"document": document}));
        let titled = dispatch(
            &mut app,
            "onSchematicTitleChange",
            json!({"value":"Renamed divider"}),
        );
        assert_eq!(titled.props["schematic-title"], "Renamed divider");
        dispatch(
            &mut app,
            "onSelectSchematicAnalysis",
            json!({"analysis":"DC sweep"}),
        );
        dispatch(
            &mut app,
            "onSelectSchematicComponent",
            json!({"reference":"V1"}),
        );
        let renamed = dispatch(
            &mut app,
            "onSchematicReferenceChange",
            json!({"value":"VDD"}),
        );
        assert_eq!(renamed.props["schematic-reference"], "VDD");
        assert_eq!(
            renamed.props["selected-schematic-analysis-source-label"],
            "VDD"
        );
        let synchronized = dispatch(&mut app, "onSynchronizeSchematic", json!({}));
        assert!(synchronized.props["netlist-text"]
            .as_str()
            .unwrap()
            .contains(".dc VDD 0 5 1"));
    }

    #[test]
    fn schematic_edit_history_restores_document_state_and_stays_out_of_snapshots() {
        let mut app = SpiceMosaicApp::default();
        app.start(protocol_two_context()).unwrap();
        let document = schematic_document("History RC");
        dispatch(&mut app, "schematicLoad", json!({"document": document}));
        dispatch(
            &mut app,
            "selectSchematicComponent",
            json!({"reference":"R1"}),
        );

        let edited = dispatch(&mut app, "schematicValueChange", json!({"value":"2k"}));
        assert_eq!(edited.props["schematic-value"], "2k");
        assert_eq!(edited.props["undo-schematic-disabled"], false);
        assert_eq!(edited.props["redo-schematic-disabled"], true);

        let undone = dispatch(&mut app, "undoSchematic", json!({}));
        assert_eq!(undone.props["schematic-value"], "1k");
        assert_eq!(undone.props["selected-schematic-label"], "R1");
        assert_eq!(undone.props["redo-schematic-disabled"], false);

        let redone = dispatch(&mut app, "redoSchematic", json!({}));
        assert_eq!(redone.props["schematic-value"], "2k");
        assert_eq!(redone.props["selected-schematic-label"], "R1");

        dispatch(&mut app, "undoSchematic", json!({}));
        let renamed = dispatch(
            &mut app,
            "schematicReferenceChange",
            json!({"value":"RLOAD"}),
        );
        assert_eq!(renamed.props["schematic-reference"], "RLOAD");
        assert_eq!(renamed.props["redo-schematic-disabled"], true);

        let removed = dispatch(&mut app, "removeSchematicComponent", json!({}));
        assert_eq!(removed.props["schematic-rows"], json!(["V1", "G1"]));
        let restored = dispatch(&mut app, "undoSchematic", json!({}));
        assert_eq!(
            restored.props["schematic-rows"],
            json!(["V1", "RLOAD", "G1"])
        );
        assert_eq!(restored.props["selected-schematic-label"], "RLOAD");

        let snapshot = app.snapshot().unwrap().unwrap();
        let mut restored_app = SpiceMosaicApp::default();
        let restored_update = restored_app.restore(snapshot).unwrap();
        assert_eq!(restored_update.props["undo-schematic-disabled"], true);
        assert_eq!(restored_update.props["redo-schematic-disabled"], true);
    }

    #[test]
    fn schematic_edit_history_replays_incomplete_palette_drafts() {
        let mut app = SpiceMosaicApp::default();
        app.start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        let placed = dispatch(
            &mut app,
            "placeSchematicComponent",
            json!({"kind":"Resistor"}),
        );
        assert_eq!(placed.props["schematic-rows"], json!(["R1"]));

        let undone = dispatch(&mut app, "undoSchematic", json!({}));
        assert_eq!(undone.props["schematic-rows"], json!([]));
        let redone = dispatch(&mut app, "redoSchematic", json!({}));
        assert_eq!(redone.props["schematic-rows"], json!(["R1"]));
        assert_eq!(redone.props["selected-schematic-label"], "R1");
    }

    #[test]
    fn palette_gestures_place_components_and_project_orthogonal_routes() {
        let mut app = SpiceMosaicApp::default();
        app.start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        let resistor = dispatch(
            &mut app,
            "onPlaceSchematicComponent",
            json!({"kind": "Resistor"}),
        );
        assert_eq!(resistor.props["schematic-rows"], json!(["R1"]));
        assert_eq!(resistor.props["selected-schematic-label"], "R1");
        let capacitor = dispatch(
            &mut app,
            "onPlaceSchematicComponent",
            json!({"kind": "Capacitor"}),
        );
        assert_eq!(capacitor.props["schematic-rows"], json!(["R1", "C1"]));
        let inductor = dispatch(
            &mut app,
            "onPlaceSchematicComponent",
            json!({"kind": "Inductor"}),
        );
        assert_eq!(inductor.props["schematic-rows"], json!(["R1", "C1", "L1"]));
        let analysis = dispatch(
            &mut app,
            "onSelectSchematicAnalysis",
            json!({"analysis": "AC sweep"}),
        );
        assert_eq!(
            analysis.props["selected-schematic-analysis-label"],
            "AC sweep"
        );
        dispatch(
            &mut app,
            "onSelectSchematicComponent",
            json!({"reference": "R1"}),
        );
        let target = dispatch(
            &mut app,
            "onSelectSchematicRouteTarget",
            json!({"reference": "C1"}),
        );
        assert_eq!(target.props["selected-schematic-route-target-label"], "C1");
        let snapshot = app.snapshot().unwrap().unwrap();
        let mut restored = SpiceMosaicApp::default();
        let restored_target = restored.restore(snapshot).unwrap();
        assert_eq!(
            restored_target.props["selected-schematic-route-target-label"],
            "C1"
        );
        let routed = dispatch(&mut app, "onRouteToSchematicTerminal", json!({"index": 0}));
        assert_eq!(
            routed.props["diagnostics"],
            "Routed R1 Terminal 1 to C1 Terminal 1 on the schematic grid."
        );
        assert_eq!(
            routed.props["schematic-wire-segments"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let undone = dispatch(&mut app, "onUndoSchematic", json!({}));
        assert_eq!(undone.props["selected-schematic-route-target-label"], "C1");
        assert!(undone.props["schematic-wire-segments"]
            .as_array()
            .unwrap()
            .is_empty());
        let redone = dispatch(&mut app, "onRedoSchematic", json!({}));
        assert_eq!(
            redone.props["schematic-wire-segments"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            routed.props["schematic-terminal-points"]
                .as_array()
                .unwrap()
                .len(),
            6
        );
        assert_eq!(
            routed.props["schematic-grid-lines"]
                .as_array()
                .unwrap()
                .len(),
            18
        );
    }
}
