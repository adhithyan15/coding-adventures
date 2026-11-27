(** Strict validator for TFV v1 text (TF01 section 2): the line format that
    carries test corpora between languages. A VALIDATOR, not a reader: its job
    here is to let the corpus runner check the pbt-v1 [corpus_reject] cases with
    a real parser. *)

val max_input : int
val max_tokens : int
val validate : string -> (unit, string) result
val valid : string -> bool
val is_lower_hex_even : string -> bool
val unhex : string -> string
