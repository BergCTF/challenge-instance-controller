#!/usr/bin/env bash
set -euo pipefail

CLUSTER_NAME="${CLUSTER_NAME:-berg-controller-test}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/../fixtures"
TEST_NS="berg-test"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test results
TESTS_PASSED=0
TESTS_FAILED=0

# Helper functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

pass_test() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((TESTS_PASSED++))
}

fail_test() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((TESTS_FAILED++))
}

wait_for_condition() {
    local resource=$1
    local condition=$2
    local timeout=${3:-60}
    local namespace=${4:-$TEST_NS}

    log_info "Waiting for $resource to meet condition: $condition (timeout: ${timeout}s)"
    if kubectl wait --for="$condition" "$resource" -n "$namespace" --timeout="${timeout}s" 2>&1; then
        return 0
    else
        log_error "Timeout waiting for $resource condition: $condition"
        kubectl get -n "$namespace" "$resource"
        return 1
    fi
}

cleanup_test_resources() {
    log_info "Cleaning up test resources..."
    kubectl delete challengeinstance --all -n "$TEST_NS" --ignore-not-found=true --wait=false
    kubectl delete challenge --all -n "$TEST_NS" --ignore-not-found=true --wait=false

    # Wait a bit for cleanup
    sleep 5

    # Force delete any stuck resources
    kubectl delete pods --all -n "$TEST_NS" --grace-period=0 --force --ignore-not-found=true 2>/dev/null || true
}

# ==============================================================================
# Test 1: Operator Deployment
# ==============================================================================
test_operator_deployment() {
    log_info "Test 1: Deploying operator..."

    # Deploy operator using helm
    if helm upgrade --install berg-controller "$SCRIPT_DIR/../../charts/berg-controller" \
        --namespace "$TEST_NS" \
        --set image.repository=berg-controller \
        --set image.tag=test \
        --set image.pullPolicy=IfNotPresent \
        --set config.challengeNamespace="$TEST_NS" \
        --set config.challengeDomain="test.local" \
        --wait --timeout=60s; then
        pass_test "Operator deployed successfully"
    else
        fail_test "Failed to deploy operator"
        kubectl describe deploy -n $TEST_NS
        kubectl describe pods -n $TEST_NS
        return 1
    fi

    # Wait for operator pod to be ready
    if wait_for_condition "pod -lapp.kubernetes.io/name=berg-controller" "condition=Ready"; then
        pass_test "Operator pod is ready"
    else
        fail_test "Operator pod failed to become ready"
        kubectl logs -l app.kubernetes.io/name=berg-controller -n "$TEST_NS" --tail=50 || true
        return 1
    fi
}

# ==============================================================================
# Test 2: CRD Installation
# ==============================================================================
test_crd_installation() {
    log_info "Test 2: Verifying CRD installation..."

    if kubectl get crd challengeinstances.berg.norelect.ch &>/dev/null; then
        pass_test "ChallengeInstance CRD is installed"
    else
        fail_test "ChallengeInstance CRD not found"
        return 1
    fi
}

# ==============================================================================
# Test 3: Challenge Creation
# ==============================================================================
test_challenge_creation() {
    log_info "Test 3: Creating Challenge..."

    if kubectl apply -f "$FIXTURES_DIR/test-challenge.yaml"; then
        pass_test "Challenge created"
    else
        fail_test "Failed to create Challenge"
        return 1
    fi

    # Verify challenge exists
    if kubectl get challenge test-web-challenge -n "$TEST_NS" &>/dev/null; then
        pass_test "Challenge exists in cluster"
    else
        fail_test "Challenge not found in cluster"
        return 1
    fi
}

# ==============================================================================
# Test 4: ChallengeInstance Creation and Reconciliation
# ==============================================================================
test_instance_lifecycle() {
    log_info "Test 4: Creating ChallengeInstance..."

    if kubectl apply -f "$FIXTURES_DIR/test-instance.yaml"; then
        pass_test "ChallengeInstance created"
    else
        fail_test "Failed to create ChallengeInstance"
        return 1
    fi

    # Wait for instance to have status
    log_info "Waiting for instance to be reconciled..."
    sleep 10

    # Check if instance has instanceId
    local instance_id
    instance_id=$(kubectl get challengeinstance test-instance -n "$TEST_NS" -o jsonpath='{.status.instanceId}' 2>/dev/null || echo "")

    if [ -n "$instance_id" ]; then
        pass_test "Instance has ID: $instance_id"
    else
        fail_test "Instance does not have instanceId in status"
        kubectl describe challengeinstance test-instance -n "$TEST_NS" || true
        return 1
    fi

    # Check if namespace was created
    local challenge_ns
    challenge_ns=$(kubectl get challengeinstance test-instance -n "$TEST_NS" -o jsonpath='{.status.namespace}' 2>/dev/null || echo "")

    if [ -n "$challenge_ns" ]; then
        pass_test "Challenge namespace created: $challenge_ns"

        # Verify namespace exists
        if kubectl get namespace "$challenge_ns" &>/dev/null; then
            pass_test "Namespace $challenge_ns exists"
        else
            fail_test "Namespace $challenge_ns not found"
            return 1
        fi
    else
        fail_test "Instance does not have namespace in status"
        return 1
    fi
}

# ==============================================================================
# Test 5: Resource Creation
# ==============================================================================
test_resource_creation() {
    log_info "Test 5: Verifying resources created in challenge namespace..."

    local instance_id
    instance_id=$(kubectl get challengeinstance test-instance -n "$TEST_NS" -o jsonpath='{.status.instanceId}' 2>/dev/null || echo "")

    if [ -z "$instance_id" ]; then
        fail_test "Cannot get instance ID"
        return 1
    fi

    local challenge_ns
    challenge_ns=$(kubectl get challengeinstance test-instance -n "$TEST_NS" -o jsonpath='{.status.namespace}' 2>/dev/null || echo "")

    if [ -z "$challenge_ns" ]; then
        fail_test "Cannot get challenge namespace"
        return 1
    fi

    # Check for deployment
    if kubectl get deployment -n "$challenge_ns" | grep -q "web"; then
        pass_test "Deployment created"
    else
        fail_test "Deployment not found"
        kubectl get all -n "$challenge_ns" || true
        return 1
    fi

    # Check for service
    if kubectl get service -n "$challenge_ns" | grep -q "web"; then
        pass_test "Service created"
    else
        fail_test "Service not found"
    fi

    # Check for ConfigMap (flag)
    if kubectl get configmap -n "$challenge_ns" | grep -q "flag-content"; then
        pass_test "Flag ConfigMap created"
    else
        log_warn "Flag ConfigMap not found (might be expected for some flag modes)"
    fi
}

# ==============================================================================
# Test 6: Pod Status
# ==============================================================================
test_pod_status() {
    log_info "Test 6: Checking pod status..."

    local challenge_ns
    challenge_ns=$(kubectl get challengeinstance test-instance -n "$TEST_NS" -o jsonpath='{.status.namespace}' 2>/dev/null || echo "")

    if [ -z "$challenge_ns" ]; then
        fail_test "Cannot get challenge namespace"
        return 1
    fi

    # Wait for pod to be ready
    if wait_for_condition "pod -l berg.norelect.ch/container=web" "condition=Ready" 120 "$challenge_ns"; then
        pass_test "Pod is ready"
    else
        fail_test "Pod failed to become ready"
        kubectl get pods -n "$challenge_ns" || true
        kubectl describe pod -l berg.norelect.ch/container=web -n "$challenge_ns" || true
        return 1
    fi
}

# ==============================================================================
# Test 7: ChallengeInstance Status Update
# ==============================================================================
test_status_update() {
    log_info "Test 7: Verifying ChallengeInstance status..."

    # Check phase
    local phase
    phase=$(kubectl get challengeinstance test-instance -n "$TEST_NS" -o jsonpath='{.status.phase}' 2>/dev/null || echo "")

    if [ "$phase" = "Running" ] || [ "$phase" = "Starting" ]; then
        pass_test "Instance phase is $phase"
    else
        fail_test "Instance phase is unexpected: $phase"
        kubectl get challengeinstance test-instance -n "$TEST_NS" -o yaml || true
    fi
}

# ==============================================================================
# Test 8: Cleanup and Finalizer
# ==============================================================================
test_cleanup() {
    log_info "Test 8: Testing cleanup and finalizer..."

    local challenge_ns
    challenge_ns=$(kubectl get challengeinstance test-instance -n "$TEST_NS" -o jsonpath='{.status.namespace}' 2>/dev/null || echo "")

    # Delete instance
    if kubectl delete challengeinstance test-instance -n "$TEST_NS" --wait=false; then
        pass_test "ChallengeInstance deletion initiated"
    else
        fail_test "Failed to delete ChallengeInstance"
        return 1
    fi

    # Wait for instance to be deleted
    log_info "Waiting for instance to be fully deleted..."
    local max_wait=60
    local waited=0
    while kubectl get challengeinstance test-instance -n "$TEST_NS" &>/dev/null; do
        sleep 2
        waited=$((waited + 2))
        if [ $waited -ge $max_wait ]; then
            fail_test "Instance deletion timed out"
            kubectl get challengeinstance test-instance -n "$TEST_NS" -o yaml || true
            return 1
        fi
    done
    pass_test "ChallengeInstance fully deleted"

    # Check if challenge namespace was cleaned up
    if [ -n "$challenge_ns" ]; then
        if ! kubectl get namespace "$challenge_ns" &>/dev/null; then
            pass_test "Challenge namespace $challenge_ns was cleaned up"
        else
            log_warn "Challenge namespace $challenge_ns still exists"
        fi
    fi
}

# ==============================================================================
# Main Test Execution
# ==============================================================================
main() {
    log_info "========================================"
    log_info "Berg Operator Integration Tests"
    log_info "========================================"
    log_info "Cluster: $CLUSTER_NAME"
    log_info "Namespace: $TEST_NS"
    log_info ""

    # Verify we're using the right cluster
    local current_context
    current_context=$(kubectl config current-context)
    if [[ "$current_context" != "kind-$CLUSTER_NAME" ]]; then
        log_error "Not using correct kubectl context. Current: $current_context, Expected: kind-$CLUSTER_NAME"
        exit 1
    fi

    # Clean up any previous test resources
    cleanup_test_resources

    # Run tests
    test_operator_deployment || true
    test_crd_installation || true
    test_challenge_creation || true
    test_instance_lifecycle || true
    test_resource_creation || true
    test_pod_status || true
    test_status_update || true
    test_cleanup || true

    # Print summary
    echo ""
    log_info "========================================"
    log_info "Test Summary"
    log_info "========================================"
    echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
    echo -e "${RED}Failed: $TESTS_FAILED${NC}"
    echo ""

    if [ $TESTS_FAILED -eq 0 ]; then
        log_info "All tests passed! 🎉"
        exit 0
    else
        log_error "Some tests failed"
        exit 1
    fi
}

# Handle script interruption
trap cleanup_test_resources EXIT

main "$@"
