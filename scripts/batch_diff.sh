#!/bin/bash

if [[ $# -ne 1 ]]; then
    echo Please pass a directory with .model files as the first argument. >&2
    exit 1
fi

make
f=($(ls "$1"/*.model | sort -V | tr '\n' ' '))
for ((i = 0; i < ${#f[@]}-1; i++)); do
    cmd=(bin/clausy "${f[i]}" "${f[i+1]}" 'diff y n n csv')
    echo "$(basename "${f[i]}" .model),$(basename "${f[i+1]}" .model),$("${cmd[@]}")"
done