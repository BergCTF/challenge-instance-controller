# ChallengeInstanceClass Refactoring

## Overview

We're refactoring the controller to use `ChallengeInstanceClass` resources for configuration instead of environment variables. This provides better flexibility and allows multiple "tiers" of instances.

## Completed ✅

1. **Created `ChallengeInstanceClass` CRD** (`src/crds/challenge_instance_class.rs`)
   - Gateway configuration (name, namespace, listeners, domain, ports)
   - Resource defaults (CPU, memory)
   - Network configuration (bandwidth, headless service)
   - Image pull configuration (policy, secret)
   - Security configuration (runtime class, security context)
   - Default timeout

2. **Updated `ChallengeInstance` CRD**
   - Added `instanceClass` field (optional, uses default if not specified)

3. **Simplified `ControllerConfig`**
   - Removed all detailed configuration
   - Now only contains:
     - `default_instance_class`: Name of default class
     - `challenge_namespace`: Fallback namespace
     - `default_timeout`: Fallback timeout

4. **Added class resolution**
   - `fetch_instance_class()` function in reconciler
   - Fetches specified class or default
   - Added `InstanceClassNotFound` error variant

5. **Updated reconciler to fetch class**
   - Reconcile function now fetches both Challenge and Class
   - Passes class to all state functions

## Remaining Work ✅ COMPLETED

### 1. Update State Functions ✅

Update all state function signatures in `src/reconciler/state.rs`:

```rust
// From:
pub async fn reconcile_pending(
    instance: Arc<ChallengeInstance>,
    challenge: Challenge,
    ctx: Arc<Context>,
) -> Result<Action>

// To:
pub async fn reconcile_pending(
    instance: Arc<ChallengeInstance>,
    challenge: Challenge,
    class: ChallengeInstanceClass,
    ctx: Arc<Context>,
) -> Result<Action>
```

Apply to:
- `reconcile_pending`
- `reconcile_creating`
- `reconcile_starting`
- `reconcile_running`

### 2. Update Resource Builders ✅

Update all resource builder functions to accept `ChallengeInstanceClass` and use it instead of `ctx.config`:

**`src/resources/gateway.rs`**:
```rust
// Update create_http_routes and create_tls_routes
pub async fn create_http_routes(
    instance: &ChallengeInstance,
    container: &ContainerSpec,
    namespace: &str,
    class: &ChallengeInstanceClass,  // ADD THIS
    ctx: &Context,
) -> Result<Vec<String>>

// Replace:
ctx.config.challenge_domain          → class.spec.gateway.domain
ctx.config.gateway_namespace         → class.spec.gateway.namespace
ctx.config.gateway_name              → class.spec.gateway.name
ctx.config.challenge_http_listener_name → class.spec.gateway.http_listener_name
ctx.config.challenge_tls_listener_name  → class.spec.gateway.tls_listener_name
ctx.config.challenge_http_port       → class.spec.gateway.http_port
ctx.config.challenge_tls_port        → class.spec.gateway.tls_port
```

**`src/resources/namespace.rs`**:
```rust
pub async fn copy_pull_secret(..., class: &ChallengeInstanceClass)

// Replace:
ctx.config.pull_secret_name → class.spec.image_pull.as_ref().and_then(|ip| ip.secret_name.as_ref())
```

**`src/resources/deployment.rs`**:
```rust
pub fn create(..., class: &ChallengeInstanceClass)

// Replace in build_resources():
ctx.config.default_cpu_request       → class.spec.default_resources?.cpu_request
ctx.config.default_cpu_limit         → class.spec.default_resources?.cpu_limit
ctx.config.default_memory_request    → class.spec.default_resources?.memory_request
ctx.config.default_memory_limit      → class.spec.default_resources?.memory_limit

// Replace runtime class:
ctx.config.default_runtime_class_name → class.spec.security?.runtime_class_name
```

**`src/reconciler/state.rs` in `reconcile_creating`**:
```rust
// Replace:
ctx.config.pull_secret_name → class.spec.image_pull.as_ref().and_then(|ip| ip.secret_name.as_ref())

// Replace in resource creation calls:
resources::namespace::copy_pull_secret(&ctx.client, secret_name, &namespace_name, &class).await?;
resources::gateway::create_http_routes(&instance, container, &namespace_name, &class, &ctx).await?;
resources::gateway::create_tls_routes(&instance, container, &namespace_name, &class, &ctx).await?;
resources::deployment::create(&instance, &challenge, container, &namespace_name, &class, &ctx).await?;
```

### 3. Update Helm Chart ✅

**Create ChallengeInstanceClass template** (`charts/berg-operator/templates/challengeinstanceclass.yaml`):

```yaml
{{- if .Values.createInstanceClass }}
apiVersion: berg.norelect.ch/v1
kind: ChallengeInstanceClass
metadata:
  name: {{ .Values.instanceClass.name }}
spec:
  default: {{ .Values.instanceClass.default }}
  challengeNamespace: {{ .Values.config.challengeNamespace }}
  gateway:
    name: {{ .Values.gateway.name }}
    namespace: {{ .Values.gateway.namespace }}
    httpListenerName: {{ .Values.gateway.httpListenerName }}
    tlsListenerName: {{ .Values.gateway.tlsListenerName }}
    domain: {{ .Values.gateway.domain }}
    httpPort: {{ .Values.gateway.httpPort }}
    tlsPort: {{ .Values.gateway.tlsPort }}
  {{- if .Values.instanceClass.defaultResources }}
  defaultResources:
    cpuRequest: {{ .Values.instanceClass.defaultResources.cpuRequest }}
    cpuLimit: {{ .Values.instanceClass.defaultResources.cpuLimit }}
    memoryRequest: {{ .Values.instanceClass.defaultResources.memoryRequest }}
    memoryLimit: {{ .Values.instanceClass.defaultResources.memoryLimit }}
  {{- end }}
  {{- if .Values.instanceClass.network }}
  network:
    egressBandwidth: {{ .Values.instanceClass.network.egressBandwidth }}
    ingressBandwidth: {{ .Values.instanceClass.network.ingressBandwidth }}
    additionalHeadlessService: {{ .Values.instanceClass.network.additionalHeadlessService }}
  {{- end }}
  {{- if .Values.instanceClass.imagePull }}
  imagePull:
    policy: {{ .Values.instanceClass.imagePull.policy }}
    {{- if .Values.instanceClass.imagePull.secretName }}
    secretName: {{ .Values.instanceClass.imagePull.secretName }}
    {{- end }}
  {{- end }}
  {{- if .Values.instanceClass.security }}
  security:
    {{- if .Values.instanceClass.security.runtimeClassName }}
    runtimeClassName: {{ .Values.instanceClass.security.runtimeClassName }}
    {{- end }}
  {{- end }}
  defaultTimeout: {{ .Values.instanceClass.defaultTimeout | default "2h" }}
{{- end }}
```

**Update `values.yaml`**:

```yaml
# Create a default ChallengeInstanceClass
createInstanceClass: true

instanceClass:
  name: standard
  default: true
  defaultTimeout: "2h"

  defaultResources:
    cpuRequest: "100m"
    cpuLimit: "1000m"
    memoryRequest: "128Mi"
    memoryLimit: "512Mi"

  network:
    egressBandwidth: "10M"
    ingressBandwidth: "10M"
    additionalHeadlessService: false

  imagePull:
    policy: "IfNotPresent"
    # secretName: ""

  # security:
  #   runtimeClassName: "gvisor"

# Gateway configuration
gateway:
  name: "berg-gateway"
  namespace: "berg-system"
  httpListenerName: "http"
  tlsListenerName: "tls"
  domain: "challenges.example.com"
  httpPort: 80
  tlsPort: 443

# Operator configuration (simplified)
config:
  challengeNamespace: "berg"

# Controller environment variables (simplified)
env:
  DEFAULT_INSTANCE_CLASS: "standard"
  CHALLENGE_NAMESPACE: "berg"
  DEFAULT_TIMEOUT: "2h"
```

**Update `deployment.yaml`** - simplify environment variables to use new config.

**Update RBAC** (`clusterrole.yaml`) - add permissions for ChallengeInstanceClass:

```yaml
- apiGroups: ["berg.norelect.ch"]
  resources: ["challengeinstanceclasses"]
  verbs: ["get", "list", "watch"]
```

### 4. Generate CRD YAML ✅

The CRD is auto-generated by the CustomResource derive macro and included in the Helm chart.

### 5. Update Tests ✅

**Update `tests/fixtures/test-instance-class.yaml`**:

```yaml
apiVersion: berg.norelect.ch/v1
kind: ChallengeInstanceClass
metadata:
  name: standard
spec:
  default: true
  challengeNamespace: "berg-test"
  gateway:
    name: "test-gateway"
    namespace: "berg-test"
    httpListenerName: "http"
    tlsListenerName: "tls"
    domain: "test.local"
    httpPort: 30080
    tlsPort: 30443
  defaultResources:
    cpuRequest: "50m"
    cpuLimit: "200m"
    memoryRequest: "64Mi"
    memoryLimit: "128Mi"
  defaultTimeout: "1h"
```

**Update integration tests** to create the class before testing.

## Benefits of This Approach

1. **Multiple Tiers**: Can have "basic", "premium", "high-memory" classes
2. **Gateway Flexibility**: Different classes can use different gateways
3. **Resource Quotas**: Enforce different resource limits per tier
4. **Cleaner Configuration**: No massive environment variable list
5. **Runtime Changes**: Can update class without restarting operator
6. **Namespace Isolation**: Different classes can target different challenge namespaces

## Example Classes

**Basic tier** (free users):
```yaml
apiVersion: berg.norelect.ch/v1
kind: ChallengeInstanceClass
metadata:
  name: basic
spec:
  default: true
  challengeNamespace: "challenges-basic"
  gateway:
    name: "public-gateway"
    namespace: "gateway-system"
    domain: "ctf.example.com"
    # ...
  defaultResources:
    cpuLimit: "500m"
    memoryLimit: "256Mi"
  defaultTimeout: "1h"
```

**Premium tier** (paid users):
```yaml
apiVersion: berg.norelect.ch/v1
kind: ChallengeInstanceClass
metadata:
  name: premium
spec:
  challengeNamespace: "challenges-premium"
  gateway:
    name: "premium-gateway"
    namespace: "gateway-system"
    domain: "premium.ctf.example.com"
    # ...
  defaultResources:
    cpuLimit: "4000m"
    memoryLimit: "4Gi"
  defaultTimeout: "4h"
  security:
    runtimeClassName: "gvisor"  # Extra isolation
```

## Migration Path

1. Deploy new CRDs alongside old config
2. Create default ChallengeInstanceClass matching current config
3. Existing instances continue using default class
4. New instances can specify different classes
5. Eventually deprecate environment variable configuration
