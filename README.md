# clausy: clausify feature-model formulas 🎅

**clausy transforms feature-model formulas into conjunctive normal form for subsequent analysis.**

## Getting Started

```
# build
gradle -p io shadowJar
cargo build --release
cp target/release/clausy clausy

# run
cat my-model.model | ./clausy
cat my-model.uvl | java -jar io.jar -.uvl | ./clausy
cat my-model.xml | java -jar io.jar -.xml | ./clausy
```

## License

The source code of this project is released under the [LGPL v3 license](LICENSE.txt).