#!/usr/bin/env bash

set -euo pipefail

usage() {
    cat <<'USAGE'
Generate N projected-slice DIMACS files for a formula.

Usage:
  scripts/generate_slices.sh --input <formula> --count <n> --out-dir <dir> [options]

Required:
  --input <path>      Input formula file accepted by clausy (e.g., .sat/.model/.cnf)
  --count <n>         Number of slices to generate (n >= 1)
  --out-dir <dir>     Output directory for generated slice files

Options:
  --slice-size <k>    Number of variables to slice away in each file
  --slice-percent-range <min:max>
                      Random slice size range as percent of all named vars (e.g. 10:35)
  --transform <name>  Add a clausy transform (repeatable, e.g. --transform nnf)
  --aux-prefix <str>  Auxiliary-variable prefix used by clausy (default: _aux_)
  --seed <int>        Seed for deterministic random slice generation
  --prefix <name>     Output filename prefix (default: slice)
  --allow-empty       Allow empty slice sets
  --allow-full        Allow slicing away all variables
  --clausy <path>     Path to clausy binary; default auto-detect
  -h, --help          Show this help

Output:
  Writes files <out-dir>/<prefix>_<i>.dimacs for i = 1..n.
  Each file is clausy `print-clauses --slice ...` output with a `c p show ... 0` line.
USAGE
}

INPUT=""
COUNT=""
OUT_DIR=""
SLICE_SIZE=""
SLICE_PERCENT_RANGE=""
SEED=""
PREFIX="slice"
ALLOW_EMPTY=0
ALLOW_FULL=0
CLAUSY_PATH=""
TRANSFORM_ARGS=()
AUX_PREFIX="_aux_"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --input)
            INPUT="${2:-}"
            shift 2
            ;;
        --count)
            COUNT="${2:-}"
            shift 2
            ;;
        --out-dir)
            OUT_DIR="${2:-}"
            shift 2
            ;;
        --slice-size)
            SLICE_SIZE="${2:-}"
            shift 2
            ;;
        --slice-percent-range)
            SLICE_PERCENT_RANGE="${2:-}"
            shift 2
            ;;
        --transform)
            TRANSFORM_ARGS+=(-t "${2:-}")
            shift 2
            ;;
        --aux-prefix)
            AUX_PREFIX="${2:-}"
            shift 2
            ;;
        --seed)
            SEED="${2:-}"
            shift 2
            ;;
        --prefix)
            PREFIX="${2:-}"
            shift 2
            ;;
        --allow-empty)
            ALLOW_EMPTY=1
            shift
            ;;
        --allow-full)
            ALLOW_FULL=1
            shift
            ;;
        --clausy)
            CLAUSY_PATH="${2:-}"
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

if [[ -z "$INPUT" || -z "$COUNT" || -z "$OUT_DIR" ]]; then
    echo "Missing required arguments." >&2
    usage >&2
    exit 1
fi

if ! [[ "$COUNT" =~ ^[0-9]+$ ]] || [[ "$COUNT" -lt 1 ]]; then
    echo "--count must be an integer >= 1, got: $COUNT" >&2
    exit 1
fi

if [[ -n "$SLICE_SIZE" && -n "$SLICE_PERCENT_RANGE" ]]; then
    echo "--slice-size and --slice-percent-range are mutually exclusive" >&2
    exit 1
fi

if [[ -n "$SLICE_SIZE" ]] && { ! [[ "$SLICE_SIZE" =~ ^[0-9]+$ ]]; }; then
    echo "--slice-size must be an integer >= 0, got: $SLICE_SIZE" >&2
    exit 1
fi

if [[ -n "$SLICE_PERCENT_RANGE" ]]; then
    if ! [[ "$SLICE_PERCENT_RANGE" =~ ^[0-9]{1,3}:[0-9]{1,3}$ ]]; then
        echo "--slice-percent-range must match <min:max> with integer percentages, got: $SLICE_PERCENT_RANGE" >&2
        exit 1
    fi
    PCT_MIN="${SLICE_PERCENT_RANGE%%:*}"
    PCT_MAX="${SLICE_PERCENT_RANGE##*:}"
    if [[ "$PCT_MIN" -gt "$PCT_MAX" ]]; then
        echo "--slice-percent-range min must be <= max, got: $SLICE_PERCENT_RANGE" >&2
        exit 1
    fi
    if [[ "$PCT_MIN" -lt 0 || "$PCT_MAX" -gt 100 ]]; then
        echo "--slice-percent-range values must be within 0..100, got: $SLICE_PERCENT_RANGE" >&2
        exit 1
    fi
fi

if [[ ! -f "$INPUT" ]]; then
    echo "Input file does not exist: $INPUT" >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

if [[ -n "$CLAUSY_PATH" ]]; then
    CLAUSY_CMD=("$CLAUSY_PATH")
elif [[ -x "$PROJECT_ROOT/build/clausy" ]]; then
    CLAUSY_CMD=("$PROJECT_ROOT/build/clausy")
elif [[ -x "$PROJECT_ROOT/bin/clausy" ]]; then
    CLAUSY_CMD=("$PROJECT_ROOT/bin/clausy")
else
    CLAUSY_CMD=(cargo run -q --)
fi

if [[ -n "$SEED" ]]; then
    if ! [[ "$SEED" =~ ^-?[0-9]+$ ]]; then
        echo "--seed must be an integer, got: $SEED" >&2
        exit 1
    fi
    RANDOM=$((SEED & 32767))
fi

mapfile -t VARS < <(
    "${CLAUSY_CMD[@]}" --quiet -i "$INPUT" "${TRANSFORM_ARGS[@]}" --aux-prefix "$AUX_PREFIX" print-clauses \
    | awk '/^c [0-9]+ /{sub(/^c [0-9]+ /, ""); print}' \
    | awk -v aux_prefix="$AUX_PREFIX" 'index($0, aux_prefix) != 1' \
    | awk '!seen[$0]++'
)

VAR_COUNT="${#VARS[@]}"
if [[ "$VAR_COUNT" -eq 0 ]]; then
    echo "No named variables found via clausy comment mapping in: $INPUT" >&2
    exit 1
fi

if [[ -n "$SLICE_SIZE" ]] && [[ "$SLICE_SIZE" -gt "$VAR_COUNT" ]]; then
    echo "--slice-size ($SLICE_SIZE) cannot exceed variable count ($VAR_COUNT)" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"

min_size=1
max_size=$((VAR_COUNT - 1))
if [[ -n "$SLICE_PERCENT_RANGE" ]]; then
    # Map percent bounds to variable-count bounds.
    # Lower bound uses ceil, upper bound uses floor.
    min_size=$(((VAR_COUNT * PCT_MIN + 99) / 100))
    max_size=$(((VAR_COUNT * PCT_MAX) / 100))
    if [[ "$max_size" -lt "$min_size" ]]; then
        max_size=$min_size
    fi
else
    if [[ "$ALLOW_EMPTY" -eq 1 ]]; then
        min_size=0
    fi
    if [[ "$ALLOW_FULL" -eq 1 ]]; then
        max_size=$VAR_COUNT
    fi
    if [[ "$VAR_COUNT" -eq 1 ]]; then
        min_size=$((ALLOW_EMPTY == 1 ? 0 : 1))
        max_size=$((ALLOW_FULL == 1 ? 1 : 1))
    fi
    if [[ "$max_size" -lt "$min_size" ]]; then
        max_size=$min_size
    fi
fi

join_by_comma() {
    local IFS=,
    echo "$*"
}

pick_slice_csv() {
    local requested_size="$1"
    local pool=("${VARS[@]}")
    local i j tmp

    for ((i = ${#pool[@]} - 1; i > 0; i--)); do
        j=$((RANDOM % (i + 1)))
        tmp="${pool[i]}"
        pool[i]="${pool[j]}"
        pool[j]="$tmp"
    done

    if [[ "$requested_size" -eq 0 ]]; then
        echo ""
        return
    fi

    local slice=("${pool[@]:0:requested_size}")
    join_by_comma "${slice[@]}"
}

declare -A SEEN

echo "Generating $COUNT slice files from '$INPUT' (vars=$VAR_COUNT) into '$OUT_DIR'"
if [[ -n "$SLICE_PERCENT_RANGE" ]]; then
    echo "Using percent-based slice-size range ${PCT_MIN}%..${PCT_MAX}% => size range ${min_size}..${max_size}"
fi
for ((idx = 1; idx <= COUNT; idx++)); do
    if [[ -n "$SLICE_SIZE" ]]; then
        size="$SLICE_SIZE"
    elif [[ "$min_size" -eq "$max_size" ]]; then
        size="$min_size"
    else
        size=$((min_size + (RANDOM % (max_size - min_size + 1))))
    fi

    attempt=0
    while :; do
        attempt=$((attempt + 1))
        csv="$(pick_slice_csv "$size")"
        key="${csv:-__EMPTY__}"
        if [[ -z "${SEEN[$key]+x}" || "$attempt" -ge 200 ]]; then
            SEEN[$key]=1
            break
        fi
    done

    out_file="$OUT_DIR/${PREFIX}_${idx}.dimacs"
    if [[ -n "$csv" ]]; then
        "${CLAUSY_CMD[@]}" --quiet -i "$INPUT" "${TRANSFORM_ARGS[@]}" --aux-prefix "$AUX_PREFIX" print-clauses --slice "$csv" > "$out_file"
    else
        "${CLAUSY_CMD[@]}" --quiet -i "$INPUT" "${TRANSFORM_ARGS[@]}" --aux-prefix "$AUX_PREFIX" print-clauses > "$out_file"
    fi

    echo "[$idx/$COUNT] wrote $out_file (slice_size=$size)"
done
