#!/usr/bin/env bash

set -euo pipefail

usage() {
    cat <<'USAGE'
Run d4 (all three modes) and Ganak on slice CNFs, collect runtimes, and compare results.

Usage:
  scripts/benchmark_d4_slices.sh --input-dir <dir> [options]

Required:
  --input-dir <dir>        Directory with slice files (default pattern: *.dimacs)

Options:
  --pattern <glob>         File glob inside input dir (default: *.dimacs)
  --csv <path>             Output CSV path (default: <input-dir>/d4_ganak_benchmark.csv)
  --d4-path <path>         d4 executable (default: d4)
  --ganak-path <path>      Ganak executable (default: ganak)
  --timeout <seconds>      Per-run timeout via `timeout` command
  -h, --help               Show help

CSV columns:
  slice,solver,mode,status,count,runtime_sec

Comparison output:
  - fastest target per slice
  - count mismatches across all successful targets
  - aggregate wins and average runtime per target
USAGE
}

INPUT_DIR=""
PATTERN="*.dimacs"
CSV_PATH=""
D4_PATH="build/d4"
GANAK_PATH="build/ganak.sh"
TIMEOUT_SEC=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --input-dir)
            INPUT_DIR="${2:-}"
            shift 2
            ;;
        --pattern)
            PATTERN="${2:-}"
            shift 2
            ;;
        --csv)
            CSV_PATH="${2:-}"
            shift 2
            ;;
        --d4-path)
            D4_PATH="${2:-}"
            shift 2
            ;;
        --ganak-path)
            GANAK_PATH="${2:-}"
            shift 2
            ;;
        --timeout)
            TIMEOUT_SEC="${2:-}"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if [[ -z "$INPUT_DIR" ]]; then
    echo "Missing required --input-dir" >&2
    usage >&2
    exit 1
fi

if [[ ! -d "$INPUT_DIR" ]]; then
    echo "Input directory does not exist: $INPUT_DIR" >&2
    exit 1
fi

if [[ -n "$TIMEOUT_SEC" ]] && ! [[ "$TIMEOUT_SEC" =~ ^[0-9]+$ ]]; then
    echo "--timeout must be an integer number of seconds, got: $TIMEOUT_SEC" >&2
    exit 1
fi

if [[ -z "$CSV_PATH" ]]; then
    CSV_PATH="$INPUT_DIR/d4_ganak_benchmark.csv"
fi

mapfile -t FILES < <(find "$INPUT_DIR" -maxdepth 1 -type f -name "$PATTERN" | sort)
if [[ "${#FILES[@]}" -eq 0 ]]; then
    echo "No files matched pattern '$PATTERN' in $INPUT_DIR" >&2
    exit 1
fi

ensure_exec() {
    local p="$1"
    local label="$2"
    if [[ "$p" == */* ]]; then
        if [[ ! -x "$p" ]]; then
            echo "$label executable not found or not executable: $p" >&2
            exit 1
        fi
    else
        if ! command -v "$p" >/dev/null 2>&1; then
            echo "$label executable not found: $p" >&2
            exit 1
        fi
    fi
}

ensure_exec "$D4_PATH" "d4"
ensure_exec "$GANAK_PATH" "ganak"

TARGETS=(
    "d4|counting"
    "d4|proj-ddnnf-compiler"
    "d4|projMC"
    "ganak|default"
)

echo "slice,solver,mode,status,count,runtime_sec" > "$CSV_PATH"

declare -A BEST_TARGET_BY_FILE
declare -A BEST_TIME_BY_FILE
declare -A COUNT_BY_FILE_TARGET
declare -A STATUS_BY_FILE_TARGET
declare -A TIME_BY_FILE_TARGET
declare -A TARGET_WIN_COUNT
declare -A TARGET_TIME_SUM
declare -A TARGET_TIME_N

for target in "${TARGETS[@]}"; do
    TARGET_WIN_COUNT[$target]=0
    TARGET_TIME_SUM[$target]=0
    TARGET_TIME_N[$target]=0
done

extract_count() {
    local stdout_file="$1"
    awk '
        /^s / {
            for (i = NF; i >= 1; i--) {
                if ($i ~ /^[0-9]+$/) {
                    print $i;
                    exit;
                }
            }
        }
    ' "$stdout_file"
}

run_target() {
    local file="$1"
    local target="$2"
    local solver mode
    local stdout_file stderr_file
    local start end elapsed status count rc

    solver="${target%%|*}"
    mode="${target##*|}"

    stdout_file=$(mktemp)
    stderr_file=$(mktemp)

    start=$(perl -MTime::HiRes=time -e 'print time')
    if [[ "$solver" == "d4" ]]; then
        if [[ -n "$TIMEOUT_SEC" ]]; then
            if timeout "$TIMEOUT_SEC" "$D4_PATH" -i "$file" -m "$mode" >"$stdout_file" 2>"$stderr_file"; then
                status="ok"
            else
                rc=$?
                if [[ "$rc" -eq 124 ]]; then
                    status="timeout"
                else
                    status="error($rc)"
                fi
            fi
        else
            if "$D4_PATH" -i "$file" -m "$mode" >"$stdout_file" 2>"$stderr_file"; then
                status="ok"
            else
                rc=$?
                status="error($rc)"
            fi
        fi
    else
        if [[ -n "$TIMEOUT_SEC" ]]; then
            if timeout "$TIMEOUT_SEC" "$GANAK_PATH" "$file" >"$stdout_file" 2>"$stderr_file"; then
                status="ok"
            else
                rc=$?
                if [[ "$rc" -eq 124 ]]; then
                    status="timeout"
                else
                    status="error($rc)"
                fi
            fi
        else
            if "$GANAK_PATH" "$file" >"$stdout_file" 2>"$stderr_file"; then
                status="ok"
            else
                rc=$?
                status="error($rc)"
            fi
        fi
    fi
    end=$(perl -MTime::HiRes=time -e 'print time')
    elapsed=$(awk -v s="$start" -v e="$end" 'BEGIN { printf "%.6f", (e-s) }')

    count=""
    if [[ "$status" == "ok" ]]; then
        count=$(extract_count "$stdout_file")
        if [[ -z "$count" ]]; then
            status="parse_error"
        fi
    fi

    rm -f "$stdout_file" "$stderr_file"
    printf '%s\n%s\n%s\n' "$status" "$count" "$elapsed"
}

echo "Running ${#FILES[@]} files across ${#TARGETS[@]} targets (3 d4 modes + Ganak)..."
for file in "${FILES[@]}"; do
    base=$(basename "$file")
    best_target=""
    best_time=""

    for target in "${TARGETS[@]}"; do
        solver="${target%%|*}"
        mode="${target##*|}"

        mapfile -t result < <(run_target "$file" "$target")
        status="${result[0]}"
        count="${result[1]}"
        runtime="${result[2]}"

        STATUS_BY_FILE_TARGET["$base|$target"]="$status"
        COUNT_BY_FILE_TARGET["$base|$target"]="$count"
        TIME_BY_FILE_TARGET["$base|$target"]="$runtime"

        echo "$base,$solver,$mode,$status,$count,$runtime" >> "$CSV_PATH"

        if [[ "$status" == "ok" ]]; then
            TARGET_TIME_SUM[$target]=$(awk -v a="${TARGET_TIME_SUM[$target]}" -v b="$runtime" 'BEGIN { printf "%.6f", a + b }')
            TARGET_TIME_N[$target]=$((TARGET_TIME_N[$target] + 1))

            if [[ -z "$best_target" ]] || awk -v r="$runtime" -v b="$best_time" 'BEGIN{exit !(r < b)}'; then
                best_target="$target"
                best_time="$runtime"
            fi
        fi
    done

    BEST_TARGET_BY_FILE["$base"]="$best_target"
    BEST_TIME_BY_FILE["$base"]="$best_time"
    if [[ -n "$best_target" ]]; then
        TARGET_WIN_COUNT[$best_target]=$((TARGET_WIN_COUNT[$best_target] + 1))
    fi
done

echo ""
echo "Per-slice fastest target"
for file in "${FILES[@]}"; do
    base=$(basename "$file")
    target="${BEST_TARGET_BY_FILE[$base]}"
    time="${BEST_TIME_BY_FILE[$base]}"
    if [[ -n "$target" ]]; then
        echo "  $base -> ${target%%|*}:${target##*|} (${time}s)"
    else
        echo "  $base -> no successful target"
    fi
done

echo ""
echo "Count consistency check"
mismatch_count=0
for file in "${FILES[@]}"; do
    base=$(basename "$file")
    ref=""
    comparable=0
    mismatch=0
    detail=""

    for target in "${TARGETS[@]}"; do
        status="${STATUS_BY_FILE_TARGET["$base|$target"]}"
        count="${COUNT_BY_FILE_TARGET["$base|$target"]}"
        label="${target%%|*}:${target##*|}"

        if [[ -n "$detail" ]]; then
            detail+="; "
        fi
        detail+="$label=$status"

        if [[ "$status" == "ok" ]]; then
            comparable=$((comparable + 1))
            if [[ -z "$ref" ]]; then
                ref="$count"
            elif [[ "$count" != "$ref" ]]; then
                mismatch=1
            fi
        fi
    done

    if [[ "$comparable" -lt 2 ]]; then
        echo "  skipped: $base ($detail)"
    elif [[ "$mismatch" -eq 1 ]]; then
        mismatch_count=$((mismatch_count + 1))
        echo "  mismatch: $base"
        for target in "${TARGETS[@]}"; do
            status="${STATUS_BY_FILE_TARGET["$base|$target"]}"
            count="${COUNT_BY_FILE_TARGET["$base|$target"]}"
            label="${target%%|*}:${target##*|}"
            echo "    $label -> status=$status, count=$count"
        done
    fi
done
if [[ "$mismatch_count" -eq 0 ]]; then
    echo "  all comparable runs agree"
fi

echo ""
echo "Aggregate comparison"
for target in "${TARGETS[@]}"; do
    n="${TARGET_TIME_N[$target]}"
    avg="n/a"
    if [[ "$n" -gt 0 ]]; then
        avg=$(awk -v s="${TARGET_TIME_SUM[$target]}" -v n="$n" 'BEGIN { printf "%.6f", s / n }')
    fi
    echo "  ${target%%|*}:${target##*|}: wins=${TARGET_WIN_COUNT[$target]}, avg_runtime=${avg}s over ${n} successful runs"
done

echo ""
echo "Wrote CSV: $CSV_PATH"

# # example
# scripts/generate_slices.sh \
#   --input meta/linux/v2.6.0\[i386\].model \
#   --count 100 \
#   --out-dir slices \
#   --slice-percent-range 10:90 \
#   --transform tseitin \
#   --seed 1
# scripts/benchmark_sharp_sat.sh --input-dir slices --timeout 60
# # Aggregate comparison
# #   d4:proj-ddnnf-compiler: wins=0, avg_runtime=41.677193s over 7 successful runs
# #   ganak:default: wins=100, avg_runtime=8.658948s over 100 successful runs