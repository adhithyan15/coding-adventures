let workload_id = "indexed-array-sum-4096"
let make_array () = Array.init 4096 (fun index -> ((17 * index) + 3) mod 251)

let add_checked total value =
  if value > 0L && total > Int64.sub Int64.max_int value then
    Error "checksum arithmetic overflow"
  else if value < 0L && total < Int64.sub Int64.min_int value then
    Error "checksum arithmetic overflow"
  else Ok (Int64.add total value)

let sum values use_index =
  let rec indexed index total =
    if index >= Array.length values then Ok total
    else
      match add_checked total (Int64.of_int values.(index)) with
      | Error error -> Error error
      | Ok next -> indexed (index + 1) next
  in
  if use_index then indexed 0 0L
  else
    let total = ref 0L and failure = ref None in
    Array.iter
      (fun value ->
        match (!failure, add_checked !total (Int64.of_int value)) with
        | None, Ok next -> total := next
        | None, Error error -> failure := Some error
        | Some _, _ -> ())
      values;
    match !failure with Some error -> Error error | None -> Ok !total

let batch values operations use_index =
  if operations < 0L then Error "operations must be nonnegative"
  else
    let rec repeat left total =
      if left = 0L then Ok total
      else
        match sum values use_index with
        | Error error -> Error error
        | Ok one -> (
            match add_checked total one with
            | Error error -> Error error
            | Ok next -> repeat (Int64.pred left) next)
    in
    repeat operations 0L

let indexed_batch values operations = batch values operations true
let iteration_batch values operations = batch values operations false

let expected_checksum =
  match sum (make_array ()) false with Ok value -> value | Error _ -> 0L
