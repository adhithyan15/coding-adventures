#![recursion_limit = "256"]

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

mod schematic;

pub use schematic::{
    SchematicAnalysis, SchematicAnalysisSettings, SchematicComponent, SchematicComponentKind,
    SchematicDocument, SchematicError, SchematicPoint, SchematicWire,
};

const SNAPSHOT_SCHEMA: &str = "spice-mosaic-app/state";
const SNAPSHOT_VERSION: u32 = 3;
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
        let schematic_value_disabled = selected_schematic_component
            .is_none_or(|component| component.kind == SchematicComponentKind::Ground);
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
                let labels = document.analysis_parameter_labels().map(str::to_owned);
                let values = document.analysis_parameter_values().map(str::to_owned);
                let source_options = if document.analysis == SchematicAnalysis::DcSweep {
                    document
                        .dc_sweep_source_references()
                        .into_iter()
                        .map(str::to_owned)
                        .collect()
                } else {
                    Vec::new()
                };
                let selected_source = if document.analysis == SchematicAnalysis::DcSweep
                    && source_options
                        .iter()
                        .any(|source| source == &document.analysis_settings.dc_source)
                {
                    document.analysis_settings.dc_source.clone()
                } else if document.analysis == SchematicAnalysis::DcSweep {
                    "Select a DC source".to_owned()
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
            "schematic-title": self.schematic.as_ref().map(|document| document.title.as_str()).unwrap_or("No schematic loaded"),
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
            "schematic-analysis-label": "Schematic analysis",
            "schematic-analysis-controls": [
                SchematicAnalysis::OperatingPoint.palette_label(),
                SchematicAnalysis::DcSweep.palette_label(),
                SchematicAnalysis::AcSweep.palette_label(),
                SchematicAnalysis::Transient.palette_label(),
            ],
            "selected-schematic-analysis-label": self.schematic.as_ref().map(|document| document.analysis.palette_label()).unwrap_or(SchematicAnalysis::default().palette_label()),
            "schematic-analysis-configuration-label": "Analysis configuration",
            "schematic-analysis-source-label": "DC sweep source",
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
            "selected-schematic-label": self.selected_schematic_component.as_deref().unwrap_or("No component selected"),
            "schematic-properties-label": "Component properties",
            "selected-schematic-kind-label": selected_schematic_component.map(|component| component.kind.palette_label()).unwrap_or("Select a component"),
            "schematic-value-label": "SPICE value",
            "schematic-value": selected_schematic_component.map(|component| component.value.as_str()).unwrap_or(""),
            "schematic-value-placeholder": "Select a non-ground component",
            "schematic-value-disabled": schematic_value_disabled,
            "route-schematic-label": "Route selected component to",
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
        document: SchematicDocument,
    ) -> Result<AppUpdate, SpiceMosaicError> {
        document
            .validate()
            .map_err(|error| invalid(error.to_string()))?;
        self.selected_schematic_component = None;
        self.schematic = Some(document);
        self.diagnostics = "Schematic loaded. Sync its canonical netlist when ready.".to_owned();
        self.mode = "Schematic";
        Ok(self.announced(self.diagnostics.clone()))
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
            "placeSchematicComponent" => {
                let kind = event.payload["kind"]
                    .as_str()
                    .ok_or_else(|| invalid("placeSchematicComponent requires a palette kind"))?;
                let kind = SchematicComponentKind::from_palette_label(kind)
                    .map_err(|error| invalid(error.to_string()))?;
                let document = self.schematic.get_or_insert_with(|| SchematicDocument {
                    title: "Untitled schematic".to_owned(),
                    components: Vec::new(),
                    wires: Vec::new(),
                    analysis: SchematicAnalysis::default(),
                    analysis_settings: SchematicAnalysisSettings::default(),
                });
                let reference = document
                    .place_palette_component(kind)
                    .map_err(|error| invalid(error.to_string()))?;
                self.selected_schematic_component = Some(reference.clone());
                self.diagnostics = format!(
                    "Placed {reference} on the grid. Select a component and route it to a target."
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
                let document = self.schematic.get_or_insert_with(|| SchematicDocument {
                    title: "Untitled schematic".to_owned(),
                    components: Vec::new(),
                    wires: Vec::new(),
                    analysis: SchematicAnalysis::default(),
                    analysis_settings: SchematicAnalysisSettings::default(),
                });
                document.analysis = analysis;
                self.diagnostics = format!(
                    "Selected {} for the canonical schematic deck.",
                    analysis.palette_label()
                );
                self.mode = "Schematic";
                Ok(self.announced(self.diagnostics.clone()))
            }
            "selectSchematicAnalysisSource" => {
                let reference = event.payload["reference"].as_str().ok_or_else(|| {
                    invalid("selectSchematicAnalysisSource requires a source reference")
                })?;
                let document = self.schematic.as_mut().ok_or_else(|| {
                    invalid("selectSchematicAnalysisSource requires a loaded schematic")
                })?;
                document
                    .set_dc_sweep_source(reference)
                    .map_err(|error| invalid(error.to_string()))?;
                self.diagnostics = format!(
                    "Selected {reference} as the canonical DC sweep source. Sync the netlist when ready."
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
                let document = self.schematic.as_mut().ok_or_else(|| {
                    invalid("schematic analysis parameter change requires a loaded schematic")
                })?;
                document
                    .set_analysis_parameter(index, value)
                    .map_err(|error| invalid(error.to_string()))?;
                self.diagnostics =
                    "Updated the canonical analysis card. Sync the netlist when ready.".to_owned();
                Ok(self.announced(self.diagnostics.clone()))
            }
            "schematicPlace" => {
                let component: SchematicComponent =
                    serde_json::from_value(event.payload["component"].clone()).map_err(
                        |error| invalid(format!("schematicPlace requires a component: {error}")),
                    )?;
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
                self.diagnostics =
                    "Component placed. Connect endpoints, then sync the netlist.".to_owned();
                Ok(self.announced(self.diagnostics.clone()))
            }
            "schematicConnect" => {
                let wire: SchematicWire = serde_json::from_value(event.payload["wire"].clone())
                    .map_err(|error| {
                        invalid(format!("schematicConnect requires a wire: {error}"))
                    })?;
                let document = self
                    .schematic
                    .as_mut()
                    .ok_or_else(|| invalid("schematicConnect requires a loaded schematic"))?;
                document
                    .connect_wire(wire)
                    .map_err(|error| invalid(error.to_string()))?;
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
                Ok(self.announced(format!("Selected {reference}.")))
            }
            "schematicValueChange" => {
                let value = event.payload["value"]
                    .as_str()
                    .ok_or_else(|| invalid("schematicValueChange requires text value"))?;
                let reference = self
                    .selected_schematic_component
                    .clone()
                    .ok_or_else(|| invalid("schematicValueChange requires a selected component"))?;
                let document = self
                    .schematic
                    .as_mut()
                    .ok_or_else(|| invalid("schematicValueChange requires a loaded schematic"))?;
                document
                    .set_component_value(&reference, value)
                    .map_err(|error| invalid(error.to_string()))?;
                self.diagnostics =
                    format!("Updated {reference} value. Sync the netlist when ready.");
                Ok(self.announced(self.diagnostics.clone()))
            }
            "routeToSchematicComponent" => {
                let target = event.payload["reference"]
                    .as_str()
                    .ok_or_else(|| invalid("routeToSchematicComponent requires reference"))?;
                let selected = self.selected_schematic_component.clone().ok_or_else(|| {
                    invalid("routeToSchematicComponent requires a selected component")
                })?;
                let document = self.schematic.as_mut().ok_or_else(|| {
                    invalid("routeToSchematicComponent requires a loaded schematic")
                })?;
                document
                    .route_components(&selected, target)
                    .map_err(|error| invalid(error.to_string()))?;
                self.diagnostics = format!("Routed {selected} to {target} on the schematic grid.");
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
        self.deck = saved.deck;
        self.analyses = analyses;
        self.selected_analysis_row = saved.selected_analysis_row;
        self.result_table = ResultTable::default();
        self.waveform_plot = WaveformPlot::default();
        self.result_text.clear();
        self.diagnostic_rows = diagnostic_rows(&self.deck);
        self.schematic = saved.schematic;
        self.selected_schematic_component = saved.selected_schematic_component;
        if let (Some(document), Some(reference)) =
            (&self.schematic, &self.selected_schematic_component)
        {
            if !document
                .components
                .iter()
                .any(|component| &component.reference == reference)
            {
                return Err(invalid("snapshot schematic selection is unknown"));
            }
        }
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
        assert_eq!(snapshot.version, 3);
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
        let routed = dispatch(
            &mut app,
            "onRouteToSchematicComponent",
            json!({"reference": "C1"}),
        );
        assert_eq!(
            routed.props["diagnostics"],
            "Routed R1 to C1 on the schematic grid."
        );
        assert_eq!(
            routed.props["schematic-wire-segments"]
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
