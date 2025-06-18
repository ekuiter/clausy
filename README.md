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

## CNF Tool Survey

This is a list of tools and resources related to CNF transformation for (feature-model or general Boolean) formulas.

### Z3 (Microsoft Research)

[Z3Prover/z3](https://github.com/Z3Prover/z3) ([Website](https://www.microsoft.com/en-us/research/project/z3-3/), [tseitin_cnf_tactic.h](https://github.com/Z3Prover/z3/blob/master/src/tactic/core/tseitin_cnf_tactic.h), [tseitin_cnf_tactic.cpp](https://github.com/Z3Prover/z3/blob/master/src/tactic/core/tseitin_cnf_tactic.cpp))

- Z3 implements a **partial Tseitin transformation**, which introduces auxiliary variables, should the configurable parameter `m_distributivity_blowup` (default: 32) be exceeded for the predicted blowup. The transformed formula is always [quasi-equivalent](https://raw.githubusercontent.com/SoftVarE-Group/Papers/main/2022/2022-ASE-Kuiter.pdf) to the original formula (NOT necessarily equivalent).
- Z3 can not only transform any Boolean formula, but also general SMT problems into CNF.
- Z3 has a command-line interface and Python bindings.
- Z3 can also be used from Java with [JavaSMT](https://github.com/sosy-lab/java-smt).
- [KClause](https://github.com/paulgazz/kmax/blob/master/kmax/kclause) uses Z3 for CNF transformation.
- Z3 is integrated in [FeatJAR](https://github.com/FeatureIDE/FeatJAR) (see [formula-analysis-javasmt](https://github.com/FeatureIDE/FeatJAR-formula-analysis-javasmt)).
- Z3 is integrated in [torte](https://github.com/ekuiter/torte) (see [smt2dimacs.py](https://github.com/ekuiter/torte/blob/main/src/docker/z3/smt2dimacs.py)).
- KClause with Z3 over JavaSMT is integrated in [torte](https://github.com/ekuiter/torte) (see [kclause](https://github.com/ekuiter/torte/blob/main/src/docker/kclause) + [ModelToSMTZ3.java](https://github.com/ekuiter/torte/blob/main/src/docker/featjar/transform/src/main/java/ModelToSMTZ3.java)).
- Z3 is released under the MIT license.

### KConfigReader (Christian Kästner)

[ckaestne/kconfigreader](https://github.com/ckaestne/kconfigreader), [ckaestne/TypeChef](https://github.com/ckaestne/TypeChef) ([SATFeatureExpr.scala](https://github.com/ckaestne/TypeChef/blob/master/FeatureExprLib/src/main/scala/de/fosd/typechef/featureexpr/sat/SATFeatureExpr.scala))

- KConfigReader implements a **partial Plaisted-Greenbaum transformation**, which introduces auxiliary variables, should the fixed parameter 16 be exceeded for the predicted blowup. The transformed formula is always [equi-assignable](https://raw.githubusercontent.com/SoftVarE-Group/Papers/main/2022/2022-ASE-Kuiter.pdf) to the original formula (NOT necessarily equi-countable). The transformation is not polarity-based (so, the formula is transformed into negation normal form first).
- KConfigReader is integrated in [torte](https://github.com/ekuiter/torte) (see [TransformIntoDIMACS.scala](https://github.com/ekuiter/torte/blob/main/src/docker/kconfigreader/TransformIntoDIMACS.scala)). This is currently the only way to transform arbitrary Boolean formulas. By default, KConfigReader can only operate on Boolean formulas that it extracts from KConfig specifications.
- KConfigReader is released under the LGPL v3 license.

### FeatureIDE

[FeatureIDE/FeatureIDE](https://github.com/FeatureIDE/FeatureIDE) ([Website](https://featureide.github.io/))

`todo`

### FeatJAR

[FeatureIDE/FeatJAR](https://github.com/FeatureIDE/FeatJAR)

`todo`

### LogicNG (Christoph Zengler)

[logic-ng/LogicNG](https://github.com/logic-ng/LogicNG) ([Website](https://logicng.org/), [tseitin_impl.hpp](https://gitlab.sai.jku.at/booleguru/booleguru/-/blob/main/src/transform/include/booleguru/transform/tseitin_impl.hpp))

`todo`

### Booleguru (Maximilian Heisinger)

[Booleguru/Booleguru](https://gitlab.sai.jku.at/booleguru/booleguru) ([Website](https://booleguru.pages.sai.jku.at/booleguru/), [tseitin_impl.hpp](https://gitlab.sai.jku.at/booleguru/booleguru/-/blob/main/src/transform/include/booleguru/transform/tseitin_impl.hpp), [plaisted_greenbaum_impl.hpp](https://gitlab.sai.jku.at/booleguru/booleguru/-/blob/main/src/transform/include/booleguru/transform/plaisted_greenbaum_impl.hpp))

- Booleguru implements a **total Tseitin transformation**, which introduces auxiliary variables for every nontrivial subformula. The transformed formula is always [quasi-equivalent](https://raw.githubusercontent.com/SoftVarE-Group/Papers/main/2022/2022-ASE-Kuiter.pdf) to the original formula (NOT necessarily equivalent).
- Booleguru also implements a **total Plaisted-Greenbaum transformation**, which introduces auxiliary variables for every nontrivial subformula. The transformed formula is always [equi-assignable](https://raw.githubusercontent.com/SoftVarE-Group/Papers/main/2022/2022-ASE-Kuiter.pdf) to the original formula (NOT necessarily equi-countable). The transformation is polarity-based (so, negation normal form is not needed).
- Booleguru has a command-line interface and Python bindings.
- Booleguru is released under the MIT license.

### Limboole (Armin Biere)

[Limboole](https://fmv.jku.at/limboole/) ([v1.2](https://fmv.jku.at/limboole/limboole1.2.tgz))

- Limboole implements a **total Tseitin transformation**, which introduces auxiliary variables for every nontrivial subformula. The transformed formula is always [quasi-equivalent](https://raw.githubusercontent.com/SoftVarE-Group/Papers/main/2022/2022-ASE-Kuiter.pdf) to the original formula (NOT necessarily equivalent).
  This is currently the only way to transform arbitrary Boolean formulas.
- Limboole has a command-line interface.
- Limboole is released under the MIT license.

### clausy (This Project)

[ekuiter/clausy](https://github.com/ekuiter/clausy)

- clausy implements a **total Tseitin transformation**, which introduces auxiliary variables for every nontrivial subformula. The transformed formula is always [quasi-equivalent](https://raw.githubusercontent.com/SoftVarE-Group/Papers/main/2022/2022-ASE-Kuiter.pdf) to the original formula (NOT necessarily equivalent).
- clausy also implements a **total distributive transformation**, which introduces no auxiliary variables. The transformed formula is always [equivalent](https://raw.githubusercontent.com/SoftVarE-Group/Papers/main/2022/2022-ASE-Kuiter.pdf) to the original formula.
- clausy has a command-line interface and can be interfaced with from Rust.
- clausy is integrated in [torte](https://github.com/ekuiter/torte).
- The above tools are often not or only sparsely documented. clausy, in contrast, is [extensively documented](https://ekuiter.github.io/clausy/).
- The above tools have many additional features that are not related to CNF transformation. clausy, and contrast, is focused and specialized, which (hopefully) makes it easier to understand and debug.
- clausy is released under the LGPL v3 license.

## License

The source code of this project is released under the [LGPL v3 license](LICENSE.txt).