# Clausy

Rust tool that transforms feature-model formulas into CNF (conjunctive normal form).

## Build

```bash
make          # build locally to build/
make test     # run tests
make doc      # view documentation
```

## Run

```bash
build/clausy test/simple.sat                    # basic usage
build/clausy file.sat to_cnf_dist to_clauses print  # verbose
cat model.uvl | build/clausy -.uvl to_cnf_tseitin count  # stdin
```

## Project Structure

- `src/shell.rs` - CLI (clap), top-level pipeline
- `src/core/formula.rs` - Formula type, diff/transform algorithms, DiffKind
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
