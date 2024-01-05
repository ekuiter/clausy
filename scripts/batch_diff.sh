#!/bin/bash

TIMEOUT=${2:-300}
DOCKER=${3:-n}

if [[ $# -lt 1 ]]; then
    echo Please pass a directory with .model files as the first argument. >&2
    exit 1
fi

if [[ $DOCKER == y ]]; then
    CLAUSY=(docker run --rm -i -v "$(readlink -f "$1")":/mnt clausy)
    DIR=/mnt
else
    CLAUSY=("$(dirname "$0")/../bin/clausy")
    DIR=$1
fi

make >/dev/null 2>&1
f=($(cd "$1" && find -not -empty -type f -name '*.model' | sort -V | tr '\n' ' '))
echo old_revision,new_revision,left_diff_kind,right_diff_kind,common_features,removed_features,added_features,common_constraints,removed_constraints,added_constraints,lost_products,removed_products,common_products,added_products,gained_products,old_revision_count,old_revision_slice_count,new_revision_count,new_revision_slice_count,common_products_count,removed_products_count,added_products_count,old_revision_slice_duration,new_revision_slice_duration,old_revision_count_duration,old_revision_slice_count_duration,new_revision_count_duration,new_revision_slice_count_duration,tseitin_duration,common_products_count_duration,removed_products_count_duration,added_products_count_duration,total_duration
for left_diff_kind in bottom-strong top-strong weak; do
    for right_diff_kind in bottom-strong top-strong weak; do
        for ((i = 0; i < ${#f[@]}-1; i++)); do
            cmd=(timeout -s KILL "$TIMEOUT" "${CLAUSY[@]}" "$DIR/${f[i]}" "$DIR/${f[i+1]}" "diff $left_diff_kind $right_diff_kind")
            start=$(date +%s%N)
            echo -n "$(basename "${f[i]}" .model),$(basename "${f[i+1]}" .model),$left_diff_kind,$right_diff_kind,$("${cmd[@]}" || echo ,,,,,,,,,,,,,,,,,,,,,,,,,,,),"
            end=$(date +%s%N)
            echo "$((end - start))"
        done
    done
done
