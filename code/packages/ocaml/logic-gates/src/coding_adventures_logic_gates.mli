(* Pure, explicitly state-threaded digital logic primitives. *)

type bit = int

type error =
  | Invalid_bit of { name : string; value : int }
  | Invalid_width of int
  | Invalid_length of { name : string; expected : int; actual : int }
  | Invalid_power_of_two of { name : string; value : int }
  | Invalid_one_hot
  | Invalid_direction of string

module Basic : sig
  val not_gate : bit -> (bit, error) result
  (* Primitive gates, NAND-derived equivalents, and n-ary reductions. *)

  val and_gate : bit -> bit -> (bit, error) result
  val or_gate : bit -> bit -> (bit, error) result
  val xor_gate : bit -> bit -> (bit, error) result
  val nand_gate : bit -> bit -> (bit, error) result
  val nor_gate : bit -> bit -> (bit, error) result
  val xnor_gate : bit -> bit -> (bit, error) result
  val nand_not : bit -> (bit, error) result
  val nand_and : bit -> bit -> (bit, error) result
  val nand_or : bit -> bit -> (bit, error) result
  val nand_xor : bit -> bit -> (bit, error) result
  val and_n : bit list -> (bit, error) result
  val or_n : bit list -> (bit, error) result
  val xor_n : bit list -> (bit, error) result
end

module Combinational : sig
  val mux2 : bit -> bit -> sel:bit -> (bit, error) result
  (* Stateless circuits. Selector and encoded bit lists are LSB-first. *)

  val mux4 : bit -> bit -> bit -> bit -> sel:bit list -> (bit, error) result
  val mux8 : bit list -> sel:bit list -> (bit, error) result
  val mux_n : bit list -> sel:bit list -> (bit, error) result
  val demux : bit -> sel:bit list -> n_outputs:int -> (bit list, error) result
  val decoder : bit list -> (bit list, error) result
  val encoder : bit list -> (bit list, error) result
  val priority_encoder : bit list -> (bit list * bit, error) result
  val tri_state : bit -> enable:bit -> (bit option, error) result
end

module Sequential : sig
  type latch_state = { q : bit; q_bar : bit }
  (* Latches and clocked circuits return their complete next state. *)

  type flip_flop_state = {
    master_q : bit;
    master_q_bar : bit;
    slave_q : bit;
    slave_q_bar : bit;
  }

  type counter_state = { value : bit list; flip_flops : flip_flop_state list }
  type direction = Left | Right

  val sr_latch :
    ?state:latch_state ->
    set_:bit ->
    reset:bit ->
    unit ->
    (latch_state, error) result

  val d_latch :
    ?state:latch_state ->
    data:bit ->
    enable:bit ->
    unit ->
    (latch_state, error) result

  val d_flip_flop :
    ?state:flip_flop_state ->
    data:bit ->
    clock:bit ->
    unit ->
    (bit * bit * flip_flop_state, error) result

  val register :
    ?state:flip_flop_state list ->
    ?width:int ->
    data:bit list ->
    clock:bit ->
    unit ->
    (bit list * flip_flop_state list, error) result

  val shift_register :
    ?state:flip_flop_state list ->
    ?width:int ->
    ?direction:direction ->
    serial_in:bit ->
    clock:bit ->
    unit ->
    (bit list * bit * flip_flop_state list, error) result

  val counter :
    ?state:counter_state ->
    ?width:int ->
    ?reset:bit ->
    clock:bit ->
    unit ->
    (bit list * counter_state, error) result
end
