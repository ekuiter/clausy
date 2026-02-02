#!/bin/bash

# Integration test script for clausy
# These tests mostly serve the purpose to detect regressions
# and may need to be updated when the transformation logic changes.
#
# Iterates through all .txt files in test/ directory. Each .txt file defines
# a test case with the following format:
#
#   <clausy invocation command>
#   ---
#   <expected output>
#
# If there is no --- separator, no specific output is expected, but the command
# must still exit successfully (exit code 0).
#
# Commands can use $clausy to reference the clausy binary.
#
# The input file (if any) is by convention a file with the same base name but
# a different extension (e.g., simple.sat for simple.txt).
#
# This should be called via `make integration-test` or `make test`.
# Optionally pass a test filter as the first argument (e.g., "dist/simple").
#
# Use --update to update expected output in test files when there's a mismatch.
# This is useful when you've confirmed the new output is correct.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TEST_DIR="$PROJECT_ROOT/test"
# shellcheck disable=SC2034
clausy="$PROJECT_ROOT/build/clausy"

# Parse arguments
update_mode=false
filter=""
for arg in "$@"; do
    if [[ "$arg" == "--update" || "$arg" == "-u" ]]; then
        update_mode=true
    else
        filter="$arg"
    fi
done

passed=0
failed=0
updated=0
failed_tests=()

# Print output limited to 20 lines, with ellipsis if truncated
print_limited() {
    local output="$1"
    local line_count
    line_count=$(echo "$output" | wc -l)
    echo "$output" | head -20 | sed 's/^/    /'
    if [[ $line_count -gt 20 ]]; then
        echo "    ..."
    fi
}

while IFS= read -r -d '' txt_file; do
    test_name="${txt_file#"$TEST_DIR/"}"

    # Skip if filter is set and doesn't match
    if [[ -n "$filter" && "$test_name" != *"$filter"* ]]; then
        continue
    fi

    # Parse the test file: command before ---, expected output after ---
    separator_line=$(grep -n '^---$' "$txt_file" | head -1 | cut -d: -f1)
    if [[ -n "$separator_line" ]]; then
        command_line=$(head -n $((separator_line - 1)) "$txt_file")
        expected_output=$(tail -n +$((separator_line + 1)) "$txt_file")
        check_output=true
    else
        command_line=$(cat "$txt_file")
        expected_output=""
        check_output=false
    fi

    # Run the command from test directory
    cd "$TEST_DIR"
    exit_code=0
    actual_output=$(eval "$command_line" 2>&1) || exit_code=$?

    if [[ $exit_code -ne 0 ]]; then
        echo "FAIL: $test_name (exit code $exit_code)"
        failed_tests+=("$test_name")
        ((failed++)) || true
        echo "  Command: $command_line"
        echo "  Expected: exit code 0"
        echo "  Actual: exit code $exit_code"
        echo "  Output:"
        print_limited "$actual_output"
    elif [[ "$check_output" == true && "$actual_output" != "$expected_output" ]]; then
        if [[ "$update_mode" == true ]]; then
            # Update the test file with actual output
            echo "$command_line" > "$txt_file"
            echo "---" >> "$txt_file"
            echo "$actual_output" >> "$txt_file"
            echo "UPDATED: $test_name"
            ((updated++)) || true
        else
            echo "FAIL: $test_name (output mismatch)"
            failed_tests+=("$test_name")
            ((failed++)) || true
            echo "  Command: $command_line"
            echo "  Expected output:"
            print_limited "$expected_output"
            echo "  Actual output:"
            print_limited "$actual_output"
        fi
    else
        echo "PASS: $test_name"
        ((passed++)) || true
    fi
done < <(find "$TEST_DIR" -name '*.txt' -print0 | sort -z)

echo ""
if [[ $updated -gt 0 ]]; then
    echo "Results: $passed passed, $failed failed, $updated updated"
else
    echo "Results: $passed passed, $failed failed"
fi

if [[ $failed -gt 0 ]]; then
    exit 1
fi
