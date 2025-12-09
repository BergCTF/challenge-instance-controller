# Berg Operator Integration Tests

Comprehensive integration tests for the Berg Challenge Instance Controller using Kind (Kubernetes in Docker).

## Prerequisites

### Using Nix (Recommended)

```bash
# Enter the development shell with all dependencies
nix develop
```

### Manual Installation

- Docker
- kubectl
- kind (Kubernetes in Docker)
- helm
- jq

## Quick Start

```bash
# 1. Build the operator image
docker build -t berg-operator:test .

# 2. Set up kind cluster
./tests/integration/setup-kind.sh

# 3. Run integration tests
./tests/integration/run-tests.sh

# 4. Clean up (when done)
./tests/integration/teardown-kind.sh
```

## Test Coverage

The integration test suite covers:

1. **Operator Deployment** - Verifies operator deploys successfully via Helm
2. **CRD Installation** - Confirms ChallengeInstance CRD is registered
3. **Challenge Creation** - Tests Challenge resource creation
4. **Instance Lifecycle** - Tests full ChallengeInstance reconciliation:
   - Instance ID generation
   - Namespace creation
   - Status updates
5. **Resource Creation** - Verifies all child resources are created:
   - Deployments
   - Services
   - ConfigMaps (flags)
   - NetworkPolicies
6. **Pod Status** - Ensures pods reach Ready state
7. **Status Updates** - Validates ChallengeInstance status reflects reality
8. **Cleanup & Finalizers** - Tests proper resource cleanup on deletion

## Test Files

- `setup-kind.sh` - Creates kind cluster with CRDs
- `run-tests.sh` - Main test runner with 8 test cases
- `teardown-kind.sh` - Destroys kind cluster
- `../fixtures/test-challenge.yaml` - Sample Challenge resource
- `../fixtures/test-instance.yaml` - Sample ChallengeInstance resource

## Environment Variables

- `CLUSTER_NAME` - Name of kind cluster (default: `berg-operator-test`)

Example:
```bash
CLUSTER_NAME=my-test-cluster ./tests/integration/setup-kind.sh
```

## Debugging Failed Tests

### View operator logs
```bash
kubectl logs -l app.kubernetes.io/name=berg-operator -n berg-test --tail=100
```

### Inspect ChallengeInstance
```bash
kubectl get challengeinstance test-instance -n berg-test -o yaml
kubectl describe challengeinstance test-instance -n berg-test
```

### Check resources in challenge namespace
```bash
# Get the challenge namespace from instance status
CHALLENGE_NS=$(kubectl get challengeinstance test-instance -n berg-test -o jsonpath='{.status.namespace}')

# View all resources
kubectl get all -n "$CHALLENGE_NS"

# Check pods
kubectl describe pods -n "$CHALLENGE_NS"
kubectl logs -n "$CHALLENGE_NS" -l berg.norelect.ch/container=web
```

### View operator events
```bash
kubectl get events -n berg-test --sort-by='.lastTimestamp'
```

## CI/CD Integration

The integration tests are designed to run in CI environments:

```yaml
# Example GitHub Actions workflow
- name: Run integration tests
  run: |
    docker build -t berg-operator:test .
    ./tests/integration/setup-kind.sh
    ./tests/integration/run-tests.sh
    ./tests/integration/teardown-kind.sh
```

## Extending Tests

To add new test cases, edit `run-tests.sh` and add a function:

```bash
test_my_new_feature() {
    log_info "Test N: Testing my new feature..."

    # Your test logic here

    if [ condition ]; then
        pass_test "Feature works"
    else
        fail_test "Feature broken"
        return 1
    fi
}
```

Then call it in the `main()` function:

```bash
main() {
    # ... existing tests ...
    test_my_new_feature || true
    # ...
}
```

## Notes

- Tests use `berg-test` namespace by default
- Each test run performs cleanup before starting
- Failed tests don't stop execution (allowing full test suite to run)
- Challenge namespaces are prefixed with `challenge-` followed by owner ID
- The operator must be built and tagged as `berg-operator:test` before running tests
