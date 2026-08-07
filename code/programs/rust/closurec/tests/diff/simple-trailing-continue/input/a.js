// SIMPLE-level trailing-continue removal (fold-control-flow 0.39.0).
//
// A bare (unlabeled) `continue` at the tail of a for/while/do-while body is a
// no-op -- it jumps to the next iteration, exactly what falling off the end of
// the body already does -- so it is removed; the shortened body then unwraps or
// normalizes:
//   for (; c;) { step(); continue; }   -> for(;c;)step();
//   while (d) { work(); continue; }    -> for(;d;)work();   (while lowers to for)
//   do { tick(); continue; } while(e); -> do tick();while(e);
//   for (; f;) { continue; }           -> for(;f;);         (body emptied)
//
// A LABELED continue (`continue L`, may target an outer loop) and a `continue`
// with dead code after it are left alone -- covered by unit tests.
for (; c;) { step(); continue; }
while (d) { work(); continue; }
do { tick(); continue; } while (e);
for (; f;) { continue; }
