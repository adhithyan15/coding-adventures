/* Native dense learning, with ownership made explicit (NN37/NN48).
 *
 * A network is a sealed box. Prediction/trace/layer inspection allocate copies,
 * never writable aliases into that box. Initialise all producer outputs to zero
 * (or NULL); release them with the matching resetting release function.
 * Readable buffers must really contain the declared number of doubles: normal
 * C pointer validity is a caller precondition, not something a library can test.
 */
#ifndef CA_NEURAL_FRAMEWORK_H
#define CA_NEURAL_FRAMEWORK_H

#include <stddef.h>
#include <stdint.h>
#include "matrix.h"
#include "bounded_json.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Closed capacities keep both allocation and arithmetic work finite. Each
 * dimension is checked BEFORE any product, copy or Mat constructor. With these
 * ceilings a saved forward tape holds at most 17*256*64 doubles (~2.2 MiB).
 * Parameter and fit-history ceilings are independent, not inferred from malloc.
 */
#define NF_MAX_LAYERS ((size_t)16)
#define NF_MAX_WIDTH ((size_t)64)
#define NF_MAX_SAMPLES ((size_t)256)
#define NF_MAX_PARAMETERS ((size_t)65536)
#define NF_MAX_EPOCHS ((size_t)100000)
#define NF_MAX_STEP UINT64_C(9007199254740991)

typedef enum { NF_OK = 0, NF_ARGUMENT, NF_SHAPE, NF_LIMIT,
               NF_NONFINITE, NF_ALLOC, NF_STEP, NF_SCHEMA } NFStatus;
typedef enum { NF_LINEAR = 0, NF_SIGMOID, NF_TANH, NF_RELU } NFActivation;
typedef struct NFNetwork NFNetwork;
typedef struct {
    size_t inputs, outputs;
    NFActivation activation;
    const double *weights; /* inputs*outputs, input-by-output row-major */
    const double *biases;  /* outputs */
} NFLayer;

/* Injection controls framework allocations, NOT the unchanged matrix package's
 * allocator. Both callbacks must be present; they must behave like malloc/free.
 * Context and callback lifetime must exceed all networks/history using them.
 * No mutable global test hooks, allocator counters or RNG state are used.
 */
typedef void *(*NFAllocate)(size_t bytes, void *context);
typedef void (*NFRelease)(void *pointer, void *context);
typedef struct { NFAllocate allocate; NFRelease release; void *context; } NFMemory;

typedef struct {
    double loss;
    Mat predictions;
    size_t layers;
    Mat weights[16], biases[16];
} NFTrace;
typedef struct {
    double *losses; /* initial, then each successful post-update loss */
    size_t count;
    NFMemory memory;
} NFHistory;

NFStatus nf_new(const NFLayer *layers, size_t count, NFNetwork **out);
NFStatus nf_new_with_memory(const NFLayer *layers, size_t count,
                            const NFMemory *memory, NFNetwork **out);
/* widths has count+1 entries; activations has count. uint32_t is the exact
 * seed domain. The wrapping LCG is reproducible initialization, not security.
 */
NFStatus nf_seeded(const size_t *widths, const NFActivation *activations,
                   size_t count, uint32_t seed, NFNetwork **out);
void nf_free(NFNetwork **network);
uint64_t nf_step(const NFNetwork *network); /* NULL yields zero */
size_t nf_layers(const NFNetwork *network);
size_t nf_input_width(const NFNetwork *network);
size_t nf_output_width(const NFNetwork *network);
NFStatus nf_layer(const NFNetwork *network, size_t index, Mat *weights,
                  Mat *biases, NFActivation *activation);
NFStatus nf_clone(const NFNetwork *network, NFNetwork **out);
/* Inputs are samples*input_width; targets are samples*output_width.
 * Callers may reuse/free those arrays immediately after the call returns.
 */
NFStatus nf_predict(const NFNetwork *network, const double *inputs,
                    size_t samples, Mat *out);
NFStatus nf_gradients(const NFNetwork *network, const double *inputs,
                      const double *targets, size_t samples, NFTrace *out);
NFStatus nf_train_step(NFNetwork *network, const double *inputs,
                       const double *targets, size_t samples, double rate,
                       NFTrace *out);
/* Each step is atomic. A later fit failure retains earlier successful steps,
 * releases unpublished history and returns its error. Fit is not a transaction
 * spanning epochs. Zero epochs validates the batch/rate and returns one loss.
 */
NFStatus nf_fit(NFNetwork *network, const double *inputs, const double *targets,
                size_t samples, size_t epochs, double rate, NFHistory *out);
void nf_trace_free(NFTrace *trace);
void nf_history_free(NFHistory *history);

/* The checkpoint object is an independently owned network snapshot (nf_clone).
 * Its step is read-only, and loading copies all parameters from the JSON view.
 * The source document may be released immediately after nf_from_value returns.
 * Text uses NN47's 4 MiB/128-container/65536-node/16 MiB arena hard ceilings.
 */
NFStatus nf_from_value(const BJsonValue *checkpoint, NFNetwork **out);
NFStatus nf_from_json(const char *text, size_t length, NFNetwork **out);
typedef struct { char *data; size_t length; NFMemory memory; } NFText;
NFStatus nf_to_json(const NFNetwork *network, NFText *out);
void nf_text_free(NFText *text);
/* Serialization consults localeconv but NEVER changes process-global locale.
 * Applications must serialize concurrent global locale changes externally,
 * as with NN47 number parsing. JSON always has '.', regardless of host locale.
 */

#ifdef __cplusplus
}
#endif
#endif
