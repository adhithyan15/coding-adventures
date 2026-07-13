//! Linear-memory WASM ABI over the **pure `task-core` engine**.
//!
//! This is the browser/Electron boundary. It follows the repo's `*-wasm`
//! convention (`alloc`/`dealloc`, `(ptr, len)` in, length-prefixed out) and holds one
//! global [`ProjectState`] for the page.
//!
//! There is **no facade and no command bus** — deliberately. Per
//! `task-app-architecture.md`, the engine is pure computation and each backend calls
//! it natively; this ABI simply surfaces the engine's own operation and query
//! functions, **one export each**. The web/React host keeps this engine in React
//! state and re-renders on change — idiomatic React, not a `dispatch`/`getProps` loop.
//!
//! Every export returns a JSON envelope: `{"ok":true}` / `{"ok":true,"data":…}` for
//! success, or `{"ok":false,"error":…,"code":…}` for a rejected operation. Nothing
//! traps the FFI boundary.

use std::alloc::{alloc as raw_alloc, dealloc as raw_dealloc, Layout};
use std::cell::RefCell;

use serde::Deserialize;
use task_core::ops::OpError;
use task_core::{
    Assignment, Constraint, Date, Decision, DependencyLink, FieldDef, FieldValue, GenericLink,
    ProjectId, ProjectState, Resource, TaskId, TaskKind, TaskSchedule, WorkflowId,
};

thread_local! {
    static STATE: RefCell<ProjectState> = RefCell::new(fresh());
}

fn fresh() -> ProjectState {
    ProjectState::empty(ProjectId::from_raw("project"))
}

// ── linear-memory plumbing (repo-standard) ────────────────────────────────────────

#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    if len == 0 {
        return std::ptr::null_mut();
    }
    match Layout::from_size_align(len, 1) {
        Ok(layout) => unsafe { raw_alloc(layout) },
        Err(_) => std::ptr::null_mut(),
    }
}

/// # Safety
/// `ptr`/`len` must exactly match a live allocation made by this module.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    if let Ok(layout) = Layout::from_size_align(len, 1) {
        unsafe { raw_dealloc(ptr, layout) };
    }
}

/// # Safety
/// `ptr` must point to `len` readable bytes, or be null with a zero length.
unsafe fn read_input(ptr: *const u8, len: usize) -> String {
    if ptr.is_null() || len == 0 {
        return String::new();
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).into_owned()
}

/// Pack a string as `[u32 little-endian length][UTF-8 bytes]` in a fresh allocation.
fn pack(value: String) -> *mut u8 {
    let bytes = value.into_bytes();
    let payload_len = bytes.len();
    let Some(total) = payload_len.checked_add(4) else {
        return std::ptr::null_mut();
    };
    let Ok(layout) = Layout::from_size_align(total, 1) else {
        return std::ptr::null_mut();
    };
    unsafe {
        let ptr = raw_alloc(layout);
        if ptr.is_null() {
            return ptr;
        }
        let len_prefix = (payload_len as u32).to_le_bytes();
        std::ptr::copy_nonoverlapping(len_prefix.as_ptr(), ptr, 4);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), payload_len);
        ptr
    }
}

// ── result envelopes ──────────────────────────────────────────────────────────────

fn ok_json() -> String {
    r#"{"ok":true}"#.to_string()
}
fn ok_data<T: serde::Serialize>(value: &T) -> String {
    serde_json::json!({ "ok": true, "data": value }).to_string()
}
fn op_error_json(err: &OpError) -> String {
    serde_json::json!({ "ok": false, "error": err, "code": err.code() }).to_string()
}
fn error_json(msg: &str) -> String {
    serde_json::json!({ "ok": false, "error": msg }).to_string()
}

fn with_state<R>(f: impl FnOnce(&ProjectState) -> R) -> R {
    STATE.with(|s| f(&s.borrow()))
}

/// Deserialize `json` into an argument type and run a validated operation.
fn run_op<A, F>(json: &str, f: F) -> String
where
    A: for<'de> Deserialize<'de>,
    F: FnOnce(&mut ProjectState, A) -> Result<(), OpError>,
{
    match serde_json::from_str::<A>(json) {
        Ok(args) => STATE.with(|s| match f(&mut s.borrow_mut(), args) {
            Ok(()) => ok_json(),
            Err(e) => op_error_json(&e),
        }),
        Err(e) => error_json(&format!("parse error: {e}")),
    }
}

/// Generate a `(ptr, len)`-input operation export (with the required safety doc).
macro_rules! export_op {
    ($(#[$doc:meta])* $name:ident, $args:ty, $f:expr) => {
        $(#[$doc])*
        ///
        /// # Safety
        /// `ptr`/`len` must describe readable bytes, or be null with a zero length.
        #[no_mangle]
        pub unsafe extern "C" fn $name(ptr: *const u8, len: usize) -> *mut u8 {
            let json = unsafe { read_input(ptr, len) };
            pack(run_op::<$args, _>(&json, $f))
        }
    };
}

/// Generate a no-argument query export returning JSON data.
macro_rules! export_query {
    ($(#[$doc:meta])* $name:ident, $f:expr) => {
        $(#[$doc])*
        #[no_mangle]
        pub extern "C" fn $name() -> *mut u8 {
            pack(with_state($f))
        }
    };
}

// ── lifecycle ─────────────────────────────────────────────────────────────────────

/// Reset to a fresh, empty project.
#[no_mangle]
pub extern "C" fn reset() {
    STATE.with(|s| *s.borrow_mut() = fresh());
}

/// Serialize the whole project (for host-owned persistence).
#[no_mangle]
pub extern "C" fn snapshot() -> *mut u8 {
    pack(with_state(|s| {
        serde_json::to_string(s).unwrap_or_else(|_| error_json("serialize error"))
    }))
}

/// Replace the project with a deserialized snapshot.
///
/// # Safety
/// `ptr`/`len` must describe readable bytes, or be null with a zero length.
#[no_mangle]
pub unsafe extern "C" fn load(ptr: *const u8, len: usize) -> *mut u8 {
    let json = unsafe { read_input(ptr, len) };
    pack(match serde_json::from_str::<ProjectState>(&json) {
        Ok(project) => {
            STATE.with(|s| *s.borrow_mut() = project);
            ok_json()
        }
        Err(e) => error_json(&format!("parse error: {e}")),
    })
}

// ── operations (one export per engine method) ─────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTaskArgs {
    id: String,
    name: String,
    parent: Option<String>,
}
export_op!(
    /// Create a leaf task.
    create_task,
    CreateTaskArgs,
    |s, a| s.create_task(TaskId::from_raw(a.id), a.name, a.parent.map(TaskId::from_raw))
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdNameArgs {
    id: String,
    name: String,
}
export_op!(
    /// Rename a task.
    rename_task,
    IdNameArgs,
    |s, a| s.rename_task(&TaskId::from_raw(a.id), a.name)
);

#[derive(Deserialize)]
struct IdArg {
    id: String,
}
export_op!(
    /// Delete a task (children reparented; links pruned).
    delete_task,
    IdArg,
    |s, a| s.delete_task(&TaskId::from_raw(a.id))
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReparentArgs {
    id: String,
    new_parent: Option<String>,
}
export_op!(
    /// Move a task under a new parent (cycle-checked).
    reparent,
    ReparentArgs,
    |s, a| s.reparent(&TaskId::from_raw(a.id), a.new_parent.map(TaskId::from_raw))
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct KindArgs {
    id: String,
    kind: TaskKind,
}
export_op!(
    /// Set a task's kind (leaf/summary/milestone).
    set_kind,
    KindArgs,
    |s, a| s.set_kind(&TaskId::from_raw(a.id), a.kind)
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompletedArgs {
    id: String,
    completed: bool,
}
export_op!(
    /// Set a task's completion flag.
    set_completed,
    CompletedArgs,
    |s, a| s.set_completed(&TaskId::from_raw(a.id), a.completed)
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PercentArgs {
    id: String,
    percent: u8,
}
export_op!(
    /// Set percent complete (clamped 0..=100).
    set_percent_complete,
    PercentArgs,
    |s, a| s.set_percent_complete(&TaskId::from_raw(a.id), a.percent)
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatusArgs {
    id: String,
    status: Option<String>,
}
export_op!(
    /// Set a task's workflow status.
    set_status,
    StatusArgs,
    |s, a| s.set_status(
        &TaskId::from_raw(a.id),
        a.status.map(task_core::StatusId::from_raw)
    )
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScheduleArgs {
    id: String,
    schedule: Option<TaskSchedule>,
}
export_op!(
    /// Set or clear a task's scheduling block.
    set_schedule,
    ScheduleArgs,
    |s, a| s.set_schedule(&TaskId::from_raw(a.id), a.schedule)
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DurationArgs {
    id: String,
    duration: task_core::Duration,
}
export_op!(
    /// Set a task's duration.
    set_duration,
    DurationArgs,
    |s, a| s.set_duration(&TaskId::from_raw(a.id), a.duration)
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConstraintArgs {
    id: String,
    constraint: Constraint,
}
export_op!(
    /// Set a task's date constraint.
    set_constraint,
    ConstraintArgs,
    |s, a| s.set_constraint(&TaskId::from_raw(a.id), a.constraint)
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeadlineArgs {
    id: String,
    deadline: Option<Date>,
}
export_op!(
    /// Set or clear a task's deadline.
    set_deadline,
    DeadlineArgs,
    |s, a| s.set_deadline(&TaskId::from_raw(a.id), a.deadline)
);

export_op!(
    /// Add a dependency (self/duplicate/cycle-checked).
    link_dependency,
    DependencyLink,
    |s, a| s.link_dependency(a)
);

#[derive(Deserialize)]
struct LinkIdArg {
    id: String,
}
export_op!(
    /// Remove a dependency by id.
    unlink_dependency,
    LinkIdArg,
    |s, a| {
        s.unlink_dependency(&task_core::LinkId::from_raw(a.id));
        Ok(())
    }
);

export_op!(
    /// Add a non-scheduling link.
    add_link,
    GenericLink,
    |s, a| s.add_link(a)
);

export_op!(
    /// Create or replace a resource.
    upsert_resource,
    Resource,
    |s, a| {
        s.upsert_resource(a);
        Ok(())
    }
);

export_op!(
    /// Assign a resource to a task.
    assign,
    Assignment,
    |s, a| s.assign(a)
);

export_op!(
    /// Create or replace a custom field definition.
    upsert_field,
    FieldDef,
    |s, a| {
        s.upsert_field(a);
        Ok(())
    }
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FieldValueArgs {
    task: String,
    field: String,
    value: Option<FieldValue>,
}
export_op!(
    /// Set or clear a task's value for a custom field.
    set_field_value,
    FieldValueArgs,
    |s, a| s.set_field_value(
        &TaskId::from_raw(a.task),
        &task_core::FieldId::from_raw(a.field),
        a.value
    )
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DecisionArgs {
    id: String,
    decision: Option<Decision>,
}
export_op!(
    /// Set or clear a task's decision (branch point).
    set_decision,
    DecisionArgs,
    |s, a| s.set_decision(&TaskId::from_raw(a.id), a.decision)
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnswerArgs {
    id: String,
    answer: bool,
}
export_op!(
    /// Answer a task's decision.
    answer_decision,
    AnswerArgs,
    |s, a| s.answer_decision(&TaskId::from_raw(a.id), a.answer)
);

#[derive(Deserialize)]
struct NameArg {
    name: String,
}
export_op!(
    /// Rename the whole project.
    set_project_name,
    NameArg,
    |s, a| {
        s.set_project_name(a.name);
        Ok(())
    }
);

// ── queries / projections (one export per view) ───────────────────────────────────

export_query!(
    /// The flattened, decision-aware checklist.
    checklist,
    |s| ok_data(&s.checklist())
);
export_query!(
    /// The flat todo list.
    todos,
    |s| ok_data(&s.todos())
);
export_query!(
    /// The flowchart graph.
    flowchart,
    |s| ok_data(&s.flowchart())
);

/// The Gantt timeline anchored at `project_start` (days since the Unix epoch).
#[no_mangle]
pub extern "C" fn gantt(project_start: i32) -> *mut u8 {
    pack(with_state(|s| ok_data(&s.gantt(Date(project_start)))))
}

/// The CPM schedule anchored at `project_start` (days since the Unix epoch).
#[no_mangle]
pub extern "C" fn schedule(project_start: i32) -> *mut u8 {
    pack(with_state(|s| match s.schedule(Date(project_start)) {
        Ok(result) => ok_data(&result),
        Err(_) => error_json("dependency cycle"),
    }))
}

/// The kanban board for the workflow whose id is passed in.
///
/// # Safety
/// `ptr`/`len` must describe readable bytes, or be null with a zero length.
#[no_mangle]
pub unsafe extern "C" fn kanban(ptr: *const u8, len: usize) -> *mut u8 {
    let id = unsafe { read_input(ptr, len) };
    pack(with_state(|s| {
        match s.workflows.get(&WorkflowId::from_raw(id.clone())) {
            Some(workflow) => ok_data(&s.kanban(workflow)),
            None => error_json("workflow not found"),
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(value: &str) -> (*mut u8, usize) {
        let bytes = value.as_bytes();
        let ptr = alloc(bytes.len());
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len()) };
        (ptr, bytes.len())
    }

    fn take(ptr: *mut u8) -> String {
        unsafe {
            let len = u32::from_le_bytes([*ptr, *ptr.add(1), *ptr.add(2), *ptr.add(3)]) as usize;
            let bytes = std::slice::from_raw_parts(ptr.add(4), len).to_vec();
            dealloc(ptr, 4 + len);
            String::from_utf8(bytes).unwrap()
        }
    }

    fn call1(f: unsafe extern "C" fn(*const u8, usize) -> *mut u8, value: &str) -> String {
        let (ptr, len) = put(value);
        let out = unsafe { f(ptr, len) };
        unsafe { dealloc(ptr, len) };
        take(out)
    }

    #[test]
    fn create_task_then_checklist_round_trips() {
        reset();
        let ok = call1(create_task, r#"{"id":"a","name":"Write spec"}"#);
        assert!(ok.contains(r#""ok":true"#), "{ok}");
        let list = take(checklist());
        assert!(list.contains(r#""ok":true"#), "{list}");
        assert!(list.contains(r#""name":"Write spec""#), "{list}");
    }

    #[test]
    fn invalid_operation_returns_error_envelope_not_a_trap() {
        reset();
        // Renaming a missing task → NotFound, as JSON, no panic.
        let out = call1(rename_task, r#"{"id":"missing","name":"X"}"#);
        assert!(out.contains(r#""ok":false"#), "{out}");
        assert!(out.contains(r#""code":1"#), "{out}");
        // Malformed JSON → parse error, no panic.
        let bad = call1(create_task, r#"{"id":"a""#);
        assert!(
            bad.contains(r#""ok":false"#) && bad.contains("parse error"),
            "{bad}"
        );
    }

    #[test]
    fn dependency_then_gantt_marks_critical() {
        reset();
        call1(create_task, r#"{"id":"a","name":"A"}"#);
        call1(create_task, r#"{"id":"b","name":"B"}"#);
        // `set_duration` creates a default schedule block with the given duration.
        let d = r#"{"id":"ID","duration":{"workingMinutes":480,"elapsed":false}}"#;
        assert!(call1(set_duration, &d.replace("ID", "a")).contains(r#""ok":true"#));
        assert!(call1(set_duration, &d.replace("ID", "b")).contains(r#""ok":true"#));
        let linked = call1(
            link_dependency,
            r#"{"id":"l1","predecessor":"a","successor":"b","kind":"finishToStart","lag":{"workingMinutes":0,"elapsed":false}}"#,
        );
        assert!(linked.contains(r#""ok":true"#), "{linked}");

        // 2026-07-13 (Monday) = 20647 days since epoch.
        let monday = Date::from_ymd(2026, 7, 13).unwrap().0;
        let g = take(gantt(monday));
        assert!(g.contains(r#""ok":true"#), "{g}");
        assert!(g.contains(r#""critical":true"#), "{g}");
    }

    #[test]
    fn snapshot_and_load_round_trip() {
        reset();
        call1(create_task, r#"{"id":"a","name":"Persisted"}"#);
        let snap = take(snapshot());
        assert!(snap.contains(r#""a":{"#), "{snap}");

        reset();
        let loaded = call1(load, &snap);
        assert!(loaded.contains(r#""ok":true"#), "{loaded}");
        let list = take(checklist());
        assert!(list.contains(r#""name":"Persisted""#), "{list}");
    }

    #[test]
    fn empty_input_is_a_json_error_not_a_trap() {
        reset();
        let out = take(unsafe { load(std::ptr::null(), 0) });
        assert!(out.contains(r#""ok":false"#), "{out}");
        unsafe { dealloc(std::ptr::null_mut(), 0) };
    }
}
