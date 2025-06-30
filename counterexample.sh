#!/bin/bash
bin/clausy /mnt/a.model to_cnf_dist to_clauses print > /mnt/ad.dimacs
bin/clausy /mnt/b.model to_cnf_dist to_clauses print > /mnt/bd.dimacs
bin/clausy /mnt/a.model to_cnf_tseitin to_clauses print > /mnt/at.dimacs
bin/clausy /mnt/b.model to_cnf_tseitin to_clauses print > /mnt/bt.dimacs
cat /mnt/at.dimacs | grep -v _aux_ > /mnt/att.dimacs
cat /mnt/bt.dimacs | grep -v _aux_ > /mnt/btt.dimacs
echo "common, removed, added"
echo "Comparing distributive transformation"
bin/clausy /mnt/ad.dimacs /mnt/bd.dimacs 'diff strong strong'
echo "Comparing Tseitin transformation (colliding variables)"
bin/clausy /mnt/at.dimacs /mnt/bt.dimacs 'diff strong strong'
echo "Comparing Tseitin transformation (distinct variables)"
bin/clausy /mnt/att.dimacs /mnt/btt.dimacs 'diff strong strong'

# the script output is:

# common, removed, added
# Comparing distributive transformation
# 52,0,44
# Comparing Tseitin transformation (colliding variables) [this must obviously be wrong, because independently created Tseitin variables do not share semantics at all]
# 24,80,72
# Comparing Tseitin transformation (distinct variables) [this is the one we care about, when we explicitly assign the Tseitin variables different names (here: anonymized names)]
# 52,13260,12236

# this means that 't(a) and not t(b)' (the last line) is not equicountable to 't(a and not b)' (the first line), proving that Tseitin transformation must be the final operation (in justifying non clausal slicing)
# however, possibly 't(a) and t(b)' (the last line) is equicountable to 't(a and b)', as I could not find a counterexample to that (52==52). abut as soon as negation is involved, it gets messy (which also fits with the intuition about Tseitin transformation, which necessarily introduces many satisfiable assignments in the negation of a formula).