#!/usr/bin/env escript
%% tests/smoke.escript -- load the nif_smoke cdylib into a real BEAM and drive every
%% part of erl-nif-bridge it uses.
%%
%%   escript tests/smoke.escript <directory that holds the built cdylib>
%%
%% BUILD builds the library with `cargo build --release --example nif_smoke
%% --target-dir target`, so the directory is target/release/examples.  cargo names
%% the file per platform (libnif_smoke.so, libnif_smoke.dylib, nif_smoke.dll) but
%% erlang:load_nif/2 wants <path>.so on Unix, macOS included, and <path>.dll on
%% Windows, so the script copies whichever file exists to that name first.
%%
%% A failed check exits nonzero, so a bad declaration in the bridge fails the
%% BUILD; there is no skip path.

main([Dir]) ->
    Loadable = stage_library(Dir),
    Src = filename:join(filename:dirname(escript:script_name()), "nif_smoke.erl"),
    {ok, nif_smoke, Beam} = compile:file(Src, [binary, return_errors]),
    {module, nif_smoke} = code:load_binary(nif_smoke, Src, Beam),
    ok = nif_smoke:load(Loadable),
    erlang:put(checks, 0),
    run(),
    io:format("erl-nif-bridge smoke: ~p checks passed against OTP ~s (erts ~s)~n",
              [erlang:get(checks), erlang:system_info(otp_release),
               erlang:system_info(version)]),
    halt(0);
main(_) ->
    io:format(standard_error, "usage: escript tests/smoke.escript <cdylib directory>~n", []),
    halt(2).

%% Copy cargo's output to <dir>/nif_smoke_loadable.<so|dll> and return the path
%% without the extension.
stage_library(Dir) ->
    {Candidates, Ext} =
        case os:type() of
            {win32, _} -> {["nif_smoke.dll"], ".dll"};
            _ -> {["libnif_smoke.so", "libnif_smoke.dylib"], ".so"}
        end,
    Found = [filename:join(Dir, C) || C <- Candidates,
                                      filelib:is_regular(filename:join(Dir, C))],
    case Found of
        [] ->
            io:format(standard_error, "no nif_smoke library in ~s (tried ~p)~n", [Dir, Candidates]),
            halt(2);
        [From | _] ->
            Base = filename:join(Dir, "nif_smoke_loadable"),
            {ok, _} = file:copy(From, Base ++ Ext),
            Base
    end.

%% -- checks -----------------------------------------------------------------

check(Label, Expected, Actual) ->
    erlang:put(checks, erlang:get(checks) + 1),
    case Expected =:= Actual of
        true -> ok;
        false ->
            io:format(standard_error, "FAIL ~s: expected ~p, got ~p~n", [Label, Expected, Actual]),
            halt(1)
    end.

%% The call must raise error:Reason.
raises(Label, Reason, Fun) ->
    erlang:put(checks, erlang:get(checks) + 1),
    try Fun() of
        Got ->
            io:format(standard_error, "FAIL ~s: expected error:~p, got ~p~n", [Label, Reason, Got]),
            halt(1)
    catch
        error:Reason -> ok;
        Class:Other ->
            io:format(standard_error, "FAIL ~s: expected error:~p, got ~p:~p~n",
                      [Label, Reason, Class, Other]),
            halt(1)
    end.

run() ->
    %% integers and floats
    check("add", 42, nif_smoke:add(40, 2)),
    check("add negative", -7, nif_smoke:add(-10, 3)),
    check("add past 32 bits", 1 bsl 40 + 1, nif_smoke:add(1 bsl 40, 1)),
    check("int64 extremes", {-(1 bsl 63), (1 bsl 63) - 1}, nif_smoke:int64_extremes()),
    raises("add rejects an atom", badarg, fun() -> nif_smoke:add(a, 1) end),
    raises("add rejects a bignum", badarg, fun() -> nif_smoke:add(1 bsl 70, 1) end),
    check("half of a float", 1.25, nif_smoke:half(2.5)),
    check("half of an integer", 2.0, nif_smoke:half(4)),
    raises("half rejects a binary", badarg, fun() -> nif_smoke:half(<<"x">>) end),

    %% atoms, tuples, results
    check("kind atom", atom, nif_smoke:kind(hello)),
    check("kind binary", binary, nif_smoke:kind(<<1, 2>>)),
    check("kind map", map, nif_smoke:kind(#{})),
    check("kind tuple", tuple, nif_smoke:kind({1})),
    check("kind list", list, nif_smoke:kind([1])),
    check("kind empty list", list, nif_smoke:kind([])),
    check("kind integer", number, nif_smoke:kind(7)),
    check("kind float", number, nif_smoke:kind(7.5)),
    check("kind pid", other, nif_smoke:kind(self())),
    check("atom_length", 5, nif_smoke:atom_length(hello)),
    raises("atom_length rejects a binary", badarg, fun() -> nif_smoke:atom_length(<<"x">>) end),
    check("swap", {b, a}, nif_smoke:swap({a, b})),
    raises("swap rejects a triple", badarg, fun() -> nif_smoke:swap({a, b, c}) end),
    check("result true", {ok, 1}, nif_smoke:result(true)),
    check("result false", {error, nope}, nif_smoke:result(false)),
    case os:type() of
        {win32, _} -> ok;
        _ -> check("variadic enif_make_tuple", {1, two}, nif_smoke:variadic_tuple())
    end,

    %% binaries
    check("upcase", <<"HELLO">>, nif_smoke:upcase(<<"hello">>)),
    check("upcase empty", <<>>, nif_smoke:upcase(<<>>)),
    %% e-acute (U+00E9) upper-cases to U+00C9, written as code points to keep this
    %% file ASCII.
    check("upcase handles UTF-8", unicode:characters_to_binary([$C, $A, $F, 16#C9]),
          nif_smoke:upcase(unicode:characters_to_binary([$c, $a, $f, 16#E9]))),
    raises("upcase rejects a list", badarg, fun() -> nif_smoke:upcase("hello") end),
    check("checksum with embedded zeros", 6, nif_smoke:checksum(<<1, 0, 2, 0, 3>>)),
    check("checksum of a sub-binary", 5, nif_smoke:checksum(binary:part(<<9, 2, 3, 9>>, 1, 2))),
    check("enif_alloc round trip", true, nif_smoke:alloc_roundtrip()),

    %% lists
    check("sum_floats", 6.5, nif_smoke:sum_floats([1, 2.5, 3])),
    check("sum_floats empty", 0.0, nif_smoke:sum_floats([])),
    raises("sum_floats rejects a non-number", badarg, fun() -> nif_smoke:sum_floats([1, a]) end),
    raises("sum_floats rejects a non-list", badarg, fun() -> nif_smoke:sum_floats(5) end),
    check("range_floats", [1.0, 2.0, 3.0], nif_smoke:range_floats(3)),
    check("range_floats(0)", [], nif_smoke:range_floats(0)),
    check("make_list", [a, b, c], nif_smoke:list_of_atoms()),

    %% maps
    check("make_map", #{a => 1, b => 2}, nif_smoke:make_map()),
    check("map_put", #{a => 1, z => 26}, nif_smoke:map_put(#{a => 1}, z, 26)),
    check("map_put overwrites", #{a => 2}, nif_smoke:map_put(#{a => 1}, a, 2)),
    check("map_lookup hit", {ok, 2}, nif_smoke:map_lookup(#{a => 1, b => 2}, b)),
    check("map_lookup miss", error, nif_smoke:map_lookup(#{a => 1}, zzz)),
    check("map_size", 3, nif_smoke:map_size(#{a => 1, b => 2, c => 3})),
    check("map_size empty", 0, nif_smoke:map_size(#{})),
    raises("map_size rejects a list", badarg, fun() -> nif_smoke:map_size([]) end),
    check("map iterator sum", 60, nif_smoke:map_sum(#{a => 10, b => 20, c => 30})),
    check("map iterator over the empty map", 0, nif_smoke:map_sum(#{})),

    %% resources
    Counter = nif_smoke:counter_new(10),
    check("counter is a reference", true, is_reference(Counter)),
    check("counter_bump", 11, nif_smoke:counter_bump(Counter)),
    check("counter_bump again", 12, nif_smoke:counter_bump(Counter)),
    raises("counter_bump rejects a non-resource", badarg, fun() -> nif_smoke:counter_bump(make_ref()) end),
    resources_are_destroyed(),

    %% processes and messages
    Self = self(),
    check("self_pid", Self, nif_smoke:self_pid()),
    {Pid, Ref} = spawn_monitor(fun() -> exit(nif_smoke:self_pid()) end),
    receive {'DOWN', Ref, process, Pid, Reported} -> check("self_pid in another process", Pid, Reported)
    after 10000 -> check("process exited", true, false) end,
    check("send_from_thread", ok, nif_smoke:send_from_thread(Self)),
    receive hello_from_thread -> check("message from a Rust thread", true, true)
    after 10000 -> check("message from a Rust thread", true, false) end,
    raises("send_from_thread rejects a non-pid", badarg, fun() -> nif_smoke:send_from_thread(a) end),

    %% errors
    raises("badarg", badarg, fun() -> nif_smoke:badarg_nif() end),
    raises("enif_raise_exception", custom_reason, fun() -> nif_smoke:raise(custom_reason) end),
    check("the library still works after errors", 3, nif_smoke:add(1, 2)),
    ok.

%% Resources created in a process that then exits are garbage collected, which
%% runs the Rust destructor.  Wait (bounded) for the BEAM to get to it.
resources_are_destroyed() ->
    Before = nif_smoke:counters_destroyed(),
    {Pid, Ref} = spawn_monitor(fun() ->
        _ = [nif_smoke:counter_new(I) || I <- lists:seq(1, 25)],
        ok
    end),
    receive {'DOWN', Ref, process, Pid, normal} -> ok
    after 10000 -> check("resource process exited", true, false) end,
    wait_destroyed(Before + 25, 100).

wait_destroyed(Target, 0) ->
    check("destructors ran", Target, nif_smoke:counters_destroyed());
wait_destroyed(Target, Tries) ->
    erlang:garbage_collect(),
    case nif_smoke:counters_destroyed() >= Target of
        true -> check("destructors ran", true, true);
        false -> timer:sleep(50), wait_destroyed(Target, Tries - 1)
    end.
