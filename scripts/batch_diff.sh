#!/usr/bin/env bash
# A useful script to evaluate a whole folder of formulas to compare.
# Can be automated to run on a remote server on multiple systems for example as follows:
# screen -dmSL diff bash -c "for system in toybox FinancialServices01 automotive2 axtls busybox embtoolkit uclibc-ng; do scripts/batch_diff.sh ../models/$system output/$system.csv 180 n kissat; done"

set -euo pipefail

[[ $# -ge 2 ]] || { echo "usage: $0 <dir> <output_csv> [timeout_sec] [docker:y/n] [sat_solver]" >&2; exit 1; }

DIR=$(readlink -f "$1") CSV=$2 TIMEOUT=${3:-300} DOCKER=${4:-n} SAT_SOLVER=${5:-}
EVALUATE="$(dirname "$0")/evaluate_diff.sh"

wrapper=""
trap 'rm -f "$wrapper"' EXIT

if [[ $DOCKER == y ]]; then
    abs_dir=$DIR
    wrapper=$(mktemp)
    chmod +x "$wrapper"
    cat > "$wrapper" <<EOF
#!/usr/bin/env bash
args=()
for arg in "\$@"; do
    [[ "\$arg" == "$abs_dir/"* ]] && arg="/mnt/\${arg#$abs_dir/}"
    args+=("\$arg")
done
docker run --platform linux/amd64 --rm -v "$abs_dir:/mnt" clausy "\${args[@]}"
EOF
    export CLAUSY="$wrapper"
else
    make -C "$(dirname "$0")/.." >/dev/null 2>&1
fi

mapfile -t files < <(cd "$DIR" && find . ! -empty -type f | sort -V)

for ((i = 0; i < ${#files[@]}-1; i++)); do
    left="$DIR/${files[i]}"
    right="$DIR/${files[i+1]}"
    "$EVALUATE" "$left" "$right" "$CSV" "$TIMEOUT" "$SAT_SOLVER"
done
