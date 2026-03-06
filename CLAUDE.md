# Clausy

Rust tool that transforms feature-model formulas into CNF (conjunctive normal form).

## Build

```bash
make          # build locally to build/
make test     # run tests
make doc      # view documentation
cargo fmt     # format code (run regularly)
```

## Run

```bash
build/clausy test/simple.sat                    # basic usage
build/clausy file.sat to_cnf_dist to_clauses print  # verbose
cat model.uvl | build/clausy -.uvl to_cnf_tseitin count  # stdin
```

## Project Structure

- `src/shell.rs` - CLI (clap), top-level pipeline
- `src/core/formula.rs` - Formula type and core transform algorithms
- `src/core/diff.rs` - DiffKind enum and `diff()` free function
- `src/core/count_inc.rs` - `count_inc()` free function and `remove_constraints` Formula method
- `src/core/arena.rs` - Arena (shared expression/variable storage)
- `src/tests.rs` - unit tests
- `test/` - integration test files (`.txt` format, run via `make integration-test`)
- `build/` - compiled binaries

## Integration Tests

Each `.txt` file in `test/` defines a test case:

```
<shell command using $clausy>
---
<expected stdout>
```

If there is no `---`, only exit code 0 is checked. Commands run from the `test/` directory.
Update golden output with `make update-tests`.

The `diff` command outputs a CSV line ending with 10 nanosecond timing fields that vary per run.
Strip them before comparison: `| sed 's/\(,[^,]*\)\{10\}$//'`

## Code Style

- Do not edit code unless explicitly asked. If a code change is needed to answer a question, ask first.
- Format code regularly with `cargo fmt`.
