%% The Erlang side of the nif_smoke example: one stub per NIF function, and
%% load/1, which asks the BEAM to load the library.
%%
%% erlang:load_nif/2 replaces every stub that has a native function of the same
%% name and arity, so the list below must match the table in
%% examples/nif_smoke.rs.  A stub that is called before (or without) a native
%% replacement raises nif_not_loaded.  tests/smoke.escript compiles and loads this
%% file and runs the checks.
-module(nif_smoke).

-export([load/1]).
-export([add/2, int64_extremes/0, half/1, kind/1, atom_length/1, swap/1, result/1,
         variadic_tuple/0, upcase/1, checksum/1, alloc_roundtrip/0, sum_floats/1,
         range_floats/1, list_of_atoms/0, make_map/0, map_put/3, map_lookup/2,
         map_size/1, map_sum/1, counter_new/1, counter_bump/1, counters_destroyed/0,
         self_pid/0, send_from_thread/1, badarg_nif/0, raise/1]).

%% Path is the library's path WITHOUT the extension: the BEAM appends .so or .dll.
load(Path) -> erlang:load_nif(Path, 0).

add(_, _) -> erlang:nif_error(nif_not_loaded).
int64_extremes() -> erlang:nif_error(nif_not_loaded).
half(_) -> erlang:nif_error(nif_not_loaded).
kind(_) -> erlang:nif_error(nif_not_loaded).
atom_length(_) -> erlang:nif_error(nif_not_loaded).
swap(_) -> erlang:nif_error(nif_not_loaded).
result(_) -> erlang:nif_error(nif_not_loaded).
%% Only the Unix library has this one (Windows has no variadic enif_make_tuple).
variadic_tuple() -> erlang:nif_error(nif_not_loaded).
upcase(_) -> erlang:nif_error(nif_not_loaded).
checksum(_) -> erlang:nif_error(nif_not_loaded).
alloc_roundtrip() -> erlang:nif_error(nif_not_loaded).
sum_floats(_) -> erlang:nif_error(nif_not_loaded).
range_floats(_) -> erlang:nif_error(nif_not_loaded).
list_of_atoms() -> erlang:nif_error(nif_not_loaded).
make_map() -> erlang:nif_error(nif_not_loaded).
map_put(_, _, _) -> erlang:nif_error(nif_not_loaded).
map_lookup(_, _) -> erlang:nif_error(nif_not_loaded).
map_size(_) -> erlang:nif_error(nif_not_loaded).
map_sum(_) -> erlang:nif_error(nif_not_loaded).
counter_new(_) -> erlang:nif_error(nif_not_loaded).
counter_bump(_) -> erlang:nif_error(nif_not_loaded).
counters_destroyed() -> erlang:nif_error(nif_not_loaded).
self_pid() -> erlang:nif_error(nif_not_loaded).
send_from_thread(_) -> erlang:nif_error(nif_not_loaded).
badarg_nif() -> erlang:nif_error(nif_not_loaded).
raise(_) -> erlang:nif_error(nif_not_loaded).
