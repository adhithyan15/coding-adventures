//! Portable VisiCalc presentation state over the existing spreadsheet engine.
//!
//! Hosts render props and translate input into semantic events. Calculations,
//! raw-cell coercion and workbook serialization stay in `spreadsheet-core`.

use mosaic_app_runtime::{
    Announcement, AppUpdate, ColorScheme, Delivery, Effect, EffectCompletionError, EffectId,
    EffectResult, Event, MosaicApp, Politeness, Snapshot, StartContext, EFFECT_PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use spreadsheet_core::{column_index_to_letters, CellAddress, SheetId, Workbook};
use std::{collections::BTreeMap, error::Error, fmt};

const ROWS: u32 = 100;
const COLS: u32 = 26;
const SNAPSHOT_SCHEMA: &str = "visicalc-mosaic-app/state";
const SNAPSHOT_VERSION: u32 = 1;
const MAX_FILE_BYTES: usize = 16 * 1024 * 1024;
const SEED: &str = include_str!("../../../../programs/mosaic/visicalc/fixtures/budget-v1.json");

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Cursor {
    row: u32,
    col: u32,
    offset: u32,
    size: u32,
    dark: bool,
    text_scale: f32,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            row: 0,
            col: 0,
            offset: 0,
            size: 30,
            dark: false,
            text_scale: 1.0,
        }
    }
}

impl Cursor {
    fn reveal(&mut self) {
        if self.row < self.offset {
            self.offset = self.row;
        } else if self.row >= self.offset + self.size {
            self.offset = self.row + 1 - self.size;
        }
        self.offset = self.offset.min(ROWS - self.size);
    }

    fn valid(&self) -> bool {
        self.row < ROWS
            && self.col < COLS
            && (1..=ROWS).contains(&self.size)
            && self.offset <= ROWS - self.size
            && self.text_scale.is_finite()
            && self.text_scale > 0.0
    }
}

#[derive(Debug)]
struct Edit {
    row: u32,
    col: u32,
    content: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedState {
    workbook: String,
    cursor: Cursor,
}

/// Standard-ABI application. Uncommitted edits are deliberately not persisted.
pub struct VisiCalcMosaicApp {
    workbook: Workbook,
    cursor: Cursor,
    edit: Option<Edit>,
    protocol_version: u32,
    next_effect_id: EffectId,
    pending_file: Option<(EffectId, bool)>, // true for Save, false for Open
    file_status: String,
}

impl Default for VisiCalcMosaicApp {
    fn default() -> Self {
        let mut workbook = Workbook::new();
        let sheet = workbook.add_sheet("Budget");
        let seed: BTreeMap<String, String> = serde_json::from_str(SEED).expect("checked-in seed");
        for (address, source) in seed {
            workbook.set_raw(
                sheet,
                CellAddress::parse(&address).expect("seed address"),
                &source,
            );
        }
        Self {
            workbook,
            cursor: Cursor::default(),
            edit: None,
            protocol_version: 1,
            next_effect_id: 0,
            pending_file: None,
            file_status: "Not saved yet".into(),
        }
    }
}

/// Invalid host input. Errors leave workbook and presentation state unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisiCalcError(pub String);

impl fmt::Display for VisiCalcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl Error for VisiCalcError {}

fn invalid(message: impl Into<String>) -> VisiCalcError {
    VisiCalcError(message.into())
}

fn index(event: &Event, field: &str, bound: u32) -> Result<u32, VisiCalcError> {
    event
        .payload
        .get(field)
        .and_then(Value::as_u64)
        .filter(|value| *value < u64::from(bound))
        .map(|value| value as u32)
        .ok_or_else(|| invalid(format!("{} requires {field} in 0..{bound}", event.name)))
}

fn name(name: &str) -> String {
    if let Some(rest) = name.strip_prefix("on") {
        let mut chars = rest.chars();
        if let Some(first) = chars.next().filter(char::is_ascii_uppercase) {
            return first.to_ascii_lowercase().to_string() + chars.as_str();
        }
    }
    name.to_owned()
}

impl VisiCalcMosaicApp {
    fn source(&self, row: u32, col: u32) -> String {
        self.workbook
            .cell_source_text(SheetId(0), CellAddress::new(row + 1, col + 1))
    }

    fn describe_cell(&self, row: u32, col: u32) -> String {
        let address = CellAddress::new(row + 1, col + 1).to_a1();
        let window = self
            .workbook
            .get_display_window(SheetId(0), row + 1, col + 1, row + 1, col + 1)
            .expect("valid selected cell");
        let value = window
            .cells
            .first()
            .map(String::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or("blank");
        let source = self.source(row, col);
        if source.starts_with('=') {
            format!("{address}, {value}, formula {source}")
        } else {
            format!("{address}, {value}")
        }
    }

    fn update(&self) -> AppUpdate {
        let cursor = &self.cursor;
        // Cursor validation bounds the allocation to at most 100 * 26 cells.
        let window = self
            .workbook
            .get_display_window(
                SheetId(0),
                cursor.offset + 1,
                1,
                cursor.offset + cursor.size,
                COLS,
            )
            .expect("valid bounded cursor and sheet");
        let rows: Vec<Vec<String>> = window
            .cells
            .chunks(COLS as usize)
            .map(<[_]>::to_vec)
            .collect();
        let address = CellAddress::new(cursor.row + 1, cursor.col + 1).to_a1();
        let formula = self
            .edit
            .as_ref()
            .map(|edit| edit.content.clone())
            .unwrap_or_else(|| self.source(cursor.row, cursor.col));
        let (edit_row, edit_col, edit_content) = self
            .edit
            .as_ref()
            .map(|edit| {
                (
                    i64::from(edit.row),
                    i64::from(edit.col),
                    edit.content.as_str(),
                )
            })
            .unwrap_or((-1, -1, ""));
        AppUpdate::new(json!({
            "workbook-empty": !self.workbook.has_cell_content(),
            "cell-address": address, "formula": formula, "read-only": self.pending_file.is_some(),
            "file-status": self.file_status,
            "selection-summary": self.describe_cell(cursor.row, cursor.col),
            "selected-row": cursor.row, "selected-col": cursor.col,
            "grid-selected-row": i64::from(cursor.row) - i64::from(cursor.offset),
            "grid-edit-row": if edit_row < 0 { -1 } else { edit_row - i64::from(cursor.offset) },
            "viewport-offset": cursor.offset, "viewport-size": cursor.size,
            "viewport-rows": rows, "total-rows": ROWS, "total-cols": COLS,
            "row-headers": (cursor.offset + 1..=cursor.offset + cursor.size).map(|row| row.to_string()).collect::<Vec<_>>(),
            "column-headers": (1..=COLS).map(column_index_to_letters).collect::<Vec<_>>(),
            "column-widths": vec![80; COLS as usize],
            "edit-row": edit_row, "edit-col": edit_col, "edit-content": edit_content,
            "editing": self.edit.is_some(), "dark-theme": cursor.dark,
            "text-scale": cursor.text_scale,
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

    fn file_message(&mut self, message: impl Into<String>) -> AppUpdate {
        self.file_status = message.into();
        self.announced(&self.file_status)
    }

    fn request_file(&mut self, saving: bool) -> Result<AppUpdate, VisiCalcError> {
        if self.protocol_version != EFFECT_PROTOCOL_VERSION {
            return Ok(self.file_message("Open and Save are unavailable in this host."));
        }
        if saving && self.edit.is_some() {
            return Ok(self.file_message("Press Enter to apply this edit before saving."));
        }
        let mut payload = json!({"mimeType": "application/json", "extension": ".visicalc"});
        if saving {
            // Capture the settled checkpoint before registering Await. The host
            // never needs to call standalone snapshot while a write is pending.
            let bytes = serde_json::to_vec(&self.snapshot()?.expect("VisiCalc snapshot"))
                .map_err(|error| invalid(error.to_string()))?;
            if bytes.len() > MAX_FILE_BYTES {
                return Ok(self.file_message("This workbook exceeds the 16 MiB file limit."));
            }
            payload["bytes"] =
                coding_adventures_base64::encode(&bytes, &coding_adventures_base64::STANDARD)
                    .into();
            payload["suggestedName"] = "Workbook.visicalc".into();
        }
        let id = self
            .next_effect_id
            .checked_add(1)
            .filter(|id| *id <= 9_007_199_254_740_991)
            .ok_or_else(|| invalid("file operation identifiers exhausted"))?;
        self.next_effect_id = id;
        self.pending_file = Some((id, saving));
        let mut update = self.file_message(if saving {
            "Choose where to save your workbook…"
        } else {
            "Choose a VisiCalc workbook…"
        });
        update.effects.push(Effect {
            id,
            delivery: Delivery::Await,
            kind: if saving { "file.save" } else { "file.open" }.into(),
            payload,
        });
        Ok(update)
    }

    fn open_file(&mut self, payload: &Value) -> Result<(), VisiCalcError> {
        let encoded = payload
            .get("bytes")
            .and_then(Value::as_str)
            .filter(|bytes| bytes.len() <= MAX_FILE_BYTES.div_ceil(3) * 4)
            .ok_or_else(|| invalid("missing or oversized file bytes"))?;
        let bytes = coding_adventures_base64::decode(encoded, &coding_adventures_base64::STANDARD)
            .map_err(|error| invalid(error.to_string()))?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err(invalid("file too large"));
        }
        let snapshot: Snapshot =
            serde_json::from_slice(&bytes).map_err(|error| invalid(error.to_string()))?;
        // restore validates a temporary workbook/cursor before replacing either.
        self.restore(snapshot)?;
        Ok(())
    }
}

impl MosaicApp for VisiCalcMosaicApp {
    type Error = VisiCalcError;

    fn start(&mut self, context: StartContext) -> Result<AppUpdate, Self::Error> {
        // Restore first: an invalid snapshot must not even change appearance.
        if let Some(snapshot) = context.restored_snapshot {
            self.restore(snapshot)?;
            self.protocol_version = context.protocol_version;
            return Ok(self.update());
        }
        if !context.text_scale.is_finite() || context.text_scale <= 0.0 {
            return Err(invalid("invalid text scale"));
        }
        self.cursor.dark = context.color_scheme == ColorScheme::Dark;
        self.cursor.text_scale = context.text_scale;
        self.protocol_version = context.protocol_version;
        Ok(self.update())
    }

    fn dispatch(&mut self, event: Event) -> Result<AppUpdate, Self::Error> {
        if !event.payload.is_object() {
            return Err(invalid("event payload must be an object"));
        }
        if self.pending_file.is_some()
            && !matches!(
                name(&event.name).as_str(),
                "scroll" | "viewportShift" | "viewportRows" | "resizeViewport"
            )
        {
            return Ok(self.announced("Finish the current file operation first."));
        }
        match name(&event.name).as_str() {
            "openWorkbook" => self.request_file(false),
            "saveWorkbook" => self.request_file(true),
            "navigate" | "gridNavigate" | "editStart" => {
                let row = if name(&event.name) == "gridNavigate" {
                    index(&event, "row", self.cursor.size)? + self.cursor.offset
                } else {
                    index(&event, "row", ROWS)?
                };
                let col = index(&event, "col", COLS)?;
                // Validate every field before changing any state.
                self.cursor.row = row;
                self.cursor.col = col;
                self.cursor.reveal();
                self.edit = if name(&event.name) == "editStart" {
                    Some(Edit {
                        row,
                        col,
                        content: self.source(row, col),
                    })
                } else {
                    None
                };
                let description = self.describe_cell(row, col);
                Ok(self.announced(if self.edit.is_some() {
                    format!("Editing {description}")
                } else {
                    description
                }))
            }
            "formulaChange" => {
                let value = event
                    .payload
                    .get("value")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("formulaChange requires text value"))?;
                let edit = self.edit.get_or_insert_with(|| Edit {
                    row: self.cursor.row,
                    col: self.cursor.col,
                    content: String::new(),
                });
                edit.content = value.to_owned();
                Ok(self.update())
            }
            "commit" | "editCommit" => {
                if let Some(edit) = self.edit.take() {
                    self.file_status = "Changes not saved".into();
                    self.workbook.set_raw(
                        SheetId(0),
                        CellAddress::new(edit.row + 1, edit.col + 1),
                        &edit.content,
                    );
                    if name(&event.name) == "editCommit" {
                        self.cursor.row = (edit.row + 1).min(ROWS - 1);
                        self.cursor.col = edit.col;
                        self.cursor.reveal();
                    }
                    let updated = self.describe_cell(edit.row, edit.col);
                    let message = if (self.cursor.row, self.cursor.col) != (edit.row, edit.col) {
                        format!(
                            "Updated {updated}. Selected {}",
                            self.describe_cell(self.cursor.row, self.cursor.col)
                        )
                    } else {
                        format!("Updated {updated}")
                    };
                    Ok(self.announced(message))
                } else {
                    Ok(self.update())
                }
            }
            "cancel" | "editCancel" => {
                if self.edit.take().is_some() {
                    Ok(self.announced(format!(
                        "Edit cancelled. {}",
                        self.describe_cell(self.cursor.row, self.cursor.col)
                    )))
                } else {
                    Ok(self.update())
                }
            }
            "scroll" => {
                let offset = index(&event, "offset", ROWS - self.cursor.size + 1)?;
                self.cursor.offset = offset;
                Ok(self.update())
            }
            "viewportShift" => {
                let rows = event
                    .payload
                    .get("rows")
                    .and_then(Value::as_i64)
                    .filter(|rows| *rows != 0)
                    .ok_or_else(|| invalid("viewport shift must be a nonzero integer"))?;
                self.cursor.offset = i64::from(self.cursor.offset)
                    .saturating_add(rows)
                    .clamp(0, i64::from(ROWS - self.cursor.size))
                    as u32;
                Ok(self.update())
            }
            "viewportRows" => {
                let rows = event
                    .payload
                    .get("rows")
                    .and_then(Value::as_u64)
                    .filter(|rows| *rows > 0)
                    .ok_or_else(|| invalid("capacity must be a positive integer"))?;
                self.cursor.size = rows.min(ROWS as u64) as u32;
                self.cursor.reveal();
                Ok(self.update())
            }
            "resizeViewport" => {
                let size = index(&event, "rows", ROWS + 1)?;
                if size == 0 {
                    return Err(invalid("viewport must contain at least one row"));
                }
                self.cursor.size = size;
                self.cursor.reveal();
                Ok(self.update())
            }
            "newWorkbook" => {
                let mut workbook = Workbook::new();
                workbook.add_sheet("Sheet1");
                self.workbook = workbook;
                self.cursor.row = 0;
                self.cursor.col = 0;
                self.cursor.offset = 0;
                self.edit = None;
                self.file_status = "New workbook · not saved".into();
                Ok(self.announced(format!("New workbook. {}", self.describe_cell(0, 0))))
            }
            _ => Err(invalid(format!("unknown VisiCalc event: {}", event.name))),
        }
    }

    fn snapshot(&self) -> Result<Option<Snapshot>, Self::Error> {
        let bytes = serde_json::to_vec(&SavedState {
            workbook: self.workbook.serialize(),
            cursor: self.cursor.clone(),
        })
        .map_err(|error| invalid(error.to_string()))?;
        Ok(Some(Snapshot {
            schema: SNAPSHOT_SCHEMA.into(),
            version: SNAPSHOT_VERSION,
            bytes,
        }))
    }

    fn complete_effect(
        &mut self,
        id: EffectId,
        result: EffectResult,
    ) -> Result<AppUpdate, EffectCompletionError<Self::Error>> {
        let Some((pending_id, saving)) = self.pending_file else {
            return Err(invalid("no pending file operation").into());
        };
        if id != pending_id {
            return Err(invalid("incorrect file operation").into());
        }
        let message = match result {
            EffectResult::Cancelled(_) => {
                "File operation cancelled. Your workbook is unchanged.".to_string()
            }
            EffectResult::Failed(error) => format!(
                "Could not {}: {}",
                if saving { "save" } else { "open" },
                error.message.chars().take(220).collect::<String>()
            ),
            EffectResult::Ok(payload) if saving => {
                let file = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("workbook");
                format!("Saved {}", file.chars().take(120).collect::<String>())
            }
            EffectResult::Ok(payload) => {
                if self.open_file(&payload).is_ok() {
                    let file = payload
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("workbook");
                    format!("Opened {}", file.chars().take(120).collect::<String>())
                } else {
                    "Could not open this file. Choose a VisiCalc workbook; your work is unchanged."
                        .into()
                }
            }
        };
        // Invalid file content is an accepted terminal operation with an error
        // view, not an eternally pending protocol completion.
        self.pending_file = None;
        Ok(self.file_message(message))
    }

    fn restore(&mut self, snapshot: Snapshot) -> Result<AppUpdate, Self::Error> {
        if snapshot.schema != SNAPSHOT_SCHEMA || snapshot.version != SNAPSHOT_VERSION {
            return Err(invalid("unsupported VisiCalc snapshot"));
        }
        let saved: SavedState =
            serde_json::from_slice(&snapshot.bytes).map_err(|error| invalid(error.to_string()))?;
        if !saved.cursor.valid() {
            return Err(invalid("invalid snapshot cursor"));
        }
        let mut workbook = Workbook::new();
        workbook.deserialize(&saved.workbook).map_err(invalid)?;
        if workbook.sheet_count() != 1 {
            return Err(invalid("expected one workbook sheet"));
        }
        self.workbook = workbook;
        self.cursor = saved.cursor;
        self.edit = None;
        Ok(self.announced(format!(
            "Workbook restored. {}",
            self.describe_cell(self.cursor.row, self.cursor.col)
        )))
    }
}

mosaic_app_capi::export_mosaic_app!(VisiCalcMosaicApp, VisiCalcMosaicApp::default());

#[cfg(test)]
mod tests {
    use super::*;
    use mosaic_app_runtime::{MosaicRuntime, Platform};

    fn dispatch(app: &mut VisiCalcMosaicApp, event: &str, payload: Value) -> AppUpdate {
        app.dispatch(Event::new(1, event, payload)).unwrap()
    }

    #[test]
    fn empty_introduction_tracks_committed_content_across_the_workbook() {
        let mut app = VisiCalcMosaicApp::default();
        assert_eq!(app.update().props["workbook-empty"], false);
        assert_eq!(dispatch(&mut app, "newWorkbook", json!({})).props["workbook-empty"], true);
        dispatch(&mut app, "onFormulaChange", json!({"value":"12"}));
        assert_eq!(app.update().props["workbook-empty"], true);
        dispatch(&mut app, "cancel", json!({}));
        assert_eq!(app.update().props["workbook-empty"], true);
        dispatch(&mut app, "navigate", json!({"row":99,"col":25}));
        dispatch(&mut app, "onFormulaChange", json!({"value":"0"}));
        dispatch(&mut app, "commit", json!({}));
        dispatch(&mut app, "scroll", json!({"offset":0}));
        assert_eq!(app.update().props["workbook-empty"], false);
        let saved = app.snapshot().unwrap().unwrap();
        dispatch(&mut app, "newWorkbook", json!({}));
        app.restore(saved).unwrap();
        assert_eq!(app.update().props["workbook-empty"], false);
        app.workbook.clear_cell(SheetId(0), CellAddress::new(100, 26));
        assert_eq!(app.update().props["workbook-empty"], true);
        let second = app.workbook.add_sheet("Other");
        app.workbook.set_formula(second, CellAddress::new(100, 26), "=A1").unwrap();
        // No recalc: authored formulas count even before a display value exists.
        assert_eq!(app.update().props["workbook-empty"], false);
    }

    fn file_app() -> MosaicRuntime<VisiCalcMosaicApp> {
        let mut runtime = MosaicRuntime::new(VisiCalcMosaicApp::default());
        runtime
            .start(StartContext {
                protocol_version: 2,
                ..StartContext::new("en", Platform::Web)
            })
            .unwrap();
        runtime
    }

    fn send(
        runtime: &mut MosaicRuntime<VisiCalcMosaicApp>,
        event: &str,
        payload: Value,
    ) -> mosaic_app_runtime::Update {
        let mut event = Event::new(runtime.next_sequence().unwrap(), event, payload);
        event.protocol_version = 2;
        runtime.dispatch(event).unwrap()
    }

    #[test]
    fn files_round_trip_committed_formulas_into_a_fresh_application() {
        let mut original = file_app();
        send(&mut original, "formulaChange", json!({"value": "20"}));
        let blocked = send(&mut original, "saveWorkbook", json!({}));
        assert!(blocked.effects.is_empty());
        assert!(blocked.props["file-status"]
            .as_str()
            .unwrap()
            .contains("Press Enter"));
        send(&mut original, "commit", json!({}));
        let saving = send(&mut original, "saveWorkbook", json!({}));
        assert_eq!(saving.effects[0].kind, "file.save");
        assert!(original.snapshot().is_err());
        send(&mut original, "newWorkbook", json!({}));
        let saved = original
            .complete_effect(
                saving.effects[0].id,
                EffectResult::Ok(json!({"name":"Budget.visicalc"})),
            )
            .unwrap();
        assert_eq!(saved.props["formula"], "20");
        assert_eq!(saved.props["file-status"], "Saved Budget.visicalc");
        let mut fresh = file_app();
        send(&mut fresh, "newWorkbook", json!({}));
        let opening = send(&mut fresh, "openWorkbook", json!({}));
        let opened = fresh
            .complete_effect(
                opening.effects[0].id,
                EffectResult::Ok(json!({
                    "name":"Budget.visicalc", "bytes":saving.effects[0].payload["bytes"]
                })),
            )
            .unwrap();
        assert_eq!(opened.props["formula"], "20");
        assert_eq!(opened.props["viewport-rows"][4][4], "174");
        assert_eq!(opened.props["editing"], false);
        assert!(fresh.snapshot().is_ok());
    }

    #[test]
    fn cancelled_failed_and_malformed_open_preserve_workbook_and_pending_edit() {
        use mosaic_app_runtime::{EffectFailure, EmptyOutcome};
        let mut runtime = file_app();
        send(
            &mut runtime,
            "formulaChange",
            json!({"value":"uncommitted"}),
        );
        let checkpoint = runtime.snapshot().unwrap();
        let mut wrong_version = checkpoint.clone().unwrap();
        wrong_version.version = 99;
        let malformed = [
            json!({"bytes":"???"}),
            json!({"bytes":"e30="}),
            json!({
                "bytes":coding_adventures_base64::encode(&serde_json::to_vec(&wrong_version).unwrap(), &coding_adventures_base64::STANDARD)
            }),
        ];
        let mut results = vec![
            EffectResult::Cancelled(EmptyOutcome {}),
            EffectResult::Failed(EffectFailure {
                message: "permission denied".into(),
            }),
        ];
        results.extend(malformed.into_iter().map(EffectResult::Ok));
        for result in results {
            let request = send(&mut runtime, "openWorkbook", json!({}));
            let update = runtime
                .complete_effect(request.effects[0].id, result)
                .unwrap();
            assert_eq!(update.props["formula"], "uncommitted");
            assert_eq!(update.props["editing"], true);
            assert_eq!(runtime.snapshot().unwrap(), checkpoint);
            assert!(runtime.pending_effects().is_empty());
        }
    }

    #[test]
    fn legacy_native_host_reports_unavailable_without_emitting_await() {
        let mut runtime = MosaicRuntime::new(VisiCalcMosaicApp::default());
        runtime
            .start(StartContext::new("en", Platform::Windows))
            .unwrap();
        for (index, event) in ["openWorkbook", "saveWorkbook"].into_iter().enumerate() {
            let update = runtime
                .dispatch(Event::new(index as u64 + 1, event, json!({})))
                .unwrap();
            assert!(update.effects.is_empty());
            assert_eq!(
                update.props["file-status"],
                "Open and Save are unavailable in this host."
            );
            assert!(runtime.snapshot().is_ok());
        }
    }

    #[test]
    fn root_grid_events_translate_the_slice_in_the_shared_adapter() {
        let mut app = VisiCalcMosaicApp::default();
        let update = dispatch(&mut app, "navigate", json!({"row": 30, "col": 0}));
        assert_eq!(update.props["viewport-offset"], 1);
        assert_eq!(update.props["grid-selected-row"], 29);
        assert_eq!(update.props["grid-edit-row"], -1);
        let update = dispatch(&mut app, "onGridNavigate", json!({"row": 0, "col": 0}));
        assert_eq!(update.props["cell-address"], "A2");
        assert_eq!(update.props["formula"], "8");
        assert_eq!(update.props["grid-selected-row"], 0);
        let update = dispatch(&mut app, "editStart", json!({"row": 1, "col": 0}));
        assert_eq!(update.props["grid-edit-row"], 0);
        let snapshot = app.snapshot().unwrap();
        assert!(app
            .dispatch(Event::new(1, "gridNavigate", json!({"row": 30, "col": 0})))
            .is_err());
        assert_eq!(app.snapshot().unwrap(), snapshot);
        assert_eq!(app.update().props, update.props);
    }

    #[test]
    fn shared_presentation_contract_matches_the_real_engine() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../programs/mosaic/visicalc/fixtures/presentation-contract-v1.json"
        ))
        .unwrap();
        assert_eq!(fixture["schemaVersion"], 1);
        let mut app = VisiCalcMosaicApp::default();
        let mut update = app
            .start(StartContext::new("en-US", Platform::Windows))
            .unwrap();
        for step in fixture["steps"].as_array().unwrap() {
            if let Some(event) = step.get("event") {
                update = dispatch(
                    &mut app,
                    event["type"].as_str().unwrap(),
                    event["payload"].clone(),
                );
            }
            let expected = &step["expected"];
            for (fixture_key, prop) in [
                ("selectedRow", "selected-row"),
                ("selectedCol", "selected-col"),
                ("viewportOffset", "viewport-offset"),
                ("viewportSize", "viewport-size"),
                ("formula", "formula"),
                ("editing", "editing"),
            ] {
                assert_eq!(
                    update.props[prop], expected["slots"][fixture_key],
                    "{}: {prop}",
                    step["id"]
                );
            }
            for (address, values) in expected["engine"].as_object().unwrap() {
                let address = CellAddress::parse(address).unwrap();
                assert_eq!(
                    app.workbook.cell_source_text(SheetId(0), address),
                    values[0].as_str().unwrap(),
                    "{} source",
                    step["id"]
                );
                assert_eq!(
                    app.workbook.get_display(SheetId(0), address),
                    values[1].as_str().unwrap(),
                    "{} display",
                    step["id"]
                );
            }
            let rows = update.props["viewport-rows"].as_array().unwrap();
            assert_eq!(rows.len(), app.cursor.size as usize);
            for (row, cells) in rows.iter().enumerate() {
                assert_eq!(cells.as_array().unwrap().len(), COLS as usize);
                for (col, display) in cells.as_array().unwrap().iter().enumerate() {
                    assert_eq!(
                        display,
                        &json!(app.workbook.get_display(
                            SheetId(0),
                            CellAddress::new(app.cursor.offset + row as u32 + 1, col as u32 + 1)
                        ))
                    );
                }
            }
        }
    }

    #[test]
    fn snapshot_restores_committed_work_and_discards_pending_edit() {
        let mut app = VisiCalcMosaicApp::default();
        dispatch(&mut app, "formulaChange", json!({"value":"20"}));
        dispatch(&mut app, "commit", json!({}));
        dispatch(&mut app, "navigate", json!({"row":35,"col":4}));
        dispatch(&mut app, "formulaChange", json!({"value":"unsaved draft"}));
        let snapshot = app.snapshot().unwrap().unwrap();
        let mut restored = VisiCalcMosaicApp::default();
        let mut context = StartContext::new("en-US", Platform::Linux);
        context.restored_snapshot = Some(snapshot);
        let update = restored.start(context).unwrap();
        assert_eq!(update.props["selected-row"], 35);
        assert_eq!(update.props["viewport-offset"], 6);
        assert_eq!(update.props["editing"], false);
        assert_eq!(update.props["formula"], "");
        assert_eq!(restored.source(0, 0), "20");
        assert_eq!(
            restored
                .workbook
                .get_display(SheetId(0), CellAddress::new(5, 5)),
            "174"
        );
        assert_eq!(restored.workbook.serialize(), app.workbook.serialize());
    }

    #[test]
    fn rejected_events_preserve_workbook_and_active_edit() {
        let mut app = VisiCalcMosaicApp::default();
        dispatch(&mut app, "formulaChange", json!({"value":"pending"}));
        for (event, payload) in [
            ("navigate", json!({"row":3,"col":26})),
            ("navigate", json!({"row":-1,"col":0})),
            ("editStart", json!({"row":1.5,"col":0})),
            ("formulaChange", json!({"value":25})),
            ("resizeViewport", json!({"rows":0})),
            ("resizeViewport", json!({"rows":101})),
            ("scroll", json!({"offset":71})),
            ("commit", Value::Null),
            ("unknown", json!({})),
        ] {
            let before = app.update();
            let saved = app.snapshot().unwrap();
            assert!(
                app.dispatch(Event::new(1, event, payload)).is_err(),
                "{event}"
            );
            assert_eq!(app.update(), before, "{event}");
            assert_eq!(app.snapshot().unwrap(), saved, "{event}");
        }
    }

    #[test]
    fn invalid_restores_and_failed_start_leave_existing_state_unchanged() {
        let mut app = VisiCalcMosaicApp::default();
        dispatch(&mut app, "formulaChange", json!({"value":"pending"}));
        let before = app.update();
        let good = app.snapshot().unwrap().unwrap();
        let mut malformed = good.clone();
        malformed.bytes = b"not-json".to_vec();
        let mut wrong_version = good.clone();
        wrong_version.version = 99;
        let mut bad_cursor = good.clone();
        let mut data: Value = serde_json::from_slice(&bad_cursor.bytes).unwrap();
        data["cursor"]["size"] = json!(0);
        bad_cursor.bytes = serde_json::to_vec(&data).unwrap();
        let mut bad_workbook = good.clone();
        data = serde_json::from_slice(&good.bytes).unwrap();
        data["workbook"] = json!("{\"version\":1,\"sheets\":[]}");
        bad_workbook.bytes = serde_json::to_vec(&data).unwrap();
        for snapshot in [malformed, wrong_version, bad_cursor, bad_workbook] {
            let mut context = StartContext::new("en-US", Platform::Web);
            context.color_scheme = ColorScheme::Dark;
            context.restored_snapshot = Some(snapshot.clone());
            assert!(app.start(context).is_err());
            assert_eq!(app.update(), before);
            assert!(app.restore(snapshot).is_err());
            assert_eq!(app.update(), before);
            assert_eq!(app.snapshot().unwrap().unwrap(), good);
        }
    }

    #[test]
    fn viewport_resize_and_bottom_edge_stay_bounded() {
        let mut app = VisiCalcMosaicApp::default();
        dispatch(&mut app, "onNavigate", json!({"row":99,"col":25}));
        let small = dispatch(&mut app, "resizeViewport", json!({"rows":5}));
        assert_eq!(small.props["viewport-offset"], 95);
        assert_eq!(small.props["viewport-rows"].as_array().unwrap().len(), 5);
        dispatch(&mut app, "editStart", json!({"row":99,"col":25}));
        dispatch(&mut app, "onFormulaChange", json!({"value":"=A1*2"}));
        let committed = dispatch(&mut app, "onEditCommit", json!({}));
        assert_eq!(committed.props["selected-row"], 99);
        assert_eq!(committed.props["viewport-rows"][4][25], "30");
        let scrolled = dispatch(&mut app, "scroll", json!({"offset":0}));
        assert_eq!(scrolled.props["selected-row"], 99);
        assert_eq!(scrolled.props["viewport-offset"], 0);
        let expanded = dispatch(&mut app, "resizeViewport", json!({"rows":100}));
        assert_eq!(expanded.props["viewport-offset"], 0);
        assert_eq!(
            expanded.props["viewport-rows"].as_array().unwrap().len(),
            100
        );
        let empty = dispatch(&mut app, "newWorkbook", json!({}));
        assert_eq!(empty.props["cell-address"], "A1");
        assert_eq!(empty.props["viewport-rows"][0][0], "");
    }

    #[test]
    fn wheel_shift_clamps_without_retargeting_selection_or_pending_edits() {
        let mut app = VisiCalcMosaicApp::default();
        dispatch(&mut app, "formulaChange", json!({"value":"27"}));
        let scrolled = dispatch(&mut app, "viewportShift", json!({"rows":i64::MAX}));
        assert_eq!(scrolled.props["viewport-offset"], 70);
        assert_eq!(scrolled.props["cell-address"], "A1");
        assert_eq!(scrolled.props["row-headers"][0], "71");
        assert_eq!(scrolled.props["formula"], "27");
        dispatch(&mut app, "commit", json!({}));
        let returned = dispatch(&mut app, "viewportShift", json!({"rows":i64::MIN}));
        assert_eq!(returned.props["viewport-offset"], 0);
        assert_eq!(returned.props["viewport-rows"][0][0], "27");
        dispatch(&mut app, "viewportShift", json!({"rows":70}));
        let clicked = dispatch(&mut app, "gridNavigate", json!({"row":0,"col":25}));
        assert_eq!(clicked.props["cell-address"], "Z71");
        let before = app.snapshot().unwrap();
        for rows in [json!(0), json!(1.5), json!("3")] {
            assert!(app
                .dispatch(Event::new(1, "viewportShift", json!({"rows":rows})))
                .is_err());
            assert_eq!(app.snapshot().unwrap(), before);
        }
    }

    #[test]
    fn measured_capacity_clamps_to_workbook_and_preserves_selection() {
        let mut app = VisiCalcMosaicApp::default();
        dispatch(&mut app, "navigate", json!({"row":99,"col":25}));
        let small = dispatch(&mut app, "onViewportRows", json!({"rows":3}));
        assert_eq!(small.props["viewport-offset"], 97);
        assert_eq!(small.props["grid-selected-row"], 2);
        assert_eq!(small.props["row-headers"], json!(["98", "99", "100"]));
        let big = dispatch(&mut app, "onViewportRows", json!({"rows":1000}));
        assert_eq!(big.props["viewport-rows"].as_array().unwrap().len(), 100);
        assert_eq!(big.props["cell-address"], "Z100");
        let before = app.snapshot().unwrap();
        for rows in [json!(0), json!(-1), json!(1.5), json!("10")] {
            assert!(app
                .dispatch(Event::new(1, "viewportRows", json!({"rows":rows})))
                .is_err());
            assert_eq!(app.snapshot().unwrap(), before);
        }
    }

    #[test]
    fn descriptions_cover_results_blank_cells_edits_and_quiet_viewport_updates() {
        let mut app = VisiCalcMosaicApp::default();
        assert_eq!(app.update().props["selection-summary"], "A1, 15");
        let formula = dispatch(&mut app, "navigate", json!({"row":0,"col":4}));
        assert_eq!(
            formula.announcements[0].message,
            "E1, 38, formula =SUM(A1:D1)"
        );
        let scrolled = dispatch(&mut app, "viewportShift", json!({"rows":70}));
        assert!(scrolled.announcements.is_empty());
        assert_eq!(
            scrolled.props["selection-summary"],
            formula.props["selection-summary"]
        );
        let typing = dispatch(&mut app, "formulaChange", json!({"value":"=1/0"}));
        assert!(typing.announcements.is_empty());
        let error = dispatch(&mut app, "commit", json!({}));
        assert_eq!(
            error.announcements[0].message,
            "Updated E1, #DIV/0!, formula =1/0"
        );
        let editing = dispatch(&mut app, "editStart", json!({"row":0,"col":4}));
        assert!(editing.announcements[0]
            .message
            .starts_with("Editing E1, #DIV/0!"));
        dispatch(&mut app, "formulaChange", json!({"value":"42"}));
        let committed = dispatch(&mut app, "editCommit", json!({}));
        assert_eq!(
            committed.announcements[0].message,
            "Updated E1, 42. Selected E2, 51, formula =SUM(A2:D2)"
        );
        dispatch(&mut app, "formulaChange", json!({"value":"99"}));
        let cancelled = dispatch(&mut app, "cancel", json!({}));
        assert_eq!(
            cancelled.announcements[0].message,
            "Edit cancelled. E2, 51, formula =SUM(A2:D2)"
        );
        assert!(dispatch(&mut app, "cancel", json!({}))
            .announcements
            .is_empty());
        let saved = app.snapshot().unwrap().unwrap();
        let empty = dispatch(&mut app, "newWorkbook", json!({}));
        assert_eq!(empty.announcements[0].message, "New workbook. A1, blank");
        let restored = app.restore(saved).unwrap();
        assert_eq!(
            restored.announcements[0].message,
            "Workbook restored. E2, 51, formula =SUM(A2:D2)"
        );
    }

    #[test]
    fn runtime_can_retry_rejected_event_without_consuming_sequence() {
        let mut runtime = MosaicRuntime::new(VisiCalcMosaicApp::default());
        runtime
            .start(StartContext::new("en-US", Platform::Web))
            .unwrap();
        assert!(runtime
            .dispatch(Event::new(1, "navigate", json!({"row":0,"col":99})))
            .is_err());
        let update = runtime
            .dispatch(Event::new(1, "navigate", json!({"row":0,"col":4})))
            .unwrap();
        assert_eq!(update.revision, 2);
        assert_eq!(update.props["formula"], "=SUM(A1:D1)");
        assert_eq!(
            update.announcements[0].message,
            "E1, 38, formula =SUM(A1:D1)"
        );
    }

    #[test]
    fn native_abi_edits_snapshots_and_restores_the_same_app() {
        use mosaic_app_capi::{MosaicBuffer, MosaicBytes, MosaicStatus};
        unsafe fn take(buffer: MosaicBuffer) -> Vec<u8> {
            let bytes = std::slice::from_raw_parts(buffer.ptr, buffer.len).to_vec();
            mosaic_buffer_free(buffer);
            bytes
        }
        // The test owns all borrowed input buffers and releases every Rust
        // output before reuse; the opaque app handle outlives each call.
        unsafe {
            let context =
                serde_json::to_vec(&StartContext::new("en-US", Platform::Windows)).unwrap();
            let mut handle = std::ptr::null_mut();
            let mut output = MosaicBuffer::empty();
            assert_eq!(
                mosaic_app_create(MosaicBytes::new(&context), &mut handle, &mut output),
                MosaicStatus::Ok
            );
            let initial: Value = serde_json::from_slice(&take(output)).unwrap();
            assert_eq!(initial["props"]["viewport-rows"][4][4], "169");
            for (sequence, event, payload) in [
                (1, "formulaChange", json!({"value":"20"})),
                (2, "commit", json!({})),
            ] {
                let bytes = serde_json::to_vec(&Event::new(sequence, event, payload)).unwrap();
                assert_eq!(
                    mosaic_app_dispatch(handle, MosaicBytes::new(&bytes), &mut output),
                    MosaicStatus::Ok
                );
                let update: Value = serde_json::from_slice(&take(output)).unwrap();
                if sequence == 2 {
                    assert_eq!(update["props"]["viewport-rows"][4][4], "174");
                }
            }
            assert_eq!(mosaic_app_snapshot(handle, &mut output), MosaicStatus::Ok);
            let saved = take(output);
            let bytes = serde_json::to_vec(&Event::new(3, "newWorkbook", json!({}))).unwrap();
            assert_eq!(
                mosaic_app_dispatch(handle, MosaicBytes::new(&bytes), &mut output),
                MosaicStatus::Ok
            );
            mosaic_buffer_free(output);
            assert_eq!(
                mosaic_app_restore(handle, MosaicBytes::new(&saved), &mut output),
                MosaicStatus::Ok
            );
            let restored: Value = serde_json::from_slice(&take(output)).unwrap();
            assert_eq!(restored["props"]["viewport-rows"][4][4], "174");
            mosaic_app_destroy(handle);
        }
    }
}

mosaic_app_wasm::export_mosaic_wasm!(VisiCalcMosaicApp, VisiCalcMosaicApp::default());
