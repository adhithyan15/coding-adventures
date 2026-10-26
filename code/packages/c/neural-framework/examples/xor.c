/* Fixed native XOR command. Checkpoint bytes travel on stdin, never in shell
 * commands or file paths. Baseline flags select only these two known counts;
 * they let the validator compare resumed models to uninterrupted fresh runs.
 */
#include "neural_framework.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static NFStatus resume_input(NFNetwork **n) {
    char *text = malloc(BJ_MAX_BYTES + 1);
    size_t length = 0;
    int c;
    NFStatus status;
    if (!text) return NF_ALLOC;
    while ((c = getchar()) != EOF) {
        if (length == BJ_MAX_BYTES) { free(text); return NF_LIMIT; }
        text[length++] = (char)c;
    }
    if (ferror(stdin)) { free(text); return NF_ARGUMENT; }
    status = nf_from_json(text, length, n);
    free(text);
    return status;
}
int main(int argc, char **argv) {
    const size_t widths[] = {2, 4, 1};
    const NFActivation acts[] = {NF_TANH, NF_SIGMOID};
    const double x[] = {0, 0, 0, 1, 1, 0, 1, 1}, y[] = {0, 1, 1, 0};
    NFNetwork *n = NULL;
    NFHistory history = {0};
    NFTrace final = {0};
    NFText checkpoint = {0};
    NFStatus status;
    size_t epochs = 3000, i;
    int resume = argc == 2 && !strcmp(argv[1], "--resume");
    if (argc == 2 && !strcmp(argv[1], "--baseline-3001")) epochs = 3001;
    else if (argc == 2 && !strcmp(argv[1], "--baseline-3002")) epochs = 3002;
    else if (argc != 1 && !resume) {
        fputs("expected no arguments, --resume, --baseline-3001 or --baseline-3002\n", stderr);
        return EXIT_FAILURE;
    }
    status = resume ? resume_input(&n) : nf_seeded(widths, acts, 2, 7, &n);
    if (resume) epochs = 1;
    /* This fixed dataset owns only four rows of 2 inputs and 1 target. A valid
     * checkpoint for a DIFFERENT task must not enlarge those readable buffers.
     */
    if (status == NF_OK && (nf_input_width(n) != 2 || nf_output_width(n) != 1))
        status = NF_SHAPE;
    if (status == NF_OK) status = nf_fit(n, x, y, 4, epochs, 1, &history);
    if (status == NF_OK) status = nf_gradients(n, x, y, 4, &final);
    if (status == NF_OK) status = nf_to_json(n, &checkpoint);
    if (status == NF_OK) {
        /* C starts in the "C" locale and this command never calls setlocale.
         * Library serialization is independently locale-normalized.
         */
        printf("{\"checkpoint\":%s,\"predictions\":[", checkpoint.data);
        for (i = 0; i < final.predictions.rows; ++i) {
            if (i) putchar(',');
            printf("[%.17g]", final.predictions.data[i]);
        }
        printf("],\"initial_loss\":%.17g,\"final_loss\":%.17g}\n",
               history.losses[0], final.loss);
        if (ferror(stdout)) status = NF_ARGUMENT;
    } else fprintf(stderr, "native C XOR failed (status %d)\n", (int)status);
    nf_text_free(&checkpoint); nf_trace_free(&final);
    nf_history_free(&history); nf_free(&n);
    return status == NF_OK ? EXIT_SUCCESS : EXIT_FAILURE;
}
