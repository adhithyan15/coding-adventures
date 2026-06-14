// Action.swift — the MosaicAction Command Pattern protocol.
//
// Per UI33-rewrite §5: each action is a value (struct) or class
// carrying its payload as stored properties, plus an `apply(to:)`
// method that expresses the state transform.  The dispatcher routes
// by calling `action.apply(to: state)` — there is no central switch
// statement or reducer registry.
//
// We use a protocol with a primary associated type (Swift 5.7+) so
// callers can write `any MosaicAction<GridState>` when type-erasing
// — this is how middleware sees the action without losing the state
// generic.

/// The Command Pattern interface every Mosaic action implements.
///
/// Conforming types are typically structs whose stored properties
/// ARE the action's payload.  `apply(to:)` is a pure function that
/// returns the next state given the current one.
///
/// Example:
///
///     struct Navigate: MosaicAction {
///         typealias State = GridState
///         let row: Int
///         let col: Int
///         func apply(to state: GridState) -> GridState {
///             var s = state
///             s.editRow = -1
///             s.editCol = -1
///             s.editContent = ""
///             s.selectedRow = row
///             s.selectedCol = col
///             return s
///         }
///     }
///
/// `apply` must be pure: it must not mutate the input (Swift value-
/// type semantics make this easy — receiving a copy is the default),
/// and it must be deterministic — the same `state` + same action
/// instance must produce the same next state.  Side effects belong
/// in middleware, not in `apply`.
public protocol MosaicAction<State> {
    associatedtype State

    /// Pure state transform.  Given the current state, return the
    /// next state.  Must not mutate input.  Must be deterministic.
    func apply(to state: State) -> State
}
