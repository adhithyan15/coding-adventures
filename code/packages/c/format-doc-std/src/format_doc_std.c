/*
 * format_doc_std.c — implementation of the pure-ISO C formatter templates.
 * =======================================================================
 *
 * Each template composes format-doc primitives (concat/group/indent/line/
 * softline/join/if_break) into a common syntax shape. Content documents flow in
 * and are consumed by the builders; a config's delimiter documents are borrowed
 * and cloned where the shape needs them more than once.
 */
#include "format_doc_std.h"

#include <stdlib.h> /* malloc, free */

const char *const fds_version = "0.1.0";

FdsBlockLikeConfig fds_block_like_config_default(void) {
    FdsBlockLikeConfig c = {1};
    return c;
}
FdsInfixChainConfig fds_infix_chain_config_default(void) {
    FdsInfixChainConfig c = {0};
    return c;
}

/* The trailing-separator document for a delimited list. Returns an owned doc. */
static FdDoc *trailing_doc(const FdDoc *separator, FdsTrailingSeparator t) {
    switch (t) {
        case FDS_TRAILING_ALWAYS:
            return fd_clone(separator);
        case FDS_TRAILING_IF_BREAK:
            return fd_if_break(fd_clone(separator), fd_nil());
        case FDS_TRAILING_NEVER:
        default:
            return fd_nil();
    }
}

FdDoc *fds_delimited_list_with(FdDoc *open, FdDoc **items, size_t n,
                               FdDoc *close,
                               const FdsDelimitedListConfig *config) {
    if (n == 0) {
        FdDoc *mid = config->empty_spacing ? fd_text(" ") : fd_nil();
        FdDoc *parts[3] = {open, mid, close};
        return fd_concat(parts, 3);
    }

    /* body = join(concat([separator, line()]), items) */
    FdDoc *joinsep_parts[2] = {fd_clone(config->separator), fd_line()};
    FdDoc *joinsep = fd_concat(joinsep_parts, 2);
    FdDoc *body = fd_join(joinsep, items, n);
    FdDoc *trailing = trailing_doc(config->separator, config->trailing_separator);

    /* group(concat([open, indent(concat([softline, body, trailing]), 1),
     *               softline, close])) */
    FdDoc *inner_parts[3] = {fd_softline(), body, trailing};
    FdDoc *indented = fd_indent(fd_concat(inner_parts, 3), 1);
    FdDoc *outer_parts[4] = {open, indented, fd_softline(), close};
    return fd_group(fd_concat(outer_parts, 4));
}

FdDoc *fds_delimited_list(FdDoc *open, FdDoc **items, size_t n, FdDoc *close) {
    FdDoc *comma = fd_text(",");
    FdsDelimitedListConfig cfg = {comma, FDS_TRAILING_NEVER, 0};
    FdDoc *result = fds_delimited_list_with(open, items, n, close, &cfg);
    fd_free(comma);
    return result;
}

FdDoc *fds_call_like(FdDoc *callee, FdDoc **args, size_t n,
                     const FdsCallLikeConfig *config) {
    FdsDelimitedListConfig list_cfg = {config->separator,
                                       config->trailing_separator, 0};
    FdDoc *list = fds_delimited_list_with(
        fd_clone(config->open), args, n, fd_clone(config->close), &list_cfg);
    FdDoc *parts[2] = {callee, list};
    return fd_concat(parts, 2);
}

FdDoc *fds_block_like_with(FdDoc *open, FdDoc *body, FdDoc *close,
                           const FdsBlockLikeConfig *config) {
    if (fd_is_nil(body)) {
        fd_free(body); /* the empty body is dropped */
        FdDoc *mid = config->empty_spacing ? fd_text(" ") : fd_nil();
        FdDoc *parts[3] = {open, mid, close};
        return fd_concat(parts, 3);
    }
    /* group(concat([open, indent(concat([line(), body]), 1), line(), close])) */
    FdDoc *inner_parts[2] = {fd_line(), body};
    FdDoc *indented = fd_indent(fd_concat(inner_parts, 2), 1);
    FdDoc *outer_parts[4] = {open, indented, fd_line(), close};
    return fd_group(fd_concat(outer_parts, 4));
}

FdDoc *fds_block_like(FdDoc *open, FdDoc *body, FdDoc *close) {
    FdsBlockLikeConfig cfg = fds_block_like_config_default();
    return fds_block_like_with(open, body, close, &cfg);
}

FdDoc *fds_infix_chain(FdDoc **operands, size_t n_operands, FdDoc **operators,
                       size_t n_operators, const FdsInfixChainConfig *config) {
    if (n_operands == 0) {
        for (size_t i = 0; i < n_operators; i++) fd_free(operators[i]);
        return fd_nil();
    }
    /* operators must number exactly one fewer than the operands. */
    if (n_operators != n_operands - 1) {
        for (size_t i = 0; i < n_operands; i++) fd_free(operands[i]);
        for (size_t i = 0; i < n_operators; i++) fd_free(operators[i]);
        return NULL;
    }
    if (n_operands == 1) return operands[0];

    /* rest = for each (operator, next operand): break-before or break-after. */
    if (n_operators > ((size_t)-1) / 4 / sizeof(FdDoc *)) {
        for (size_t i = 0; i < n_operands; i++) fd_free(operands[i]);
        for (size_t i = 0; i < n_operators; i++) fd_free(operators[i]);
        return NULL;
    }
    FdDoc **rest = (FdDoc **)malloc(n_operators * 4 * sizeof(FdDoc *));
    if (rest == NULL) {
        for (size_t i = 0; i < n_operands; i++) fd_free(operands[i]);
        for (size_t i = 0; i < n_operators; i++) fd_free(operators[i]);
        return NULL;
    }
    size_t ri = 0;
    for (size_t i = 0; i < n_operators; i++) {
        if (config->break_before_operators) {
            /* <line><op><space><operand> */
            rest[ri++] = fd_line();
            rest[ri++] = operators[i];
            rest[ri++] = fd_text(" ");
            rest[ri++] = operands[i + 1];
        } else {
            /* <space><op><line><operand> */
            rest[ri++] = fd_text(" ");
            rest[ri++] = operators[i];
            rest[ri++] = fd_line();
            rest[ri++] = operands[i + 1];
        }
    }
    FdDoc *rest_doc = fd_concat(rest, ri);
    free(rest);
    FdDoc *indented = fd_indent(rest_doc, 1);
    FdDoc *outer_parts[2] = {operands[0], indented};
    return fd_group(fd_concat(outer_parts, 2));
}
