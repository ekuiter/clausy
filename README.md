# clausy: clausify feature-model formulas 🎅

**clausy transforms feature-model formulas into conjunctive normal form (CNF) for subsequent analysis.**

## Getting Started

To transform any [`.sat`](meta/satformat.pdf), [`.model`](https://github.com/ckaestne/kconfigreader), or [FeatureIDE](https://featureide.github.io/)-compatible file into [`.cnf`](meta/satformat.pdf) (aka [`.dimacs`](meta/satformat.pdf)), run:

```
git clone https://github.com/ekuiter/clausy.git
cd clausy

# option 1: build Docker container
docker build -t clausy .
cat meta/test.sat | docker run -i clausy

# option 2: build into bin/ directory
./build.sh
bin/clausy meta/test.sat

# equivalent, but more verbose
bin/clausy meta/test.sat to_nnf to_cnf_dist to_clauses print

# read from standard input and count solutions
cat model.uvl | bin/clausy -.uvl to_nnf to_cnf_dist to_clauses count
```

## Tests & Documentation

Documentation for clausy is available [online](https://ekuiter.github.io/clausy/), see [`clausy::core::formula::Formula`](https://ekuiter.github.io/clausy/clausy/core/formula/struct.Formula.html) for details on algorithm usage.

```
# run tests
cargo test

# view documentation
cargo doc --no-deps --open

# view live documentation (for developers)
sudo apt-get update
sudo apt-get install -y inotify-tools nodejs npm
npm install -g browser-sync
while inotifywait -re close_write,moved_to,create src; do cargo doc --no-deps; done &
(cd target/doc; browser-sync start --server --files "*.html")
```

## License

The source code of this project is released under the [LGPL v3 license](LICENSE.txt).