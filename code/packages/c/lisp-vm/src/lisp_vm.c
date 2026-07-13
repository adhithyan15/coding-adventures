/*
 * lisp_vm.c — implementation of the pure-ISO C Lisp bytecode VM.
 * =============================================================
 *
 * A stack machine over the compiler's bytecode. Values (`LcValue`) are cloned
 * onto the stack / into variables / into heap objects and freed as they are
 * consumed, so the whole VM is malloc-owned. Closures run via `execute_closure`,
 * which saves the caller's variable/local state, binds parameters, runs the
 * body, and — for a tail call — rebinds and restarts instead of recursing.
 */
#include "lisp_vm.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* strcmp, strlen, memcpy, memset */

static char *str_dup(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p != NULL) memcpy(p, s, n);
    return p;
}

/* ── Heap objects & VM state ───────────────────────────────────────────────*/

typedef struct {
    char *key;
    LcValue val;
} Binding;

typedef struct {
    char *name;
    size_t addr;
} SymEntry;

struct LvHeapObject {
    LvHeapKind kind;
    union {
        struct {
            LcValue car, cdr;
        } cons;
        struct {
            char *name;
        } symbol;
        struct {
            LcCodeObject *code; /* owned */
            Binding *env;       /* captured environment (owned) */
            size_t n_env;
            char **params; /* owned */
            size_t n_params;
        } closure;
    } as;
};

struct LispVm {
    LcValue *stack;
    size_t stack_n, stack_cap;
    Binding *vars; /* global variables */
    size_t vars_n, vars_cap;
    LcValue *locals;
    size_t locals_n, locals_cap;
    LvHeapObject *heap;
    size_t heap_n, heap_cap;
    SymEntry *symtab;
    size_t symtab_n, symtab_cap;
    size_t pc;
    int halted;
    char **output;
    size_t output_n, output_cap;
    size_t call_depth; /* native-recursion guard for non-tail calls */
    int errored;
    char errmsg[128];
};

/* Bound on nested non-tail closure calls. Tail calls loop in `execute_closure`
 * and don't grow the C stack, but a plain CALL_FUNCTION recurses natively; an
 * adversarial deep (non-tail) recursion would otherwise overflow the C stack
 * (undefined behaviour). This caps native depth and fails cleanly instead.
 * Matches the C++ port so a program errors at the same depth on both. */
#define LV_MAX_CALL_DEPTH 256

static void vm_fail(LispVm *vm, const char *msg) {
    if (vm->errored) return;
    vm->errored = 1;
    size_t i = 0;
    for (; msg[i] != '\0' && i + 1 < sizeof vm->errmsg; i++)
        vm->errmsg[i] = msg[i];
    vm->errmsg[i] = '\0';
}

/* ── Value stack ───────────────────────────────────────────────────────────*/

/* Push takes ownership of `v`. */
static void vm_push(LispVm *vm, LcValue v) {
    if (vm->errored) {
        lc_value_free(&v);
        return;
    }
    if (vm->stack_n == vm->stack_cap) {
        size_t nc = vm->stack_cap ? vm->stack_cap : 16;
        if (nc > ((size_t)-1) / 2 / sizeof(LcValue)) {
            lc_value_free(&v);
            vm_fail(vm, "VmError: out of memory");
            return;
        }
        nc *= 2;
        LcValue *ns = (LcValue *)realloc(vm->stack, nc * sizeof(LcValue));
        if (ns == NULL) {
            lc_value_free(&v);
            vm_fail(vm, "VmError: out of memory");
            return;
        }
        vm->stack = ns;
        vm->stack_cap = nc;
    }
    vm->stack[vm->stack_n++] = v;
}

/* Pop returns an owned value; on underflow returns Nil and fails. */
static LcValue vm_pop(LispVm *vm) {
    if (vm->stack_n == 0) {
        vm_fail(vm, "VmError: Stack underflow");
        LcValue nil;
        memset(&nil, 0, sizeof nil);
        nil.kind = LC_VAL_NIL;
        return nil;
    }
    return vm->stack[--vm->stack_n];
}

/* ── Variables (name → value map) ──────────────────────────────────────────*/

static LcValue *vars_get(LispVm *vm, const char *name) {
    for (size_t i = 0; i < vm->vars_n; i++)
        if (strcmp(vm->vars[i].key, name) == 0) return &vm->vars[i].val;
    return NULL;
}
/* Insert/replace; takes ownership of `val`. Returns 0 on OOM. */
static int vars_set(LispVm *vm, const char *name, LcValue val) {
    for (size_t i = 0; i < vm->vars_n; i++)
        if (strcmp(vm->vars[i].key, name) == 0) {
            lc_value_free(&vm->vars[i].val);
            vm->vars[i].val = val;
            return 1;
        }
    if (vm->vars_n == vm->vars_cap) {
        size_t nc = vm->vars_cap ? vm->vars_cap : 8;
        if (nc > ((size_t)-1) / 2 / sizeof(Binding)) {
            lc_value_free(&val);
            return 0;
        }
        nc *= 2;
        Binding *nv = (Binding *)realloc(vm->vars, nc * sizeof(Binding));
        if (nv == NULL) {
            lc_value_free(&val);
            return 0;
        }
        vm->vars = nv;
        vm->vars_cap = nc;
    }
    char *key = str_dup(name);
    if (key == NULL) {
        lc_value_free(&val);
        return 0;
    }
    vm->vars[vm->vars_n].key = key;
    vm->vars[vm->vars_n].val = val;
    vm->vars_n++;
    return 1;
}
static void bindings_free(Binding *b, size_t n) {
    if (b == NULL) return; /* nothing allocated (e.g. an OOM cleanup path) */
    for (size_t i = 0; i < n; i++) {
        free(b[i].key);
        lc_value_free(&b[i].val);
    }
    free(b);
}
/* Deep-copy the variable table into a fresh Binding array (owned). */
static int bindings_clone(const Binding *src, size_t n, Binding **out) {
    *out = NULL;
    if (n == 0) return 1;
    Binding *c = (Binding *)calloc(n, sizeof(Binding));
    if (c == NULL) return 0;
    for (size_t i = 0; i < n; i++) {
        c[i].key = str_dup(src[i].key);
        c[i].val = lc_value_clone(&src[i].val);
        if (c[i].key == NULL) {
            bindings_free(c, i + 1);
            return 0;
        }
    }
    *out = c;
    return 1;
}

/* ── Locals ────────────────────────────────────────────────────────────────*/

static void locals_free(LcValue *l, size_t n) {
    for (size_t i = 0; i < n; i++) lc_value_free(&l[i]);
    free(l);
}
static int locals_clone(const LcValue *src, size_t n, LcValue **out) {
    *out = NULL;
    if (n == 0) return 1;
    LcValue *c = (LcValue *)malloc(n * sizeof(LcValue));
    if (c == NULL) return 0;
    for (size_t i = 0; i < n; i++) c[i] = lc_value_clone(&src[i]);
    *out = c;
    return 1;
}

/* ── Heap ──────────────────────────────────────────────────────────────────*/

static void heap_object_free(LvHeapObject *o) {
    switch (o->kind) {
        case LV_CONS:
            lc_value_free(&o->as.cons.car);
            lc_value_free(&o->as.cons.cdr);
            break;
        case LV_SYMBOL:
            free(o->as.symbol.name);
            break;
        case LV_CLOSURE:
            lc_code_object_free(o->as.closure.code);
            free(o->as.closure.code);
            bindings_free(o->as.closure.env, o->as.closure.n_env);
            for (size_t i = 0; i < o->as.closure.n_params; i++)
                free(o->as.closure.params[i]);
            free(o->as.closure.params);
            break;
    }
}

/* Append a heap object (takes ownership) and return its address; SIZE_MAX on
 * OOM (with the object freed). */
static size_t heap_alloc(LispVm *vm, LvHeapObject obj) {
    if (vm->heap_n == vm->heap_cap) {
        size_t nc = vm->heap_cap ? vm->heap_cap : 16;
        if (nc > ((size_t)-1) / 2 / sizeof(LvHeapObject)) {
            heap_object_free(&obj);
            vm_fail(vm, "VmError: out of memory");
            return (size_t)-1;
        }
        nc *= 2;
        LvHeapObject *nh =
            (LvHeapObject *)realloc(vm->heap, nc * sizeof(LvHeapObject));
        if (nh == NULL) {
            heap_object_free(&obj);
            vm_fail(vm, "VmError: out of memory");
            return (size_t)-1;
        }
        vm->heap = nh;
        vm->heap_cap = nc;
    }
    size_t addr = vm->heap_n;
    vm->heap[vm->heap_n++] = obj;
    return addr;
}

static int is_valid_address(const LispVm *vm, size_t addr) {
    return addr < vm->heap_n;
}

static size_t intern_symbol(LispVm *vm, const char *name) {
    for (size_t i = 0; i < vm->symtab_n; i++)
        if (strcmp(vm->symtab[i].name, name) == 0) return vm->symtab[i].addr;
    LvHeapObject o;
    o.kind = LV_SYMBOL;
    o.as.symbol.name = str_dup(name);
    if (o.as.symbol.name == NULL) {
        vm_fail(vm, "VmError: out of memory");
        return (size_t)-1;
    }
    size_t addr = heap_alloc(vm, o);
    if (vm->errored) return (size_t)-1;
    if (vm->symtab_n == vm->symtab_cap) {
        size_t nc = vm->symtab_cap ? vm->symtab_cap : 8;
        if (nc > ((size_t)-1) / 2 / sizeof(SymEntry)) {
            vm_fail(vm, "VmError: out of memory");
            return addr;
        }
        nc = vm->symtab_cap ? nc * 2 : 8;
        SymEntry *nt = (SymEntry *)realloc(vm->symtab, nc * sizeof(SymEntry));
        if (nt == NULL) {
            vm_fail(vm, "VmError: out of memory");
            return addr;
        }
        vm->symtab = nt;
        vm->symtab_cap = nc;
    }
    vm->symtab[vm->symtab_n].name = str_dup(name);
    vm->symtab[vm->symtab_n].addr = addr;
    if (vm->symtab[vm->symtab_n].name != NULL) vm->symtab_n++;
    return addr;
}

/* ── Value constructors ────────────────────────────────────────────────────*/

static LcValue v_nil(void) {
    LcValue v;
    memset(&v, 0, sizeof v);
    v.kind = LC_VAL_NIL;
    return v;
}
static LcValue v_int(int64_t n) {
    LcValue v = v_nil();
    v.kind = LC_VAL_INTEGER;
    v.integer = n;
    return v;
}
static LcValue v_bool(int b) {
    LcValue v = v_nil();
    v.kind = LC_VAL_BOOL;
    v.boolean = b ? 1 : 0;
    return v;
}
static LcValue v_cons_addr(size_t a) {
    LcValue v = v_nil();
    v.kind = LC_VAL_CONS_ADDR;
    v.addr = a;
    return v;
}
static LcValue v_closure_addr(size_t a) {
    LcValue v = v_nil();
    v.kind = LC_VAL_CLOSURE_ADDR;
    v.addr = a;
    return v;
}

/* ── Value formatting ──────────────────────────────────────────────────────*/

typedef struct {
    char *buf;
    size_t len, cap;
    int oom;
} Sb;
static void sb_puts(Sb *sb, const char *s) {
    if (sb->oom) return;
    size_t n = strlen(s);
    if (sb->len + n + 1 > sb->cap) {
        size_t nc = sb->cap ? sb->cap : 32;
        while (nc < sb->len + n + 1) {
            if (nc > ((size_t)-1) / 2) {
                sb->oom = 1;
                return;
            }
            nc *= 2;
        }
        char *nb = (char *)realloc(sb->buf, nc);
        if (nb == NULL) {
            sb->oom = 1;
            return;
        }
        sb->buf = nb;
        sb->cap = nc;
    }
    memcpy(sb->buf + sb->len, s, n);
    sb->len += n;
    sb->buf[sb->len] = '\0';
}

static void format_into(const LispVm *vm, const LcValue *v, Sb *sb,
                        size_t *visited, size_t *n_visited);

/* Format the cons list starting at `addr`. */
static void format_cons(const LispVm *vm, size_t addr, Sb *sb, size_t *visited,
                        size_t *n_visited) {
    Sb parts;
    memset(&parts, 0, sizeof parts);
    int first = 1;
    size_t current = addr;
    for (;;) {
        for (size_t i = 0; i < *n_visited; i++)
            if (visited[i] == current) {
                if (!first) sb_puts(&parts, " ");
                sb_puts(&parts, "...");
                sb_puts(sb, "(");
                sb_puts(sb, parts.buf ? parts.buf : "");
                sb_puts(sb, ")");
                free(parts.buf);
                return;
            }
        visited[(*n_visited)++] = current;
        if (!is_valid_address(vm, current) ||
            vm->heap[current].kind != LV_CONS) {
            sb_puts(sb, "(");
            sb_puts(sb, parts.buf ? parts.buf : "");
            sb_puts(sb, ")");
            free(parts.buf);
            return;
        }
        const LvHeapObject *cell = &vm->heap[current];
        if (!first) sb_puts(&parts, " ");
        first = 0;
        format_into(vm, &cell->as.cons.car, &parts, visited, n_visited);

        const LcValue *cdr = &cell->as.cons.cdr;
        if (cdr->kind == LC_VAL_NIL) {
            sb_puts(sb, "(");
            sb_puts(sb, parts.buf ? parts.buf : "");
            sb_puts(sb, ")");
            free(parts.buf);
            return;
        }
        if (cdr->kind == LC_VAL_CONS_ADDR && is_valid_address(vm, cdr->addr) &&
            vm->heap[cdr->addr].kind == LV_CONS) {
            current = cdr->addr;
            continue;
        }
        /* dotted tail */
        Sb tail;
        memset(&tail, 0, sizeof tail);
        format_into(vm, cdr, &tail, visited, n_visited);
        sb_puts(sb, "(");
        sb_puts(sb, parts.buf ? parts.buf : "");
        sb_puts(sb, " . ");
        sb_puts(sb, tail.buf ? tail.buf : "");
        sb_puts(sb, ")");
        free(parts.buf);
        free(tail.buf);
        return;
    }
}

static void format_into(const LispVm *vm, const LcValue *v, Sb *sb,
                        size_t *visited, size_t *n_visited) {
    char num[64];
    switch (v->kind) {
        case LC_VAL_NIL: sb_puts(sb, "nil"); break;
        case LC_VAL_BOOL: sb_puts(sb, v->boolean ? "t" : "nil"); break;
        case LC_VAL_INTEGER:
            snprintf(num, sizeof num, "%lld", (long long)v->integer);
            sb_puts(sb, num);
            break;
        case LC_VAL_STRING:
        case LC_VAL_SYMBOL: sb_puts(sb, v->str ? v->str : ""); break;
        case LC_VAL_CONS_ADDR:
            if (is_valid_address(vm, v->addr)) {
                const LvHeapObject *o = &vm->heap[v->addr];
                if (o->kind == LV_CONS)
                    format_cons(vm, v->addr, sb, visited, n_visited);
                else if (o->kind == LV_SYMBOL)
                    sb_puts(sb, o->as.symbol.name);
                else {
                    snprintf(num, sizeof num, "<closure @%zu>", v->addr);
                    sb_puts(sb, num);
                }
            } else {
                snprintf(num, sizeof num, "<invalid @%zu>", v->addr);
                sb_puts(sb, num);
            }
            break;
        case LC_VAL_CLOSURE_ADDR:
            snprintf(num, sizeof num, "<closure @%zu>", v->addr);
            sb_puts(sb, num);
            break;
        case LC_VAL_CODE: sb_puts(sb, "<code>"); break;
    }
}

char *lv_format_value(const LispVm *vm, const LcValue *v) {
    Sb sb;
    memset(&sb, 0, sizeof sb);
    /* `visited` is bounded by the heap size (cons cells are the only cycles). */
    size_t cap = vm->heap_n + 1;
    size_t *visited = (size_t *)malloc(cap * sizeof(size_t));
    size_t n_visited = 0;
    if (visited == NULL) return str_dup("");
    format_into(vm, v, &sb, visited, &n_visited);
    free(visited);
    if (sb.oom) {
        free(sb.buf);
        return str_dup("");
    }
    return sb.buf ? sb.buf : str_dup("");
}

/* ── Execution ─────────────────────────────────────────────────────────────*/

static void execute_instruction(LispVm *vm, const LcInstruction *instr,
                                 const LcCodeObject *code);
static void execute_closure(LispVm *vm, size_t closure_addr, LcValue *args,
                            size_t n_args);

static void binary_int(LispVm *vm, LcOp op) {
    LcValue b = vm_pop(vm);
    LcValue a = vm_pop(vm);
    if (a.kind == LC_VAL_INTEGER && b.kind == LC_VAL_INTEGER) {
        if (op == LC_DIV && b.integer == 0) {
            vm_fail(vm, "VmError: Division by zero");
        } else {
            int64_t r = 0;
            switch (op) {
                case LC_ADD: r = a.integer + b.integer; break;
                case LC_SUB: r = a.integer - b.integer; break;
                case LC_MUL: r = a.integer * b.integer; break;
                case LC_DIV: r = a.integer / b.integer; break;
                case LC_CMP_LT: r = a.integer < b.integer ? 1 : 0; break;
                case LC_CMP_GT: r = a.integer > b.integer ? 1 : 0; break;
                default: break;
            }
            vm_push(vm, v_int(r));
        }
    } else {
        vm_fail(vm, "VmError: expected two integers");
    }
    lc_value_free(&a);
    lc_value_free(&b);
}

static void execute_instruction(LispVm *vm, const LcInstruction *instr,
                                 const LcCodeObject *code) {
    size_t idx = instr->operand;
    switch (instr->opcode) {
        case LC_LOAD_CONST:
            if (idx >= code->n_constants) {
                vm_fail(vm, "VmError: Constant index out of bounds");
            } else {
                vm_push(vm, lc_value_clone(&code->constants[idx]));
            }
            vm->pc++;
            break;
        case LC_POP: {
            LcValue v = vm_pop(vm);
            lc_value_free(&v);
            vm->pc++;
            break;
        }
        case LC_LOAD_NIL: vm_push(vm, v_nil()); vm->pc++; break;
        case LC_LOAD_TRUE: vm_push(vm, v_bool(1)); vm->pc++; break;
        case LC_STORE_NAME: {
            if (idx >= code->n_names) {
                vm_fail(vm, "VmError: Name index out of bounds");
            } else {
                LcValue v = vm_pop(vm);
                if (!vars_set(vm, code->names[idx], v))
                    vm_fail(vm, "VmError: out of memory");
            }
            vm->pc++;
            break;
        }
        case LC_LOAD_NAME: {
            if (idx >= code->n_names) {
                vm_fail(vm, "VmError: Name index out of bounds");
            } else {
                LcValue *found = vars_get(vm, code->names[idx]);
                if (found == NULL)
                    vm_fail(vm, "VmError: Undefined variable");
                else
                    vm_push(vm, lc_value_clone(found));
            }
            vm->pc++;
            break;
        }
        case LC_STORE_LOCAL: {
            LcValue v = vm_pop(vm);
            while (vm->locals_n <= idx && !vm->errored) {
                if (vm->locals_n == vm->locals_cap) {
                    size_t nc = vm->locals_cap ? vm->locals_cap : 8;
                    if (nc > ((size_t)-1) / 2 / sizeof(LcValue)) {
                        vm_fail(vm, "VmError: out of memory");
                        break;
                    }
                    nc = vm->locals_cap ? nc * 2 : 8;
                    LcValue *nl =
                        (LcValue *)realloc(vm->locals, nc * sizeof(LcValue));
                    if (nl == NULL) {
                        vm_fail(vm, "VmError: out of memory");
                        break;
                    }
                    vm->locals = nl;
                    vm->locals_cap = nc;
                }
                vm->locals[vm->locals_n++] = v_nil();
            }
            if (!vm->errored && idx < vm->locals_n) {
                lc_value_free(&vm->locals[idx]);
                vm->locals[idx] = v;
            } else {
                lc_value_free(&v);
            }
            vm->pc++;
            break;
        }
        case LC_LOAD_LOCAL:
            vm_push(vm, idx < vm->locals_n ? lc_value_clone(&vm->locals[idx])
                                           : v_nil());
            vm->pc++;
            break;
        case LC_ADD:
        case LC_SUB:
        case LC_MUL:
        case LC_DIV:
        case LC_CMP_LT:
        case LC_CMP_GT: binary_int(vm, instr->opcode); vm->pc++; break;
        case LC_CMP_EQ: {
            LcValue b = vm_pop(vm);
            LcValue a = vm_pop(vm);
            int r;
            if (a.kind == LC_VAL_NIL && b.kind == LC_VAL_NIL)
                r = 1;
            else if (a.kind == LC_VAL_NIL || b.kind == LC_VAL_NIL)
                r = 0;
            else
                r = lc_value_equal(&a, &b) ? 1 : 0;
            lc_value_free(&a);
            lc_value_free(&b);
            vm_push(vm, v_int(r));
            vm->pc++;
            break;
        }
        case LC_JUMP: vm->pc = idx; break;
        case LC_JUMP_IF_FALSE: {
            LcValue v = vm_pop(vm);
            int falsy = lc_value_is_falsy(&v);
            lc_value_free(&v);
            vm->pc = falsy ? idx : vm->pc + 1;
            break;
        }
        case LC_JUMP_IF_TRUE: {
            LcValue v = vm_pop(vm);
            int falsy = lc_value_is_falsy(&v);
            lc_value_free(&v);
            vm->pc = !falsy ? idx : vm->pc + 1;
            break;
        }
        case LC_MAKE_CLOSURE: {
            size_t param_count = idx;
            LcValue code_val = vm_pop(vm);
            if (code_val.kind != LC_VAL_CODE || code_val.code == NULL) {
                /* NULL code means a prior clone hit OOM; treat as an error
                 * rather than dereferencing it below. */
                vm_fail(vm, "VmError: MAKE_CLOSURE expected CodeObject");
                lc_value_free(&code_val);
                vm->pc++;
                break;
            }
            LcCodeObject *fc = code_val.code; /* take ownership */
            code_val.code = NULL;
            /* Extract parameter names from the trailing string constants. */
            char **params = NULL;
            if (param_count > 0) {
                params = (char **)calloc(param_count, sizeof(char *));
                if (params == NULL) vm_fail(vm, "VmError: out of memory");
            }
            for (size_t i = 0; i < param_count && !vm->errored; i++) {
                const char *pname = NULL;
                char fallback[24];
                if (fc->n_constants >= param_count) {
                    size_t start = fc->n_constants - param_count;
                    const LcValue *cv = &fc->constants[start + i];
                    if (cv->kind == LC_VAL_STRING) pname = cv->str;
                }
                if (pname == NULL) {
                    snprintf(fallback, sizeof fallback, "_p%zu", i);
                    pname = fallback;
                }
                params[i] = str_dup(pname);
                if (params[i] == NULL) vm_fail(vm, "VmError: out of memory");
            }
            /* Capture the current environment. */
            Binding *env = NULL;
            if (!vm->errored && !bindings_clone(vm->vars, vm->vars_n, &env))
                vm_fail(vm, "VmError: out of memory");
            if (vm->errored) {
                lc_code_object_free(fc);
                free(fc);
                for (size_t i = 0; i < param_count; i++) free(params ? params[i] : NULL);
                free(params);
                bindings_free(env, vm->vars_n);
                vm->pc++;
                break;
            }
            LvHeapObject o;
            o.kind = LV_CLOSURE;
            o.as.closure.code = fc;
            o.as.closure.env = env;
            o.as.closure.n_env = vm->vars_n;
            o.as.closure.params = params;
            o.as.closure.n_params = param_count;
            size_t addr = heap_alloc(vm, o);
            if (!vm->errored) vm_push(vm, v_closure_addr(addr));
            vm->pc++;
            break;
        }
        case LC_CALL_FUNCTION:
        case LC_TAIL_CALL: {
            size_t argc = idx;
            LcValue func = vm_pop(vm);
            LcValue *args = argc > 0 ? (LcValue *)malloc(argc * sizeof(LcValue))
                                     : NULL;
            if (argc > 0 && args == NULL) {
                vm_fail(vm, "VmError: out of memory");
                lc_value_free(&func);
                vm->pc++;
                break;
            }
            /* Arguments were pushed left-to-right; pop into reverse order. */
            for (size_t i = 0; i < argc; i++) args[argc - 1 - i] = vm_pop(vm);
            if (func.kind == LC_VAL_CLOSURE_ADDR) {
                execute_closure(vm, func.addr, args, argc);
            } else {
                vm_fail(vm, "VmError: cannot call non-closure");
                for (size_t i = 0; i < argc; i++) lc_value_free(&args[i]);
                free(args);
            }
            lc_value_free(&func);
            break;
        }
        case LC_RETURN: vm->pc++; break; /* handled by execute_closure */
        case LC_CONS: {
            LcValue car = vm_pop(vm);
            LcValue cdr = vm_pop(vm);
            LvHeapObject o;
            o.kind = LV_CONS;
            o.as.cons.car = car;
            o.as.cons.cdr = cdr;
            size_t addr = heap_alloc(vm, o);
            if (!vm->errored) vm_push(vm, v_cons_addr(addr));
            vm->pc++;
            break;
        }
        case LC_CAR:
        case LC_CDR: {
            LcValue addr_val = vm_pop(vm);
            if (addr_val.kind == LC_VAL_CONS_ADDR &&
                is_valid_address(vm, addr_val.addr) &&
                vm->heap[addr_val.addr].kind == LV_CONS) {
                const LvHeapObject *cell = &vm->heap[addr_val.addr];
                vm_push(vm, lc_value_clone(instr->opcode == LC_CAR
                                               ? &cell->as.cons.car
                                               : &cell->as.cons.cdr));
            } else {
                vm_fail(vm, "VmError: not a cons cell");
            }
            lc_value_free(&addr_val);
            vm->pc++;
            break;
        }
        case LC_MAKE_SYMBOL: {
            if (idx >= code->n_constants ||
                code->constants[idx].kind != LC_VAL_STRING) {
                vm_fail(vm, "VmError: MAKE_SYMBOL constant is not a string");
            } else {
                size_t addr = intern_symbol(vm, code->constants[idx].str);
                if (!vm->errored) vm_push(vm, v_cons_addr(addr));
            }
            vm->pc++;
            break;
        }
        case LC_IS_ATOM: {
            LcValue v = vm_pop(vm);
            int r = 1;
            if (v.kind == LC_VAL_CONS_ADDR && is_valid_address(vm, v.addr) &&
                vm->heap[v.addr].kind == LV_CONS)
                r = 0;
            lc_value_free(&v);
            vm_push(vm, v_int(r));
            vm->pc++;
            break;
        }
        case LC_IS_NIL: {
            LcValue v = vm_pop(vm);
            int r = v.kind == LC_VAL_NIL ? 1 : 0;
            lc_value_free(&v);
            vm_push(vm, v_int(r));
            vm->pc++;
            break;
        }
        case LC_PRINT: {
            LcValue v = vm_pop(vm);
            char *text = lv_format_value(vm, &v);
            lc_value_free(&v);
            if (text != NULL) {
                if (vm->output_n == vm->output_cap) {
                    size_t nc = vm->output_cap ? vm->output_cap : 8;
                    if (nc > ((size_t)-1) / 2 / sizeof(char *)) {
                        free(text);
                        vm_fail(vm, "VmError: out of memory");
                        vm->pc++;
                        break;
                    }
                    nc = vm->output_cap ? nc * 2 : 8;
                    char **no = (char **)realloc(vm->output, nc * sizeof(char *));
                    if (no == NULL) {
                        free(text);
                        vm_fail(vm, "VmError: out of memory");
                        vm->pc++;
                        break;
                    }
                    vm->output = no;
                    vm->output_cap = nc;
                }
                vm->output[vm->output_n++] = text;
            }
            vm->pc++;
            break;
        }
        case LC_HALT: vm->halted = 1; break;
    }
}

static void execute_closure(LispVm *vm, size_t closure_addr, LcValue *args,
                            size_t n_args) {
    if (!is_valid_address(vm, closure_addr) ||
        vm->heap[closure_addr].kind != LV_CLOSURE) {
        vm_fail(vm, "VmError: not a closure");
        for (size_t i = 0; i < n_args; i++) lc_value_free(&args[i]);
        free(args);
        return;
    }
    if (vm->call_depth >= LV_MAX_CALL_DEPTH) {
        vm_fail(vm, "VmError: call stack exhausted");
        for (size_t i = 0; i < n_args; i++) lc_value_free(&args[i]);
        free(args);
        return;
    }
    vm->call_depth++;

    /* Save caller state. */
    size_t saved_pc = vm->pc;
    int saved_halted = vm->halted;
    Binding *saved_vars = NULL;
    LcValue *saved_locals = NULL;
    if (!bindings_clone(vm->vars, vm->vars_n, &saved_vars)) {
        vm_fail(vm, "VmError: out of memory");
        for (size_t i = 0; i < n_args; i++) lc_value_free(&args[i]);
        free(args);
        vm->call_depth--;
        return;
    }
    size_t saved_vars_n = vm->vars_n;
    if (!locals_clone(vm->locals, vm->locals_n, &saved_locals)) {
        vm_fail(vm, "VmError: out of memory");
        bindings_free(saved_vars, saved_vars_n);
        for (size_t i = 0; i < n_args; i++) lc_value_free(&args[i]);
        free(args);
        vm->call_depth--;
        return;
    }
    size_t saved_locals_n = vm->locals_n;

    /* Restore the closure's captured environment on top of current vars. */
    const LvHeapObject *cl = &vm->heap[closure_addr];
    for (size_t i = 0; i < cl->as.closure.n_env && !vm->errored; i++)
        vars_set(vm, cl->as.closure.env[i].key,
                 lc_value_clone(&cl->as.closure.env[i].val));

    /* TCO loop: `cur_addr`/`cur_args` are the closure and args to run. */
    size_t cur_addr = closure_addr;
    LcValue *cur_args = args;
    size_t cur_nargs = n_args;
    LcValue return_value = v_nil();

    while (!vm->errored) {
        const LvHeapObject *closure = &vm->heap[cur_addr];
        const LcCodeObject *body = closure->as.closure.code;

        /* locals = cur_args (take ownership) */
        locals_free(vm->locals, vm->locals_n);
        vm->locals = cur_args;
        vm->locals_n = cur_nargs;
        vm->locals_cap = cur_nargs;
        cur_args = NULL;
        vm->pc = 0;
        vm->halted = 0;

        /* Bind parameters into variables (clone from locals). */
        for (size_t i = 0; i < closure->as.closure.n_params && !vm->errored;
             i++)
            if (i < vm->locals_n)
                vars_set(vm, closure->as.closure.params[i],
                         lc_value_clone(&vm->locals[i]));

        int did_tail = 0;
        size_t tail_addr = 0;
        LcValue *tail_args = NULL;
        size_t tail_nargs = 0;

        while (!vm->errored && !vm->halted && vm->pc < body->n_instructions) {
            LcInstruction ins = body->instructions[vm->pc];
            if (ins.opcode == LC_RETURN) {
                lc_value_free(&return_value);
                return_value = vm->stack_n > 0 ? vm_pop(vm) : v_nil();
                break;
            }
            if (ins.opcode == LC_HALT) break;
            if (ins.opcode == LC_TAIL_CALL) {
                size_t argc = ins.operand;
                LcValue func = vm_pop(vm);
                LcValue *na = argc > 0
                                  ? (LcValue *)malloc(argc * sizeof(LcValue))
                                  : NULL;
                if (argc > 0 && na == NULL) {
                    vm_fail(vm, "VmError: out of memory");
                    lc_value_free(&func);
                    break;
                }
                for (size_t i = 0; i < argc; i++) na[argc - 1 - i] = vm_pop(vm);
                if (func.kind == LC_VAL_CLOSURE_ADDR &&
                    is_valid_address(vm, func.addr) &&
                    vm->heap[func.addr].kind == LV_CLOSURE) {
                    did_tail = 1;
                    tail_addr = func.addr;
                    tail_args = na;
                    tail_nargs = argc;
                } else {
                    vm_fail(vm, "VmError: cannot tail-call non-closure");
                    for (size_t i = 0; i < argc; i++) lc_value_free(&na[i]);
                    free(na);
                }
                lc_value_free(&func);
                break;
            }
            /* `body` is stable (closures live for the VM's lifetime). */
            execute_instruction(vm, &ins, body);
        }

        if (did_tail && !vm->errored) {
            /* Reset vars to caller-saved + new closure's env, then loop. */
            const LvHeapObject *nc = &vm->heap[tail_addr];
            bindings_free(vm->vars, vm->vars_n);
            vm->vars = NULL;
            vm->vars_n = vm->vars_cap = 0;
            Binding *rv = NULL;
            if (!bindings_clone(saved_vars, saved_vars_n, &rv)) {
                vm_fail(vm, "VmError: out of memory");
                for (size_t i = 0; i < tail_nargs; i++) lc_value_free(&tail_args[i]);
                free(tail_args);
                break;
            }
            vm->vars = rv;
            vm->vars_n = saved_vars_n;
            vm->vars_cap = saved_vars_n;
            for (size_t i = 0; i < nc->as.closure.n_env && !vm->errored; i++)
                vars_set(vm, nc->as.closure.env[i].key,
                         lc_value_clone(&nc->as.closure.env[i].val));
            cur_addr = tail_addr;
            cur_args = tail_args;
            cur_nargs = tail_nargs;
            continue;
        }
        if (did_tail) { /* errored during tail setup */
            for (size_t i = 0; i < tail_nargs; i++) lc_value_free(&tail_args[i]);
            free(tail_args);
        }
        break; /* normal return */
    }

    /* Restore caller state. */
    bindings_free(vm->vars, vm->vars_n);
    vm->vars = saved_vars;
    vm->vars_n = saved_vars_n;
    vm->vars_cap = saved_vars_n;
    locals_free(vm->locals, vm->locals_n);
    vm->locals = saved_locals;
    vm->locals_n = saved_locals_n;
    vm->locals_cap = saved_locals_n;
    vm->pc = saved_pc;
    vm->halted = saved_halted;

    if (cur_args != NULL) { /* unconsumed on an error path */
        for (size_t i = 0; i < cur_nargs; i++) lc_value_free(&cur_args[i]);
        free(cur_args);
    }

    vm_push(vm, return_value);
    vm->pc++;
    vm->call_depth--;
}

/* ── Public API ────────────────────────────────────────────────────────────*/

LispVm *lv_new(void) { return (LispVm *)calloc(1, sizeof(LispVm)); }

void lv_free(LispVm *vm) {
    if (vm == NULL) return;
    for (size_t i = 0; i < vm->stack_n; i++) lc_value_free(&vm->stack[i]);
    free(vm->stack);
    bindings_free(vm->vars, vm->vars_n);
    locals_free(vm->locals, vm->locals_n);
    for (size_t i = 0; i < vm->heap_n; i++) heap_object_free(&vm->heap[i]);
    free(vm->heap);
    for (size_t i = 0; i < vm->symtab_n; i++) free(vm->symtab[i].name);
    free(vm->symtab);
    for (size_t i = 0; i < vm->output_n; i++) free(vm->output[i]);
    free(vm->output);
    free(vm);
}

int lv_execute(LispVm *vm, const LcCodeObject *code, LvError *err) {
    vm->pc = 0;
    vm->halted = 0;
    while (!vm->errored && !vm->halted && vm->pc < code->n_instructions) {
        LcInstruction ins = code->instructions[vm->pc];
        execute_instruction(vm, &ins, code);
    }
    if (vm->errored) {
        if (err != NULL) {
            size_t k = 0;
            for (; vm->errmsg[k] != '\0' && k + 1 < sizeof err->message; k++)
                err->message[k] = vm->errmsg[k];
            err->message[k] = '\0';
        }
        return 0;
    }
    return 1;
}

size_t lv_stack_len(const LispVm *vm) { return vm->stack_n; }
const LcValue *lv_stack_at(const LispVm *vm, size_t i) {
    return i < vm->stack_n ? &vm->stack[i] : NULL;
}
const LcValue *lv_stack_top(const LispVm *vm) {
    return vm->stack_n > 0 ? &vm->stack[vm->stack_n - 1] : NULL;
}
size_t lv_heap_len(const LispVm *vm) { return vm->heap_n; }
const LvHeapObject *lv_heap_at(const LispVm *vm, size_t addr) {
    return addr < vm->heap_n ? &vm->heap[addr] : NULL;
}
LvHeapKind lv_heap_kind(const LvHeapObject *o) { return o->kind; }
const LcValue *lv_cons_car(const LvHeapObject *o) { return &o->as.cons.car; }
const LcValue *lv_cons_cdr(const LvHeapObject *o) { return &o->as.cons.cdr; }
const char *lv_symbol_name(const LvHeapObject *o) { return o->as.symbol.name; }
size_t lv_output_len(const LispVm *vm) { return vm->output_n; }
const char *lv_output_at(const LispVm *vm, size_t i) {
    return i < vm->output_n ? vm->output[i] : NULL;
}

int lv_run(const char *source, LcValue *out, LvError *err) {
    LcCodeObject code;
    LcCompileError cerr;
    if (!lc_compile(source, &code, &cerr)) {
        if (err != NULL)
            snprintf(err->message, sizeof err->message, "VmError: %s",
                     cerr.message);
        return 0;
    }
    LispVm *vm = lv_new();
    int ok = vm != NULL && lv_execute(vm, &code, err);
    if (ok) {
        const LcValue *top = lv_stack_top(vm);
        *out = top != NULL ? lc_value_clone(top) : v_nil();
    } else if (vm == NULL && err != NULL) {
        snprintf(err->message, sizeof err->message, "VmError: out of memory");
    }
    lv_free(vm);
    lc_code_object_free(&code);
    return ok;
}

int lv_run_with_output(const char *source, LcValue *out, char ***out_lines,
                       size_t *n_lines, LvError *err) {
    *out_lines = NULL;
    *n_lines = 0;
    LcCodeObject code;
    LcCompileError cerr;
    if (!lc_compile(source, &code, &cerr)) {
        if (err != NULL)
            snprintf(err->message, sizeof err->message, "VmError: %s",
                     cerr.message);
        return 0;
    }
    LispVm *vm = lv_new();
    int ok = vm != NULL && lv_execute(vm, &code, err);
    if (ok) {
        const LcValue *top = lv_stack_top(vm);
        *out = top != NULL ? lc_value_clone(top) : v_nil();
        if (vm->output_n > 0) {
            char **lines = (char **)malloc(vm->output_n * sizeof(char *));
            if (lines != NULL) {
                for (size_t i = 0; i < vm->output_n; i++)
                    lines[i] = str_dup(vm->output[i]);
                *out_lines = lines;
                *n_lines = vm->output_n;
            }
        }
    }
    lv_free(vm);
    lc_code_object_free(&code);
    return ok;
}
