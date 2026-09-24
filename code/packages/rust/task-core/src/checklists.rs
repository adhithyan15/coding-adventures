//! Checklists — templates and runs (C1 of #14018; spec
//! `code/specs/task-app-checklists-v1.md`).
//!
//! The standalone Checklist app is being folded into Trestle. Its yes/no decision
//! node already existed here as [`Decision`]; what it had and the task model
//! lacked was the checklist *as a thing*, and the split between a **template**
//! (authored once) and its **runs** (worked through, one per occasion):
//!
//! ```text
//!   create_checklist_template ──▶ template ──instantiate_checklist──▶ run
//!                                  (items +          (deep copy:       │ tick, answer
//!                                   decisions)        unticked,        ▼
//!                                                     unanswered)  complete / abandon
//! ```
//!
//! A checklist names a subtree of ordinary tasks, so this module adds no second
//! item model: it walks the outline under a checklist's `root`, following a
//! decision into its answered branch only. That is the same walk
//! [`ProjectState::checklist`] has always done project-wide.
//!
//! Every op validates before it writes, like the rest of [`crate::ops`].

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::ops::OpError;
use crate::projections::ChecklistRow;
use crate::ProjectState;
use crate::{Checklist, ChecklistId, ChecklistRun, RunStatus, Task, TaskId, TaskKind};

/// Most items one checklist may hold. A run is a deep copy of its template, so an
/// unbounded template would let one call allocate without limit; real checklists
/// have tens of items.
pub const MAX_CHECKLIST_ITEMS: usize = 10_000;

/// Longest run id, in bytes. Every copied item is named `"{run}/{task}"`, so the
/// run id is repeated once per item; bounding it bounds what one instantiate
/// stores (a 1 MB id on a full template would otherwise be ~10 GB of keys).
pub const MAX_RUN_ID_BYTES: usize = 256;

/// How a checklist walk treats a decision's branches.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reveal {
    /// Only the answered branch — what someone working through a run sees.
    Answered,
    /// Both branches — what someone editing a template needs.
    Both,
}

impl ProjectState {
    // ── operations ───────────────────────────────────────────────────────────────

    /// Create an empty checklist template. Its `root` task is created here as a
    /// Summary named after the checklist; items are then added beneath it with
    /// `create_task(.., Some(parent))` and branch points with `set_decision`.
    pub fn create_checklist_template(
        &mut self,
        id: ChecklistId,
        root: TaskId,
        name: impl Into<String>,
        description: impl Into<String>,
        now: u64,
    ) -> Result<(), OpError> {
        if self.checklists.contains_key(&id) || self.tasks.contains_key(&root) {
            return Err(OpError::Duplicate);
        }
        let name = name.into();
        let mut task = Task::new(root.clone(), name.clone());
        task.kind = TaskKind::Summary;
        self.tasks.insert(root.clone(), task);
        self.checklists.insert(
            id.clone(),
            Checklist {
                id,
                name,
                description: description.into(),
                root,
                created_at: now,
                run: None,
            },
        );
        Ok(())
    }

    /// Start a run of `template`: a deep copy of its whole subtree, every item
    /// unticked and every decision unanswered, status `InProgress`.
    ///
    /// **Copied ids are derived, not minted:** the core never generates ids, so the
    /// copy of template task `t` is named `"{run}/{t}"`. That is deterministic and
    /// asks nothing more of the host. Any collision is rejected before anything is
    /// written. Dependencies and links between template items are not copied —
    /// a checklist is an ordered list, not a schedule.
    pub fn instantiate_checklist(
        &mut self,
        template: &ChecklistId,
        run: ChecklistId,
        now: u64,
    ) -> Result<(), OpError> {
        let tpl = self.checklists.get(template).ok_or(OpError::NotFound)?;
        if tpl.run.is_some() {
            return Err(OpError::Invalid("only a template can be instantiated"));
        }
        if self.checklists.contains_key(&run) {
            return Err(OpError::Duplicate);
        }
        if run.as_str().len() > MAX_RUN_ID_BYTES {
            return Err(OpError::Invalid("run id too long"));
        }
        if !self.tasks.contains_key(&tpl.root) {
            return Err(OpError::NotFound);
        }
        let subtree = self.subtree_of(&tpl.root);
        if subtree.len() > MAX_CHECKLIST_ITEMS {
            return Err(OpError::Invalid("checklist too large to instantiate"));
        }
        let map: HashMap<TaskId, TaskId> = subtree
            .iter()
            .map(|t| {
                (
                    t.clone(),
                    TaskId::from_raw(format!("{}/{}", run.as_str(), t.as_str())),
                )
            })
            .collect();
        if map.values().any(|new| self.tasks.contains_key(new)) {
            return Err(OpError::Duplicate);
        }
        let (name, description, root) =
            (tpl.name.clone(), tpl.description.clone(), tpl.root.clone());

        // All checks passed — now write.
        let remap = |id: &TaskId| map.get(id).cloned().unwrap_or_else(|| id.clone());
        for old in &subtree {
            let Some(src) = self.tasks.get(old) else {
                continue;
            };
            let mut copy = src.clone();
            copy.id = remap(old);
            copy.parent = if old == &root {
                None
            } else {
                src.parent.as_ref().map(remap)
            };
            copy.completed = false;
            copy.percent_complete = 0;
            copy.status = None;
            if let Some(d) = &mut copy.decision {
                d.answer = None;
                d.yes_children = d.yes_children.iter().map(remap).collect();
                d.no_children = d.no_children.iter().map(remap).collect();
            }
            self.tasks.insert(copy.id.clone(), copy);
        }
        self.checklists.insert(
            run.clone(),
            Checklist {
                id: run,
                name,
                description,
                root: remap(&root),
                created_at: now,
                run: Some(ChecklistRun {
                    template: template.clone(),
                    status: RunStatus::InProgress,
                    finished_at: None,
                }),
            },
        );
        Ok(())
    }

    /// Finish a run. **Rejected unless the run is complete** — every visible item
    /// ticked and every visible question answered (see [`ChecklistProgress`]). The
    /// standalone app enforced this only by disabling a button; the engine now
    /// enforces it for every host.
    pub fn complete_checklist_run(&mut self, id: &ChecklistId, now: u64) -> Result<(), OpError> {
        let progress = self.checklist_run(id).ok_or(OpError::NotFound)?.progress;
        self.finish_run(id, now, RunStatus::Completed, progress.complete)
    }

    /// Stop a run early.
    pub fn abandon_checklist_run(&mut self, id: &ChecklistId, now: u64) -> Result<(), OpError> {
        self.finish_run(id, now, RunStatus::Abandoned, true)
    }

    fn finish_run(
        &mut self,
        id: &ChecklistId,
        now: u64,
        status: RunStatus,
        allowed: bool,
    ) -> Result<(), OpError> {
        let checklist = self.checklists.get_mut(id).ok_or(OpError::NotFound)?;
        let run = checklist
            .run
            .as_mut()
            .ok_or(OpError::Invalid("a template has no run to finish"))?;
        if run.status != RunStatus::InProgress {
            return Err(OpError::Invalid("the run is already finished"));
        }
        if !allowed {
            return Err(OpError::Invalid(
                "every visible item must be ticked and every visible question answered",
            ));
        }
        run.status = status;
        run.finished_at = Some(now);
        Ok(())
    }

    /// Delete a checklist and its whole subtree. Runs of a deleted template
    /// survive — they are records of what happened.
    pub fn delete_checklist(&mut self, id: &ChecklistId) -> Result<(), OpError> {
        let root = self
            .checklists
            .get(id)
            .ok_or(OpError::NotFound)?
            .root
            .clone();
        let doomed: HashSet<TaskId> = self.subtree_of(&root).into_iter().collect();
        self.checklists.remove(id);
        self.tasks.retain(|tid, _| !doomed.contains(tid));
        self.dependencies
            .retain(|d| !doomed.contains(&d.predecessor) && !doomed.contains(&d.successor));
        self.links
            .retain(|l| !doomed.contains(&l.from) && !doomed.contains(&l.to));
        self.assignments.retain(|a| !doomed.contains(&a.task));
        for n in self.notes.values_mut() {
            if n.attached_task.as_ref().is_some_and(|t| doomed.contains(t)) {
                n.attached_task = None;
            }
        }
        for t in self.tasks.values_mut() {
            if let Some(d) = &mut t.decision {
                d.yes_children.retain(|c| !doomed.contains(c));
                d.no_children.retain(|c| !doomed.contains(c));
            }
        }
        Ok(())
    }

    // ── projections ──────────────────────────────────────────────────────────────

    /// Every template and run, for a checklist library: templates first, then runs,
    /// each newest first, ties broken by id.
    pub fn checklists(&self) -> Vec<ChecklistSummary> {
        // One outline index for every run: rebuilding it per run made the library
        // runs × tasks, which a few years of daily runs turns into seconds.
        let index = self.children_index();
        let mut out: Vec<ChecklistSummary> = self
            .checklists
            .values()
            .map(|c| ChecklistSummary {
                id: c.id.clone(),
                name: c.name.clone(),
                template: c.run.as_ref().map(|r| r.template.clone()),
                status: c.run.as_ref().map(|r| r.status),
                progress: c
                    .run
                    .as_ref()
                    .map(|_| progress(&self.walk(&index, &c.root, Reveal::Answered))),
                items: {
                    let mut subtree = HashSet::new();
                    collect_subtree(&index, &c.root, &mut subtree);
                    subtree.len().saturating_sub(1)
                },
                created_at: c.created_at,
                finished_at: c.run.as_ref().and_then(|r| r.finished_at),
            })
            .collect();
        out.sort_by(|a, b| {
            a.template
                .is_some()
                .cmp(&b.template.is_some())
                .then(b.created_at.cmp(&a.created_at))
                .then(a.id.cmp(&b.id))
        });
        out
    }

    /// What someone working through a run sees: the visible rows (a decision shows
    /// only its answered branch; nothing until answered), progress, and duration.
    /// `None` for an unknown id or a template.
    pub fn checklist_run(&self, id: &ChecklistId) -> Option<ChecklistRunView> {
        let c = self.checklists.get(id)?;
        let run = c.run.as_ref()?;
        let rows = self.walk(&self.children_index(), &c.root, Reveal::Answered);
        Some(ChecklistRunView {
            checklist: c.id.clone(),
            name: c.name.clone(),
            status: run.status,
            progress: progress(&rows),
            rows,
            duration_ms: run.finished_at.map(|f| f.saturating_sub(c.created_at)),
        })
    }

    /// What someone editing a template sees: every item, both branches of every
    /// decision, each row labelled with the branch it hangs from.
    pub fn checklist_outline(&self, id: &ChecklistId) -> Option<Vec<ChecklistOutlineRow>> {
        let c = self.checklists.get(id)?;
        let branch_of: HashMap<&TaskId, bool> = self
            .tasks
            .values()
            .filter_map(|t| t.decision.as_ref())
            .flat_map(|d| {
                d.yes_children
                    .iter()
                    .map(|c| (c, true))
                    .chain(d.no_children.iter().map(|c| (c, false)))
            })
            .collect();
        Some(
            self.walk(&self.children_index(), &c.root, Reveal::Both)
                .into_iter()
                .map(|r| ChecklistOutlineRow {
                    branch: branch_of.get(&r.task).copied(),
                    task: r.task,
                    name: r.name,
                    depth: r.depth,
                    is_decision: r.is_decision,
                })
                .collect(),
        )
    }

    // ── helpers shared with ops and the general views ────────────────────────────

    /// Every task that belongs to some checklist (roots included). The general
    /// views — List/Sheet/Calendar, todos, board, flowchart, the legacy
    /// project-wide checklist — leave these out, or a template's items would show
    /// up as open todos. Public so an app that lists tasks itself (Trestle's
    /// `task-mosaic-app`) can leave them out the same way.
    pub fn checklist_owned(&self) -> HashSet<TaskId> {
        if self.checklists.is_empty() {
            return HashSet::new();
        }
        let index = self.children_index();
        let mut owned = HashSet::new();
        for c in self.checklists.values() {
            collect_subtree(&index, &c.root, &mut owned);
        }
        owned
    }

    /// The checklist whose subtree contains `task`, found by walking up the
    /// parent chain to a checklist root (bounded, like every chain walk here).
    pub(crate) fn checklist_containing(&self, task: &TaskId) -> Option<&Checklist> {
        if self.checklists.is_empty() {
            return None;
        }
        // Two checklists sharing a root is malformed (only a hostile snapshot gets
        // there), and the answer decides what may be edited — so the most
        // restrictive owner wins: a template, then a finished run, then a live run.
        let strictness = |c: &Checklist| match &c.run {
            None => 0,
            Some(r) if r.status != RunStatus::InProgress => 1,
            Some(_) => 2,
        };
        let mut roots: BTreeMap<&TaskId, &Checklist> = BTreeMap::new();
        for c in self.checklists.values() {
            let slot = roots.entry(&c.root).or_insert(c);
            if strictness(c) < strictness(slot) {
                *slot = c;
            }
        }
        let mut cur = Some(task.clone());
        let mut guard = 0;
        while let Some(id) = cur {
            if let Some(c) = roots.get(&id) {
                return Some(c);
            }
            cur = self.tasks.get(&id).and_then(|t| t.parent.clone());
            guard += 1;
            if guard > self.tasks.len() {
                break;
            }
        }
        None
    }

    /// Reject changing an item of a template (ticked and answered only in runs) or
    /// of a finished run (a record). Items outside any checklist are unaffected.
    pub(crate) fn ensure_checklist_item_editable(&self, task: &TaskId) -> Result<(), OpError> {
        match self.checklist_containing(task) {
            None => Ok(()),
            Some(c) => match &c.run {
                None => Err(OpError::Invalid(
                    "templates are not ticked or answered; instantiate a run",
                )),
                Some(r) if r.status != RunStatus::InProgress => {
                    Err(OpError::Invalid("the run is finished"))
                }
                Some(_) => Ok(()),
            },
        }
    }

    /// Reject changing the *structure* of a finished run — adding, deleting or
    /// moving its items, or re-branching its decisions. A finished run is a record.
    pub(crate) fn ensure_checklist_structure_editable(&self, task: &TaskId) -> Result<(), OpError> {
        match self.checklist_containing(task).and_then(|c| c.run.as_ref()) {
            Some(r) if r.status != RunStatus::InProgress => {
                Err(OpError::Invalid("the run is finished"))
            }
            _ => Ok(()),
        }
    }

    /// The id of the checklist containing `task`, if any — for "does this move
    /// cross a checklist boundary" checks.
    pub(crate) fn checklist_id_of(&self, task: &TaskId) -> Option<ChecklistId> {
        self.checklist_containing(task).map(|c| c.id.clone())
    }

    /// Every checklist task mapped to its checklist, plus the set of roots — built
    /// once (O(tasks)) for ops that ask "which checklist?" about many tasks.
    /// On a shared root (malformed snapshot) the most restrictive owner wins, as
    /// in [`ProjectState::checklist_containing`].
    pub(crate) fn checklist_membership(&self) -> ChecklistMembership {
        let mut members = HashMap::new();
        let mut roots = HashSet::new();
        if self.checklists.is_empty() {
            return ChecklistMembership { members, roots };
        }
        let index = self.children_index();
        for c in self.checklists.values() {
            roots.insert(c.root.clone());
            let Some(owner) = self.checklist_containing(&c.root) else {
                continue;
            };
            let mut subtree = HashSet::new();
            collect_subtree(&index, &c.root, &mut subtree);
            for t in subtree {
                members.entry(t).or_insert_with(|| owner.id.clone());
            }
        }
        ChecklistMembership { members, roots }
    }

    /// Whether `task` is some checklist's root.
    pub(crate) fn is_checklist_root(&self, task: &TaskId) -> bool {
        self.checklists.values().any(|c| &c.root == task)
    }

    /// Outline children per parent, in outline order.
    pub(crate) fn children_index(&self) -> HashMap<TaskId, Vec<TaskId>> {
        let mut index: HashMap<TaskId, Vec<TaskId>> = HashMap::new();
        for t in self.tasks.values() {
            if let Some(p) = &t.parent {
                index.entry(p.clone()).or_default().push(t.id.clone());
            }
        }
        for kids in index.values_mut() {
            kids.sort_by_key(|id| {
                self.tasks
                    .get(id)
                    .map(|t| (t.order, t.id.0.clone()))
                    .unwrap_or((0, id.0.clone()))
            });
        }
        index
    }

    /// `root` and every task beneath it, in outline order. Branch children are
    /// outline children (the decision invariant), so this is the whole checklist.
    fn subtree_of(&self, root: &TaskId) -> Vec<TaskId> {
        let index = self.children_index();
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut stack = vec![root.clone()];
        while let Some(id) = stack.pop() {
            if !self.tasks.contains_key(&id) || !seen.insert(id.clone()) {
                continue;
            }
            if let Some(kids) = index.get(&id) {
                stack.extend(kids.iter().rev().cloned());
            }
            out.push(id);
        }
        out
    }

    /// The rows under `root` (root itself excluded), honouring decisions per
    /// `reveal`. Iterative with a visited set, like [`ProjectState::checklist`], so
    /// depth is heap-bounded and a malformed cycle cannot loop.
    fn walk(
        &self,
        index: &HashMap<TaskId, Vec<TaskId>>,
        root: &TaskId,
        reveal: Reveal,
    ) -> Vec<ChecklistRow> {
        let mut rows = Vec::new();
        let mut visited = HashSet::new();
        visited.insert(root.clone());
        let mut stack: Vec<(TaskId, u32)> = index
            .get(root)
            .map(|kids| kids.iter().rev().map(|k| (k.clone(), 0)).collect())
            .unwrap_or_default();
        while let Some((id, depth)) = stack.pop() {
            if !visited.insert(id.clone()) {
                continue;
            }
            let Some(t) = self.tasks.get(&id) else {
                continue;
            };
            rows.push(ChecklistRow {
                task: id.clone(),
                name: t.name.clone(),
                depth,
                completed: t.completed,
                is_decision: t.decision.is_some(),
                answered: t.decision.as_ref().and_then(|d| d.answer),
            });
            let children: Vec<TaskId> = match (&t.decision, reveal) {
                (Some(d), Reveal::Answered) => match d.answer {
                    Some(true) => d.yes_children.clone(),
                    Some(false) => d.no_children.clone(),
                    None => Vec::new(),
                },
                (Some(d), Reveal::Both) => d
                    .yes_children
                    .iter()
                    .chain(d.no_children.iter())
                    .cloned()
                    .collect(),
                (None, _) => index.get(&id).cloned().unwrap_or_default(),
            };
            for c in children.into_iter().rev() {
                stack.push((c, depth + 1));
            }
        }
        rows
    }
}

/// See [`ProjectState::checklist_membership`].
pub(crate) struct ChecklistMembership {
    members: HashMap<TaskId, ChecklistId>,
    /// Every checklist root.
    pub(crate) roots: HashSet<TaskId>,
}

impl ChecklistMembership {
    /// The checklist containing `task`, if any.
    pub(crate) fn get(&self, task: &TaskId) -> Option<&ChecklistId> {
        self.members.get(task)
    }
}

fn collect_subtree(index: &HashMap<TaskId, Vec<TaskId>>, root: &TaskId, out: &mut HashSet<TaskId>) {
    let mut stack = vec![root.clone()];
    while let Some(id) = stack.pop() {
        if out.insert(id.clone()) {
            if let Some(kids) = index.get(&id) {
                stack.extend(kids.iter().cloned());
            }
        }
    }
}

/// Progress over the **visible** rows only — an unanswered decision's branches
/// are not yet part of the run — exactly as the standalone app's `computeStats`.
fn progress(rows: &[ChecklistRow]) -> ChecklistProgress {
    let (mut total, mut checked, mut decisions, mut answered) = (0u32, 0u32, 0u32, 0u32);
    for r in rows {
        if r.is_decision {
            decisions += 1;
            answered += u32::from(r.answered.is_some());
        } else {
            total += 1;
            checked += u32::from(r.completed);
        }
    }
    let percent = if total == 0 {
        0
    } else {
        // checked <= total, so this is at most 100 and fits a u8.
        (u64::from(checked) * 100 / u64::from(total)) as u8
    };
    ChecklistProgress {
        total,
        checked,
        decisions,
        answered,
        percent,
        complete: (total == 0 || checked == total) && answered == decisions,
    }
}

// ── output types ─────────────────────────────────────────────────────────────────

/// How far through a run is, over its visible items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ChecklistProgress {
    /// Visible check items.
    pub total: u32,
    /// Of those, ticked.
    pub checked: u32,
    /// Visible decisions.
    pub decisions: u32,
    /// Of those, answered.
    pub answered: u32,
    /// `checked / total` as a whole percent; 0 when there are no check items.
    pub percent: u8,
    /// Every visible item ticked and every visible question answered — the only
    /// state `complete_checklist_run` accepts.
    pub complete: bool,
}

/// One run, as someone working through it sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ChecklistRunView {
    /// The run.
    pub checklist: ChecklistId,
    /// Its name (the template's, at the time it was run).
    pub name: String,
    /// Where it is in its life.
    pub status: RunStatus,
    /// The visible rows, in order.
    pub rows: Vec<ChecklistRow>,
    /// Progress over those rows.
    pub progress: ChecklistProgress,
    /// Milliseconds from start to finish, once finished.
    pub duration_ms: Option<u64>,
}

/// One entry in the checklist library.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ChecklistSummary {
    /// The checklist.
    pub id: ChecklistId,
    /// Its name.
    pub name: String,
    /// For a run, the template it came from; `None` for a template.
    pub template: Option<ChecklistId>,
    /// For a run, its status.
    pub status: Option<RunStatus>,
    /// For a run, its progress.
    pub progress: Option<ChecklistProgress>,
    /// How many items it holds, every branch included (the root is not an
    /// item). Counted from the same one-pass index, so a library of many
    /// templates never re-walks the project per template.
    pub items: usize,
    /// When it was created.
    pub created_at: u64,
    /// When a run finished.
    pub finished_at: Option<u64>,
}

/// One row of a template's outline: every item, both branches shown.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ChecklistOutlineRow {
    /// The task.
    pub task: TaskId,
    /// Its name.
    pub name: String,
    /// Indentation depth under the checklist root.
    pub depth: u32,
    /// Whether it is a yes/no decision.
    pub is_decision: bool,
    /// `Some(true)` if it hangs from a yes branch, `Some(false)` a no branch.
    pub branch: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Decision, ProjectId};

    fn t(s: &str) -> TaskId {
        TaskId::from_raw(s)
    }
    fn c(s: &str) -> ChecklistId {
        ChecklistId::from_raw(s)
    }

    /// "Pre-flight": Fuel · Passengers? (yes: Brief · Seatbelts / no: Stow seats) · Doors
    fn preflight() -> ProjectState {
        let mut p = ProjectState::empty(ProjectId::from_raw("p"));
        p.create_task(t("loose"), "Unrelated work", None).unwrap();
        p.create_checklist_template(c("tpl"), t("root"), "Pre-flight", "before takeoff", 1)
            .unwrap();
        for (id, name, parent) in [
            ("fuel", "Fuel checked", "root"),
            ("pax", "Passengers?", "root"),
            ("brief", "Brief passengers", "pax"),
            ("belts", "Seatbelts on", "pax"),
            ("stow", "Stow seats", "pax"),
            ("doors", "Doors closed", "root"),
        ] {
            p.create_task(t(id), name, Some(t(parent))).unwrap();
        }
        for (i, id) in ["fuel", "pax", "doors"].iter().enumerate() {
            p.set_order(&t(id), i as i64).unwrap();
        }
        p.set_decision(
            &t("pax"),
            Some(Decision {
                question: "Passengers?".into(),
                answer: None,
                yes_children: vec![t("brief"), t("belts")],
                no_children: vec![t("stow")],
            }),
        )
        .unwrap();
        p
    }

    fn names(rows: &[ChecklistRow]) -> Vec<&str> {
        rows.iter().map(|r| r.name.as_str()).collect()
    }

    #[test]
    fn a_run_is_a_deep_independent_copy() {
        let mut p = preflight();
        p.instantiate_checklist(&c("tpl"), c("r1"), 10).unwrap();
        p.answer_decision(&t("r1/pax"), true).unwrap();
        p.set_completed(&t("r1/fuel"), true).unwrap();
        // The template is untouched.
        let tpl = p.tasks.get(&t("pax")).unwrap().decision.as_ref().unwrap();
        assert_eq!(tpl.answer, None);
        assert!(!p.tasks[&t("fuel")].completed);
        // The copy's branches point at copies, not at template tasks.
        let run = p.tasks[&t("r1/pax")].decision.as_ref().unwrap();
        assert_eq!(run.yes_children, vec![t("r1/brief"), t("r1/belts")]);
        assert_eq!(p.tasks[&t("r1/brief")].parent, Some(t("r1/pax")));
        assert_eq!(p.checklists[&c("r1")].root, t("r1/root"));
        assert_eq!(p.checklists[&c("r1")].created_at, 10);
    }

    #[test]
    fn an_unanswered_decision_hides_both_branches_and_later_items_still_show() {
        let mut p = preflight();
        p.instantiate_checklist(&c("tpl"), c("r1"), 10).unwrap();
        let view = p.checklist_run(&c("r1")).unwrap();
        assert_eq!(
            names(&view.rows),
            ["Fuel checked", "Passengers?", "Doors closed"]
        );
        p.answer_decision(&t("r1/pax"), false).unwrap();
        let view = p.checklist_run(&c("r1")).unwrap();
        assert_eq!(
            names(&view.rows),
            ["Fuel checked", "Passengers?", "Stow seats", "Doors closed"]
        );
        assert_eq!(view.rows[2].depth, 1);
    }

    #[test]
    fn switching_an_answer_keeps_hidden_branch_state() {
        let mut p = preflight();
        p.instantiate_checklist(&c("tpl"), c("r1"), 10).unwrap();
        p.answer_decision(&t("r1/pax"), true).unwrap();
        p.set_completed(&t("r1/brief"), true).unwrap();
        p.answer_decision(&t("r1/pax"), false).unwrap();
        p.answer_decision(&t("r1/pax"), true).unwrap();
        assert!(
            p.tasks[&t("r1/brief")].completed,
            "switching back restores the work"
        );
        p.clear_decision_answer(&t("r1/pax")).unwrap();
        assert_eq!(p.checklist_run(&c("r1")).unwrap().rows.len(), 3);
    }

    #[test]
    fn progress_counts_visible_items_and_gates_completion() {
        let mut p = preflight();
        p.instantiate_checklist(&c("tpl"), c("r1"), 10).unwrap();
        let pr = p.checklist_run(&c("r1")).unwrap().progress;
        assert_eq!(
            (pr.total, pr.checked, pr.decisions, pr.answered),
            (2, 0, 1, 0)
        );
        assert!(!pr.complete);
        assert_eq!(
            p.complete_checklist_run(&c("r1"), 20),
            Err(OpError::Invalid(
                "every visible item must be ticked and every visible question answered"
            ))
        );

        p.answer_decision(&t("r1/pax"), true).unwrap();
        for id in ["r1/fuel", "r1/brief", "r1/belts", "r1/doors"] {
            p.set_completed(&t(id), true).unwrap();
        }
        let pr = p.checklist_run(&c("r1")).unwrap().progress;
        assert_eq!((pr.total, pr.checked, pr.percent), (4, 4, 100));
        assert!(pr.complete, "stow seats is hidden, so it does not count");

        p.complete_checklist_run(&c("r1"), 70).unwrap();
        let view = p.checklist_run(&c("r1")).unwrap();
        assert_eq!(view.status, RunStatus::Completed);
        assert_eq!(view.duration_ms, Some(60));
    }

    #[test]
    fn percent_is_zero_with_no_check_items_and_a_decisions_only_run_can_complete() {
        let mut p = ProjectState::empty(ProjectId::from_raw("p"));
        p.create_checklist_template(c("tpl"), t("root"), "Q", "", 0)
            .unwrap();
        p.create_task(t("q"), "Ready?", Some(t("root"))).unwrap();
        p.set_decision(
            &t("q"),
            Some(Decision {
                question: "Ready?".into(),
                answer: None,
                yes_children: vec![],
                no_children: vec![],
            }),
        )
        .unwrap();
        p.instantiate_checklist(&c("tpl"), c("r"), 0).unwrap();
        assert_eq!(p.checklist_run(&c("r")).unwrap().progress.percent, 0);
        p.answer_decision(&t("r/q"), true).unwrap();
        p.complete_checklist_run(&c("r"), 1).unwrap();
    }

    #[test]
    fn templates_are_not_ticked_or_answered_and_finished_runs_are_read_only() {
        let mut p = preflight();
        let tpl_err = Err(OpError::Invalid(
            "templates are not ticked or answered; instantiate a run",
        ));
        assert_eq!(p.answer_decision(&t("pax"), true), tpl_err);
        assert_eq!(p.set_completed(&t("fuel"), true), tpl_err);
        let mut d = p.tasks[&t("pax")].decision.clone().unwrap();
        d.answer = Some(true);
        assert_eq!(
            p.set_decision(&t("pax"), Some(d)),
            Err(OpError::Invalid(
                "templates are not answered; instantiate a run"
            ))
        );

        p.instantiate_checklist(&c("tpl"), c("r1"), 10).unwrap();
        p.abandon_checklist_run(&c("r1"), 11).unwrap();
        assert_eq!(
            p.set_completed(&t("r1/fuel"), true),
            Err(OpError::Invalid("the run is finished"))
        );
        assert_eq!(
            p.abandon_checklist_run(&c("r1"), 12),
            Err(OpError::Invalid("the run is already finished"))
        );
        // Ordinary tasks are unaffected.
        p.set_completed(&t("loose"), true).unwrap();
    }

    #[test]
    fn instantiate_guards() {
        let mut p = preflight();
        p.instantiate_checklist(&c("tpl"), c("r1"), 10).unwrap();
        assert_eq!(
            p.instantiate_checklist(&c("tpl"), c("r1"), 11),
            Err(OpError::Duplicate)
        );
        assert_eq!(
            p.instantiate_checklist(&c("r1"), c("r2"), 11),
            Err(OpError::Invalid("only a template can be instantiated"))
        );
        assert_eq!(
            p.instantiate_checklist(&c("nope"), c("r2"), 11),
            Err(OpError::NotFound)
        );
        // A derived id that already exists is rejected before anything is written.
        p.create_task(t("r3/fuel"), "squatter", None).unwrap();
        let before = p.clone();
        assert_eq!(
            p.instantiate_checklist(&c("tpl"), c("r3"), 12),
            Err(OpError::Duplicate)
        );
        assert_eq!(p, before);
        assert_eq!(
            p.create_checklist_template(c("tpl"), t("x"), "dup", "", 0),
            Err(OpError::Duplicate)
        );
    }

    #[test]
    fn checklist_items_stay_out_of_the_general_views() {
        let mut p = preflight();
        p.instantiate_checklist(&c("tpl"), c("r1"), 10).unwrap();
        let todos: Vec<String> = p.todos().into_iter().map(|r| r.task.0).collect();
        assert_eq!(todos, ["loose"]);
        let legacy: Vec<String> = p.checklist().into_iter().map(|r| r.task.0).collect();
        assert_eq!(legacy, ["loose"]);
        assert!(p.flowchart().nodes.iter().all(|n| n.task == t("loose")));
    }

    #[test]
    fn library_and_outline() {
        let mut p = preflight();
        p.instantiate_checklist(&c("tpl"), c("r1"), 10).unwrap();
        p.instantiate_checklist(&c("tpl"), c("r2"), 20).unwrap();
        let lib: Vec<(String, Option<RunStatus>)> = p
            .checklists()
            .into_iter()
            .map(|s| (s.id.0, s.status))
            .collect();
        assert_eq!(
            lib,
            [
                ("tpl".to_string(), None),
                ("r2".to_string(), Some(RunStatus::InProgress)),
                ("r1".to_string(), Some(RunStatus::InProgress)),
            ]
        );
        let outline = p.checklist_outline(&c("tpl")).unwrap();
        // `items` counts every item, both branches, never the root: the
        // template's outline length, and the same for each copy of it.
        for summary in p.checklists() {
            assert_eq!(summary.items, outline.len(), "{}", summary.id.0);
        }
        let shown: Vec<(&str, Option<bool>)> = outline
            .iter()
            .map(|r| (r.name.as_str(), r.branch))
            .collect();
        assert_eq!(
            shown,
            [
                ("Fuel checked", None),
                ("Passengers?", None),
                ("Brief passengers", Some(true)),
                ("Seatbelts on", Some(true)),
                ("Stow seats", Some(false)),
                ("Doors closed", None),
            ]
        );
        assert!(
            p.checklist_run(&c("tpl")).is_none(),
            "a template has no run view"
        );
    }

    #[test]
    fn deleting_a_template_keeps_its_runs_and_removes_only_its_own_tasks() {
        let mut p = preflight();
        p.instantiate_checklist(&c("tpl"), c("r1"), 10).unwrap();
        assert_eq!(
            p.delete_task(&t("root")),
            Err(OpError::Invalid("delete the checklist, not its root task"))
        );
        p.delete_checklist(&c("tpl")).unwrap();
        assert!(!p.tasks.contains_key(&t("fuel")));
        assert!(p.tasks.contains_key(&t("r1/fuel")));
        assert!(p.tasks.contains_key(&t("loose")));
        assert_eq!(
            p.checklists[&c("r1")].run.as_ref().unwrap().template,
            c("tpl")
        );
        assert!(
            p.checklist_run(&c("r1")).is_some(),
            "a run of a deleted template survives"
        );
        assert_eq!(p.delete_checklist(&c("tpl")), Err(OpError::NotFound));
    }

    #[test]
    fn set_decision_enforces_the_invariant() {
        let mut p = ProjectState::empty(ProjectId::from_raw("p"));
        for id in ["d", "a", "b", "x", "other"] {
            p.create_task(t(id), id, None).unwrap();
        }
        let dec = |yes: Vec<TaskId>, no: Vec<TaskId>| {
            Some(Decision {
                question: "?".into(),
                answer: None,
                yes_children: yes,
                no_children: no,
            })
        };
        let before = p.clone();
        assert_eq!(
            p.set_decision(&t("d"), dec(vec![t("zz")], vec![])),
            Err(OpError::NotFound)
        );
        assert_eq!(
            p.set_decision(&t("d"), dec(vec![t("d")], vec![])),
            Err(OpError::WouldCycle)
        );
        assert_eq!(
            p.set_decision(&t("d"), dec(vec![t("a")], vec![t("a")])),
            Err(OpError::Invalid("a branch child is listed twice"))
        );
        assert_eq!(p, before, "rejections write nothing");

        // Listed children are reparented under the decision.
        p.set_decision(&t("d"), dec(vec![t("a")], vec![t("b")]))
            .unwrap();
        assert_eq!(p.tasks[&t("a")].parent, Some(t("d")));
        // Another decision cannot claim them; an ancestor cannot be a branch child.
        assert_eq!(
            p.set_decision(&t("other"), dec(vec![t("a")], vec![])),
            Err(OpError::Invalid(
                "a branch child already belongs to another decision"
            ))
        );
        assert_eq!(
            p.set_decision(&t("a"), dec(vec![t("d")], vec![])),
            Err(OpError::WouldCycle)
        );
        // An outline child outside both branches is refused.
        p.create_task(t("stray"), "stray", Some(t("d"))).unwrap();
        assert_eq!(
            p.set_decision(&t("d"), dec(vec![t("a")], vec![t("b")])),
            Err(OpError::Invalid(
                "every outline child of a decision must be in its yes or no branch"
            ))
        );
    }

    #[test]
    fn deleting_a_branch_child_removes_its_reference() {
        let mut p = preflight();
        p.delete_task(&t("brief")).unwrap();
        let d = p.tasks[&t("pax")].decision.as_ref().unwrap();
        assert_eq!(d.yes_children, vec![t("belts")]);
    }

    #[test]
    fn finished_runs_are_frozen_through_every_op() {
        // Security review: set_decision, set_status and the structural ops used to
        // bypass the finished-run rule that set_completed/answer_decision enforced.
        let mut p = preflight();
        p.instantiate_checklist(&c("tpl"), c("r"), 1).unwrap();
        p.abandon_checklist_run(&c("r"), 2).unwrap();
        let finished = Err(OpError::Invalid("the run is finished"));
        let mut d = p.tasks[&t("r/pax")].decision.clone().unwrap();
        d.answer = Some(true);
        assert_eq!(p.set_decision(&t("r/pax"), Some(d)), finished);
        assert_eq!(p.create_task(t("new"), "x", Some(t("r/root"))), finished);
        assert_eq!(p.delete_task(&t("r/fuel")), finished);
        assert_eq!(p.reparent(&t("r/doors"), Some(t("r/pax"))), finished);
        assert_eq!(
            p.set_decision(&t("r/pax"), None),
            finished,
            "clearing edits the record too"
        );
        assert_eq!(
            p.set_status(&t("r/fuel"), Some(crate::StatusId::from_raw("done"))),
            finished
        );
    }

    #[test]
    fn templates_cannot_be_ticked_through_a_status() {
        let mut p = preflight();
        assert_eq!(
            p.set_status(&t("fuel"), Some(crate::StatusId::from_raw("done"))),
            Err(OpError::Invalid(
                "templates are not ticked or answered; instantiate a run"
            ))
        );
        // The default workflow's backfill skips checklist items.
        p.ensure_default_workflow().unwrap();
        assert!(p.tasks[&t("fuel")].status.is_none());
        assert!(!p.tasks[&t("fuel")].completed);
        assert!(p.tasks[&t("loose")].status.is_some());
    }

    #[test]
    fn nothing_crosses_a_checklist_boundary_and_roots_stay_put() {
        let mut p = preflight();
        p.instantiate_checklist(&c("tpl"), c("r"), 1).unwrap();
        p.answer_decision(&t("r/pax"), true).unwrap();
        let boundary = Err(OpError::Invalid(
            "items cannot move across a checklist boundary",
        ));
        // An answered run decision cannot be moved into a template…
        assert_eq!(p.reparent(&t("r/pax"), Some(t("root"))), boundary);
        // …an ordinary task cannot slip into one, nor an item out of one…
        assert_eq!(p.reparent(&t("loose"), Some(t("root"))), boundary);
        assert_eq!(p.reparent(&t("fuel"), None), boundary);
        // …and a root never moves (reparenting a run's root under its template
        // doubled the template on every instantiate).
        assert_eq!(
            p.reparent(&t("r/root"), Some(t("root"))),
            Err(OpError::Invalid("a checklist's root cannot be moved"))
        );
        // A decision cannot claim a branch child from another checklist.
        let mut d = p.tasks[&t("r/pax")].decision.clone().unwrap();
        d.no_children.push(t("fuel"));
        assert_eq!(
            p.set_decision(&t("r/pax"), Some(d)),
            Err(OpError::Invalid(
                "branch children must belong to the decision's checklist"
            ))
        );
        // Moving within one checklist is fine.
        p.reparent(&t("doors"), Some(t("fuel"))).unwrap();
    }

    #[test]
    fn a_run_id_is_bounded() {
        let mut p = preflight();
        let long = "r".repeat(MAX_RUN_ID_BYTES + 1);
        assert_eq!(
            p.instantiate_checklist(&c("tpl"), c(&long), 1),
            Err(OpError::Invalid("run id too long"))
        );
        p.instantiate_checklist(&c("tpl"), c(&"r".repeat(MAX_RUN_ID_BYTES)), 1)
            .unwrap();
    }

    #[test]
    fn instantiating_a_template_with_a_missing_root_is_not_found() {
        let mut p = preflight();
        p.checklists.get_mut(&c("tpl")).unwrap().root = t("ghost");
        assert_eq!(
            p.instantiate_checklist(&c("tpl"), c("r"), 1),
            Err(OpError::NotFound)
        );
    }

    #[test]
    fn a_shared_root_is_governed_by_its_strictest_owner() {
        // Malformed (hostile-snapshot) state: a live run claiming the template's
        // root must not unlock the template's items.
        let mut p = preflight();
        let mut rogue = p.checklists[&c("tpl")].clone();
        rogue.id = c("zz");
        rogue.run = Some(ChecklistRun {
            template: c("tpl"),
            status: RunStatus::InProgress,
            finished_at: None,
        });
        p.checklists.insert(c("zz"), rogue);
        assert!(p.set_completed(&t("fuel"), true).is_err());
    }

    #[test]
    fn the_library_scales_with_runs() {
        // Security review: the library walked once per run and rebuilt the outline
        // index each time (runs × tasks). 300 runs of this template must be quick.
        let mut p = preflight();
        for i in 0..300 {
            p.instantiate_checklist(&c("tpl"), c(&format!("r{i}")), i)
                .unwrap();
        }
        let start = std::time::Instant::now();
        assert_eq!(p.checklists().len(), 301);
        assert!(
            start.elapsed() < std::time::Duration::from_secs(2),
            "{:?}",
            start.elapsed()
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn a_pre_checklist_snapshot_still_loads() {
        let p = ProjectState::empty(ProjectId::from_raw("p"));
        let mut v = serde_json::to_value(&p).unwrap();
        v.as_object_mut().unwrap().remove("checklists");
        let back: ProjectState = serde_json::from_value(v).unwrap();
        assert!(back.checklists.is_empty());
    }
}
