/*
 * pbt_budget.c -- the shrink budget tree (VER05e section 4.1, VER05i section 6).
 *
 * WHY A BUDGET AT ALL
 * -------------------
 * A shrinker calls the user's property many times, and each call may copy a big
 * candidate.  Counting only "accepted steps" does not bound that: a 1 MiB value
 * whose every smaller candidate is rejected costs ~16,000 attempts x 1M
 * elements of copying.  So two things are charged:
 *
 *   attempts   one per property evaluation
 *   work       elements copied (at least 1 per charge)
 *
 * WHY A TREE
 * ----------
 * A property may itself run a nested check, a custom shrinker may recurse into
 * pbt_gen_shrink, a filter predicate may shrink.  Rather than deciding per call
 * whether it is "inside" something (four review rounds found holes in every
 * such classification in the Java lane), every public entry point opens a CHILD
 * of the scope it was handed with its own local cap, and every charge walks to
 * the root and succeeds only if EVERY node on the path has room.  The total work
 * of one top-level check is therefore bounded by its root, whatever the nesting.
 *
 *            root (cap R)
 *              |
 *            check scope
 *              |
 *            shrink scope (cap S <= R)         <- a charge here checks all three
 *
 * A charge is ALL OR NOTHING: if any node refuses, no counter changes.
 *
 * There is no lock (single-threaded by contract) and no ambient scope: the
 * node to charge is passed explicitly, so re-entrancy needs no global state.
 */
#include "pbt.h"
#include "pbt_hooks.h"

uint64_t pbt_attempt_budget(int max_steps)
{
    /* A negative step count must not wrap into a huge cap: clamp it to zero. */
    uint64_t steps = max_steps < 0 ? 0 : (uint64_t)max_steps;
    return (uint64_t)PBT_ATTEMPT_FACTOR * steps + PBT_ATTEMPT_SLACK;
}

/* Fill `storage` as a new owning node under `parent` (or a root when NULL). */
static pbt_budget *make_node(pbt_budget *storage, pbt_budget *parent, uint64_t max_attempts, uint64_t max_work)
{
    storage->parent = parent;
    storage->root = parent != NULL ? parent->root : storage;
    storage->depth = parent != NULL ? parent->depth + 1 : 0;
    storage->owning = 1;
    storage->refused = 0;
    storage->nest = parent != NULL ? parent->nest : 0;
    storage->max_attempts = max_attempts;
    storage->max_work = max_work;
    storage->attempts = 0;
    storage->work = 0;
    return storage;
}

/* Beyond the depth cap a scope joins its parent: the charge walk stays short. */
static pbt_budget *join(pbt_budget *storage, pbt_budget *parent)
{
    storage->owning = 0;
    storage->parent = NULL;
    storage->root = parent->root;
    return parent;
}

pbt_budget *pbt_budget_open_child_cap(pbt_budget *storage, pbt_budget *parent, int max_steps, uint64_t max_work)
{
    if (parent == NULL) {
        uint64_t cap = max_work == PBT_MAX_SHRINK_WORK && pbt_hooks.default_max_work != 0 ? pbt_hooks.default_max_work
                                                                                          : max_work;
        return make_node(storage, NULL, pbt_attempt_budget(max_steps), cap);
    }
    if (parent->depth + 1 >= PBT_BUDGET_MAX_DEPTH) {
        return join(storage, parent);
    }
    return make_node(storage, parent, pbt_attempt_budget(max_steps), max_work);
}

pbt_budget *pbt_budget_open_child(pbt_budget *storage, pbt_budget *parent, int max_steps)
{
    return pbt_budget_open_child_cap(storage, parent, max_steps, PBT_MAX_SHRINK_WORK);
}

pbt_budget *pbt_budget_open_check(pbt_budget *storage, pbt_budget *parent, uint64_t num_tests, int max_steps)
{
    uint64_t attempts = num_tests + pbt_attempt_budget(max_steps);
    if (parent == NULL) {
        uint64_t cap = pbt_hooks.default_max_work != 0 ? pbt_hooks.default_max_work : PBT_MAX_SHRINK_WORK;
        storage = make_node(storage, NULL, attempts, cap);
        storage->nest = 1;
        return storage;
    }
    if (parent->depth + 1 >= PBT_BUDGET_MAX_DEPTH) {
        return join(storage, parent);
    }
    storage = make_node(storage, parent, attempts, PBT_MAX_SHRINK_WORK);
    storage->nest++;
    return storage;
}

void pbt_budget_close(pbt_budget *storage)
{
    if (storage->owning) {
        pbt_hooks.last_work = storage->work;
        pbt_hooks.last_attempts = storage->attempts;
        storage->owning = 0;
    }
}

/*
 * The core: charge `cost` work and `attempt_cost` attempts to b and every
 * ancestor, all or nothing.  The room test `cost > max - used` cannot wrap
 * because used <= max always holds.  Note the attempt ceiling also refuses a
 * work-only charge once attempts are spent (the Java lane does the same).
 */
static int take(pbt_budget *b, uint64_t elements, int attempt_cost)
{
    uint64_t cost = elements < 1 ? 1 : elements;
    pbt_budget *n;
    for (n = b; n != NULL; n = n->parent) {
        if (n->attempts >= n->max_attempts || cost > n->max_work - n->work) {
            b->root->refused = 1;
            return 0;
        }
    }
    for (n = b; n != NULL; n = n->parent) {
        n->attempts += (uint64_t)attempt_cost;
        n->work += cost;
    }
    return 1;
}

int pbt_budget_charge(pbt_budget *b, uint64_t elements)
{
    return take(b, elements, 1);
}

int pbt_budget_charge_copy(pbt_budget *b, uint64_t elements)
{
    return take(b, elements, 0);
}

int pbt_budget_has_room(const pbt_budget *b)
{
    const pbt_budget *n;
    for (n = b; n != NULL; n = n->parent) {
        if (n->attempts >= n->max_attempts || n->work >= n->max_work) {
            return 0;
        }
    }
    return 1;
}

int pbt_budget_is_nested(const pbt_budget *b)
{
    return b->parent != NULL;
}

int pbt_budget_was_refused(const pbt_budget *b)
{
    return b->root->refused;
}

uint64_t pbt_budget_attempts(const pbt_budget *b)
{
    return b->attempts;
}

uint64_t pbt_budget_work(const pbt_budget *b)
{
    return b->work;
}
