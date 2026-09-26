- EOF reached with authored open templates now reports one dedicated
  template-mode parse error per open template before preserving any residual
  table, select, or generic EOF diagnostic. Synthetic template fragment
  contexts and closed templates remain quiet.
