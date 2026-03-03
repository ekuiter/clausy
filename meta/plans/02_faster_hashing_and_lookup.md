# Plan 02: Faster Hashing and Expression Lookup

## Objective
Lower overhead in expression canonicalization (`expr`, `get_expr`, `inval_expr`) by replacing default hash maps with faster hashers and reducing redundant hash work.

## Why this should help
Expression insertion/lookup is on the hottest path. Default SipHash is secure but slower than needed for in-process trusted data.

## Scope
- `src/core/arena.rs`
- possibly `Cargo.toml` (add `rustc-hash` or `ahash`)
- map types for `exprs_inv` (and optionally `vars_inv`)

## Implementation steps
1. Introduce fast hasher crate (`rustc-hash` preferred for simplicity).
2. Replace map types:
- `HashMap<u64, Vec<ExprId>>` -> `FxHashMap<u64, Vec<ExprId>>`
- evaluate `vars_inv` similarly.
3. Avoid duplicate hash calculation where feasible:
- compute expression hash once in insertion/invalidation paths.
4. Add cheap cleanup strategy for long `exprs_inv` buckets if stale IDs accumulate.

## Validation
- `cargo test`
- `make integration-test`
- benchmark before/after on same three workloads

## Success criteria
- No correctness change.
- Lower CPU time in transformation workloads.

## Risks
- Different hashers can change iteration order; output must remain deterministic where required.

## Rollback
Revert map aliases to std `HashMap`.

tried this and it works quite well, a simple dropin replacement which consistently brings down the runtime a bit