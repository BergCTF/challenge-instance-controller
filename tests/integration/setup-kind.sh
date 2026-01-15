#!/usr/bin/env bash
set -euo pipefail

CLUSTER_NAME="${CLUSTER_NAME:-berg-controller-test}"
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
networking:
  disableDefaultCNI: true
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

echo "Installing cilium"
cat <<EOF | helm --kube-context kind-berg-dev-cluster install --wait cilium cilium/cilium -n cilium --version 1.17.4 --create-namespace -f -
ipam:
  mode: kubernetes
image:
  pullPolicy: IfNotPresent
operator:
  replicas: 1
bandwidthManager:
  enabled: true
hubble:
  enabled: true
  relay:
    enabled: true
  ui:
    enabled: true
    ingress:
      enabled: true
      annotations:
        cert-manager.io/cluster-issuer: mkcert
      className: traefik
      hosts:
        - hubble.localhost
      tls:
        - secretName: hubble-tls
          hosts:
            - hubble.localhost
EOF

kubectl apply -f https://github.com/kubernetes-sigs/gateway-api/releases/download/v1.4.1/experimental-install.yaml

# Wait for cluster to be ready
echo "==> Waiting for cluster to be ready..."
kubectl wait --for=condition=Ready nodes --all --timeout=60s

# Install CRDs
echo "==> Installing CRDs..."
kubectl apply -f "$SCRIPT_DIR/../../crds/crd.yaml"

# Create test namespace
echo "==> Creating test namespace..."
kubectl create namespace berg-test || true

# Install test ChallengeInstanceClass
echo "==> Installing test ChallengeInstanceClass..."
kubectl apply -f "$SCRIPT_DIR/../fixtures/test-instance-class.yaml"

# Load operator image if it exists
if docker images berg-controller:test | grep -q berg-controller; then
    echo "==> Loading controller image into kind..."
    kind load docker-image berg-controller:test --name "$CLUSTER_NAME"
else
    echo "==> Warning: berg-controller:test image not found, will need to build"
fi

echo ""
echo "==> Kind cluster ready!"
echo "    Cluster name: $CLUSTER_NAME"
echo "    kubectl context: kind-$CLUSTER_NAME"
echo ""
echo "Next steps:"
echo "  1. Build operator image: docker build -t berg-controller:test ."
echo "  2. Load image: kind load docker-image berg-controller:test --name $CLUSTER_NAME"
echo "  3. Run tests: ./tests/integration/run-tests.sh"
