// Action.cs — the IMosaicAction Command Pattern interface.
//
// Per UI33-rewrite §5: each action is a type (typically a `sealed
// record` carrying its payload as positional or init-only
// properties) implementing IMosaicAction<TState> with an Apply
// method that expresses the state transform.  The dispatcher routes
// by calling action.Apply(state) directly — no central switch, no
// reducer registry.
//
// Example:
//
//     public sealed record Navigate(int Row, int Col) : IMosaicAction<GridState>
//     {
//         public GridState Apply(GridState state) => state with
//         {
//             EditRow = -1, EditCol = -1, EditContent = "",
//             SelectedRow = Row, SelectedCol = Col,
//         };
//     }
//
// `Apply` MUST be pure: it must not mutate the input (use C# `with`
// expressions on records to return new instances), and it must be
// deterministic — same state + same action ⇒ same next state.
// Side effects belong in middleware, not in Apply.

namespace Mosaic.Flux;

/// <summary>
/// The Command Pattern interface every Mosaic action implements.
/// Implementations are typically <c>sealed record</c> types whose
/// constructor parameters carry the action's payload.
/// </summary>
/// <typeparam name="TState">The state type the action transforms.</typeparam>
public interface IMosaicAction<TState>
{
    /// <summary>
    /// Pure state transform.  Returns the next state given the
    /// current one.  Must not mutate input.  Must be deterministic.
    /// </summary>
    TState Apply(TState state);
}
