#!/usr/bin/env escript
%% THROWAWAY PROBE (do not merge): print what the Windows OTP install provides for
%% NIFs, so the Rust erl-nif-bridge can be ported with the real header in hand.
main(_) ->
    Root = code:root_dir(),
    io:format("OTP_RELEASE=~s ERTS=~s ARCH=~s~n",
              [erlang:system_info(otp_release), erlang:system_info(version),
               erlang:system_info(system_architecture)]),
    io:format("ROOT=~s~n", [Root]),
    io:format("ERTS_BIN=~p~n", [filelib:wildcard(Root ++ "/erts-*/bin/*")]),
    Inc = Root ++ "/usr/include/",
    io:format("INCLUDES=~p~n", [filelib:wildcard(Inc ++ "*")]),
    Bins = filelib:wildcard(Root ++ "/erts-*/bin/*.dll")
        ++ filelib:wildcard(Root ++ "/erts-*/bin/*.exe"),
    lists:foreach(
      fun(F) ->
          case file:read_file(F) of
              {ok, B} ->
                  io:format("BIN ~s size=~p enif_make_atom=~p enif_alloc=~p TWinDynNifCallbacks=~p~n",
                            [F, byte_size(B), binary:match(B, <<"enif_make_atom">>),
                             binary:match(B, <<"enif_alloc">>),
                             binary:match(B, <<"TWinDynNifCallbacks">>)]);
              Err -> io:format("BIN ~s unreadable: ~p~n", [F, Err])
          end
      end, Bins),
    io:format("~n=== erl_nif.h ===~n"), dump(Inc ++ "erl_nif.h"),
    io:format("~n=== erl_nif_api_funcs.h ===~n"), dump(Inc ++ "erl_nif_api_funcs.h"),
    io:format("~n=== END PROBE ===~n"),
    halt(1).

dump(F) ->
    case file:read_file(F) of
        {ok, B} -> io:put_chars(B);
        E -> io:format("cannot read ~s: ~p~n", [F, E])
    end.
