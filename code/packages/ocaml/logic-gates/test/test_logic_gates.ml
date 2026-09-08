open Coding_adventures_logic_gates

let bit = Alcotest.int
let bits = Alcotest.list bit

let get = function
  | Ok value -> value
  | Error _ -> Alcotest.fail "unexpected error"

let expect_error = function
  | Error _ -> ()
  | Ok _ -> Alcotest.fail "expected an error"

let test_primitive_truth_tables () =
  let unary = [ (0, 1); (1, 0) ] in
  List.iter
    (fun (input, expected) ->
      Alcotest.check bit "not" expected (get (Basic.not_gate input)))
    unary;
  let rows = [ (0, 0); (0, 1); (1, 0); (1, 1) ] in
  List.iter
    (fun (a, b) ->
      Alcotest.check bit "and" (a land b) (get (Basic.and_gate a b));
      Alcotest.check bit "or" (a lor b) (get (Basic.or_gate a b));
      Alcotest.check bit "xor" (a lxor b) (get (Basic.xor_gate a b));
      Alcotest.check bit "nand" (1 - (a land b)) (get (Basic.nand_gate a b));
      Alcotest.check bit "nor" (1 - (a lor b)) (get (Basic.nor_gate a b));
      Alcotest.check bit "xnor" (1 - (a lxor b)) (get (Basic.xnor_gate a b));
      Alcotest.check bit "nand not"
        (get (Basic.not_gate a))
        (get (Basic.nand_not a));
      Alcotest.check bit "nand and"
        (get (Basic.and_gate a b))
        (get (Basic.nand_and a b));
      Alcotest.check bit "nand or"
        (get (Basic.or_gate a b))
        (get (Basic.nand_or a b));
      Alcotest.check bit "nand xor"
        (get (Basic.xor_gate a b))
        (get (Basic.nand_xor a b)))
    rows

let test_invalid_bits () =
  expect_error (Basic.not_gate 2);
  expect_error (Basic.and_gate (-1) 0);
  expect_error (Combinational.mux2 0 1 ~sel:3)

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
          Alcotest.test_case "invalid bits" `Quick test_invalid_bits;
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
