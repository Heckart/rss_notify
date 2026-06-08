#!/usr/bin/env bash
#shellcheck disable=SC2312

BGreen='\033[1;32m'
BIRed='\033[1;91m'
BIPurple='\033[1;35m'
NC='\033[0m'

echo -e "${BIPurple}Starting test suite...\n"

TEMP_FILE=$(mktemp)
ANYTHING_FAILED=false

echo -e "${BIPurple}Running build/syntax tests..."

echo -e "${NC}Running Rust build test..."
cargo build >"${TEMP_FILE}" 2>&1
rc=$?
if ((rc == 0)); then
	echo -e "${BGreen}Rust build succeeded."
else
	echo -e "${BIRed}Rust build failed:\n$(cat "${TEMP_FILE}")\nPlease fix to pass CI."
	ANYTHING_FAILED=true
fi

echo -e "${NC}Running Bash syntax test..."
bash -n -- *.sh >"${TEMP_FILE}" 2>&1
rc=$?
if ((rc == 0)); then
	echo -e "${BGreen}Bash syntax looks good."
else
	echo -e "${BIRed}Bash has improper syntax:\n$(cat "${TEMP_FILE}")\nPlease fix to pass CI."
	ANYTHING_FAILED=true
fi

echo -e "${BIPurple}Done.\n"

echo -e "${BIPurple}Checking formatting..."

echo -e "${NC}Running Rust formatting check..."
cargo fmt -v --all --check >"${TEMP_FILE}" 2>&1
rc=$?
if ((rc == 0)); then
	echo -e "${BGreen}Rust properly formatted."
else
	echo -e "${BIRed}Rust not properly formatted:\n$(cat "${TEMP_FILE}")\nRun 'cargo fmt --all' to fix."
	ANYTHING_FAILED=true
fi

echo -e "${NC}Running Bash formatting check..."
shfmt -ln bash -d -- *.sh >"${TEMP_FILE}" 2>&1
rc=$?
if ((rc == 0)); then
	echo -e "${BGreen}Bash properly formatted."
else
	echo -e "${BIRed}Bash not properly formatted:\n$(cat "${TEMP_FILE}")\nRun 'shfmt -ln bash -w -- *.sh' to fix."
	ANYTHING_FAILED=true
fi

echo -e "${BIPurple}Done.\n"

echo -e "${BIPurple}Running lint/static analysis..."

echo -e "${NC}Running clippy lint..."
cargo clippy --all-targets -- -Dwarnings >"${TEMP_FILE}" 2>&1
rc=$?
if ((rc == 0)); then
	echo -e "${BGreen}Rust static analysis passed."
else
	echo -e "${BIRed}Rust static analysis failed:\n$(cat "${TEMP_FILE}")\nPlease fix to pass CI."
	ANYTHING_FAILED=true
fi

echo -e "${NC}Running shellcheck lint..."
shellcheck -x -o all -s bash -S style -- *.sh >"${TEMP_FILE}" 2>&1
rc=$?
if ((rc == 0)); then
	echo -e "${BGreen}Bash static analysis passed."
else
	echo -e "${BIRed}Bash static analysis failed:\n$(cat "${TEMP_FILE}")\nPlease fix to pass CI."
	ANYTHING_FAILED=true
fi

echo -e "${BIPurple}Done.\n"

echo -e "${BIPurple}Test suite finished."
rm "${TEMP_FILE}"

if [[ "${ANYTHING_FAILED}" == true ]]; then
	echo -e "${BIRed}Some CI checks failed. Fix them before committing.${NC}"
else
	echo -e "${BGreen}All CI checks passed!${NC}"
fi
