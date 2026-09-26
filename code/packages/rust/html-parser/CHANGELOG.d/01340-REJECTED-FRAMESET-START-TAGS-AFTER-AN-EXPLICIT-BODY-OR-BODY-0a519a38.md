- Rejected `frameset` start tags after an explicit body or body-incompatible
  content now report the Standard's in-body parse error without changing DOM
  recovery, closing 39 previously silent malformed corpus cases.
