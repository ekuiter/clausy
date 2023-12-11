#!/bin/bash

if [[ $# -ne 1 ]]; then
    echo Please pass a directory with .model files as the first argument. >&2
    exit 1
fi

make >/dev/null 2>&1
f=($(find "$1" -not -empty -type f -name '*.model' | sort -V | tr '\n' ' '))
echo old_revision,new_revision,common_features,removed_features,added_features,common_constraints,removed_constraints,added_constraints,lost_products,removed_products,common_products,added_products,gained_products,old_revision_count,new_revision_count,old_revision_slice_count,new_revision_slice_count,old_revision_diff_count,new_revision_diff_count,common_products_count,removed_products_count,added_products_count,old_revision_slice_duration,new_revision_slice_duration,old_revision_count_duration,new_revision_count_duration,old_revision_slice_count_duration,new_revision_slice_count_duration,old_revision_diff_count_duration,new_revision_diff_count_duration,tseitin_duration,common_products_count_duration,removed_products_count_duration,added_products_count_duration,total_duration
for ((i = 0; i < ${#f[@]}-1; i++)); do
    cmd=("$(dirname "$0")/../bin/clausy" "${f[i]}" "${f[i+1]}" 'diff csv y n n')
    echo "$(basename "${f[i]}" .model),$(basename "${f[i+1]}" .model),$("${cmd[@]}")"
done