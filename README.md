# clausy: clausify feature-model formulas 🎅

**clausy transforms feature-model formulas into conjunctive normal form (CNF) for subsequent analysis.**

## Getting Started

To transform any [`.sat`](meta/satformat.pdf), [`.model`](https://github.com/ckaestne/kconfigreader), or [FeatureIDE](https://featureide.github.io/)-compatible file into [`.cnf`](meta/satformat.pdf) (aka [`.dimacs`](meta/satformat.pdf)), run:

```
git clone --recursive https://github.com/ekuiter/clausy.git
cd clausy

# option 1: build as Docker image
docker build -t clausy .
cat meta/simple.sat | docker run --rm -i clausy

# option 2: build locally into build/ directory
make
make external # optional: compile external solvers
build/clausy -i meta/simple.sat

# option 3: download precompiled binaries for 64-bit Linux
wget https://nightly.link/ekuiter/clausy/workflows/static/main/build.zip
unzip build.zip
```

You can also [click here](https://nightly.link/ekuiter/clausy/workflows/static/main/build.zip) to download precompiled binaries for 64-bit Linux.

Depending on the invocation, external tools may be needed (e.g., Java for parsing FeatureIDE-compatible files).

## Advanced Usage, Documentation, and Tests

Documentation for clausy is available [online](https://elias-kuiter.de/clausy/).

```
# equivalent to the above, but more explicit
build/clausy -i meta/simple.sat -t cnf-dist print

# read from standard input and count solutions with Tseitin transformation
cat model.uvl | build/clausy -i -.uvl -t cnf-tseitin count

# read from command line and find some solution
echo '(!def(a)|def(b))' | build/clausy -i -.model -t cnf-dist satisfy

# prove a tautology
echo '(def(a)|!def(a))' | build/clausy -i -.model -i '(-1)' -t cnf-tseitin satisfy &>/dev/null; test $? -eq 20

# prove model equivalence
build/clausy -i a.model -i b.model -i '+(*(-1 2) *(1 -2))' -t cnf-tseitin satisfy &>/dev/null; test $? -eq 20

# compute diff statistics
build/clausy -i a.model -i b.model diff

# serialize diff
build/clausy -i a.model -i b.model diff --left weak --right weak --output-prefix a_to_b
 
# simplify a given CNF
build/clausy -i model.dimacs

# advanced usage via Docker (file I/O with standard input)
cat meta/simple.sat | docker run --rm -i clausy -i -.sat -t cnf-tseitin count

# advanced usage via Docker (file I/O with volumes)
docker run --rm -v ./a.xml:/a.xml -v ./b.xml:/b.xml -v ./diff:/diff clausy -i /a.xml -i /b.xml diff --left weak --right weak --output-prefix /diff/a_to_b

# run tests
make test

# view documentation
make doc

# view live documentation (for developers)
make doc-live
```

## CNF Transformation Zoo

We also maintain a [detailed table](https://elias-kuiter.de/torte-research/#transformations) of tools that implement a CNF transformation.
Key differences to clausy include:

- Most of the listed tools are not or only sparsely documented.
  clausy, in contrast, is <a href="https://ekuiter.github.io/clausy/" target="_blank">extensively documented</a>.
- Some of the listed tools have additional features that are not related to CNF transformation.
  clausy, in contrast, is focused and specialized, making it easier to understand/debug and ideal for learning and experimentation purposes.
- Most of the listed tools have relatively limited options for input formats.
  clausy, in contrast, has a diverse set of inputs, as it extends FeatureIDE's inputs with additional formats.

## License

The source code of this project is released under the [LGPL v3 license](LICENSE.txt).
