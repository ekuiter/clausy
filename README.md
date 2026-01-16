# clausy: clausify feature-model formulas 🎅

**clausy transforms feature-model formulas into conjunctive normal form (CNF) for subsequent analysis.**

## Getting Started

To transform any [`.sat`](meta/satformat.pdf), [`.model`](https://github.com/ckaestne/kconfigreader), or [FeatureIDE](https://featureide.github.io/)-compatible file into [`.cnf`](meta/satformat.pdf) (aka [`.dimacs`](meta/satformat.pdf)), run:

```
git clone --recursive https://github.com/ekuiter/clausy.git
cd clausy

# option 1: build as Docker image (on any operating system and architecture)
docker build -t clausy .
cat meta/test.sat | docker run --rm -i clausy

# option 2: build locally into build/ directory
make
make external # optionally compile external solvers (only on Linux)
build/clausy meta/test.sat
```

Depending on the invocation, external tools may be needed (e.g., Java for parsing FeatureIDE-compatible files).

## Advanced Usage, Documentation, and Tests

Documentation for clausy is available [online](https://elias-kuiter.de/clausy/).

```
# equivalent to the above, but more verbose
build/clausy meta/test.sat to_cnf_dist to_clauses print

# read from standard input and count solutions with Tseitin transformation
cat model.uvl | build/clausy -.uvl to_cnf_tseitin count

# read from command line and find some solution
echo '(!def(a)|def(b))' | build/clausy -.model to_cnf_dist satisfy

# prove a tautology
! echo '(def(a)|!def(a))' | build/clausy -.model '(-1)' to_cnf_tseitin satisfy &>/dev/null

# prove model equivalence
! build/clausy a.model b.model '+(*(-1 2) *(1 -2))' to_cnf_tseitin satisfy &>/dev/null

# compute diff statistics
build/clausy a.model b.model diff

# serialize diff
build/clausy a.model b.model 'diff weak weak a_to_b'
 
# simplify a given CNF
build/clausy model.dimacs

# advanced usage via Docker (file I/O with standard input)
cat meta/test.sat | docker run --rm -i clausy -.sat to_cnf_tseitin count

# advanced usage via Docker (file I/O with volumes)
docker run --rm -v ./a.xml:/a.xml -v ./b.xml:/b.xml -v ./diff:/diff clausy /a.xml /b.xml 'diff weak weak /diff/a_to_b'

# run tests
make test

# view documentation
make doc

# view live documentation (for developers)
make doc-live
```

## CNF Transformation Zoo

We also maintain a [detailed table](./meta/README.md) of tools that implement a CNF transformation.

## License

The source code of this project is released under the [LGPL v3 license](LICENSE.txt).