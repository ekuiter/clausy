#!/usr/bin/env bash

set -euo pipefail

[[ $# -ge 3 ]] || { echo "usage: $0 <left_formula> <right_formula> <output_csv> [timeout_sec] [sat_solver]" >&2; exit 1; }

LEFT=$1 RIGHT=$2 CSV=$3 TIMEOUT=${4:-0} SAT_SOLVER=${5:-}
CLAUSY="${CLAUSY:-$(dirname "$0")/../build/clausy}"

[[ -f "$LEFT" ]]   || { echo "left formula not found: $LEFT" >&2; exit 1; }
[[ -f "$RIGHT" ]]  || { echo "right formula not found: $RIGHT" >&2; exit 1; }
[[ -x "$CLAUSY" ]] || { echo "clausy not found: $CLAUSY" >&2; exit 1; }

DIFF_HEADER="common_vars,removed_vars,added_vars,common_constraints,removed_constraints,added_constraints,left_sliced_duration,right_sliced_duration,left_count_duration,left_sliced_count_duration,right_count_duration,right_sliced_count_duration,left_count,left_sliced_count,right_count,right_sliced_count,lost_solutions,gained_solutions,tseitin_or_featureide_duration,common_solutions_count_duration,common_solutions_count,removed_solutions_count_duration,added_solutions_count_duration,removed_solutions_count,added_solutions_count,removed_solutions,common_solutions,added_solutions,classification,total_duration"
echo "left_formula,right_formula,left_diff_kind,right_diff_kind,method,engine,cnf_transform,negate,$DIFF_HEADER,total_duration_shell" > "$CSV"
EMPTY=$(printf ',%.0s' {1..29})
TOOL_FLAGS=()

run() {
    # run <left_mode> <right_mode> <method> <engine> <transform> <negate> <diff subcommand flags...>
    local lm=$1 rm=$2 method=$3 engine=$4 transform=$5 negate=$6; shift 6
    local out ns
    local cmd=("$CLAUSY" "${TOOL_FLAGS[@]}" -i "$LEFT" -i "$RIGHT" diff --no-header --left "$lm" --right "$rm" "$@")
    echo >&2
    echo "==> ${cmd[*]}" >&2
    ns=$(date +%s%N)
    if [[ $TIMEOUT -gt 0 ]]; then
        out=$(timeout -s KILL "$TIMEOUT" "${cmd[@]}" || true)
    else
        out=$("${cmd[@]}" || true)
    fi
    ns=$(($(date +%s%N) - ns))
    if [[ -z "$out" ]]; then
        out="$EMPTY"
    else
        IFS=',' read -ra _fields <<< "$out"
        if (( ${#_fields[@]} < 30 )); then
            while (( ${#_fields[@]} < 30 )); do _fields+=(""); done
            out=$(IFS=','; echo "${_fields[*]}")
        fi
    fi
    echo "$(basename "$LEFT"),$(basename "$RIGHT"),$lm,$rm,$method,$engine,$transform,$negate,$out,$ns" | tee -a "$CSV"
}

# shellcheck disable=SC2043
# We do not evaluate every combination of diff modes here, because the experiment just takes too much time otherwise.
# The two most extreme combinations are false-false (outer diff) and slice-slice (inner diff), and they are also the most interesting in practice.
# Also, in our pre-experiments, distributive transformation almost always times out, except for --satisfy --simplified.
# This is expected for larger formula inputs, even when --negate is not specified.
# for lm in false true slice; do
# for rm in false true slice; do
for lm in false slice; do
for rm in false slice; do
    if [[ $lm != "$rm" ]]; then
        continue
    fi

    for engine in d4 ganak; do
        case $engine in
            d4)    TOOL_FLAGS=() ;;
            ganak) TOOL_FLAGS=(--sharp-sat-path ganak.sh) ;;
        esac
        # for transform in tseitin dist; do
        for transform in tseitin; do
            tf=(); [[ $transform == dist ]] && tf=(--dist)
            for negate in false true; do
                nf=(); [[ $negate == true ]] && nf=(--negate)
                run "$lm" "$rm" count "$engine" "$transform" "$negate" \
                    --count "${tf[@]}" "${nf[@]}"
            done
        done
    done

    for engine in d4-counting d4-proj-ddnnf-compiler d4-projMC ganak-pmc; do
        case $engine in
            d4-*) TOOL_FLAGS=(--d4-projection-mode "${engine#d4-}") ;;
            ganak-pmc) TOOL_FLAGS=(--sharp-sat-path ganak.sh) ;;
        esac
        # for transform in tseitin dist; do
        for transform in tseitin; do
            tf=(); [[ $transform == dist ]] && tf=(--dist)
            for negate in false true; do
                nf=(); uf=()
                [[ $negate == true ]] && nf=(--negate) && uf=(--unsafe)
                run "$lm" "$rm" projected-count "$engine" "$transform" "$negate" \
                    --projected-count "${tf[@]}" "${nf[@]}" "${uf[@]}"
            done
        done
    done

    if [[ -n $SAT_SOLVER ]]; then
        TOOL_FLAGS=(--sat-path "$SAT_SOLVER")
        # for transform in tseitin dist; do
        for transform in tseitin; do
            tf=(); [[ $transform == dist ]] && tf=(--dist)
            run "$lm" "$rm" satisfy "$SAT_SOLVER" "$transform" true \
                --satisfy --negate "${tf[@]}"
        done
        run "$lm" "$rm" satisfy-simplified "$SAT_SOLVER" dist true \
            --satisfy --simplified --negate --dist
    fi

done
done
