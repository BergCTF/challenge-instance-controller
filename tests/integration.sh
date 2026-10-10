#!/usr/bin/env bash

set -euo pipefail

function collect_logs() {
  echo "==> Operator logs:"
  kubectl logs -l app.kubernetes.io/name=berg-controller -n berg-test --tail=300 || true
  echo ""
  echo "==> Operator pod status:"
  kubectl get pods -l app.kubernetes.io/name=berg-controller -n berg-test -o wide || true
  echo ""
  echo "==> Challenge namespaces:"
  kubectl get namespaces -l app.kubernetes.io/managed-by=berg || true
  echo ""
  echo "==> ChallengeInstance status:"
  kubectl get challengeinstance -n berg-test -o yaml || true
  echo ""
  echo "==> Events:"
  kubectl get events -n berg-test --sort-by='.lastTimestamp' || true
}

teardown_cluster() {
  ./tests/integration/teardown-kind.sh || true
}
trap teardown_cluster EXIT

# Use a pre-built image if one exists (e.g. built with a GHA cache in CI);
# otherwise fall back to a local docker build.
if docker image inspect berg-controller:test >/dev/null 2>&1; then
  echo "==> Using pre-built image berg-controller:test"
else
  docker build -t berg-controller:test -f Dockerfile .
fi

if ! ./tests/integration/setup-kind.sh; then
  echo "::error::Failed to set up the kind cluster"
  exit 1
fi

test_status=0
./tests/integration/run-tests.sh || test_status=1

if [ "$test_status" -ne 0 ]; then
  echo "::error::Integration tests failed, collecting logs"
  collect_logs
fi

exit "$test_status"
