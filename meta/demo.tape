# https://github.com/tamnd/ascii-gif
# $(go env GOPATH)/bin/ascii-gif render meta/demo.tape
Set TypingSpeed 40ms
Set PlaybackSpeed 1.0
Sleep 1s
Type "build/clausy -q -i meta/simple.sat"
Enter
Sleep 3s
Type "build/clausy -q -i meta/simple.sat -t cnf-tseitin count"
Enter
Sleep 3s
Type "echo '(!def(a)&def(a))' | build/clausy -q -i -.model -t cnf-dist satisfy"
Enter
Sleep 3s
Type "build/clausy -q -i test/input/simple_a.model -i test/input/simple_b.model diff --count --no-header; echo"
Enter
Sleep 5s