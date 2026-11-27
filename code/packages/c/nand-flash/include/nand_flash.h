#ifndef CA_NAND_FLASH_H
#define CA_NAND_FLASH_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct NandFlash NandFlash;
typedef enum {
    NAND_FLASH_OK = 0,
    NAND_FLASH_INVALID_ARGUMENT,
    NAND_FLASH_GEOMETRY_ERROR,
    NAND_FLASH_ADDRESS_BLOCK_ERROR,
    NAND_FLASH_ADDRESS_PAGE_ERROR,
    NAND_FLASH_PAYLOAD_ERROR,
    NAND_FLASH_PAYLOAD_SIZE_ERROR,
    NAND_FLASH_PROGRAM_ERROR,
    NAND_FLASH_ERASE_LIMIT_ERROR,
    NAND_FLASH_ALLOCATION_ERROR
} NandFlashStatus;

/* out must be writable and must not own an existing handle. It is set to NULL
 * before geometry validation; success transfers a new handle to the caller. */
NandFlashStatus nand_flash_create(size_t blocks, size_t pages_per_block,
    size_t page_size_bytes, bool has_max_erase_cycles,
    uint64_t max_erase_cycles, NandFlash **out);
void nand_flash_destroy(NandFlash *flash);
const char *nand_flash_status_message(NandFlashStatus status);
size_t nand_flash_block_count(const NandFlash *flash);
size_t nand_flash_pages_per_block(const NandFlash *flash);
size_t nand_flash_page_size_bytes(const NandFlash *flash);
/* Geometry getters return zero for NULL; valid geometry is always positive. */
NandFlashStatus nand_flash_read(const NandFlash *flash, size_t block,
    size_t page, uint8_t *out, size_t out_length);
NandFlashStatus nand_flash_program(NandFlash *flash, size_t block,
    size_t page, const uint8_t *data, size_t data_length);
NandFlashStatus nand_flash_erase_block(NandFlash *flash, size_t block);
NandFlashStatus nand_flash_erase_count(const NandFlash *flash, size_t block,
    uint64_t *out);
NandFlashStatus nand_flash_is_programmed(const NandFlash *flash, size_t block,
    size_t page, bool *out);

#ifdef __cplusplus
}
#endif
#endif
