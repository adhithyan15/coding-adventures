type bit = int

(* Bits stay as integers at the public boundary so circuit examples resemble
    truth tables. Every operation validates the domain before evaluating, so a
    malformed wire never quietly becomes a truthy value. *)

type error =
  | Invalid_bit of { name : string; value : int }
  | Invalid_width of int
  | Invalid_length of { name : string; expected : int; actual : int }
  | Invalid_power_of_two of { name : string; value : int }
  | Invalid_one_hot
  | Invalid_direction of string

let valid_bit name value =
  if value = 0 || value = 1 then Ok value
  else Error (Invalid_bit { name; value })

let rec validate_bits name = function
  | [] -> Ok ()
  | value :: rest -> (
      match valid_bit name value with
      | Error error -> Error error
      | Ok _ -> validate_bits name rest)

let is_power_of_two value = value > 0 && value land (value - 1) = 0

let rec log2_exact accumulator value =
  if value = 1 then accumulator else log2_exact (accumulator + 1) (value / 2)

let bind result f =
  match result with Ok value -> f value | Error error -> Error error

module Basic = struct
  (* NAND is functionally complete. The derived gates intentionally route
      through [nand_gate] so their implementation mirrors the textbook gate
      construction instead of merely repeating the host-language operator. *)
  let not_gate a = bind (valid_bit "a" a) (fun value -> Ok (1 - value))

  let binary operation a b =
    bind (valid_bit "a" a) (fun left ->
        bind (valid_bit "b" b) (fun right -> Ok (operation left right)))

  let and_gate = binary ( land )
  let or_gate = binary ( lor )
  let xor_gate = binary ( lxor )
  let nand_gate a b = bind (and_gate a b) (fun value -> Ok (1 - value))
  let nor_gate a b = bind (or_gate a b) (fun value -> Ok (1 - value))
  let xnor_gate a b = bind (xor_gate a b) (fun value -> Ok (1 - value))
  let nand_not a = nand_gate a a
  let nand_and a b = bind (nand_gate a b) (fun value -> nand_gate value value)

  let nand_or a b =
    bind (nand_not a) (fun left ->
        bind (nand_not b) (fun right -> nand_gate left right))

  let nand_xor a b =
    bind (nand_gate a b) (fun both ->
        bind (nand_gate a both) (fun left ->
            bind (nand_gate b both) (fun right -> nand_gate left right)))

  let reduce name identity operation inputs =
    bind (validate_bits name inputs) (fun () ->
        Ok (List.fold_left operation identity inputs))

  let and_n inputs = reduce "inputs" 1 ( land ) inputs
  let or_n inputs = reduce "inputs" 0 ( lor ) inputs
  let xor_n inputs = reduce "inputs" 0 ( lxor ) inputs
end

module Combinational = struct
  (* Selectors are LSB-first: [[1; 0]] selects index one. The fixed decoder
      ceiling prevents [2^N] allocation from becoming a platform-dependent
      resource failure. *)
  let max_decoder_width = 16

  let index_of_selectors selectors =
    let rec loop shift total = function
      | [] -> total
      | bit :: rest -> loop (shift + 1) (total lor (bit lsl shift)) rest
    in
    loop 0 0 selectors

  let mux2 d0 d1 ~sel =
    bind
      (validate_bits "data" [ d0; d1 ])
      (fun () ->
        bind (valid_bit "sel" sel) (fun selector ->
            Ok (if selector = 0 then d0 else d1)))

  let mux_n inputs ~sel =
    let count = List.length inputs in
    if not (is_power_of_two count) then
      Error (Invalid_power_of_two { name = "inputs"; value = count })
    else
      bind (validate_bits "inputs" inputs) (fun () ->
          bind (validate_bits "sel" sel) (fun () ->
              let expected = log2_exact 0 count in
              let actual = List.length sel in
              if actual <> expected then
                Error (Invalid_length { name = "sel"; expected; actual })
              else Ok (List.nth inputs (index_of_selectors sel))))

  let mux4 d0 d1 d2 d3 ~sel = mux_n [ d0; d1; d2; d3 ] ~sel

  let mux8 inputs ~sel =
    if List.length inputs <> 8 then
      Error
        (Invalid_length
           { name = "inputs"; expected = 8; actual = List.length inputs })
    else mux_n inputs ~sel

  let demux data ~sel ~n_outputs =
    if not (is_power_of_two n_outputs) then
      Error (Invalid_power_of_two { name = "n_outputs"; value = n_outputs })
    else
      bind (valid_bit "data" data) (fun value ->
          bind (validate_bits "sel" sel) (fun () ->
              let expected = log2_exact 0 n_outputs in
              let actual = List.length sel in
              if actual <> expected then
                Error (Invalid_length { name = "sel"; expected; actual })
              else
                let selected = index_of_selectors sel in
                Ok
                  (List.init n_outputs (fun index ->
                       if index = selected then value else 0))))

  let decoder inputs =
    bind (validate_bits "inputs" inputs) (fun () ->
        let width = List.length inputs in
        if width > max_decoder_width then Error (Invalid_width width)
        else
          let selected = index_of_selectors inputs in
          Ok
            (List.init (1 lsl width) (fun index ->
                 if index = selected then 1 else 0)))

  let bits_of_index width index =
    List.init width (fun shift -> (index lsr shift) land 1)

  let encoder inputs =
    let count = List.length inputs in
    if not (is_power_of_two count) then
      Error (Invalid_power_of_two { name = "inputs"; value = count })
    else
      bind (validate_bits "inputs" inputs) (fun () ->
          let active =
            List.mapi (fun index value -> (index, value)) inputs
            |> List.filter (fun (_, value) -> value = 1)
          in
          match active with
          | [ (index, _) ] -> Ok (bits_of_index (log2_exact 0 count) index)
          | _ -> Error Invalid_one_hot)

  let priority_encoder inputs =
    let count = List.length inputs in
    if not (is_power_of_two count) then
      Error (Invalid_power_of_two { name = "inputs"; value = count })
    else
      bind (validate_bits "inputs" inputs) (fun () ->
          let rec highest index found = function
            | [] -> found
            | value :: rest ->
                highest (index + 1)
                  (if value = 1 then Some index else found)
                  rest
          in
          let width = log2_exact 0 count in
          match highest 0 None inputs with
          | None -> Ok (List.init width (fun _ -> 0), 0)
          | Some index -> Ok (bits_of_index width index, 1))

  let tri_state data ~enable =
    bind (valid_bit "data" data) (fun value ->
        bind (valid_bit "enable" enable) (fun enabled ->
            Ok (if enabled = 1 then Some value else None)))
end

module Sequential = struct
  (* Stateful circuits carry both outputs of every latch. A low clock updates
      the master side; the following high clock transfers it to the slave side.
      Callers therefore own explicit snapshots and no simulation state is
      hidden in module globals. *)
  type latch_state = { q : bit; q_bar : bit }

  type flip_flop_state = {
    master_q : bit;
    master_q_bar : bit;
    slave_q : bit;
    slave_q_bar : bit;
  }

  type counter_state = { value : bit list; flip_flops : flip_flop_state list }
  type direction = Left | Right

  let initial_latch = { q = 0; q_bar = 1 }

  let initial_flip_flop =
    { master_q = 0; master_q_bar = 1; slave_q = 0; slave_q_bar = 1 }

  let validate_latch state =
    bind
      (validate_bits "latch state" [ state.q; state.q_bar ])
      (fun () ->
        if state.q = state.q_bar then Error Invalid_one_hot else Ok state)

  let validate_flip_flop state =
    bind
      (validate_bits "flip-flop state"
         [
           state.master_q; state.master_q_bar; state.slave_q; state.slave_q_bar;
         ])
      (fun () ->
        if
          state.master_q = state.master_q_bar
          || state.slave_q = state.slave_q_bar
        then Error Invalid_one_hot
        else Ok state)

  let sr_latch ?(state = initial_latch) ~set_ ~reset () =
    bind (validate_latch state) (fun previous ->
        bind
          (validate_bits "sr latch" [ set_; reset ])
          (fun () ->
            match (set_, reset) with
            | 0, 0 -> Ok previous
            | 1, 0 -> Ok { q = 1; q_bar = 0 }
            | 0, 1 -> Ok { q = 0; q_bar = 1 }
            | _ -> Error Invalid_one_hot))

  let d_latch ?(state = initial_latch) ~data ~enable () =
    bind (validate_latch state) (fun previous ->
        bind (valid_bit "data" data) (fun value ->
            bind (valid_bit "enable" enable) (fun enabled ->
                if enabled = 0 then Ok previous
                else Ok { q = value; q_bar = 1 - value })))

  let d_flip_flop ?(state = initial_flip_flop) ~data ~clock () =
    bind (validate_flip_flop state) (fun previous ->
        bind (valid_bit "data" data) (fun value ->
            bind (valid_bit "clock" clock) (fun level ->
                let next =
                  if level = 0 then
                    { previous with master_q = value; master_q_bar = 1 - value }
                  else
                    {
                      previous with
                      slave_q = previous.master_q;
                      slave_q_bar = previous.master_q_bar;
                    }
                in
                Ok (next.slave_q, next.slave_q_bar, next))))

  let register ?state ?width ~data ~clock () =
    let actual = List.length data in
    let width = match width with Some value -> value | None -> actual in
    if width <= 0 then Error (Invalid_width width)
    else if actual <> width then
      Error (Invalid_length { name = "data"; expected = width; actual })
    else
      let states =
        match state with
        | Some values -> values
        | None -> List.init width (fun _ -> initial_flip_flop)
      in
      if List.length states <> width then
        Error
          (Invalid_length
             { name = "state"; expected = width; actual = List.length states })
      else
        bind (validate_bits "data" data) (fun () ->
            let rec loop values states outputs next_states =
              match (values, states) with
              | [], [] -> Ok (List.rev outputs, List.rev next_states)
              | value :: value_rest, current :: state_rest ->
                  bind (d_flip_flop ~state:current ~data:value ~clock ())
                    (fun (q, _, next) ->
                      loop value_rest state_rest (q :: outputs)
                        (next :: next_states))
              | _ -> assert false
            in
            loop data states [] [])

  let current_bits states = List.map (fun state -> state.slave_q) states

  let shift_register ?state ?(width = 8) ?(direction = Left) ~serial_in ~clock
      () =
    (* Lists are LSB-first. A left shift inserts at the head and emits the old
        MSB; a right shift appends at the tail and emits the old LSB. *)
    if width <= 0 then Error (Invalid_width width)
    else
      bind (valid_bit "serial_in" serial_in) (fun value ->
          let states =
            match state with
            | Some values -> values
            | None -> List.init width (fun _ -> initial_flip_flop)
          in
          if List.length states <> width then
            Error
              (Invalid_length
                 {
                   name = "state";
                   expected = width;
                   actual = List.length states;
                 })
          else
            let current = current_bits states in
            let serial_out, shifted =
              match direction with
              | Left ->
                  ( List.nth current (width - 1),
                    value
                    :: List.filteri (fun index _ -> index < width - 1) current
                  )
              | Right -> (List.hd current, List.tl current @ [ value ])
            in
            bind (register ~state:states ~width ~data:shifted ~clock ())
              (fun (outputs, next) -> Ok (outputs, serial_out, next)))

  let increment_lsb bits =
    let rec loop carry = function
      | [] -> []
      | bit :: rest ->
          let total = bit + carry in
          (total land 1) :: loop (total lsr 1) rest
    in
    loop 1 bits

  let counter ?state ?(width = 8) ?(reset = 0) ~clock () =
    (* Increment is ripple-carry over the LSB-first list. Dropping the final
        carry gives the natural modulo-[2^width] wraparound. *)
    if width <= 0 then Error (Invalid_width width)
    else
      bind (valid_bit "reset" reset) (fun reset_bit ->
          bind (valid_bit "clock" clock) (fun _ ->
              let current =
                match state with
                | Some value -> value
                | None ->
                    {
                      value = List.init width (fun _ -> 0);
                      flip_flops = List.init width (fun _ -> initial_flip_flop);
                    }
              in
              if List.length current.value <> width then
                Error
                  (Invalid_length
                     {
                       name = "counter value";
                       expected = width;
                       actual = List.length current.value;
                     })
              else if reset_bit = 1 then
                let value = List.init width (fun _ -> 0) in
                let flip_flops = List.init width (fun _ -> initial_flip_flop) in
                Ok (value, { value; flip_flops })
              else
                let next_value = increment_lsb current.value in
                bind
                  (register ~state:current.flip_flops ~width ~data:next_value
                     ~clock ()) (fun (value, flip_flops) ->
                    Ok (value, { value; flip_flops }))))
end
