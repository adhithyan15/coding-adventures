(** Reusable sample medians and exact distribution-free median intervals. *)

type interval = { low : float; high : float }

val median : float array -> (float, string) result
(** Returns a non-mutating sample median; even samples use a correctly rounded
    arithmetic midpoint, including subnormal and extreme finite inputs. *)

val median_interval : float array -> int -> (interval option, string) result
(** Returns exact binomial order-statistic endpoints; [None] means no rank
    qualifies at the requested confidence percentage. *)

val exact_median_rank : int -> int -> (int, string) result
(** Largest one-based rank satisfying the exact VER07c cutoff. *)
