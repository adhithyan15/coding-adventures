(** Pure, validated digital logic with explicitly threaded state.

    A {!bit} remains an integer so examples read like truth tables, but every
    public operation accepts only [0] and [1]. Lists of selector, encoded,
    register, and counter bits are least-significant-bit first. Validation is
    deterministic and no operation retains hidden global state. *)

type bit = int
(** A binary digit. Only [0] and [1] are valid at public boundaries. *)

type error =
  | Invalid_bit of { name : string; value : int }
      (** [name] identifies the rejected input and [value] is its value. *)
  | Invalid_width of int
      (** A width is zero, negative, or above a documented fixed ceiling. *)
  | Invalid_length of { name : string; expected : int; actual : int }
      (** A named list has [actual] elements instead of [expected]. *)
  | Invalid_power_of_two of { name : string; value : int }
      (** A named size must be a nonzero power of two. *)
  | Invalid_one_hot
      (** A one-hot input or complementary state invariant is invalid. *)
  | Invalid_counter_state
      (** Counter value bits disagree with the slave outputs of its flip-flops.
      *)

module Basic : sig
  (** Validated primitive gates, NAND constructions, and n-ary reductions. *)

  val not_gate : bit -> (bit, error) result
  (** Logical NOT. *)

  val and_gate : bit -> bit -> (bit, error) result
  (** Logical AND. *)

  val or_gate : bit -> bit -> (bit, error) result
  (** Logical OR. *)

  val xor_gate : bit -> bit -> (bit, error) result
  (** Exclusive OR. *)

  val nand_gate : bit -> bit -> (bit, error) result
  (** Logical NAND. *)

  val nor_gate : bit -> bit -> (bit, error) result
  (** Logical NOR. *)

  val xnor_gate : bit -> bit -> (bit, error) result
  (** Logical equivalence. *)

  val nand_not : bit -> (bit, error) result
  (** NOT constructed only from {!nand_gate}. *)

  val nand_and : bit -> bit -> (bit, error) result
  (** AND constructed only from {!nand_gate}. *)

  val nand_or : bit -> bit -> (bit, error) result
  (** OR constructed only from {!nand_gate}. *)

  val nand_xor : bit -> bit -> (bit, error) result
  (** XOR constructed only from {!nand_gate}. *)

  val and_n : bit list -> (bit, error) result
  (** Left-to-right AND reduction. The empty identity is [1]. *)

  val or_n : bit list -> (bit, error) result
  (** Left-to-right OR reduction. The empty identity is [0]. *)

  val xor_n : bit list -> (bit, error) result
  (** Left-to-right XOR reduction. The empty identity is [0]. *)
end

module Combinational : sig
  (** Stateless circuits whose selector and encoded lists are LSB-first. *)

  val mux2 : bit -> bit -> sel:bit -> (bit, error) result
  (** Selects the first input for [sel=0] and the second for [sel=1]. *)

  val mux4 : bit -> bit -> bit -> bit -> sel:bit list -> (bit, error) result
  (** Selects one of four inputs using exactly two selector bits. *)

  val mux8 : bit list -> sel:bit list -> (bit, error) result
  (** Selects one of exactly eight inputs using three selector bits. *)

  val mux_n : bit list -> sel:bit list -> (bit, error) result
  (** Selects from a nonempty power-of-two input list. The selector width is
      exactly [log2 (List.length inputs)]; a singleton accepts [[]]. *)

  val demux : bit -> sel:bit list -> n_outputs:int -> (bit list, error) result
  (** Routes the input to one selected output in a power-of-two-sized result. *)

  val decoder : bit list -> (bit list, error) result
  (** Produces a [2^width] one-hot result. Width is capped at 16 and [[]]
      decodes to [[1]]. *)

  val encoder : bit list -> (bit list, error) result
  (** Encodes a nonempty power-of-two one-hot input as an LSB-first index. *)

  val priority_encoder : bit list -> (bit list * bit, error) result
  (** Encodes the highest active index and returns [(index, valid)]. An all-zero
      input returns zero index bits and [valid=0]. *)

  val tri_state : bit -> enable:bit -> (bit option, error) result
  (** Returns [Some bit] when enabled and [None] for high impedance. *)
end

module Sequential : sig
  (** Sequential circuits with immutable caller-threaded snapshots.

      A low clock samples the master latch; a high clock transfers the previous
      master value to the slave latch. *)

  type latch_state = {
    q : bit;  (** Output bit. *)
    q_bar : bit;  (** Complementary output; must equal [1 - q]. *)
  }
  (** Complete SR/D latch snapshot. *)

  type flip_flop_state = {
    master_q : bit;  (** Master-latch output. *)
    master_q_bar : bit;  (** Complement of {!master_q}. *)
    slave_q : bit;  (** Slave-latch output. *)
    slave_q_bar : bit;  (** Complement of {!slave_q}. *)
  }
  (** Complete master/slave D flip-flop snapshot. *)

  type counter_state = {
    value : bit list;
        (** LSB-first value, equal to the slave outputs in {!flip_flops}. *)
    flip_flops : flip_flop_state list;
        (** One valid flip-flop per value bit, in the same order. *)
  }
  (** Complete counter snapshot. *)

  type direction =
    | Left  (** Insert at list head and emit the old final/MSB bit. *)
    | Right  (** Append at list tail and emit the old first/LSB bit. *)

  val sr_latch :
    ?state:latch_state ->
    set_:bit ->
    reset:bit ->
    unit ->
    (latch_state, error) result
  (** Applies hold, set, or reset to an SR latch. Simultaneous set and reset is
      {!Invalid_one_hot}; omitted state is [(q=0, q_bar=1)]. *)

  val d_latch :
    ?state:latch_state ->
    data:bit ->
    enable:bit ->
    unit ->
    (latch_state, error) result
  (** Holds while disabled and captures [data] while enabled. *)

  val d_flip_flop :
    ?state:flip_flop_state ->
    data:bit ->
    clock:bit ->
    unit ->
    (bit * bit * flip_flop_state, error) result
  (** Returns [(q, q_bar, next_state)] after the requested clock level. *)

  val register :
    ?state:flip_flop_state list ->
    ?width:int ->
    data:bit list ->
    clock:bit ->
    unit ->
    (bit list * flip_flop_state list, error) result
  (** Clocks all bits simultaneously. Width defaults to [List.length data];
      data, state, and output are LSB-first and must have exact width. *)

  val shift_register :
    ?state:flip_flop_state list ->
    ?width:int ->
    ?direction:direction ->
    serial_in:bit ->
    clock:bit ->
    unit ->
    (bit list * bit * flip_flop_state list, error) result
  (** Shifts a register and returns [(outputs, serial_out, next_state)]. Width
      defaults to 8 and direction to {!Left}. *)

  val counter :
    ?state:counter_state ->
    ?width:int ->
    ?reset:bit ->
    clock:bit ->
    unit ->
    (bit list * counter_state, error) result
  (** Advances a modulo-[2^width] counter through the low/high sampling
      sequence. Reset immediately returns a valid zero snapshot. Supplied state
      is fully validated before reset or increment; width defaults to 8. *)
end
