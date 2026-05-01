#!/usr/bin/env bash

set -euo pipefail

[[ $# -ge 3 ]] || { echo "usage: $0 <left_formula> <right_formula> <output_csv> [timeout_sec] [sat_solver]" >&2; exit 1; }

LEFT=$1 RIGHT=$2 CSV=$3 TIMEOUT=${4:-0} SAT_SOLVER=${5:-}
CLAUSY="${CLAUSY:-$(dirname "$0")/../build/clausy}"

[[ -f "$LEFT" ]]   || { echo "left formula not found: $LEFT" >&2; exit 1; }
[[ -f "$RIGHT" ]]  || { echo "right formula not found: $RIGHT" >&2; exit 1; }
[[ -x "$CLAUSY" ]] || { echo "clausy not found: $CLAUSY" >&2; exit 1; }

DIFF_HEADER="common_vars,removed_vars,added_vars,common_constraints,removed_constraints,added_constraints,left_sliced_duration,right_sliced_duration,left_count_duration,left_sliced_count_duration,right_count_duration,right_sliced_count_duration,left_count,left_sliced_count,right_count,right_sliced_count,lost_solutions,gained_solutions,tseitin_or_featureide_duration,common_solutions_count_duration,common_solutions_count,removed_solutions_count_duration,added_solutions_count_duration,removed_solutions_count,added_solutions_count,removed_solutions,common_solutions,added_solutions,classification,total_duration"
[[ -s "$CSV" ]] || echo "left_formula,right_formula,left_diff_kind,right_diff_kind,method,engine,cnf_transform,negate,$DIFF_HEADER,total_duration_shell" > "$CSV"
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
# We also omit the projected model counting implementation projMC in d4, because it almost always performed consistently worse than its alternatives.
# for example on BusyBox, at times out on 118 comparisons after 3 minutes, in contrast to the alternatives, which time out on 17 or less comparisons.
# We include ganak + ganak-pmc, which performs badly on Automotive, but never times out on BusyBox.
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

    # for engine in d4-counting d4-proj-ddnnf-compiler d4-projMC ganak-pmc; do
    for engine in d4-counting d4-proj-ddnnf-compiler ganak-pmc; do
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
        if [[ $SAT_SOLVER != kissat ]]; then
            TOOL_FLAGS=(--sat-path "$SAT_SOLVER")
        fi
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

# approximate counting seems not to work at all due to nondeterminism, which is why we omit it here. my hypothesis why this is the case: we need to calculate three numbers (common removed and added), all of which are estimated, but their estimates are not "in sync" with each other, i.e., each solver call is initialized differently and cannot be expected to return a result that makes sense in relation to other previous results (which unfortunately is a requirement of most feature-model analyses, even basic once such as feature cardinalities).
# for run in {1..20}; do docker run --platform linux/amd64 --rm -v ./test/input:/mnt clausy -q -i /mnt/embtoolkit-1.0.0.model -i /mnt/embtoolkit-1.1.0.model --sharp-sat-path approcs-fm.sh diff --left slice --right slice --count --negate --no-header; done
# the correct difference between these two versions is 95% common, 5% added:
# 0,0.9466894327452097,0.053310567254790366,Generalization
# approximate counting yields the follows with --negate:
# 0,0.000000000000000000000000021928702648155287,1,Generalization
# 0,0.04312872077720065,0.9568712792227994,Generalization
# 0,0.09649855404193043,0.9035014459580696,Generalization
# 0,0.9839393088397025,0.01606069116029756,Generalization
# 0,0.9849755086538787,0.01502449134612129,Generalization
# 0,0.9919565718383653,0.00804342816163477,Generalization
# 0,0.9919799485654769,0.008020051434523154,Generalization
# 0,0.9979809418934963,0.0020190581065036893,Generalization
# 0,0.9998462650876122,0.00015373491238784539,Generalization
# which is essentially random garbage. even taking the median run is still pretty off.
# the runtimes are too: running d4 exactly takes about 10-16 seconds, depending on whether we do project at mode counting or not. the measured run times of approximate counting are 22,41,79,42,13,31,74,82, which are almost all slower and very erratic.