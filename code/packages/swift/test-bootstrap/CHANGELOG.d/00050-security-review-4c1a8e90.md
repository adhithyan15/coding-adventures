## 0.2.1

- Security review of the fuzz engine: a frame snapshot is tagged with the thrown error's type and used
  only for an escaping error of that type (a caught error no longer lends frames to a later one);
  `crashesDropped` counts distinct dropped signatures (bounded set);
  scope labels are truncated to 128 bytes at
  push time; the unused `campaign`/`epoch` fields of `BudgetSentinel` and `Handle` are removed (the
  `===` identity check is the guarantee).
