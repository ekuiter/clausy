# Plan 01: Replace HashSet Traversal Bookkeeping with Mark Vectors

## Objective
Reduce traversal overhead in `preorder_rev`, `postorder_rev`, and `prepostorder_rev` by replacing `HashSet<ExprId>` with arena-local mark vectors plus an epoch counter.

## Why this should help
Current traversals spend significant time hashing expression IDs and allocating hash table buckets. Mark vectors provide O(1) array access and less allocation pressure.

## Scope
- `src/core/arena.rs` traversal helpers
- minimal arena state additions for marks/epochs
- no behavior change to formula semantics

## Implementation steps
1. Add mark storage in `Arena`:
- `visit_seen: Vec<u32>`
- `visit_done: Vec<u32>`
- `visit_epoch: u32`
2. Add helper methods:
- ensure-capacity for mark vectors as expression arena grows
- epoch bump with overflow-safe reset path
3. Rewrite traversal loops to use mark vectors instead of `HashSet`s.
4. Keep root reset logic unchanged (`Formula::reset_root_expr`).
5. Add/adjust tests for traversal stability and idempotence.

## Validation
- `cargo test`
- `make integration-test`
- micro-benchmark repeated runs of dist/tseitin workloads

## Success criteria
- All tests pass unchanged.
- Traversal-heavy transformations (`to_nnf`, `to_cnf_dist`, `to_cnf_tseitin`) show measurable speedup.

## Risks
- Incorrect epoch handling can create hard-to-debug missed visits.
- Needs careful handling if expression storage becomes sparse/deletable.

## Rollback
Single-file rollback in `arena.rs` plus removal of added mark fields.

minor performance improvement (150 milliseconds to 140), not worth the more complex code