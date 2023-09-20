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
```

## Documentation

Documentation for clausy is available via rustdoc and can be viewed in any browser.
Most relevant information can be found in `clausy > formula > Formula`.

```
# view documentation
cargo doc --open

# view live documentation (for developers)
sudo apt-get update
sudo apt-get install -y inotify-tools nodejs npm
npm install -g browser-sync
while inotifywait -re close_write,moved_to,create src; do cargo doc; done &
(cd target/doc; browser-sync start --server --files "*.html")
# then visit http://localhost:3000/clausy/
```

## License

The source code of this project is released under the [LGPL v3 license](LICENSE.txt).