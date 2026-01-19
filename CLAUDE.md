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

- `src/` - Rust source code
- `meta/` - test files and format specs
- `build/` - compiled binaries
