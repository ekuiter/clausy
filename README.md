# clausy: clausify feature-model formulas 🎅

**clausy transforms feature-model formulas into conjunctive normal form (CNF) for subsequent analysis.**

## Getting Started

To transform any [`.sat`](meta/satformat.pdf), [`.model`](https://github.com/ckaestne/kconfigreader), or [FeatureIDE](https://featureide.github.io/)-compatible file into [`.cnf`](meta/satformat.pdf) (aka [`.dimacs`](meta/satformat.pdf)), run:

```
git clone --recursive https://github.com/ekuiter/clausy.git
cd clausy

# option 1: full build as Docker image
# (works on any operating system and architecture)
docker build -t clausy .
cat meta/test.sat | docker run --rm -i clausy

# option 2: full build into bin/ directory
# (works only on Linux, as it compiles external solvers)
make
bin/clausy meta/test.sat

# option 3: minimum build into bin/ directory
# (works anywhere, but cannot use external solvers)
make clausy
bin/clausy meta/test.sat
```

## Advanced Usage, Documentation, and Tests

Documentation for clausy is available [online](https://ekuiter.github.io/clausy/).

```
# equivalent to the above, but more verbose
bin/clausy meta/test.sat to_cnf_dist print

# read from standard input and count solutions with Tseitin transformation
cat model.uvl | bin/clausy -.uvl to_cnf_tseitin count

# read from command line and find some solution
echo '(!def(a)|def(b))' | bin/clausy -.model to_cnf_dist satisfy

# prove a tautology
! echo '(def(a)|!def(a))' | bin/clausy -.model '(-1)' to_cnf_tseitin satisfy &>/dev/null

# prove model equivalence
! bin/clausy a.model b.model '+(*(-1 2) *(1 -2))' to_cnf_tseitin satisfy &>/dev/null

# compute diff statistics
bin/clausy a.model b.model diff

# serialize diff
bin/clausy a.model b.model 'diff weak weak a_to_b'
 
# simplify a given CNF
bin/clausy model.dimacs

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

We also maintain a [detailed list](./meta/README.md) of tools table of tools that implement a CNF transformation.

## License

The source code of this project is released under the [LGPL v3 license](LICENSE.txt).