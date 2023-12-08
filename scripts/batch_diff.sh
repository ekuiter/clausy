#!/bin/bash

if [[ $# -ne 1 ]]; then
    echo Please pass a directory with .model files as the first argument. >&2
    exit 1
fi

make >/dev/null 2>&1
f=($(find "$1" -not -empty -type f -name '*.model' | sort -V | tr '\n' ' '))
for ((i = 0; i < ${#f[@]}-1; i++)); do
    cmd=("$(dirname "$0")/../bin/clausy" "${f[i]}" "${f[i+1]}" 'diff csv y n n')
    echo "$(basename "${f[i]}" .model),$(basename "${f[i+1]}" .model),$("${cmd[@]}")"
done