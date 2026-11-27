(* The budget tree (VER05q section 5, ported from VER05e 4.1).

   A property may itself call [check], a custom shrinker may call [Gen.shrink],
   a filter predicate may do either.  If each got a fresh budget the work would
   multiply with the nesting depth.  Here every public entry point that runs
   user code opens a CHILD of the scope it is given, with its own local cap, and
   a charge walks from the node to the outermost root: every node on the path
   must have room, then every counter is incremented, all or nothing.

   One mutex, owned by the root and shared by every node of the tree, guards all
   counters.  It is held only around integer arithmetic: no user code runs under
   it, and nothing inside can raise, so a charge can never strand a counter half
   updated.  A scope is closed with Fun.protect, so it closes however its body
   is left (Sys.Break included). *)

type t = {
  parent : t option;
  depth : int;
  checks : int; (* enclosing checks, this one included *)
  max_attempts : int;
  max_work : int;
  mutable attempts : int;
  mutable work : int;
  mutable closed : bool;
  refused : bool Atomic.t; (* shared by the whole tree *)
  lock : Mutex.t; (* shared by the whole tree *)
  config : Types.config;
}

let new_node config parent ~max_attempts ~max_work ~checks =
  {
    parent;
    depth = (match parent with None -> 0 | Some p -> p.depth + 1);
    checks;
    max_attempts;
    max_work;
    attempts = 0;
    work = 0;
    closed = false;
    refused =
      (match parent with None -> Atomic.make false | Some p -> p.refused);
    lock = (match parent with None -> Mutex.create () | Some p -> p.lock);
    config = (match parent with None -> config | Some p -> p.config);
  }

(* The ambient scope.  split_from_parent is the identity: a domain spawned inside
   a scope starts with that node (like Java's InheritableThreadLocal). *)
let ambient : t option Domain.DLS.key =
  Domain.DLS.new_key ~split_from_parent:(fun x -> x) (fun () -> None)

let is_spent b =
  Mutex.protect b.lock (fun () ->
      let rec any n =
        n.closed || match n.parent with None -> false | Some p -> any p
      in
      any b)

(* The first node of [n]'s chain whose whole chain is open, else None. *)
let rec skip_closed = function
  | None -> None
  | Some n -> if is_spent n then skip_closed n.parent else Some n

(* Opens an owning scope for the duration of the body: it becomes ambient, and
   is closed and un-ambient'd in [finally]. *)
let owned node body =
  let saved = Domain.DLS.get ambient in
  Domain.DLS.set ambient (Some node);
  Fun.protect
    ~finally:(fun () ->
      (* restore only if this scope is still the current one: with interleaved
         scopes (systhreads, effect fibres) another scope may have replaced it,
         and writing a stale [saved] back would resurrect a closed node *)
      (match Domain.DLS.get ambient with
      | Some c when c == node ->
          (* sanitised: never write a closed node back as the ambient *)
          Domain.DLS.set ambient (skip_closed saved)
      | _ -> ());
      Mutex.protect node.lock (fun () -> node.closed <- true))
    (fun () -> body node)

let with_root config ~max_attempts body =
  owned
    (new_node config None ~max_attempts:(max 0 max_attempts)
       ~max_work:config.Types.max_work ~checks:0)
    body

let with_child config parent ~max_steps body =
  match parent with
  | None ->
      owned
        (new_node config None
           ~max_attempts:(Types.attempt_budget max_steps)
           ~max_work:config.Types.max_work ~checks:0)
        body
  | Some p when p.depth + 1 >= Types.max_budget_depth -> body p
  | Some p ->
      owned
        (new_node config (Some p)
           ~max_attempts:(Types.attempt_budget max_steps)
           ~max_work:Types.max_shrink_work ~checks:p.checks)
        body

let mark_refused b = Atomic.set b.refused true

let with_check config parent ~num_tests ~max_steps body =
  let cap = num_tests + Types.attempt_budget max_steps in
  match parent with
  | None ->
      Some
        (owned
           (new_node config None ~max_attempts:cap
              ~max_work:config.Types.max_work ~checks:1)
           body)
  | Some p
    when p.checks >= Types.max_check_nesting
         || p.depth + 1 >= Types.max_budget_depth ->
      mark_refused p;
      None
  | Some p ->
      Some
        (owned
           (new_node config (Some p) ~max_attempts:cap
              ~max_work:Types.max_shrink_work ~checks:(p.checks + 1))
           body)

(* Every node on the path is open, under its attempt cap, and can afford the
   cost.  The affordability test is [cost <= max - used]: used <= max always, so
   the subtraction can neither go negative nor wrap, and max_int is refused. *)
let rec path_has_room n cost =
  (not n.closed)
  && n.attempts < n.max_attempts
  && cost <= n.max_work - n.work
  && match n.parent with None -> true | Some p -> path_has_room p cost

let rec bump n cost attempt_cost =
  n.attempts <- n.attempts + attempt_cost;
  n.work <- n.work + cost;
  match n.parent with None -> () | Some p -> bump p cost attempt_cost

let take node elements attempt_cost =
  let cost = max 1 elements in
  let ok =
    Mutex.protect node.lock (fun () ->
        if path_has_room node cost then (
          bump node cost attempt_cost;
          true)
        else false)
  in
  if not ok then mark_refused node;
  ok

let charge b elements = take b elements 1
let charge_copy b elements = take b elements 0
let room b = Mutex.protect b.lock (fun () -> path_has_room b 1)
let attempts b = Mutex.protect b.lock (fun () -> b.attempts)
let work b = Mutex.protect b.lock (fun () -> b.work)
let is_closed b = Mutex.protect b.lock (fun () -> b.closed)

(* The ambient scope of the calling domain.  It is PER DOMAIN, not per thread or
   fibre: concurrent checks in different threads or fibres of one domain must
   pass [?parent] explicitly (otherwise they may be charged to another check's
   tree and, if it closes first, get property-exhausted: fail-closed). *)
let current () =
  match Domain.DLS.get ambient with
  | None -> None
  | Some n as amb -> (
      (* a closed child with an open ancestor resolves to that ancestor; if the
         WHOLE chain is closed (only possible for an ambient inherited by a
         domain that outlived its scope) the spent node itself is returned, so
         every charge is refused and a check under it is property-exhausted:
         never a fresh root *)
      match skip_closed amb with
      | Some o -> Some o
      | None -> Some n)

let resolve = function Some p -> Some p | None -> current ()
let rec root b = match b.parent with None -> b | Some p -> root p
let is_refused b = Atomic.get b.refused
let is_nested b = match b.parent with None -> false | Some _ -> true
let depth b = b.depth
let checks b = b.checks
let config b = b.config
let max_attempts b = b.max_attempts
let max_work b = b.max_work

(* Wraps a falsifies function so each evaluation is charged one attempt first
   (false once refused, WITHOUT running the property). *)
let charged_falsifies b f v = if charge b 1 then f v else false
