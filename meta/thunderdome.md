# Thunderdome Reimplementation Notes

This file summarizes the **working prototype integration** we previously implemented, so it can be replayed in a future session after a revert.

## Goal

Replace hand-written expression arena storage with `thunderdome` so expression nodes can be removed, and add a simple GC pass to release unreachable sub-formulas (especially after distributive CNF transforms).

## What worked (high-level)

1. Add `thunderdome` dependency.
2. Switch `ExprId` from `usize` to `thunderdome::Index`.
3. Switch `Arena.exprs` from `Vec<Expr>` to `thunderdome::Arena<Expr>`.
4. Adapt all places that assumed integer IDs (`*2`, formatting, parser sentinels).
5. Add conservative reachability GC and call it after `to_cnf_dist`.
6. Run full correctness checks (`cargo test` + integration tests).

## Files touched

- `Cargo.toml`
- `src/core/expr.rs`
- `src/core/arena.rs`
- `src/core/formula.rs`
- `src/parser/sat.rs`
- `src/parser/cnf.rs`

(Formatting-only diffs may appear in other files after `cargo fmt`.)

## Step-by-step replay

### 1) Add dependency

In `Cargo.toml`:

```toml
[dependencies]
thunderdome = "0.6.1"
```

### 2) Change expression ID type

In `src/core/expr.rs`:

- add `use thunderdome::Index;`
- change:

```rust
pub(crate) type ExprId = usize;
```

to:

```rust
pub(crate) type ExprId = Index;
```

### 3) Replace expression storage type

In `src/core/arena.rs`:

- import alias:

```rust
use thunderdome::Arena as ThunderdomeArena;
```

- change field:

```rust
pub(super) exprs: Vec<Expr>
```

to:

```rust
pub(super) exprs: ThunderdomeArena<Expr>
```

- initialize with `ThunderdomeArena::new()` in `Arena::new()`.

- `add_expr` changes from `push/len` to `insert`:

```rust
fn add_expr(&mut self, expr: Expr) -> ExprId {
    let hash = expr.calc_hash();
    let id = self.exprs.insert(expr);
    self.exprs_inv.entry(hash).or_default().push(id);
    id
}
```

### 4) Fix `ExprId` assumptions that used integer ops

In `src/core/arena.rs`, simplification sort key in macro `simp_expr!` used arithmetic on IDs:

```rust
Not(grandchild_id) => grandchild_id * 2 + 1,
_ => *child_id * 2,
```

For `Index`, replace with:

```rust
Not(grandchild_id) => grandchild_id.to_bits() * 2 + 1,
_ => child_id.to_bits() * 2,
```

### 5) Canonical lookup must ignore removed IDs

In `get_expr`, filter invalid IDs:

```rust
.filter(|id| self.exprs.contains(**id) && self.exprs[**id] == *expr)
```

Also make `inval_expr` robust:

```rust
if !self.exprs.contains(id) {
    return;
}
```

### 6) Update ID printing

`Index` does not implement `Display`. In `format_expr`:

```rust
format!("@{}", id.to_bits())
```

instead of `format!("@{id}")`.

### 7) Parser sentinel IDs

In `src/parser/sat.rs` and `src/parser/cnf.rs`, `vec![0]` no longer works.

- add `use thunderdome::Index;`
- change:

```rust
let mut vars: Vec<ExprId> = vec![0];
```

to:

```rust
let mut vars: Vec<ExprId> = vec![Index::DANGLING];
```

This keeps the existing 1-based variable indexing behavior.

### 8) Add conservative GC pass

In `src/core/arena.rs`, add method:

```rust
pub(super) fn gc_unreachable_from(&mut self, root_id: ExprId)
```

Algorithm used:

1. DFS/BFS from `root_id` to collect reachable expression IDs.
2. Iterate all arena IDs (`self.exprs.iter().map(|(id, _)| id)`).
3. Remove IDs not in reachable set (`self.exprs.remove(id)`).
4. Cleanup `exprs_inv` vectors via `retain(|id| self.exprs.contains(*id))`.

This was intentionally conservative and simple.

### 9) Call GC after distributive transform

In `src/core/formula.rs`, at end of `to_cnf_dist`:

```rust
arena.gc_unreachable_from(self.root_id);
```

We only wired GC there in the prototype because distributive conversion is where abandoned subgraphs were most likely.

## Validation commands (used)

```bash
cargo test
make integration-test
```

Correctness criterion used: all unit and integration tests pass exactly.

## Benchmark method we used

Because `/usr/bin/time -l` RSS reporting was blocked in sandbox (`sysctl` permission), we measured:

- runtime: wall-clock elapsed
- memory: sampled peak RSS via `ps -o rss -p <pid>` while process is running

Commands benchmarked:

```bash
clausy -q -i 'linux/v2.5.53[arm].model' -t cnf-dist print
clausy -q -i 'meta/busybox_1_3_0.dimacs' -t cnf-dist print
clausy -q -i 'linux/v6.12[x86].model' -t cnf-tseitin print
```

## Observed caveats in prototype

- It was correct, but performance was mixed and often worse.
- Full-sweep GC after every `to_cnf_dist` can be expensive.
- `Index::to_bits()` in hot ordering paths adds overhead compared to integer IDs.

## If you re-implement again, likely improvements

1. Make GC optional or threshold-based (run only above expression-count deltas).
2. Run GC less frequently (e.g., once after full transform pipeline, not per stage).
3. Avoid repeated `to_bits()` work in hot loops (cache sort keys when useful).

## Quick reimplementation checklist

- [ ] Add `thunderdome` dependency
- [ ] Switch `ExprId` alias to `Index`
- [ ] Switch `exprs` store to `ThunderdomeArena<Expr>`
- [ ] Replace integer-ID arithmetic with `to_bits()` or equivalent stable ordering
- [ ] Replace parser `vec![0]` sentinels with `Index::DANGLING`
- [ ] Add `contains` guards where stale IDs are possible
- [ ] Implement and wire `gc_unreachable_from`
- [ ] Run `cargo test`
- [ ] Run `make integration-test`
- [ ] Re-measure same three workloads
