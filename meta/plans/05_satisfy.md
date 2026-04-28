# Plan: `--satisfy` flag (TBK algorithm)

**Reference**: Thüm, Batory, Kästner — *Reasoning About Edits to Feature Models* (ICSE 2009)

## Overview

Adds TBK edit classification to the `diff` command. The classification assigns one of four labels to a feature-model edit based on whether removed and/or added solutions exist:

| `removed > 0` | `added > 0` | Label |
|---|---|---|
| No | No | `Refactoring` |
| Yes | No | `Specialization` |
| No | Yes | `Generalization` |
| Yes | Yes | `ArbitraryEdit` |

## New CLI flags

| Flag | Requires | Conflicts with | Description |
|---|---|---|---|
| `--satisfy` | `--negate` | `--count`, `--projected-count` | TBK SAT-based classification |
| `--simplified` | `--satisfy`, `--dist` | — | TBK clause-iteration variant |

`--satisfy` implies negation-based reasoning (required by clap `requires = "negate"`), so the user must pass both `--satisfy --negate`.

## CSV output change

New `classification` column inserted after `added_constraints` (before the solution-ratio columns). Populated whenever `cnt_removed` and `cnt_added` are non-negative — i.e., when `--count`, `--projected-count`, or `--satisfy` is active. Empty string otherwise.

Updated header:
```
common_vars,removed_vars,added_vars,common_constraints,removed_constraints,added_constraints,
classification,lost_solutions,removed_solutions,...
```

## Base `--satisfy` (SAT-based TBK)

Two SAT queries replace the two #SAT queries for removed/added solutions.

**`diff_helper` change**: new `satisfy: bool` parameter. When `true`, calls `clauses.satisfy().is_some()` and returns `BigInt` 0 (UNSAT) or 1 (SAT) instead of `clauses.count()`. The `any_count` parameter is set to `count || projected_count || satisfy`; `satisfy` is set to `satisfy` (only when in the removed/added diff_helper calls, not for common).

**`Clauses::assume(literals: &[VarId])`**: new method returning a cloned `Clauses` with additional unit clauses forcing the given literals. Used by the simplified path; each literal is a signed local variable ID.

## `--satisfy --simplified` (clause-iteration TBK, §3.2)

Replaces the single SAT call on `P(f) ∧ ¬P(g)` with `|pg|` smaller SAT calls, where `pg` = clauses unique to `P(g)`. Requires `--dist` because explicit CNF (no Tseitin auxiliaries) is needed for clause-set comparison across the two formulas.

**Key identity** (eq. 6 in the paper):
```
P(f) ∧ ¬P(g)  ≡  P(f) ∧ ¬pg  ≡  ∨_i (P(f) ∧ ¬Ri)
```
Each `¬Ri` is a conjunction of unit clauses (one per negated literal of `Ri`), so each SAT query is tiny. Early termination on the first SAT result.

**`satisfy_simplified(a_sliced, b_sliced, arena)`** (free function in `diff.rs`):

1. Build dist-CNF `Clauses` for both formulas independently (cloned arenas).
2. Invert each `var_remap` (arena_id → local_id) to get local_id → arena_id.
3. **Canonicalize** each clause: translate local literals to signed arena-IDs, sort. Required because two independently-built `Clauses` instances assign local IDs in traversal order, so the same semantic clause may have different local IDs and literal ordering.
4. Compute `pf` = clauses in `a` not in `b`; `pg` = clauses in `b` not in `a` (via `HashSet` lookup on sorted `Vec<VarId>`).
5. For removed check: for each `Ri ∈ pg`, translate back to `a`'s local space, negate, call `clauses_a.assume(&negated).satisfy()`. Stop at first SAT.
6. Symmetric for added check over `pf`.
7. Returns `(cnt_removed, cnt_added, dur_removed, dur_added)` — accumulated SAT-solver runtimes go into `removed_solutions_count_duration` / `added_solutions_count_duration` columns.

The `satisfy && simplified` branch inside `if negate { ... }` calls `satisfy_simplified` directly and skips the normal `and_not` formula construction.

## Implementation files changed

- `src/core/clauses.rs`: `Clauses::assume()`
- `src/core/diff.rs`: `diff_helper` `satisfy` param, `satisfy_simplified` function, `diff` signature (`satisfy`, `simplified` params), CSV header, classification derivation, output format
- `src/shell.rs`: `--satisfy` / `--simplified` flags, `diff()` call site

## FeatureIDE blackbox baseline

Deferred. Would extend `io.jar` with a `satisfy` action calling `ModelComparator.compare()` and invoke via the existing `exec::io()` path.
