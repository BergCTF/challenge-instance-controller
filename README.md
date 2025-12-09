# Berg Operator

Kubernetes operator for managing CTF challenge instances with dynamic flag injection, network isolation, and Gateway API routing.

## Features

- **Dynamic Flag Injection**: Support for environment variable, file content, and executable binary flag modes
- **Network Isolation**: CiliumNetworkPolicy-based egress control with DNS filtering
- **Gateway API Integration**: HTTPRoute and TLSRoute support for external access
- **Multi-Tier Instances**: ChallengeInstanceClass resources for different resource tiers
- **Lifecycle Management**: Automated instance creation, monitoring, and timeout-based cleanup
- **Resource Limits**: Configurable CPU, memory, and bandwidth limits per instance class

## Quick Start

### Prerequisites

- Kubernetes 1.31+ cluster
- kubectl configured
- Helm 3.x
- Gateway API CRDs installed
- Cilium CNI (for network policies)

### Installation

1. **Generate and install CRDs**:
   ```bash
   make crdgen
   make install-crds
   ```

2. **Deploy the operator**:
   ```bash
   helm install berg-operator charts/berg-operator \
     --namespace berg-system \
     --create-namespace \
     --set gateway.domain=challenges.example.com \
     --set gateway.name=your-gateway-name
   ```

3. **Create a Challenge resource** (see example in `tests/fixtures/test-challenge.yaml`)

4. **Create a ChallengeInstance**:
   ```yaml
   apiVersion: berg.norelect.ch/v1
   kind: ChallengeInstance
   metadata:
     name: my-instance
     namespace: default
   spec:
     challengeRef:
       name: example-challenge
       namespace: challenges
     ownerId: user-123
     flag: "flag{example_flag_12345}"
     timeout: "2h"
   ```

## Development

### Building

```bash
# Build the operator
cargo build --release

# Run tests
cargo test

# Build everything (binary + tests + CRDs)
make dev-build
```

### CRD Generation

The operator includes a `crdgen` binary that generates CRD YAML for:
- Challenge
- ChallengeInstance
- ChallengeInstanceClass

**Generate CRDs**:
```bash
# Using make
make crdgen

# Or directly with cargo
cargo run --bin crdgen > charts/berg-operator/templates/crd.yaml
```

**Note**: External CRDs (CiliumNetworkPolicy, HTTPRoute, TLSRoute) are not generated as they come from other projects.

### Integration Testing

The project includes Kind-based integration tests:

```bash
# Setup Kind cluster
make test-setup

# Run integration tests
make test-integration

# Teardown cluster
make test-teardown
```

See [`tests/integration/README.md`](tests/integration/README.md) for more details.

## Architecture

### Custom Resources

1. **Challenge**: Defines a CTF challenge template (containers, ports, flags)
2. **ChallengeInstance**: A running instance of a challenge for a specific user/team
3. **ChallengeInstanceClass**: Configuration template for instance tiers (resources, gateway, etc.)

### Reconciliation Flow

```
Pending → Creating → Starting → Running → Terminating → Terminated
          ↓
       Resources Created:
       - Namespace (challenge-<ownerId>)
       - NetworkPolicy (egress rules)
       - Services (ClusterIP/NodePort)
       - Gateway Routes (HTTP/TLS)
       - ConfigMaps (flag injection)
       - Deployments (challenge containers)
       - PodDisruptionBudgets
```

### Flag Injection Modes

1. **Environment Variable**: Flag injected as env var
2. **File Content**: Flag written to mounted file
3. **Executable Binary**: Minimal ELF that outputs flag via syscall

## Configuration

The operator uses **ChallengeInstanceClass** resources for configuration:

```yaml
apiVersion: berg.norelect.ch/v1
kind: ChallengeInstanceClass
metadata:
  name: premium
spec:
  default: false
  challengeNamespace: challenges
  gateway:
    name: berg-gateway
    namespace: gateway-system
    domain: ctf.example.com
    httpPort: 80
    tlsPort: 443
    httpListenerName: http
    tlsListenerName: tls
  defaultResources:
    cpuRequest: "100m"
    cpuLimit: "2000m"
    memoryRequest: "256Mi"
    memoryLimit: "2Gi"
  imagePull:
    policy: "IfNotPresent"
  security:
    runtimeClassName: "gvisor"
```

## Helm Chart

### Values

Key configuration values:

```yaml
# Create default ChallengeInstanceClass
createInstanceClass: true

# Gateway configuration
gateway:
  name: "berg-gateway"
  namespace: "berg-system"
  domain: "challenges.example.com"

# Instance class defaults
instanceClass:
  name: standard
  defaultTimeout: "2h"
  defaultResources:
    cpuRequest: "100m"
    cpuLimit: "1000m"
```

See [`charts/berg-operator/values.yaml`](charts/berg-operator/values.yaml) for all options.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes (don't forget to run `make crdgen` if you modified CRDs)
4. Run tests: `make test`
5. Submit a pull request

## License

[Add your license here]
