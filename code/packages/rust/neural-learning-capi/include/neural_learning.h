#ifndef CODING_ADVENTURES_NEURAL_LEARNING_H
#define CODING_ADVENTURES_NEURAL_LEARNING_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif
#define NEURAL_LEARNING_ABI_VERSION_V1 UINT32_C(0x00010000)
#define NEURAL_LEARNING_OK UINT32_C(0)
#define NEURAL_LEARNING_NULL_POINTER UINT32_C(1)
#define NEURAL_LEARNING_EMPTY_INPUT UINT32_C(2)
#define NEURAL_LEARNING_BUFFER_TOO_SMALL UINT32_C(3)
#define NEURAL_LEARNING_VALUE_TOO_LARGE UINT32_C(4)
#define NEURAL_LEARNING_NON_FINITE UINT32_C(5)
#define NEURAL_LEARNING_PANIC UINT32_C(6)
#define NEURAL_LEARNING_OVERLAPPING_BUFFER UINT32_C(7)
#define NEURAL_LEARNING_MISALIGNED_POINTER UINT32_C(8)

uint32_t neural_learning_abi_version(void);

const char *neural_learning_status_message_v1(uint32_t status);

uint32_t neural_learning_weighted_sum_f64_v1(
    const double *inputs,
    const double *weights,
    uint64_t input_count,
    double bias,
    double *contributions_out,
    uint64_t contributions_capacity,
    double *prediction_out);

#ifdef __cplusplus
}
#endif

#endif
