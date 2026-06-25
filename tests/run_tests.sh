#!/usr/bin/env bash
set -u

GREEN='\033[0;32m'
RED='\033[0;31m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
NC='\033[0m'

PASS="${GREEN}[PASS]${NC}"
FAIL="${RED}[FAIL]${NC}"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MLTV="$ROOT/target/release/mltv"
TDIR="$ROOT/tests"
TOTAL=0
PCOUNT=0
FCOUNT=0

if [ ! -f "$MLTV" ]; then
    MLTV="$ROOT/target/debug/mltv"
fi
if [ ! -f "$MLTV" ]; then
    echo -e "${RED}Build mltv first: cargo build${NC}"
    exit 1
fi

KBUG="${YELLOW}[KNOWN BUG]${NC}"

run_test() {
    local file="$TDIR/$1"
    local name="$2"
    local expected="$3"
    local expect_fail="${4:-}"

    TOTAL=$((TOTAL + 1))

    if [ ! -f "$file" ]; then
        echo -e "  $FAIL $name (missing $file)"
        FCOUNT=$((FCOUNT + 1))
        return
    fi

    local output ec elapsed
    local start=$(date +%s%N)
    output=$("$MLTV" "$file" 2>/dev/null)
    ec=$?
    local end=$(date +%s%N)
    elapsed=$(echo "scale=3; ($end - $start) / 1000000000" | bc 2>/dev/null || echo "$(( (end - start) / 1000000000 )).000")
    local runtime_line="${elapsed}s"

    local matched=true
    IFS='|' read -ra exps <<< "$expected"
    for exp in "${exps[@]}"; do
        if ! echo "$output" | grep -qF -- "$exp"; then
            matched=false
            break
        fi
    done

    if $matched; then
        if [ "$expect_fail" = "known_bug" ]; then
            echo -e "  ${YELLOW}[UNEXPECTED PASS]${NC} $name ($runtime_line)"
        else
            echo -e "  $PASS $name ($runtime_line)"
        fi
        PCOUNT=$((PCOUNT + 1))
    else
        if [ "$expect_fail" = "known_bug" ]; then
            echo -e "  $KBUG $name ($runtime_line)"
            echo -e "       ${YELLOW}got (exit $ec):${NC} $(echo "$output" | head -3 | tr '\n' ';' | sed 's/;$//')"
            PCOUNT=$((PCOUNT + 1))
        else
            echo -e "  $FAIL $name ($runtime_line)"
            echo -e "       ${YELLOW}expected:${NC} $expected"
            local brief
            brief=$(echo "$output" | head -5 | tr '\n' ';' | sed 's/;$//')
            echo -e "       ${YELLOW}got (exit $ec):${NC} $brief"
            FCOUNT=$((FCOUNT + 1))
        fi
    fi
}

cd "$ROOT"

echo -e "${BOLD}${CYAN}╔════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}${CYAN}║       Molotov Language Test Suite          ║${NC}"
echo -e "${BOLD}${CYAN}╚════════════════════════════════════════════╝${NC}"
echo ""

# ── Section 1: Literals & Values ──────────────────────────────────
echo -e "${BOLD}Literals & Values${NC}"
run_test test_literals.mltv     "literals"           "42|3.14|hello|True|False"
run_test test_operators.mltv    "operators"          "3|7|20|5|2|1|8|-42"
run_test test_variables.mltv    "variables"          "42|99|hello|True"
run_test test_arith_edge.mltv   "arithmetic edge"    "8|1|3|-4|-4|14|14"
run_test test_typeconv.mltv     "type conversions"   "42|123"
run_test test_negation.mltv     "negation"           "-42|10|-5"
run_test test_complex_expr.mltv "complex expr"       "19|7|512|-9|92"
run_test test_str_repeat.mltv   "string repeat"      "hihihi|-----"
run_test test_import.mltv       "higher-order fn"    "42"

# ── Section 2: Variables ──────────────────────────────────────────
echo -e "\n${BOLD}Variables${NC}"
run_test test_variable_reassign.mltv "reassign"       "20"
run_test test_mixed_types.mltv "mixed types"         "42 is the answer|value: 99|x = 10"
run_test test_pass_stmt.mltv  "pass statement"       "ok"

# ── Section 3: Control Flow ───────────────────────────────────────
echo -e "\n${BOLD}Control Flow${NC}"
run_test test_control.mltv    "if/elif/else/loop"    "if_true|non_pos|medium|6|0|1|2|3|4"
run_test test_break_continue.mltv "break/continue"   "0|1|2"
run_test test_nested.mltv     "nested if"            "both_pos"
run_test test_boolean.mltv    "boolean logic"        "and_ok|or_ok|not_ok"
run_test test_comparisons.mltv "comparisons"         "eq|ne|lt|gt|le|ge"
run_test test_short_circuit.mltv "short circuit"     "or_skipped|and_not_called"
run_test test_boolean_ops.mltv "boolean ops"         "True|False|True|False|False|True"
run_test test_comparison_chain.mltv "comparison chain" "True|True|False|True|False|True|True"
run_test test_nested_loops.mltv "nested loops"       "9"
run_test test_while_complex.mltv "while complex"     "6"
run_test test_for_break_continue_nested.mltv "break/continue nested" "1|3"
run_test test_for_range.mltv   "for range"           "0|1|2"
run_test test_for_range_ab.mltv "for range(a,b)"     "2|3|4"
run_test test_range_step.mltv  "range step"          "0|3|6|9"
run_test test_for_enumerate.mltv "for enumerate"      "0|a|1|b|2|c"
run_test test_for_zip.mltv     "for zip"             "5|7|9"
run_test test_for_tuple_unpack.mltv "for tuple unpack" "1|a|2|b|3|c"

# ── Section 4: Functions ──────────────────────────────────────────
echo -e "\n${BOLD}Functions${NC}"
run_test test_functions.mltv   "functions"           "hello from func|7|120|42"
run_test test_func_multi_params.mltv "multi params"   "6|6"
run_test test_func_return.mltv "function return"     "3|1|42"
run_test test_func_recursion.mltv "recursion"        "120|8"
run_test test_func_scope.mltv  "variable scope"      "10|20"

# ── Section 5: Lists ──────────────────────────────────────────────
echo -e "\n${BOLD}Lists${NC}"
run_test test_lists.mltv       "lists"               "2|3|4|4|2|True|True"
run_test test_list_ops2.mltv   "list ops"            "9|1|5"
run_test test_list_copy_clear.mltv "list copy/clear" "4|3|0"
run_test test_list_slice.mltv  "list indexing"       "1|3|4"
run_test test_list_nested.mltv "nested list"         "2|5|3"
run_test test_list_empty.mltv  "empty list"          "2|1|1"
run_test test_list_subscript_assign.mltv "subscript assign" "99|11"

run_test test_list_sort_reverse.mltv "sort/reverse"  "1|5|3|1"
run_test test_list_contains.mltv "list contains"     "True|False|False|True"

# ── Section 6: Strings ────────────────────────────────────────────
echo -e "\n${BOLD}Strings${NC}"
run_test test_strings.mltv     "strings"             "HELLO|hello|hi|a,b,c|5|True|True"
run_test test_split_join.mltv  "split"               "a|b"
run_test test_string_checks.mltv "string checks"     "True|True|True|True|False|True"
run_test test_string_ops2.mltv "string ops"          "Hello world|Hello World|hELLO"
run_test test_string_strip.mltv "string strip"       "hi|hi|hi"
run_test test_string_slice.mltv "string index"       "b|d"
run_test test_string_index.mltv "string index"       "a|d"
run_test test_string_concat.mltv "string concat"     "hello world|foobar"

# ── Section 7: Dictionaries ───────────────────────────────────────
echo -e "\n${BOLD}Dictionaries${NC}"
run_test test_dicts.mltv       "dicts"               "1|3|3"
run_test test_dict_ops2.mltv   "dict ops"            "2|2"
run_test test_dict_in.mltv     "dict in"             "True|False|False|True"
run_test test_dict_nested.mltv "dict len"            "3|2"
run_test test_dict_empty.mltv  "empty dict"          "0|1"

# ── Section 8: Classes ────────────────────────────────────────────
echo -e "\n${BOLD}Classes${NC}"
run_test test_classes.mltv     "classes"             "5|6"

# ── Section 9: Tuples ─────────────────────────────────────────────
echo -e "\n${BOLD}Tuples${NC}"
run_test test_tuple_ops.mltv   "tuples"              "1"

# ── Section 10: Builtins ──────────────────────────────────────────
echo -e "\n${BOLD}Built-in Functions${NC}"
run_test test_builtins.mltv    "builtins"            "42|42|5|3"
run_test test_builtins_more.mltv "builtins more"     "5|3|3|7|10"
run_test test_builtins_math.mltv "builtins math"     "2.5|4|5|0|10"

# ── Section 11: Comprehensions ────────────────────────────────────
echo -e "\n${BOLD}Comprehensions${NC}"
run_test test_comprehensions.mltv "list comp"        "0|8|4|6"
run_test test_comp_nested.mltv "nested comp"         "3|4|6|8"
run_test test_comp_dict.mltv   "dict comp"           "0|2"

# ── Section 12: F-Strings ─────────────────────────────────────────
echo -e "\n${BOLD}F-Strings${NC}"
run_test test_fstrings.mltv    "f-strings"           "hello world|x=42"

# ── Section 13: Error Handling ────────────────────────────────────
echo -e "\n${BOLD}Error Handling${NC}"
run_test test_tryexcept.mltv   "try/except"           "1"
run_test test_try_except_else_finally.mltv "try/except/else/finally" "1|3|4"

# ── Section 14: Augmented Assignment ──────────────────────────────
echo -e "\n${BOLD}Augmented Assignment${NC}"
run_test test_augmented.mltv   "augmented assign"     "15|12|48"

# ── Section 15: Operators ─────────────────────────────────────────
echo -e "\n${BOLD}Operators${NC}"
run_test test_is_in_ops.mltv   "is/in/not in"         "True|False|False|True|is_none"
run_test test_float_ops.mltv   "float ops"            "10|3.5"

# ── Section 16: Delete / Misc ─────────────────────────────────────
echo -e "\n${BOLD}Miscellaneous${NC}"
run_test test_delete.mltv       "del"                 "42|99"
run_test test_io_stress.mltv   "I/O stress"          "2000 files in|deleted in"
run_test test_empty.mltv        "empty containers"    "0|0|0|1|1"
run_test test_none.mltv         "None"                "is_none"

# ── Summary ───────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}${CYAN}════════════════════════════════════════════${NC}"
echo -e "${BOLD}Results:${NC} ${GREEN}${PCOUNT} passed${NC}, ${RED}${FCOUNT} failed${NC}, $((PCOUNT + FCOUNT)) total"
echo -e "${BOLD}${CYAN}════════════════════════════════════════════${NC}"

exit $FCOUNT
