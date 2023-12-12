# clausy: clausify feature-model formulas 🎅

**clausy transforms feature-model formulas into conjunctive normal form (CNF) for subsequent analysis.**

## Getting Started

To transform any [`.sat`](meta/satformat.pdf), [`.model`](https://github.com/ckaestne/kconfigreader), or [FeatureIDE](https://featureide.github.io/)-compatible file into [`.cnf`](meta/satformat.pdf) (aka [`.dimacs`](meta/satformat.pdf)), run:

```
git clone https://github.com/ekuiter/clausy.git
cd clausy

# option 1: build Docker image
docker build -t clausy .
cat meta/test.sat | docker run -i clausy

# option 2: build into bin/ directory
make
bin/clausy meta/test.sat
```

## Advanced Usage, Documentation, and Tests

Documentation for clausy is available [online](https://ekuiter.github.io/clausy/).

```
# equivalent to the above, but more verbose
bin/clausy meta/test.sat to_cnf_dist print

# read from standard input and count solutions with tseitin transformation
cat model.uvl | bin/clausy -.uvl to_cnf_tseitin count

# read from command line and find some solution
echo '(!def(a)|def(b))' | bin/clausy -.model to_cnf_dist satisfy

# prove a tautology
! echo '(def(a)|!def(a))' | bin/clausy -.model '(-1)' to_cnf_tseitin satisfy &>/dev/null

# prove model equivalence
! bin/clausy a.model b.model '+(*(-1 2) *(1 -2))' to_cnf_tseitin satisfy &>/dev/null

# run tests
make test

# view documentation
make doc

# view live documentation (for developers)
make doc-live
```

## License

The source code of this project is released under the [LGPL v3 license](LICENSE.txt).