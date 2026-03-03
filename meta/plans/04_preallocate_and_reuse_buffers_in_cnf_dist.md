# Plan 04: Pre-Allocate and Reuse Buffers in Distributive CNF

## Objective
Reduce allocator churn in `cnf_dist_visitor` by pre-sizing vectors and reusing temporary buffers.

## Why this should help
Current distributive expansion builds many intermediate `Vec<Vec<ExprId>>` values; allocator overhead is significant.

## Scope
- `src/core/arena.rs` (`cnf_dist_visitor`)

## Implementation steps
1. Estimate clause growth early per OR node and reserve capacity.
2. Replace repeated collect/flatten pipelines with explicit loops using reusable `next_clauses` buffer.
3. Swap buffers (`std::mem::take`/`swap`) between iterations to avoid reallocating.
4. Keep canonicalization/simplification semantics intact.

## Validation
- `cargo test`
- `make integration-test`
- focused benchmark on distributive transformations

## Success criteria
- Stable outputs.
- Reduced runtime and peak allocation pressure on dist workloads.

## Risks
- Incorrect buffer reuse can retain stale data.

## Rollback
Revert `cnf_dist_visitor` to previous implementation.

tried this and it did not perform better, and it made the code much more complex