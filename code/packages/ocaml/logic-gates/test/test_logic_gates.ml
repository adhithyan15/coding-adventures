open Coding_adventures_logic_gates

let bit = Alcotest.int
let bits = Alcotest.list bit

let get = function
  | Ok value -> value
  | Error _ -> Alcotest.fail "unexpected error"

let expect_error = function
  | Error _ -> ()
  | Ok _ -> Alcotest.fail "expected an error"

(* Literal expected tables: none of these values is computed with the host
   operators, so the tests cannot share a bug with the implementation. Rows
   are (a, b, expected). *)
let unary_not = [ (0, 1); (1, 0) ]

let binary_tables =
  [
    ("and", Basic.and_gate, [ (0, 0, 0); (0, 1, 0); (1, 0, 0); (1, 1, 1) ]);
    ("or", Basic.or_gate, [ (0, 0, 0); (0, 1, 1); (1, 0, 1); (1, 1, 1) ]);
    ("xor", Basic.xor_gate, [ (0, 0, 0); (0, 1, 1); (1, 0, 1); (1, 1, 0) ]);
    ("nand", Basic.nand_gate, [ (0, 0, 1); (0, 1, 1); (1, 0, 1); (1, 1, 0) ]);
    ("nor", Basic.nor_gate, [ (0, 0, 1); (0, 1, 0); (1, 0, 0); (1, 1, 0) ]);
    ("xnor", Basic.xnor_gate, [ (0, 0, 1); (0, 1, 0); (1, 0, 0); (1, 1, 1) ]);
    ("nand_and", Basic.nand_and, [ (0, 0, 0); (0, 1, 0); (1, 0, 0); (1, 1, 1) ]);
    ("nand_or", Basic.nand_or, [ (0, 0, 0); (0, 1, 1); (1, 0, 1); (1, 1, 1) ]);
    ("nand_xor", Basic.nand_xor, [ (0, 0, 0); (0, 1, 1); (1, 0, 1); (1, 1, 0) ]);
  ]

let test_primitive_truth_tables () =
  List.iter
    (fun (input, expected) ->
      Alcotest.check bit "not" expected (get (Basic.not_gate input));
      Alcotest.check bit "nand_not" expected (get (Basic.nand_not input)))
    unary_not;
  List.iter
    (fun (name, gate, rows) ->
      List.iter
        (fun (a, b, expected) ->
          Alcotest.check bit
            (Printf.sprintf "%s %d %d" name a b)
            expected
            (get (gate a b)))
        rows)
    binary_tables

(* The NAND constructions must agree with the direct gates. *)
let test_nand_equals_direct () =
  List.iter
    (fun (a, b) ->
      let same name direct built =
        Alcotest.check bit name (get (direct a b)) (get (built a b))
      in
      same "and" Basic.and_gate Basic.nand_and;
      same "or" Basic.or_gate Basic.nand_or;
      same "xor" Basic.xor_gate Basic.nand_xor)
    [ (0, 0); (0, 1); (1, 0); (1, 1) ];
  List.iter
    (fun a ->
      Alcotest.check bit "not" (get (Basic.not_gate a)) (get (Basic.nand_not a)))
    [ 0; 1 ]

(* OCaml has no implicit numeric conversion: [and_gate 1.0 0],
   [and_gate true 0] and [and_gate '1' 0] do not compile, so only ints can be
   invalid and these are exhaustively exercised below. *)
let invalid_values =
  List.filter (fun v -> v <> 0 && v <> 1) (List.init 2049 (fun i -> i - 1024))
  @ [ min_int; min_int + 1; max_int - 1; max_int ]

let expect_invalid label param value = function
  | Error (Invalid_bit { name; value = v }) ->
      Alcotest.check Alcotest.string (label ^ " param") param name;
      Alcotest.check Alcotest.int (label ^ " value") value v
  | Error _ -> Alcotest.fail (label ^ ": wrong error constructor")
  | Ok _ -> Alcotest.fail (label ^ ": expected Invalid_bit")

let test_invalid_bits () =
  List.iter
    (fun v ->
      expect_invalid "not" "a" v (Basic.not_gate v);
      expect_invalid "nand_not" "a" v (Basic.nand_not v))
    invalid_values;
  List.iter
    (fun (name, gate, _) ->
      List.iter
        (fun v ->
          List.iter
            (fun other ->
              expect_invalid (name ^ " a") "a" v (gate v other);
              expect_invalid (name ^ " b") "b" v (gate other v))
            [ 0; 1 ];
          (* Both invalid: the FIRST parameter is reported. *)
          expect_invalid (name ^ " both") "a" v (gate v (-7)))
        invalid_values)
    binary_tables

(* A valid input must never yield an error, and results are exactly 0 or 1. *)
let test_valid_never_errors () =
  List.iter
    (fun (name, gate, _) ->
      List.iter
        (fun (a, b) ->
          let r = get (gate a b) in
          if r <> 0 && r <> 1 then Alcotest.fail (name ^ ": result not a bit"))
        [ (0, 0); (0, 1); (1, 0); (1, 1) ])
    binary_tables

let test_nary () =
  Alcotest.check bit "and empty identity" 1 (get (Basic.and_n []));
  Alcotest.check bit "and four" 0 (get (Basic.and_n [ 1; 1; 0; 1 ]));
  Alcotest.check bit "or three" 1 (get (Basic.or_n [ 0; 0; 1 ]));
  Alcotest.check bit "xor parity" 1 (get (Basic.xor_n [ 1; 0; 1; 1 ]));
  expect_error (Basic.xor_n [ 0; 2 ])

let test_muxes () =
  Alcotest.check bit "mux2 d0" 1 (get (Combinational.mux2 1 0 ~sel:0));
  Alcotest.check bit "mux2 d1" 1 (get (Combinational.mux2 0 1 ~sel:1));
  let inputs = [ 0; 1; 0; 1; 1; 0; 1; 0 ] in
  List.iteri
    (fun index expected ->
      let selectors =
        [ index land 1; (index lsr 1) land 1; (index lsr 2) land 1 ]
      in
      Alcotest.check bit "mux8 index" expected
        (get (Combinational.mux8 inputs ~sel:selectors));
      Alcotest.check bit "mux_n index" expected
        (get (Combinational.mux_n inputs ~sel:selectors)))
    inputs;
  Alcotest.check bit "mux4" 1 (get (Combinational.mux4 0 0 1 0 ~sel:[ 0; 1 ]));
  expect_error (Combinational.mux_n [ 0; 1; 0 ] ~sel:[ 0; 0 ]);
  expect_error (Combinational.mux8 [ 0; 1 ] ~sel:[ 0 ])

let test_decode_encode_and_bus () =
  Alcotest.check bits "demux" [ 0; 0; 1; 0 ]
    (get (Combinational.demux 1 ~sel:[ 0; 1 ] ~n_outputs:4));
  Alcotest.check bits "decoder lsb first" [ 0; 1; 0; 0 ]
    (get (Combinational.decoder [ 1; 0 ]));
  Alcotest.check bits "encoder" [ 1; 1 ]
    (get (Combinational.encoder [ 0; 0; 0; 1 ]));
  let encoded, valid = get (Combinational.priority_encoder [ 0; 1; 0; 1 ]) in
  Alcotest.check bits "highest priority" [ 1; 1 ] encoded;
  Alcotest.check bit "valid" 1 valid;
  let encoded_none, valid_none =
    get (Combinational.priority_encoder [ 0; 0; 0; 0 ])
  in
  Alcotest.check bits "no active input" [ 0; 0 ] encoded_none;
  Alcotest.check bit "invalid" 0 valid_none;
  Alcotest.check (Alcotest.option bit) "high z" None
    (get (Combinational.tri_state 1 ~enable:0));
  Alcotest.check (Alcotest.option bit) "driven" (Some 0)
    (get (Combinational.tri_state 0 ~enable:1));
  expect_error (Combinational.encoder [ 1; 1; 0; 0 ])

let test_shape_errors () =
  expect_error (Combinational.mux_n [ 0; 1; 0; 1 ] ~sel:[ 0 ]);
  expect_error (Combinational.demux 1 ~sel:[ 0; 1 ] ~n_outputs:3);
  expect_error (Combinational.demux 1 ~sel:[ 0 ] ~n_outputs:4);
  Alcotest.check Alcotest.int "maximum decoder width" 65_536
    (List.length (get (Combinational.decoder (List.init 16 (fun _ -> 0)))));
  expect_error (Combinational.decoder (List.init 17 (fun _ -> 0)));
  expect_error (Combinational.encoder [ 0; 0; 0 ]);
  expect_error (Combinational.priority_encoder [ 0; 0; 0 ])

let test_latches () =
  let open Sequential in
  let initial = { q = 0; q_bar = 1 } in
  Alcotest.check bit "hold" 0
    (get (sr_latch ~state:initial ~set_:0 ~reset:0 ())).q;
  Alcotest.check bit "set" 1
    (get (sr_latch ~state:initial ~set_:1 ~reset:0 ())).q;
  Alcotest.check bit "reset" 0
    (get (sr_latch ~state:{ q = 1; q_bar = 0 } ~set_:0 ~reset:1 ())).q;
  expect_error (sr_latch ~state:initial ~set_:1 ~reset:1 ());
  Alcotest.check bit "d latch hold" 1
    (get (d_latch ~state:{ q = 1; q_bar = 0 } ~data:0 ~enable:0 ())).q;
  Alcotest.check bit "d latch store" 1
    (get (d_latch ~state:initial ~data:1 ~enable:1 ())).q

let test_flip_flop_and_register () =
  let open Sequential in
  let _, _, low = get (d_flip_flop ~data:1 ~clock:0 ()) in
  let q, q_bar, high = get (d_flip_flop ~state:low ~data:0 ~clock:1 ()) in
  Alcotest.check bit "captured q" 1 q;
  Alcotest.check bit "captured q bar" 0 q_bar;
  let _, low_register = get (register ~width:3 ~data:[ 1; 0; 1 ] ~clock:0 ()) in
  let value, _ =
    get (register ~state:low_register ~width:3 ~data:[ 0; 0; 0 ] ~clock:1 ())
  in
  Alcotest.check bits "simultaneous capture" [ 1; 0; 1 ] value;
  expect_error (register ~width:3 ~data:[ 1; 0 ] ~clock:0 ());
  ignore high

let test_shift_register () =
  let open Sequential in
  let _, _, left_low =
    get (shift_register ~width:3 ~direction:Left ~serial_in:1 ~clock:0 ())
  in
  let left, left_out, _ =
    get
      (shift_register ~state:left_low ~width:3 ~direction:Left ~serial_in:0
         ~clock:1 ())
  in
  Alcotest.check bits "left inserts lsb" [ 1; 0; 0 ] left;
  Alcotest.check bit "left serial out" 0 left_out;
  let _, _, right_low =
    get (shift_register ~width:3 ~direction:Right ~serial_in:1 ~clock:0 ())
  in
  let right, right_out, _ =
    get
      (shift_register ~state:right_low ~width:3 ~direction:Right ~serial_in:0
         ~clock:1 ())
  in
  Alcotest.check bits "right inserts msb" [ 0; 0; 1 ] right;
  Alcotest.check bit "right serial out" 0 right_out

let pulse_counter state =
  let open Sequential in
  let _, low = get (counter ?state ~width:3 ~clock:0 ()) in
  get (counter ~state:low ~width:3 ~clock:1 ())

let test_counter () =
  let one, state_one = pulse_counter None in
  Alcotest.check bits "zero to one" [ 1; 0; 0 ] one;
  let two, state_two = pulse_counter (Some state_one) in
  Alcotest.check bits "one to two" [ 0; 1; 0 ] two;
  let captured bit =
    {
      Sequential.master_q = bit;
      master_q_bar = 1 - bit;
      slave_q = bit;
      slave_q_bar = 1 - bit;
    }
  in
  let seven =
    {
      Sequential.value = [ 1; 1; 1 ];
      flip_flops = List.init 3 (fun _ -> captured 1);
    }
  in
  let zero, _ = pulse_counter (Some seven) in
  Alcotest.check bits "wrap" [ 0; 0; 0 ] zero;
  let reset, _ =
    get (Sequential.counter ~state:state_two ~width:3 ~reset:1 ~clock:1 ())
  in
  Alcotest.check bits "reset" [ 0; 0; 0 ] reset

let test_sequential_state_errors () =
  let open Sequential in
  let invalid_latch = { q = 0; q_bar = 0 } in
  let invalid_flip_flop =
    { master_q = 0; master_q_bar = 0; slave_q = 0; slave_q_bar = 1 }
  in
  expect_error (sr_latch ~state:invalid_latch ~set_:0 ~reset:0 ());
  expect_error (d_latch ~state:invalid_latch ~data:0 ~enable:0 ());
  expect_error (d_flip_flop ~state:invalid_flip_flop ~data:0 ~clock:0 ());
  expect_error (register ~width:0 ~data:[] ~clock:0 ());
  expect_error (register ~state:[] ~width:1 ~data:[ 0 ] ~clock:0 ());
  expect_error (shift_register ~width:0 ~serial_in:0 ~clock:0 ());
  expect_error (shift_register ~state:[] ~width:1 ~serial_in:0 ~clock:0 ());
  expect_error (counter ~width:0 ~clock:0 ());
  let invalid_counter = { value = []; flip_flops = [ invalid_flip_flop ] } in
  expect_error (counter ~state:invalid_counter ~width:1 ~clock:0 ());
  let valid_zero =
    { master_q = 0; master_q_bar = 1; slave_q = 0; slave_q_bar = 1 }
  in
  let invalid_value = { value = [ 2 ]; flip_flops = [ valid_zero ] } in
  expect_error (counter ~state:invalid_value ~width:1 ~clock:0 ());
  let inconsistent = { value = [ 1 ]; flip_flops = [ valid_zero ] } in
  expect_error (counter ~state:inconsistent ~width:1 ~clock:0 ())

let () =
  Alcotest.run "coding-adventures-logic-gates"
    [
      ( "basic",
        [
          Alcotest.test_case "truth tables" `Quick test_primitive_truth_tables;
          Alcotest.test_case "nand equals direct" `Quick test_nand_equals_direct;
          Alcotest.test_case "invalid bits" `Quick test_invalid_bits;
          Alcotest.test_case "valid never errors" `Quick test_valid_never_errors;
          Alcotest.test_case "n-ary" `Quick test_nary;
        ] );
      ( "combinational",
        [
          Alcotest.test_case "muxes" `Quick test_muxes;
          Alcotest.test_case "decode encode bus" `Quick
            test_decode_encode_and_bus;
          Alcotest.test_case "shape errors" `Quick test_shape_errors;
        ] );
      ( "sequential",
        [
          Alcotest.test_case "latches" `Quick test_latches;
          Alcotest.test_case "flip flop and register" `Quick
            test_flip_flop_and_register;
          Alcotest.test_case "shift register" `Quick test_shift_register;
          Alcotest.test_case "counter" `Quick test_counter;
          Alcotest.test_case "state errors" `Quick test_sequential_state_errors;
        ] );
    ]
