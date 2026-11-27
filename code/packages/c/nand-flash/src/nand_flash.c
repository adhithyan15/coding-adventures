/* Logical SLC NAND: pages are bytes, and a single marker—not the payload—
 * records whether a page has consumed its one program opportunity. */
#include "nand_flash.h"

#include <stdlib.h>
#include <string.h>

#define NF_MAX_BYTES ((size_t)16777216)
#define NF_MAX_ERASE UINT64_C(9007199254740991)
_Static_assert(SIZE_MAX >= 16777216, "DB01 storage cap must fit size_t");

struct NandFlash {
    size_t blocks, pages_per_block, page_size;
    bool has_limit;
    uint64_t limit;
    uint8_t *pages;
    uint8_t *programmed;
    uint64_t *erase_counts;
};

/* Status text is part of DB01's cross-language contract. Keep the table in
 * enum order so callers can display the same refusal regardless of language. */
static const char *const messages[] = {
    "success", "invalid API argument", "geometry values are outside supported bounds",
    "block index out of range", "page index out of range", "page data must be bytes",
    "page data must match page_size_bytes", "page has already been programmed since erase",
    "block erase limit reached", "allocation failed"
};

const char *nand_flash_status_message(NandFlashStatus status) {
    if ((unsigned)status >= sizeof messages / sizeof messages[0]) return "unknown status";
    return messages[(unsigned)status];
}

NandFlashStatus nand_flash_create(size_t blocks, size_t ppb, size_t page_size,
    bool has_limit, uint64_t limit, NandFlash **out) {
    size_t pages, bytes;
    NandFlash *f;
    if (out == NULL) return NAND_FLASH_INVALID_ARGUMENT;
    *out = NULL;
    if (blocks < 1 || blocks > 1024 || ppb < 1 || ppb > 256 ||
        page_size < 1 || page_size > 65536 || (has_limit && limit > NF_MAX_ERASE))
        return NAND_FLASH_GEOMETRY_ERROR;
    /* Range checks bound this product to 1024 * 256. Division tests the
     * storage cap before the second multiplication, so even a huge supplied
     * page size cannot wrap an allocation length. */
    pages = blocks * ppb;
    if (pages > NF_MAX_BYTES / page_size) return NAND_FLASH_GEOMETRY_ERROR;
    bytes = pages * page_size;
    /* The compile-time assertion proves this capped result fits size_t. */
    f = (NandFlash *)calloc(1, sizeof *f);
    if (f == NULL) return NAND_FLASH_ALLOCATION_ERROR;
    f->pages = (uint8_t *)malloc(bytes);
    f->programmed = (uint8_t *)calloc(pages, sizeof *f->programmed);
    f->erase_counts = (uint64_t *)calloc(blocks, sizeof *f->erase_counts);
    if (f->pages == NULL || f->programmed == NULL || f->erase_counts == NULL) {
        free(f->pages); free(f->programmed); free(f->erase_counts); free(f);
        return NAND_FLASH_ALLOCATION_ERROR;
    }
    memset(f->pages, 0xFF, bytes);
    f->blocks = blocks; f->pages_per_block = ppb; f->page_size = page_size;
    f->has_limit = has_limit; f->limit = limit;
    *out = f;
    return NAND_FLASH_OK;
}

void nand_flash_destroy(NandFlash *f) {
    if (f != NULL) { free(f->pages); free(f->programmed); free(f->erase_counts); free(f); }
}
size_t nand_flash_block_count(const NandFlash *f) { return f == NULL ? 0 : f->blocks; }
size_t nand_flash_pages_per_block(const NandFlash *f) { return f == NULL ? 0 : f->pages_per_block; }
size_t nand_flash_page_size_bytes(const NandFlash *f) { return f == NULL ? 0 : f->page_size; }

static NandFlashStatus address(const NandFlash *f, size_t block, size_t page, size_t *index) {
    if (f == NULL) return NAND_FLASH_INVALID_ARGUMENT;
    if (block >= f->blocks) return NAND_FLASH_ADDRESS_BLOCK_ERROR;
    if (page >= f->pages_per_block) return NAND_FLASH_ADDRESS_PAGE_ERROR;
    /* All components are within validated geometry; this linearized index is
     * therefore below the already checked page count. */
    *index = block * f->pages_per_block + page;
    return NAND_FLASH_OK;
}

NandFlashStatus nand_flash_read(const NandFlash *f, size_t block, size_t page,
    uint8_t *out, size_t out_length) {
    size_t i; NandFlashStatus s = address(f, block, page, &i);
    if (s != NAND_FLASH_OK) return s;
    if (out == NULL) return NAND_FLASH_PAYLOAD_ERROR;
    if (out_length != f->page_size) return NAND_FLASH_PAYLOAD_SIZE_ERROR;
    /* Validation is complete before touching the caller's destination. */
    memcpy(out, f->pages + i * f->page_size, f->page_size);
    return NAND_FLASH_OK;
}

NandFlashStatus nand_flash_program(NandFlash *f, size_t block, size_t page,
    const uint8_t *data, size_t data_length) {
    size_t i; NandFlashStatus s = address(f, block, page, &i);
    if (s != NAND_FLASH_OK) return s;
    if (data == NULL) return NAND_FLASH_PAYLOAD_ERROR;
    if (data_length != f->page_size) return NAND_FLASH_PAYLOAD_SIZE_ERROR;
    if (f->programmed[i] != 0) return NAND_FLASH_PROGRAM_ERROR;
    /* One marker enforces the strict one-program rule, including all-FF
     * payloads. Copy then mark only after every refusal condition is cleared. */
    memcpy(f->pages + i * f->page_size, data, f->page_size);
    f->programmed[i] = 1;
    return NAND_FLASH_OK;
}

NandFlashStatus nand_flash_erase_block(NandFlash *f, size_t block) {
    size_t first, count;
    if (f == NULL) return NAND_FLASH_INVALID_ARGUMENT;
    if (block >= f->blocks) return NAND_FLASH_ADDRESS_BLOCK_ERROR;
    if (f->has_limit && f->erase_counts[block] >= f->limit) return NAND_FLASH_ERASE_LIMIT_ERROR;
    if (f->erase_counts[block] >= NF_MAX_ERASE) return NAND_FLASH_ERASE_LIMIT_ERROR;
    first = block * f->pages_per_block;
    count = f->pages_per_block * f->page_size;
    /* Geometry makes both offsets bounded. There are no fallible operations
     * after budget validation, so the whole-block transition is atomic from
     * the caller's point of view. */
    memset(f->pages + first * f->page_size, 0xFF, count);
    memset(f->programmed + first, 0, f->pages_per_block);
    f->erase_counts[block]++;
    return NAND_FLASH_OK;
}

NandFlashStatus nand_flash_erase_count(const NandFlash *f, size_t block, uint64_t *out) {
    if (f == NULL) return NAND_FLASH_INVALID_ARGUMENT;
    if (out == NULL) return NAND_FLASH_INVALID_ARGUMENT;
    if (block >= f->blocks) return NAND_FLASH_ADDRESS_BLOCK_ERROR;
    *out = f->erase_counts[block]; return NAND_FLASH_OK;
}

NandFlashStatus nand_flash_is_programmed(const NandFlash *f, size_t block, size_t page, bool *out) {
    size_t i; NandFlashStatus s;
    if (f == NULL) return NAND_FLASH_INVALID_ARGUMENT;
    if (out == NULL) return NAND_FLASH_INVALID_ARGUMENT;
    s = address(f, block, page, &i);
    if (s != NAND_FLASH_OK) return s;
    *out = f->programmed[i] != 0; return NAND_FLASH_OK;
}
