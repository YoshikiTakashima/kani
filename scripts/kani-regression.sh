#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

if [[ -z $KANI_REGRESSION_KEEP_GOING ]]; then
  set -o errexit
fi
set -o pipefail
set -o nounset

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
export PATH=$SCRIPT_DIR:$PATH
KANI_DIR=$SCRIPT_DIR/../../..

# Formatting check
${KANI_DIR}/scripts/kani-fmt.sh --check

# Build all packages in the workspace
cargo build --workspace

cd $SCRIPT_DIR/../proptest

# Unit tests
cargo test
# proptest-derive not implemented for now

# Run proptest's internal tests using kani. None for now.
cargo kani --only-codegen

# Check that documentation compiles.
cargo doc --workspace --no-deps --exclude std

echo
echo "All proptest regressions completed successfully."
echo
