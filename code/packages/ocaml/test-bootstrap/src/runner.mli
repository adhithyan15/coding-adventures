(** The honest runner (VER05 7.1, VER05e 5, VER05q 7). *)

val check :
  ?parent:Budget.t ->
  ?shrink_override:'a Gen.shrinker ->
  seed:int64 ->
  'a Gen.t ->
  Types.options ->
  ('a -> bool) ->
  Types.verdict
(** Runs [num_tests] cases from [SplitMix64(seed)]. [parent] defaults to the
    ambient scope of the calling domain ({!Budget.current}), so a [check] called
    from inside a property is nested automatically. Always returns a verdict;
    invalid options raise [Error.Argument_error] instead. *)

val val_work : Val.t -> int
(** An approximate size of a value in elements (work). *)
