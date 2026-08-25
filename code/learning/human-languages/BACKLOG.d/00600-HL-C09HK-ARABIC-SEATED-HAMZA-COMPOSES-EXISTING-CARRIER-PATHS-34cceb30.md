## HL-C09HK — Arabic seated Hamza composes existing carrier paths

The Hamza lesson resolves the first post-alphabet debt without adding fake base
letters. It inventories **أ, إ, ؤ, and ئ** as Hamza combined with **ا, و,** or a
dotless **ي** seat, and explicitly orders the carrier first and Hamza afterward.
The canonical data now records U+0654 Hamza Above and U+0655 Hamza Below with
Unicode-normalized examples, composition order, and source provenance.

Arabic remains **29 unique source-verified rows** because these are compositions
of existing carrier and Hamza paths, not new alphabet rows. The next ending-form
audit is **ة** first, then **ى**; audit obligatory **لا** separately as a ligature.
The production `script-data` batch must remain below the 250 kB authored-data
target; this build measures the Arabic-bearing batch at **46.96 kB**.

