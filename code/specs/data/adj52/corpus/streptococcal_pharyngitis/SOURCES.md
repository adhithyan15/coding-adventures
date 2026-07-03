# streptococcal pharyngitis — grounded corpus provenance

Forward byte-provenance crawl to primary data. **8/8 finding LRs grounded** (prior grounded).

| finding | LR | formula | primary source | verdict |
|---|---|---|---|---|
| prior(base_rate) | 0.37 | Prior is a base-rate, reported as a probability (not an | Shaikh N, Leonard E, Martin JM. Prevalence of Streptococcal  | grounded |
| tonsillar_exudate(present) | 3.4 | Finding f1 = tonsillar_exudate(present), so the relevan | Ebell MH, Smith MA, Barry HC, Ives K, Carey M. The Rational  | grounded |
| tender_anterior_cervical_nodes(present) | 1.65 | LR+ = sens/(1-spec) = 0.67/(1-0.59) = 0.67/0.41 = 1.63  | Aalbers J, O'Brien KK, Chan WS, et al. Predicting streptococ | grounded |
| history_of_fever(present) | 1.65 | Finding present (history_of_fever(present)) -> use LR+. | Aalbers J, O'Brien KK, Chan WS, et al. Predicting streptococ | grounded |
| cough(absent) | 1.46 | Finding f4 = cough ABSENT is a PRESENT positive finding | Aalbers J, O'Brien KK, Chan WS, et al. Predicting streptococ | grounded |
| age(under_15) | 1.58 | LR(age 3-14) = P(age 3-14 | GAS+) / P(age 3-14 | GAS-). | Fine AM, Nizet V, Mandl KD. Large-Scale Validation of the Ce | grounded |
| rapid_antigen_test(positive) | 18.6 | LR+ = sensitivity / (1 - specificity) = 0.856 / (1 - 0. | Cohen JF, Bertille R, Cohen R, Chalumeau M. Rapid antigen de | grounded |
| rapid_antigen_test(negative) | 0.1509 | LR- = (1 - sensitivity)/specificity = (1 - 0.856)/0.954 | Cohen JF, Bertille N, Cohen R, Chalumeau M. Rapid antigen de | grounded |
| throat_culture(positive) | 92.5 | LR+ = sensitivity / (1 - specificity). Using byte-ancho | Shulman et al., IDSA 2012 Clinical Practice Guideline for th | grounded |
