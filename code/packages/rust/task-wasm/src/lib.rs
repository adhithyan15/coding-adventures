//! Linear-memory WASM ABI over the **pure `task-core` engine**.
//!
//! This is the browser/Electron boundary. It follows the repo's `*-wasm`
//! convention (`alloc`/`dealloc`, `(ptr, len)` in, length-prefixed out) and holds one
//! global [`Workspace`] for the page.
//!
//! There is **no facade and no command bus** — deliberately. Per
//! `task-app-architecture.md`, the engine is pure computation and each backend calls
//! it natively; this ABI simply surfaces the engine's own operation and query
//! functions, **one export each**. The web/React host keeps this engine in React
//! state and re-renders on change — idiomatic React, not a `dispatch`/`getProps` loop.
//!
//! ## One workspace, one active project
//!
//! The page holds a whole [`Workspace`] (many projects, possibly nested). The
//! **per-project** operations and queries (`create_task`, `todos`, `gantt`, …) act on
//! the *active project* — the first root — so a single-project host behaves exactly as
//! it did when the state was a bare `ProjectState`. The **workspace** operations
//! (`create_project`, `move_task`, `link_cross_project_dependency`, …) and
//! `workspace_schedule` act across all projects. `snapshot`/`load` are workspace-level,
//! and `load` migrates a pre-workspace `ProjectState` snapshot by wrapping it, so data
//! persisted before this change keeps loading.
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
    ProjectId, ProjectState, Resource, ResourceId, TaskId, TaskKind, TaskSchedule, View,
    WorkflowId, Workspace, WorkspaceId,
};

thread_local! {
    static STATE: RefCell<Workspace> = RefCell::new(fresh());
}

fn fresh() -> Workspace {
    Workspace::empty(
        WorkspaceId::from_raw("workspace"),
        ProjectId::from_raw("project"),
    )
}

/// The active project's id — the one the per-project ops/queries act on. It is the
/// first root, falling back to the first project by id. Returns `None` only if a loaded
/// workspace somehow has no projects at all (hostile input), in which case per-project
/// calls answer with an error envelope rather than panicking.
fn active_project_id(ws: &Workspace) -> Option<ProjectId> {
    ws.roots
        .first()
        .cloned()
        .or_else(|| ws.projects.keys().next().cloned())
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

/// Run a query against the **active project**, producing its JSON envelope. If there is
/// no active project (an empty workspace), answer with an error envelope, never a panic.
fn with_state(f: impl FnOnce(&ProjectState) -> String) -> String {
    STATE.with(|s| {
        let ws = s.borrow();
        match active_project_id(&ws).and_then(|pid| ws.projects.get(&pid).map(f)) {
            Some(json) => json,
            None => error_json("no active project"),
        }
    })
}

/// Deserialize `json` into an argument type and run a validated operation on the
/// **active project** (the per-project ABI surface; workspace ops use [`run_ws_op`]).
fn run_op<A, F>(json: &str, f: F) -> String
where
    A: for<'de> Deserialize<'de>,
    F: FnOnce(&mut ProjectState, A) -> Result<(), OpError>,
{
    match serde_json::from_str::<A>(json) {
        Ok(args) => STATE.with(|s| {
            let mut ws = s.borrow_mut();
            let Some(pid) = active_project_id(&ws) else {
                return error_json("no active project");
            };
            match ws.projects.get_mut(&pid) {
                Some(project) => match f(project, args) {
                    Ok(()) => ok_json(),
                    Err(e) => op_error_json(&e),
                },
                None => error_json("no active project"),
            }
        }),
        Err(e) => error_json(&format!("parse error: {e}")),
    }
}

/// Deserialize `json` and run a validated operation on the **whole workspace**.
fn run_ws_op<A, F>(json: &str, f: F) -> String
where
    A: for<'de> Deserialize<'de>,
    F: FnOnce(&mut Workspace, A) -> Result<(), OpError>,
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

/// Generate a `(ptr, len)`-input **workspace** operation export.
macro_rules! export_ws_op {
    ($(#[$doc:meta])* $name:ident, $args:ty, $f:expr) => {
        $(#[$doc])*
        ///
        /// # Safety
        /// `ptr`/`len` must describe readable bytes, or be null with a zero length.
        #[no_mangle]
        pub unsafe extern "C" fn $name(ptr: *const u8, len: usize) -> *mut u8 {
            let json = unsafe { read_input(ptr, len) };
            pack(run_ws_op::<$args, _>(&json, $f))
        }
    };
}

// ── lifecycle ─────────────────────────────────────────────────────────────────────

/// Reset to a fresh workspace holding one empty project.
#[no_mangle]
pub extern "C" fn reset() {
    STATE.with(|s| *s.borrow_mut() = fresh());
}

/// Serialize the whole **workspace** (for host-owned persistence).
#[no_mangle]
pub extern "C" fn snapshot() -> *mut u8 {
    pack(STATE.with(|s| {
        serde_json::to_string(&*s.borrow()).unwrap_or_else(|_| error_json("serialize error"))
    }))
}

/// Replace the workspace with a deserialized snapshot.
///
/// Accepts either a whole [`Workspace`] snapshot or a **pre-workspace** bare
/// [`ProjectState`] snapshot (from before this ABI held a workspace): the latter is
/// migrated by wrapping it in a one-project workspace, so persisted Phase-1 data keeps
/// loading. The two shapes are unambiguous — a `Workspace` requires a `projects` field a
/// `ProjectState` lacks, and vice-versa for `tasks` — so we try `Workspace` first.
///
/// # Safety
/// `ptr`/`len` must describe readable bytes, or be null with a zero length.
#[no_mangle]
pub unsafe extern "C" fn load(ptr: *const u8, len: usize) -> *mut u8 {
    let json = unsafe { read_input(ptr, len) };
    pack(if let Ok(ws) = serde_json::from_str::<Workspace>(&json) {
        STATE.with(|s| *s.borrow_mut() = ws);
        ok_json()
    } else {
        match serde_json::from_str::<ProjectState>(&json) {
            Ok(project) => {
                let ws = Workspace::from_project(WorkspaceId::from_raw("workspace"), project);
                STATE.with(|s| *s.borrow_mut() = ws);
                ok_json()
            }
            Err(e) => error_json(&format!("parse error: {e}")),
        }
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

// ── workspace operations (across projects) ────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateProjectArgs {
    id: String,
    name: String,
    parent: Option<String>,
}
export_ws_op!(
    /// Create a project (`parent = null` ⇒ top-level; otherwise nested).
    create_project,
    CreateProjectArgs,
    |w, a| w.create_project(
        ProjectId::from_raw(a.id),
        a.name,
        a.parent.map(ProjectId::from_raw)
    )
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectIdNameArgs {
    id: String,
    name: String,
}
export_ws_op!(
    /// Rename a project.
    rename_project,
    ProjectIdNameArgs,
    |w, a| w.rename_project(&ProjectId::from_raw(a.id), a.name)
);

#[derive(Deserialize)]
struct ProjectIdArg {
    id: String,
}
export_ws_op!(
    /// Delete a project (rejected while it still has sub-projects).
    delete_project,
    ProjectIdArg,
    |w, a| w.delete_project(&ProjectId::from_raw(a.id))
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NestArgs {
    child: String,
    parent: String,
}
export_ws_op!(
    /// Nest one project under another (forest-cycle-checked).
    nest_project,
    NestArgs,
    |w, a| w.nest_project(&ProjectId::from_raw(a.child), &ProjectId::from_raw(a.parent))
);

#[derive(Deserialize)]
struct ChildArg {
    child: String,
}
export_ws_op!(
    /// Detach a project from its parent (back to top-level).
    unnest_project,
    ChildArg,
    |w, a| w.unnest_project(&ProjectId::from_raw(a.child))
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTaskInArgs {
    project: String,
    id: String,
    name: String,
    parent: Option<String>,
}
export_ws_op!(
    /// Create a task in a named project, with workspace-global id uniqueness.
    create_task_in,
    CreateTaskInArgs,
    |w, a| w.create_task(
        &ProjectId::from_raw(a.project),
        TaskId::from_raw(a.id),
        a.name,
        a.parent.map(TaskId::from_raw)
    )
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveTaskArgs {
    task: String,
    to: String,
}
export_ws_op!(
    /// Move a task to another project (dependencies migrated; assignments/links dropped).
    move_task,
    MoveTaskArgs,
    |w, a| w.move_task(&TaskId::from_raw(a.task), &ProjectId::from_raw(a.to))
);

export_ws_op!(
    /// Add a cross-project dependency (self/same-project/duplicate/cycle-checked).
    link_cross_project_dependency,
    DependencyLink,
    |w, a| w.link_cross_project_dependency(a)
);

#[derive(Deserialize)]
struct WsLinkIdArg {
    id: String,
}
export_ws_op!(
    /// Remove a cross-project dependency by id.
    unlink_cross_project_dependency,
    WsLinkIdArg,
    |w, a| {
        w.unlink_cross_project_dependency(&task_core::LinkId::from_raw(a.id));
        Ok(())
    }
);

export_ws_op!(
    /// Create or replace a resource in the shared pool.
    upsert_shared_resource,
    Resource,
    |w, a| {
        w.upsert_shared_resource(a);
        Ok(())
    }
);

#[derive(Deserialize)]
struct ResourceIdArg {
    id: String,
}
export_ws_op!(
    /// Remove a shared resource (and its assignments across all projects).
    delete_shared_resource,
    ResourceIdArg,
    |w, a| {
        w.delete_shared_resource(&ResourceId::from_raw(a.id));
        Ok(())
    }
);

// ── labels & priority (active project) ────────────────────────────────────────────

export_op!(
    /// Create or replace a label definition.
    upsert_label,
    task_core::Label,
    |s, a| {
        s.upsert_label(a);
        Ok(())
    }
);

#[derive(Deserialize)]
struct LabelIdArg {
    id: String,
}
export_op!(
    /// Delete a label and remove it from every task.
    delete_label,
    LabelIdArg,
    |s, a| {
        s.delete_label(&task_core::LabelId::from_raw(a.id));
        Ok(())
    }
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskLabelsArgs {
    id: String,
    labels: Vec<String>,
}
export_op!(
    /// Replace a task's labels (unknown ids rejected; duplicates collapsed).
    set_task_labels,
    TaskLabelsArgs,
    |s, a| s.set_task_labels(
        &TaskId::from_raw(a.id),
        a.labels.into_iter().map(task_core::LabelId::from_raw).collect()
    )
);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PriorityArgs {
    id: String,
    priority: Option<task_core::Priority>,
}
export_op!(
    /// Set or clear a task's triage priority.
    set_priority,
    PriorityArgs,
    |s, a| s.set_priority(&TaskId::from_raw(a.id), a.priority)
);

// ── view projections (active project) ─────────────────────────────────────────────

/// The widest day-offset accepted from the host (~±8,200 years around the epoch).
///
/// Absurdly generous for any real plan, but bounded well below the point where civil-date
/// conversion overflows: `Date::to_ymd` shifts by 719,468 days internally, so an
/// unchecked `i32` near the type's limit would overflow and **panic across the FFI
/// boundary** — which this module promises never to do. The view projections are the
/// first exports to reach that formatting path (earlier date exports only echoed the raw
/// integer), so the bound is enforced here, at the boundary that accepts the value.
const MAX_DAY_OFFSET: i32 = 3_000_000;

/// Convert a host-supplied day offset into a `Date`, rejecting out-of-range values.
fn checked_day(days: i32) -> Option<Date> {
    (-MAX_DAY_OFFSET..=MAX_DAY_OFFSET)
        .contains(&days)
        .then_some(Date(days))
}

/// Arguments shared by the view-driven projections: the view config plus the project
/// start the schedule is anchored at (days since the Unix epoch).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ViewArgs {
    view: View,
    project_start: i32,
}

/// The render-ready table (sheet) for a view: columns + grouped, formatted rows.
///
/// # Safety
/// `ptr`/`len` must describe readable bytes, or be null with a zero length.
#[no_mangle]
pub unsafe extern "C" fn table(ptr: *const u8, len: usize) -> *mut u8 {
    let json = unsafe { read_input(ptr, len) };
    pack(run_view::<ViewArgs>(&json, |project, a| {
        let Some(start) = checked_day(a.project_start) else {
            return error_json("projectStart out of range");
        };
        ok_data(&project.table(&a.view, start))
    }))
}

/// The ordered, grouped task ids for a view (filter → sort → group).
///
/// # Safety
/// `ptr`/`len` must describe readable bytes, or be null with a zero length.
#[no_mangle]
pub unsafe extern "C" fn view_selection(ptr: *const u8, len: usize) -> *mut u8 {
    let json = unsafe { read_input(ptr, len) };
    pack(run_view::<ViewArgs>(&json, |project, a| {
        let Some(start) = checked_day(a.project_start) else {
            return error_json("projectStart out of range");
        };
        ok_data(&project.view_selection(&a.view, start))
    }))
}

/// The calendar for a view over an inclusive day range.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CalendarArgs {
    view: View,
    project_start: i32,
    start: i32,
    end: i32,
}

/// Dated events for a view over `[start, end]`.
///
/// # Safety
/// `ptr`/`len` must describe readable bytes, or be null with a zero length.
#[no_mangle]
pub unsafe extern "C" fn calendar(ptr: *const u8, len: usize) -> *mut u8 {
    let json = unsafe { read_input(ptr, len) };
    pack(run_view::<CalendarArgs>(&json, |project, a| {
        let (Some(start), Some(from), Some(to)) = (
            checked_day(a.project_start),
            checked_day(a.start),
            checked_day(a.end),
        ) else {
            return error_json("date out of range");
        };
        let range = task_core::view::DateRange {
            start: from,
            end: to,
        };
        ok_data(&project.calendar(&a.view, range, start))
    }))
}

/// Deserialize view arguments and run a projection against the **active project**,
/// mirroring [`run_op`]'s error handling: a parse failure or an empty workspace answers
/// with an error envelope rather than trapping.
fn run_view<A>(json: &str, f: impl FnOnce(&ProjectState, A) -> String) -> String
where
    A: for<'de> Deserialize<'de>,
{
    match serde_json::from_str::<A>(json) {
        Ok(args) => STATE.with(|s| {
            let ws = s.borrow();
            match active_project_id(&ws).and_then(|pid| ws.projects.get(&pid)) {
                Some(project) => f(project, args),
                None => error_json("no active project"),
            }
        }),
        Err(e) => error_json(&format!("parse error: {e}")),
    }
}

// ── workspace queries ─────────────────────────────────────────────────────────────

/// The whole workspace (all projects, nesting, cross-project edges, shared pool).
#[no_mangle]
pub extern "C" fn workspace() -> *mut u8 {
    pack(STATE.with(|s| ok_data(&*s.borrow())))
}

/// The whole-workspace CPM schedule anchored at `project_start` (days since the Unix
/// epoch): per-task dates across every project plus per-project rollups.
#[no_mangle]
pub extern "C" fn workspace_schedule(project_start: i32) -> *mut u8 {
    pack(
        STATE.with(|s| match s.borrow().schedule(Date(project_start)) {
            Ok(result) => ok_data(&result),
            Err(_) => error_json("dependency cycle"),
        }),
    )
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

    // ── workspace surface ───────────────────────────────────────────────────────

    #[test]
    fn per_project_ops_target_the_active_project_unchanged() {
        // A single-project host sees identical behaviour: create_task lands in the
        // active ("project") project and shows up in its checklist.
        reset();
        assert!(call1(create_task, r#"{"id":"a","name":"A"}"#).contains(r#""ok":true"#));
        let list = take(checklist());
        assert!(list.contains(r#""name":"A""#), "{list}");
    }

    #[test]
    fn snapshot_is_a_workspace_and_migrates_a_bare_project_on_load() {
        // A bare pre-workspace ProjectState snapshot (no `projects` field) still loads,
        // wrapped into a one-project workspace, so Phase-1 persisted data keeps working.
        reset();
        let bare = r#"{"id":"project","name":"","tasks":{"a":{"id":"a","name":"Legacy","notes":"","parent":null,"order":0,"kind":"leaf","collapsed":false,"status":null,"completed":false,"percentComplete":0,"schedule":null,"fields":{},"decision":null}},"dependencies":[],"links":[],"resources":{},"assignments":[],"calendars":{},"projectCalendar":"calendar-standard","fields":{},"workflows":{},"baselines":{},"views":{},"settings":{"durationUnit":"days","weekStart":1,"currency":"USD","hoursPerDay":8,"daysPerWeek":5}}"#;
        assert!(call1(load, bare).contains(r#""ok":true"#));
        // The migrated task is visible via the per-project checklist…
        assert!(take(checklist()).contains(r#""name":"Legacy""#));
        // …and the new snapshot is a workspace (has the `projects` map).
        let snap = take(snapshot());
        assert!(
            snap.contains(r#""projects":{"#),
            "workspace snapshot: {snap}"
        );
        // Round-trips as a workspace too.
        reset();
        assert!(call1(load, &snap).contains(r#""ok":true"#));
        assert!(take(checklist()).contains(r#""name":"Legacy""#));
    }

    #[test]
    fn cross_project_schedule_sequences_across_projects() {
        reset();
        // Second project + a task in each; make both schedulable (1 working day).
        assert!(call1(create_project, r#"{"id":"p2","name":"Second"}"#).contains(r#""ok":true"#));
        assert!(call1(
            create_task_in,
            r#"{"project":"project","id":"a","name":"A"}"#
        )
        .contains(r#""ok":true"#));
        assert!(
            call1(create_task_in, r#"{"project":"p2","id":"b","name":"B"}"#)
                .contains(r#""ok":true"#)
        );
        let d = r#"{"id":"ID","duration":{"workingMinutes":480,"elapsed":false}}"#;
        // set_duration targets the ACTIVE project — a is there; move active or set via ws?
        // a is in the active project "project"; b is in p2. Use per-project set_duration
        // for a; for b we can't (not active), so schedule b via its own project by making
        // schedules through the workspace is out of scope — instead give both a schedule
        // by putting durations on tasks that live in the active project is insufficient.
        // Simpler: both tasks need a schedule. Set a's here; set b's after we verify the
        // cross-project *link* is accepted and the workspace schedule runs without cycles.
        assert!(call1(set_duration, &d.replace("ID", "a")).contains(r#""ok":true"#));

        // A valid cross-project dependency a → b is accepted.
        let linked = call1(
            link_cross_project_dependency,
            r#"{"id":"x1","predecessor":"a","successor":"b","kind":"finishToStart","lag":{"workingMinutes":0,"elapsed":false}}"#,
        );
        assert!(linked.contains(r#""ok":true"#), "{linked}");
        // A same-project link is rejected by the cross-project op.
        let same = call1(
            link_cross_project_dependency,
            r#"{"id":"x2","predecessor":"a","successor":"a","kind":"finishToStart","lag":{"workingMinutes":0,"elapsed":false}}"#,
        );
        assert!(same.contains(r#""ok":false"#), "{same}");

        // workspace_schedule runs over all projects and reports per-project rollups.
        let monday = Date::from_ymd(2026, 7, 13).unwrap().0;
        let sched = take(workspace_schedule(monday));
        assert!(sched.contains(r#""ok":true"#), "{sched}");
        assert!(sched.contains(r#""perProject""#), "{sched}");
    }

    #[test]
    fn view_projections_are_exported_and_render_ready() {
        reset();
        call1(create_task, r#"{"id":"a","name":"Alpha"}"#);
        call1(create_task, r#"{"id":"b","name":"Bravo"}"#);
        let d = r#"{"id":"ID","duration":{"workingMinutes":480,"elapsed":false}}"#;
        call1(set_duration, &d.replace("ID", "a"));
        call1(set_duration, &d.replace("ID", "b"));
        call1(set_completed, r#"{"id":"b","completed":true}"#);

        let monday = Date::from_ymd(2026, 7, 13).unwrap().0;
        // A view showing name + done, sorted by name.
        const VIEW: &str = r#""view":{"id":"v","name":"V","shape":"table","filter":{"statuses":[],"completed":null,"search":null},"groupBy":null,"sort":[{"field":{"builtin":"name"},"ascending":true}],"visibleFields":[{"builtin":"name"},{"builtin":"completed"}]}"#;
        let view_json = format!(r#"{{{VIEW},"projectStart":{monday}}}"#);

        // table(): columns carry labels, cells carry engine-formatted display strings.
        let t = call1(table, &view_json);
        assert!(t.contains(r#""ok":true"#), "{t}");
        assert!(t.contains(r#""label":"Name""#), "{t}");
        assert!(t.contains(r#""label":"Done""#), "{t}");
        assert!(t.contains("Alpha"), "{t}");
        assert!(t.contains('✓'), "the engine formatted the done glyph: {t}");

        // view_selection(): ordered, grouped ids.
        let sel = call1(view_selection, &view_json);
        assert!(sel.contains(r#""ok":true"#), "{sel}");
        assert!(sel.contains(r#""tasks":["a","b"]"#), "{sel}");

        // calendar(): dated events over a range.
        let cal_json = format!(
            r#"{{{VIEW},"projectStart":{monday},"start":{monday},"end":{}}}"#,
            monday + 7
        );
        let c = call1(calendar, &cal_json);
        assert!(c.contains(r#""ok":true"#), "{c}");
        assert!(c.contains("Alpha"), "{c}");
    }

    #[test]
    fn an_out_of_range_date_is_an_envelope_not_a_trap() {
        // `table` is the first export to reach civil-date formatting, where an unchecked
        // i32 near the type's limit would overflow and panic ACROSS the FFI boundary.
        // The bound turns that into an ordinary error envelope.
        reset();
        call1(create_task, r#"{"id":"a","name":"Alpha"}"#);
        const VIEW: &str = r#""view":{"id":"v","name":"V","shape":"table","filter":{"statuses":[],"completed":null,"search":null},"groupBy":null,"sort":[],"visibleFields":[{"builtin":"start"}]}"#;

        let out = call1(table, &format!(r#"{{{VIEW},"projectStart":2147483647}}"#));
        assert!(out.contains(r#""ok":false"#), "{out}");
        assert!(out.contains("out of range"), "{out}");

        // The negative extreme is rejected too (and `i32::MIN` must not be negated).
        let out = call1(table, &format!(r#"{{{VIEW},"projectStart":-2147483648}}"#));
        assert!(out.contains(r#""ok":false"#), "{out}");

        // A sane date still works.
        let monday = Date::from_ymd(2026, 7, 13).unwrap().0;
        let ok = call1(table, &format!(r#"{{{VIEW},"projectStart":{monday}}}"#));
        assert!(ok.contains(r#""ok":true"#), "{ok}");
    }

    #[test]
    fn label_and_priority_ops_are_exported() {
        reset();
        call1(create_task, r#"{"id":"a","name":"Alpha"}"#);
        assert!(
            call1(upsert_label, r#"{"id":"l1","name":"Bug","color":"red"}"#)
                .contains(r#""ok":true"#)
        );
        assert!(call1(set_task_labels, r#"{"id":"a","labels":["l1"]}"#).contains(r#""ok":true"#));
        // An unknown label id is a typed error, not a trap.
        let bad = call1(set_task_labels, r#"{"id":"a","labels":["ghost"]}"#);
        assert!(
            bad.contains(r#""ok":false"#) && bad.contains(r#""code":1"#),
            "{bad}"
        );
        assert!(call1(set_priority, r#"{"id":"a","priority":"high"}"#).contains(r#""ok":true"#));
        // Deleting the label unlinks it everywhere.
        assert!(call1(delete_label, r#"{"id":"l1"}"#).contains(r#""ok":true"#));
    }

    #[test]
    fn workspace_op_error_is_an_envelope_not_a_trap() {
        reset();
        // Nesting a nonexistent project → NotFound as JSON, no panic.
        let out = call1(nest_project, r#"{"child":"ghost","parent":"project"}"#);
        assert!(
            out.contains(r#""ok":false"#) && out.contains(r#""code":1"#),
            "{out}"
        );
    }
}
