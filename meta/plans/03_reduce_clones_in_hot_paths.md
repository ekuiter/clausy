# Plan 03: Remove Avoidable Clones in Hot Transformation Paths

## Objective
Reduce allocation and copy overhead by eliminating unnecessary `clone()` calls in traversal visitors and expression construction.

## Why this should help
Visitors like `cnf_dist_visitor`, `nnf_visitor`, `cnf_tseitin_visitor` are called frequently. Small clone reductions can compound.

## Scope
- `src/core/arena.rs`
- optional small changes in `src/core/formula.rs`

## Implementation steps
1. Audit all `clone()` calls in transformation code paths.
2. Convert clone-heavy logic to borrowed-slice iteration where safe.
3. For places needing owned vectors, build only final vectors once.
4. Re-check canonicalization invariants after refactor.

## Validation
- `cargo test`
- `make integration-test`
- compare allocations/time using repeated runs

## Success criteria
- No output change.
- Lower runtime and reduced allocator pressure.

## Risks
- Borrow checker refactors can accidentally change update ordering.

## Rollback
Revert visitor refactors file-by-file.

tried this with no perceivable performance impact