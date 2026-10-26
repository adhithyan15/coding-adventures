/* A SEPARATE installed consumer: no repository-relative headers or sources. */
#include "neural_framework.h"
#include <stdio.h>
int main(void) {
    const size_t widths[] = {2, 4, 1};
    const NFActivation acts[] = {NF_TANH, NF_SIGMOID};
    const double x[] = {0, 0, 0, 1, 1, 0, 1, 1}, y[] = {0, 1, 1, 0};
    NFNetwork *n = NULL, *loaded = NULL;
    NFHistory h = {0};
    NFText text = {0};
    Mat predicted = {0};
    NFTrace trace = {0};
    int okay = nf_seeded(widths, acts, 2, 7, &n) == NF_OK &&
        nf_fit(n, x, y, 4, 3000, 1, &h) == NF_OK && h.losses[3000] < .01 &&
        nf_to_json(n, &text) == NF_OK &&
        nf_from_json(text.data, text.length, &loaded) == NF_OK &&
        nf_step(loaded) == 3000 && nf_predict(loaded, x, 4, &predicted) == NF_OK &&
        nf_train_step(loaded, x, y, 4, 1, &trace) == NF_OK && nf_step(loaded) == 3001;
    nf_trace_free(&trace); mat_free(&predicted); nf_text_free(&text);
    nf_history_free(&h); nf_free(&loaded); nf_free(&n);
    if (okay) puts("installed native C consumer: prediction/training/checkpoint/resume PASS");
    return okay ? 0 : 1;
}
