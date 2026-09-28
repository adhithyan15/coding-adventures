- `form` start tags processed in table mode now report the required parse
  error whether the form element pointer is initially null or already set.
  The first form retains its special detached insertion behavior, while a
  repeated form is still ignored.
