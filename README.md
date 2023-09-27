# clausy: clausify feature-model formulas 🎅

**clausy transforms feature-model formulas into conjunctive normal form (CNF) for subsequent analysis.**

## Getting Started

To transform any [`.sat`](meta/satformat.pdf), [`.model`](https://github.com/ckaestne/kconfigreader), or [FeatureIDE](https://featureide.github.io/)-compatible file into [`.cnf`](meta/satformat.pdf) (aka [`.dimacs`](meta/satformat.pdf)), run:

```
# install dependencies (Ubuntu assumed, other systems analogous)
sudo apt update && sudo apt install default-jre curl
curl https://sh.rustup.rs -sSf | sh

# build
io/gradlew -p io shadowJar
cargo build --release && cp target/release/clausy bin/clausy
curl https://github.com/ekuiter/torte/raw/main/docker/solver/model-counting-competition-2022/d4 -Lo bin/d4 && chmod +x bin/d4

# run
bin/clausy meta/test.sat to_nnf to_cnf_dist to_cnf print

# test
cargo test
```

## Documentation

Documentation for clausy is available [here](https://ekuiter.github.io/clausy/) or can be generated with rustdoc.
Most relevant information can be found in [`clausy > formula > Formula`](https://ekuiter.github.io/clausy/clausy/formula/struct.Formula.html).

```
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