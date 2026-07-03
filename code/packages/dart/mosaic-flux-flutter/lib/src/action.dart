// action.dart — the MosaicAction Command Pattern interface.
//
// Per UI33-rewrite §5: each action is a class carrying its payload
// as constructor fields plus an `apply(state) → state` method that
// expresses the state transform. The dispatcher routes by calling
// `action.apply(state)` directly — no central switch, no reducer
// registry.
//
// Example:
//
//   class Navigate extends MosaicAction<GridState> {
//     final int row;
//     final int col;
//     Navigate(this.row, this.col);
//     @override
//     GridState apply(GridState state) => state.copyWith(
//       editRow: -1, editCol: -1, editContent: '',
//       selectedRow: row, selectedCol: col,
//     );
//   }
//
// `apply` MUST be pure. In Dart that means: don't mutate input
// (return a new instance via copyWith or similar), and be
// deterministic — same state + action ⇒ same next state. Side
// effects belong in middleware.

/// The Command Pattern interface every Mosaic action implements.
abstract class MosaicAction<S> {
  const MosaicAction();

  /// Pure state transform. Returns the next state given the current
  /// one. Must not mutate input. Must be deterministic.
  S apply(S state);
}
