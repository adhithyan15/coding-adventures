# bacterial meningitis — grounded corpus provenance

Forward byte-provenance crawl to primary data. **8/8 finding LRs grounded** (prior grounded).

| finding | LR | formula | primary source | verdict |
|---|---|---|---|---|
| prior(base_rate) | 0.037 | prior(base_rate) = pretest prevalence of bacterial meni | Nigrovic LE, Kuppermann N, Macias CG, et al. Clinical predic | grounded |
| csf_gram_stain(positive) | 85 | LR+ = sensitivity / (1 - specificity) = 0.85 / (1 - 0.9 | WHO guidelines on meningitis diagnosis, treatment and care ( | grounded |
| csf_neutrophilic_pleocytosis(high) | 15 | Published pooled LR+ used directly. Straus JAMA 2006 re | Straus SE, Thorpe KE, Holroyd-Leduc J. The Rational Clinical | grounded |
| csf_glucose(low) | 18 | Primary (Straus 2006, pooled, reported directly): LR+ = | Straus SE, Thorpe KE, Holroyd-Leduc J. How do I perform a lu | grounded |
| csf_protein(elevated) | 9.33 | LR+ = sensitivity / (1 - specificity) = 0.84 / (1 - 0.9 | Viallon et al. CSF protein threshold for bacterial vs viral  | grounded |
| csf_lactate(elevated) | 22.9 | Published pooled LR+ used directly = 22.9 (Sakushima 20 | Sakushima K, et al. Diagnostic accuracy of cerebrospinal flu | grounded |
| serum_procalcitonin(elevated) | 27.3 | Published pooled LR+ used directly = 27.3 (95% CI 8.2-9 | Vikse J, Henry BM, Roy J, Ramakrishnan PK, Tomaszewski KA, W | grounded |
| seizure(present) | 5.84 | Single-finding LR+ from Table 2 univariate 2x2. sensiti | Nigrovic LE, Kuppermann N, Malley R. Development and Validat | grounded |
| csf_culture(positive) | 271 | LR+ = sensitivity / (1 - specificity) = 0.813 / (1 - 0. | Wu HM, Cordeiro SM, Harcourt BH, et al. Accuracy of real-tim | grounded |
