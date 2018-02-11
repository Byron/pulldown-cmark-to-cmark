#!/bin/bash

set -eu -o pipefail
exe=(cargo run --example stupicat --)

root="$(cd "${0%/*}" && pwd)"
# shellcheck source=./tests/utilities.sh
source "$root/utilities.sh"

SUCCESSFULLY=0

title ""
fixture="$root/fixtures"
snapshot="$fixture/snapshots"

(with "a table"
  it "succeeds" && \
    WITH_SNAPSHOT="$snapshot/stupicat-table-output" \
    expect_run_sh $SUCCESSFULLY "${exe[*]} $fixture/table.md 2>/dev/null"
)

(with "a more complex ordered list"
  it "succeeds" && \
    WITH_SNAPSHOT="$snapshot/stupicat-ordered-output" \
    expect_run_sh $SUCCESSFULLY "${exe[*]} $fixture/ordered.md 2>/dev/null"
)

(with "a more complex unordered list"
  it "succeeds" && \
    WITH_SNAPSHOT="$snapshot/stupicat-unordered-output" \
    expect_run_sh $SUCCESSFULLY "${exe[*]} $fixture/unordered.md 2>/dev/null"
)

(with "a standard common-mark example file"
  it "succeeds" && \
    WITH_SNAPSHOT="$snapshot/stupicat-output" \
    expect_run_sh $SUCCESSFULLY "${exe[*]} $fixture/common-mark.md 2>/dev/null"
)

