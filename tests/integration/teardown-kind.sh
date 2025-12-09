#!/usr/bin/env bash
set -euo pipefail

CLUSTER_NAME="${CLUSTER_NAME:-berg-operator-test}"

echo "==> Tearing down kind cluster: $CLUSTER_NAME"

if kind get clusters | grep -q "^${CLUSTER_NAME}$"; then
    kind delete cluster --name "$CLUSTER_NAME"
    echo "==> Cluster $CLUSTER_NAME deleted"
else
    echo "==> Cluster $CLUSTER_NAME does not exist"
fi
