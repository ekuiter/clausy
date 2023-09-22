# clausy: clausify feature-model formulas 🎅

**clausy transforms feature-model formulas into conjunctive normal form (CNF) for subsequent analysis.**

## Getting Started

Run the following to transform any feature-model format accepted by [FeatureIDE](https://featureide.github.io/) into CNF (printed in DIMACS format).

```
# install Rust
curl https://sh.rustup.rs -sSf | sh

# build
io/gradlew -p io shadowJar
cargo build --release
cp target/release/clausy clausy

# run
cat my-model.model | ./clausy
cat my-model.uvl | java -jar io.jar -.uvl | ./clausy
cat my-model.xml | java -jar io.jar -.xml | ./clausy

# test
curl https://github.com/ekuiter/torte/raw/main/docker/solver/model-counting-competition-2022/d4 -Lo d4
chmod +x d4
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