#!/usr/bin/env bash
set -euo pipefail

CLUSTER_NAME="${CLUSTER_NAME:-berg-operator-test}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "==> Setting up kind cluster: $CLUSTER_NAME"

# Check if cluster already exists
if kind get clusters | grep -q "^${CLUSTER_NAME}$"; then
    echo "==> Cluster $CLUSTER_NAME already exists, deleting..."
    kind delete cluster --name "$CLUSTER_NAME"
fi

# Create kind cluster with custom config
echo "==> Creating kind cluster..."
cat <<EOF | kind create cluster --name "$CLUSTER_NAME" --config=-
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
nodes:
- role: control-plane
  extraPortMappings:
  - containerPort: 30080
    hostPort: 30080
    protocol: TCP
  - containerPort: 30443
    hostPort: 30443
    protocol: TCP
EOF

# Wait for cluster to be ready
echo "==> Waiting for cluster to be ready..."
kubectl wait --for=condition=Ready nodes --all --timeout=60s

# Install CRDs
echo "==> Installing CRDs..."
kubectl apply -f "$SCRIPT_DIR/../../charts/berg-operator/templates/crd.yaml"

# Create test namespace
echo "==> Creating test namespace..."
kubectl create namespace berg-test || true

# Load operator image if it exists
if docker images berg-operator:test | grep -q berg-operator; then
    echo "==> Loading operator image into kind..."
    kind load docker-image berg-operator:test --name "$CLUSTER_NAME"
else
    echo "==> Warning: berg-operator:test image not found, will need to build"
fi

echo ""
echo "==> Kind cluster ready!"
echo "    Cluster name: $CLUSTER_NAME"
echo "    kubectl context: kind-$CLUSTER_NAME"
echo ""
echo "Next steps:"
echo "  1. Build operator image: docker build -t berg-operator:test ."
echo "  2. Load image: kind load docker-image berg-operator:test --name $CLUSTER_NAME"
echo "  3. Run tests: ./tests/integration/run-tests.sh"
