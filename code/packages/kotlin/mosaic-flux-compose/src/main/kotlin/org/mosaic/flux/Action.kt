// Action.kt — the MosaicAction Command Pattern interface.
//
// Per UI33-rewrite §5: each action is a class (typically a data class
// carrying its payload as constructor parameters) with an apply()
// method that expresses the state transform.  The dispatcher routes
// by calling action.apply(state) directly — there is no central
// switch statement or reducer registry.
//
// Example:
//
//   data class Navigate(val row: Int, val col: Int) : MosaicAction<GridState> {
//       override fun apply(state: GridState): GridState = state.copy(
//           editRow = -1, editCol = -1, editContent = "",
//           selectedRow = row, selectedCol = col
//       )
//   }
//
// apply() MUST be pure: it must not mutate the input (Kotlin data
// classes' copy() method makes this easy), and it must be
// deterministic — same state + same action = same next state.
// Side effects belong in middleware, not in apply().

package org.mosaic.flux

/**
 * The Command Pattern interface every Mosaic action implements.
 * Conforming types are typically `data class` declarations whose
 * constructor parameters carry the action's payload.
 */
interface MosaicAction<S> {
    /**
     * Pure state transform.  Returns the next state given the
     * current one.  Must not mutate input.  Must be deterministic.
     */
    fun apply(state: S): S
}
