# Berg Challenge Instance Controller - Technical Specification

## 1. Overview

### 1.1 Purpose
The Berg Challenge Instance Controller is a Kubernetes operator that automates the lifecycle management of CTF challenge instances. It is a purely Kubernetes-native controller that creates and manages challenge infrastructure based on ChallengeInstance custom resources.

### 1.2 Goals
- **Declarative Management**: Instance lifecycle defined through Kubernetes custom resources
- **Event-Driven**: React to ChallengeInstance CR changes via watch mechanism
- **Idempotent**: Reconciliation ensures desired state matches actual state
- **Observable**: Status conditions and events for lifecycle visibility
- **Resource Isolation**: Per-player namespace isolation with network policies
- **Flag Injection**: Support for environment variable, file content, and executable flag injection modes
- **Gateway Integration**: Automatic HTTPRoute/TLSRoute provisioning for challenge ingress
- **Database-Free**: No database connectivity required, all state in Kubernetes

### 1.3 Scope

**The controller manages:**
- ChallengeInstance custom resource reconciliation
- Namespace creation and cleanup
- Deployment, Service, ConfigMap, PodDisruptionBudget creation
- Gateway API HTTPRoute and TLSRoute resources
- Cilium NetworkPolicy configuration
- Flag injection from CR spec (environment, file, or executable)
- Timeout-based termination
- Status reporting and condition tracking

**The controller does NOT manage:**
- Challenge CR lifecycle (managed by challenge authors)
- Player authentication/authorization (handled by Berg API)
- Flag generation (handled by Berg API before CR creation)
- Database persistence (handled by Berg API via CR watch)
- Flag submission validation (handled by Berg API)
- Scoring and solve tracking (handled by Berg API)

### 1.4 Design Principles

- **Single Responsibility**: Controller only manages Kubernetes resources
- **Separation of Concerns**: Business logic (flag generation, database) in API, infrastructure management in controller
- **Kubernetes-Native**: All state stored in Kubernetes API server (CR status)
- **Stateless**: Controller can be restarted without losing state
- **Owner-Based Cleanup**: Owner references ensure automatic garbage collection

---

## 2. Architecture

### 2.1 Controller Pattern

```
┌─────────────────────────────────────────────────────────────┐
│                    Kubernetes API Server                     │
│                                                               │
│  ┌──────────────┐          ┌────────────────────────┐       │
│  │  Challenge   │          │  ChallengeInstance     │       │
│  │     CRD      │          │        CRD             │       │
│  └──────────────┘          └────────────────────────┘       │
└─────────────────────────────────────────────────────────────┘
         │                              │
         │ Watch                        │ Watch
         │                              │
         ▼                              ▼
┌─────────────────────────────────────────────────────────────┐
│          Challenge Instance Controller                       │
│                                                               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │           Informer Cache (Challenge CRs)              │  │
│  └───────────────────────────────────────────────────────┘  │
│                              │                               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │        Informer Cache (ChallengeInstance CRs)         │  │
│  └───────────────────────────────────────────────────────┘  │
│                              │                               │
│                              ▼                               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              Reconciliation Work Queue                 │  │
│  └───────────────────────────────────────────────────────┘  │
│                              │                               │
│                              ▼                               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              Reconciliation Loop                       │  │
│  │                                                         │  │
│  │  1. Fetch ChallengeInstance                           │  │
│  │  2. Fetch referenced Challenge                        │  │
│  │  3. Determine desired state                           │  │
│  │  4. Compare with current state                        │  │
│  │  5. Execute reconciliation actions                    │  │
│  │  6. Update ChallengeInstance status                   │  │
│  │  7. Requeue if needed                                 │  │
│  └───────────────────────────────────────────────────────┘  │
│                              │                               │
│                              ▼                               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │           Kubernetes Resource Manager                  │  │
│  │                                                         │  │
│  │  - Namespace operations                               │  │
│  │  - Deployment lifecycle                               │  │
│  │  - Service provisioning                               │  │
│  │  - ConfigMap for flags                                │  │
│  │  - HTTPRoute/TLSRoute creation                        │  │
│  │  - NetworkPolicy enforcement                          │  │
│  │  - PodDisruptionBudget management                     │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
         │
         │ Creates/Updates/Deletes
         ▼
┌─────────────────────────────────────────────────────────────┐
│                  Kubernetes Resources                        │
│                                                               │
│  Namespace → Deployment → Service → HTTPRoute/TLSRoute      │
│           → ConfigMap → PodDisruptionBudget → NetworkPolicy │
└─────────────────────────────────────────────────────────────┘


                              ┌────────────────┐
                              │   Berg API     │
                              │                │
                              │ - Auth/Authz   │
                              │ - Generate flag│
                              │ - Create CR    │
                              │ - Watch status │
                              │ - Sync to DB   │
                              └────────────────┘
                                      │
                                      │ Create CR
                                      ▼
                              ChallengeInstance
                                      │
                                      │ Watch status updates
                                      ▼
                              Sync to PostgreSQL
```

### 2.2 Component Responsibilities

#### 2.2.1 Controller Manager
- Manages controller lifecycle and leader election
- Initializes informers and work queues
- Handles graceful shutdown
- Provides metrics and health endpoints

#### 2.2.2 Reconciler
- Core reconciliation logic for ChallengeInstance resources
- Implements idempotent state convergence
- Manages owned resource creation/updates/deletions
- Updates ChallengeInstance status and conditions
- Handles error conditions and retry logic

#### 2.2.3 Resource Builder
- Constructs Kubernetes manifests from Challenge and ChallengeInstance specs
- Applies labels and ownership references
- Handles resource naming conventions
- Injects flags from ChallengeInstance spec into containers

#### 2.2.4 Timeout Manager
- Periodic reconciliation to detect expired instances
- Updates ChallengeInstance with termination status
- Triggers namespace deletion for cleanup

---

## 3. Custom Resource Definitions

### 3.1 ChallengeInstance CRD

```yaml
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: challengeinstances.berg.norelect.ch
spec:
  group: berg.norelect.ch
  scope: Namespaced
  names:
    plural: challengeinstances
    singular: challengeinstance
    kind: ChallengeInstance
    shortNames:
      - ci
      - instance
    categories:
      - all
  versions:
    - name: v1
      served: true
      storage: true
      schema:
        openAPIV3Schema:
          type: object
          required:
            - spec
          properties:
            spec:
              type: object
              required:
                - challengeRef
                - ownerId
              properties:
                challengeRef:
                  type: object
                  description: "Reference to the Challenge resource to instantiate"
                  required:
                    - name
                  properties:
                    name:
                      type: string
                      description: "Name of the Challenge resource"
                      maxLength: 64
                    namespace:
                      type: string
                      description: "Namespace of the Challenge resource (optional, defaults to controller's challenge namespace)"
                      maxLength: 63
                ownerId:
                  type: string
                  description: "UUID of the user (player/team) who owns this instance"
                  pattern: '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                flag:
                  type: string
                  description: "Flag for this instance (pre-generated by API if dynamic flags enabled)"
                  maxLength: 1024
                timeout:
                  type: string
                  description: "Duration after which instance auto-terminates (e.g., '2h', '30m')"
                  pattern: '^([0-9]+h)?([0-9]+m)?([0-9]+s)?$'
                  default: "2h"
                terminationReason:
                  type: string
                  description: "Reason for instance termination if manually stopped"
                  enum:
                    - UserRequest
                    - Timeout
                    - AdminTermination
            status:
              type: object
              properties:
                instanceId:
                  type: string
                  description: "UUID uniquely identifying this instance (generated by controller)"
                  pattern: '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                phase:
                  type: string
                  description: "Current lifecycle phase of the instance"
                  enum:
                    - Pending
                    - Creating
                    - Starting
                    - Running
                    - Terminating
                    - Terminated
                    - Failed
                namespace:
                  type: string
                  description: "Kubernetes namespace containing instance resources"
                  maxLength: 63
                services:
                  type: array
                  description: "Publicly accessible service endpoints"
                  items:
                    type: object
                    required:
                      - name
                      - hostname
                      - port
                    properties:
                      name:
                        type: string
                        description: "Service name from Challenge spec"
                      hostname:
                        type: string
                        description: "Fully-qualified domain name for access"
                      port:
                        type: integer
                        description: "Port number for access"
                        minimum: 1
                        maximum: 65535
                      protocol:
                        type: string
                        description: "Network protocol (TCP, UDP)"
                        enum: [TCP, UDP]
                      appProtocol:
                        type: string
                        description: "Application protocol (HTTP, HTTPS, SSH, etc.)"
                      tls:
                        type: boolean
                        description: "Whether TLS is enabled for this service"
                startedAt:
                  type: string
                  format: date-time
                  description: "Timestamp when instance creation began"
                readyAt:
                  type: string
                  format: date-time
                  description: "Timestamp when all pods became ready"
                terminatedAt:
                  type: string
                  format: date-time
                  description: "Timestamp when instance termination completed"
                expiresAt:
                  type: string
                  format: date-time
                  description: "Timestamp when instance will auto-terminate"
                conditions:
                  type: array
                  description: "Detailed status conditions for troubleshooting"
                  items:
                    type: object
                    required:
                      - type
                      - status
                    properties:
                      type:
                        type: string
                        description: "Condition type (e.g., NamespaceCreated, PodsReady)"
                      status:
                        type: string
                        enum: [True, False, Unknown]
                      lastTransitionTime:
                        type: string
                        format: date-time
                      reason:
                        type: string
                        description: "Machine-readable reason code"
                      message:
                        type: string
                        description: "Human-readable message"
                observedGeneration:
                  type: integer
                  description: "Generation of spec that was last reconciled"
      subresources:
        status: {}
      additionalPrinterColumns:
        - name: Challenge
          type: string
          jsonPath: .spec.challengeRef.name
        - name: Owner
          type: string
          jsonPath: .spec.ownerId
        - name: Phase
          type: string
          jsonPath: .status.phase
        - name: Namespace
          type: string
          jsonPath: .status.namespace
        - name: Age
          type: date
          jsonPath: .metadata.creationTimestamp
        - name: Expires
          type: date
          jsonPath: .status.expiresAt
```

**Key Design Changes**:
- **`spec.challengeRef`**: Kubernetes-native reference with optional namespace scoping
- **`spec.ownerId`**: Replaces `playerId` - can represent a player or team UUID
- **`spec.flag`**: Flag is provided by the API (pre-generated), not generated by controller
- **`status.instanceId`**: Generated by controller for tracking (not in spec)
- **`status.namespace`**: Derived from ownerId as `challenge-<ownerId>`, not in spec

### 3.2 Challenge CRD (Existing)
The Challenge CRD exists at `/home/cfi/code/personal/m0unt41n/berg/charts/berg/crds/challenge.yaml`. The controller reads from this CRD but does not modify it.

**Key fields used by the controller**:
- `spec.containers[]` - Container specifications to deploy
- `spec.containers[].ports[]` - Networking configuration
- `spec.containers[].dynamicFlag` - Flag injection mode configuration
- `spec.containers[].resourceLimits/resourceRequests` - Resource quotas
- `spec.allowOutboundTraffic` - Network policy control

---

## 4. Reconciliation Logic

### 4.1 Reconciliation States

```
┌─────────┐
│ Pending │  Initial state when ChallengeInstance is created
└────┬────┘
     │
     ▼
┌──────────┐
│ Creating │  Controller is creating namespace and base resources
└────┬─────┘
     │
     ▼
┌──────────┐
│ Starting │  Deployments created, waiting for pods to be ready
└────┬─────┘
     │
     ▼
┌──────────┐
│ Running  │  All pods ready, services accessible
└────┬─────┘
     │
     │ (Timeout or Delete request)
     ▼
┌─────────────┐
│ Terminating │  Namespace deletion in progress
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Terminated  │  All resources cleaned up (CR can be deleted)
└─────────────┘

┌────────┐
│ Failed │  Unrecoverable error during reconciliation
└────────┘
```

### 4.2 Reconciliation Algorithm

**High-Level Flow**:

1. **Fetch ChallengeInstance**: Read the ChallengeInstance CR from the API server
   - If not found (404), return immediately (already deleted)
   - If other error, requeue with backoff

2. **Handle Deletion** (if `metadata.deletionTimestamp` is set):
   - Execute cleanup logic (see section 4.3.5)
   - Remove finalizer
   - Return (Kubernetes will delete the CR)

3. **Ensure Finalizer**: Add `challengeinstance.berg.norelect.ch/finalizer` if not present
   - Update ChallengeInstance metadata
   - Requeue immediately

4. **Fetch Referenced Challenge**: Read Challenge CR from `spec.challengeRef`
   - Namespace: Use `spec.challengeRef.namespace` if provided, otherwise default to controller's configured challenge namespace
   - Name: Use `spec.challengeRef.name`
   - If not found, set `ChallengeFound` condition to False
   - Set phase to `Failed`
   - Return with error

5. **Initialize Instance ID** (if `status.instanceId` is empty):
   - Generate sequential UUID for tracking
   - Set `status.startedAt` to current timestamp
   - Calculate `status.expiresAt` based on `spec.timeout` (default 2h)
   - Set `status.phase` to `Pending`
   - Update status
   - Requeue immediately

6. **Check Timeout Expiration**:
   - Compare current time with `status.expiresAt`
   - If expired:
     - Set `spec.terminationReason` to `Timeout`
     - Delete ChallengeInstance CR (triggers finalizer cleanup)
     - Return

7. **Reconcile Based on Phase**:
   - Dispatch to phase-specific reconciliation function
   - Return result and requeue duration

### 4.3 Phase Reconciliation Details

#### 4.3.1 Pending → Creating

**Objective**: Validate configuration and transition to resource creation.

**Actions**:
1. Validate that `spec.flag` is provided if Challenge requires dynamic flags
   - If Challenge has `spec.containers[].dynamicFlag` configuration but `spec.flag` is empty:
     - Set `FlagValidation` condition to False
     - Set phase to `Failed`
     - Return with error
2. Update `status.phase` to `Creating`
3. Set `FlagValidation` condition to True
4. Update status
5. Requeue immediately

**Conditions Updated**:
- `FlagValidation`: True (flag provided when required) / False (flag missing)

**Requeue**: Immediate

#### 4.3.2 Creating

**Objective**: Create namespace and all owned resources.

**Resource Creation Sequence**:

1. **Determine Namespace Name**: Derive from ownerId as `challenge-<ownerId>`
   - Example: ownerId `a1b2c3d4-e5f6-7890-abcd-ef1234567890` → namespace `challenge-a1b2c3d4-e5f6-7890-abcd-ef1234567890`

2. **Namespace**: Create namespace for this instance
   ```yaml
   apiVersion: v1
   kind: Namespace
   metadata:
     name: challenge-a1b2c3d4-e5f6-7890-abcd-ef1234567890
     labels:
       app.kubernetes.io/managed-by: berg
       app.kubernetes.io/component: challenge
       berg.norelect.ch/challenge: <challengeRef.name>
       berg.norelect.ch/challenge-namespace: <challengeRef.namespace>
       berg.norelect.ch/owner-id: <ownerId>
       berg.norelect.ch/instance-id: <instanceId>
   ```

3. **Image Pull Secret** (if configured):
   - Read Secret from controller namespace
   - Copy to instance namespace with same name
   - Type: `kubernetes.io/dockerconfigjson`

4. **Cilium NetworkPolicy**:
   ```yaml
   apiVersion: cilium.io/v2
   kind: CiliumNetworkPolicy
   metadata:
     name: challenge-network-policy
     namespace: challenge-<ownerId>
   spec:
     endpointSelector: {}  # Applies to all pods
     egress:
       # Allow DNS to kube-dns
       - toEndpoints:
           - matchLabels:
               k8s:io.kubernetes.pod.namespace: kube-system
               k8s:k8s-app: kube-dns
         toPorts:
           - ports:
               - port: "53"
             rules:
               dns:  # Only if allowOutboundTraffic: false
                 - matchPattern: "*.challenge-<ownerId>.svc.cluster.local."

       # Allow traffic to pods in same namespace
       - toEndpoints:
           - {}

       # Allow traffic to host for OIDC callbacks
       - toEntities:
           - host
         toPorts:
           - ports:
               - port: "<challengeHttpPort>"
               - port: "<challengeTlsPort>"

       # Allow internet (only if allowOutboundTraffic: true)
       - toEntities:
           - world
   ```

5. **For Each Container in Challenge**:

   **a. Services** (ClusterIP, NodePort, Headless as needed)

   **b. HTTPRoutes** (for ports with `type: publicHttpRoute`)

   **c. TLSRoutes** (for ports with `type: publicTlsRoute`)

   **d. ConfigMaps** (for flag injection):

   - **Flag Content** (if `dynamicFlag.content` specified):
     ```yaml
     apiVersion: v1
     kind: ConfigMap
     metadata:
       name: flag-content
       namespace: challenge-<ownerId>
       labels:
         app.kubernetes.io/managed-by: berg
         app.kubernetes.io/component: flag-content
     binaryData:
       content: <base64-encoded-flag-from-spec-with-newline>
     ```
     **Flag source**: `spec.flag` from ChallengeInstance CR

   - **Flag Executable** (if `dynamicFlag.executable` specified):
     ```yaml
     apiVersion: v1
     kind: ConfigMap
     metadata:
       name: flag-executable
       namespace: challenge-<ownerId>
       labels:
         app.kubernetes.io/managed-by: berg
         app.kubernetes.io/component: flag-executable
     binaryData:
       executable: <base64-encoded-elf-binary-with-embedded-flag>
     ```
     **Flag source**: `spec.flag` from ChallengeInstance CR embedded into executable

   **e. PodDisruptionBudget**

   **f. Deployment**:
   ```yaml
   apiVersion: apps/v1
   kind: Deployment
   metadata:
     name: <container.hostname>
     namespace: challenge-<ownerId>
   spec:
     replicas: 1
     selector:
       matchLabels:
         app.kubernetes.io/managed-by: berg
         app.kubernetes.io/component: challenge-pod
         berg.norelect.ch/challenge: <challengeRef.name>
         berg.norelect.ch/owner-id: <ownerId>
         berg.norelect.ch/container: <container.hostname>
     template:
       metadata:
         labels:
           app.kubernetes.io/managed-by: berg
           app.kubernetes.io/component: challenge-pod
           berg.norelect.ch/challenge: <challengeRef.name>
           berg.norelect.ch/owner-id: <ownerId>
           berg.norelect.ch/container: <container.hostname>
         annotations:
           kubernetes.io/egress-bandwidth: <egressBandwidth>
           kubernetes.io/ingress-bandwidth: <ingressBandwidth>
           cluster-autoscaler.kubernetes.io/safe-to-evict: "false"
       spec:
         hostname: <container.hostname>
         enableServiceLinks: false
         automountServiceAccountToken: false
         terminationGracePeriodSeconds: 0
         runtimeClassName: <container.runtimeClassName || default>
         imagePullSecrets:
           - name: <pullSecretName>
         containers:
           - name: <container.hostname>
             image: <container.image>
             imagePullPolicy: <imagePullPolicy>
             env:
               # From container.environment
               - name: <KEY>
                 value: <VALUE>

               # Service endpoints
               - name: <PORT_NAME>_ENDPOINT
                 value: <hostname>:<port>

               # Instance metadata
               - name: CHALLENGE_NAMESPACE
                 value: challenge-<ownerId>

               # Dynamic flag (if env mode)
               - name: <dynamicFlag.env.name>
                 value: <spec.flag>

             volumeMounts:
               # Dynamic flag content (if content mode)
               - name: content
                 mountPath: <dynamicFlag.content.path>
                 subPath: <filename>
                 readOnly: true

               # Dynamic flag executable (if executable mode)
               - name: executable
                 mountPath: <dynamicFlag.executable.path>
                 subPath: <filename>
                 readOnly: true

             resources:
               limits:
                 cpu: <container.resourceLimits.cpu || default>
                 memory: <container.resourceLimits.memory || default>
               requests:
                 cpu: <container.resourceRequests.cpu || default>
                 memory: <container.resourceRequests.memory || default>

             readinessProbe: <container.readinessProbe>
             livenessProbe: <container.livenessProbe>

             securityContext:
               privileged: false
               allowPrivilegeEscalation: true
               capabilities:
                 add: <container.additionalCapabilities>
                 drop:
                   - DAC_OVERRIDE  # If executable flag mode

         volumes:
           - name: content
             configMap:
               name: flag-content
               items:
                 - key: content
                   path: <filename>
                   mode: <dynamicFlag.content.mode>

           - name: executable
             configMap:
               name: flag-executable
               items:
                 - key: executable
                   path: <filename>
                   mode: <dynamicFlag.executable.mode>
   ```

6. **Update Status**:
   - Set `status.namespace` to `challenge-<ownerId>`
   - Set `status.phase` to `Starting`
   - Set conditions: `NamespaceCreated`, `NetworkPolicyCreated`, `ServicesCreated`, `DeploymentsCreated` to True
   - Update status

7. **Requeue**: After 2 seconds

**Path Entropy Substitution**:
When `dynamicFlag.content.path` or `dynamicFlag.executable.path` contains `{entropy}`, replace with 12 random hex characters.

#### 4.3.3 Starting

**Objective**: Wait for all pods to become ready.

**Actions**:
1. List all Pods in instance namespace with label selector:
   ```
   app.kubernetes.io/managed-by=berg,
   app.kubernetes.io/component=challenge-pod
   ```

2. Check pod readiness:
   - All pods must have `status.phase: Running`
   - All pods must have condition `type=Ready` with `status=True`

3. **If all pods ready**:
   - Set `status.readyAt` to current timestamp
   - Build `status.services` array by discovering endpoints
   - Set `status.phase` to `Running`
   - Set `PodsReady` condition to True
   - Update status
   - Requeue at expiration time

4. **If any pods not ready**:
   - Set `PodsReady` condition to Unknown
   - Keep `status.phase` as `Starting`
   - Requeue after 5 seconds

**Service Endpoint Discovery**:
For each container port:
- **InternalPort**: Skip
- **PublicPort**: Extract NodePort from Service
- **PublicHttpRoute**: Extract hostname from HTTPRoute label
- **PublicTlsRoute**: Extract hostname from TLSRoute label

**Conditions Updated**:
- `PodsReady`: True / Unknown

**Requeue**: 5 seconds (if not ready), or at expiration time (if ready)

#### 4.3.4 Running

**Objective**: Monitor instance health and enforce timeout.

**Actions**:
1. Compare current time with `status.expiresAt`
2. If expired:
   - Set `spec.terminationReason` to `Timeout`
   - Delete ChallengeInstance CR
   - Return
3. If not expired:
   - Verify pod health
   - Requeue at expiration time

**Conditions Updated**:
- `Healthy`: True / False

**Requeue**: At expiration time

#### 4.3.5 Terminating

**Objective**: Clean up all instance resources.

**Actions** (Finalizer Cleanup):
1. Delete namespace `challenge-<ownerId>` if it exists
2. Wait for namespace deletion (Kubernetes cascades delete)
3. Set `status.terminatedAt` to current timestamp
4. Set `status.phase` to `Terminated`
5. Set `NamespaceDeleted` condition to True
6. Update status
7. Remove finalizer
8. Return (no requeue)

**Conditions Updated**:
- `NamespaceDeleted`: True / False

**Requeue**: None

### 4.4 Finalizer Pattern

**Finalizer Name**: `challengeinstance.berg.norelect.ch/finalizer`

Ensures namespace cleanup before CR deletion.

---

## 5. Configuration

### 5.1 Configuration Parameters

| Parameter | Type | Description | Default |
|-----------|------|-------------|---------|
| `CHALLENGE_NAMESPACE` | string | Namespace where Challenge CRs are stored | `berg` |
| `CHALLENGE_DOMAIN` | string | Base domain for instance hostnames | `challenges.example.com` |
| `CHALLENGE_HTTP_PORT` | int | Port for HTTP gateway listener | `80` |
| `CHALLENGE_TLS_PORT` | int | Port for TLS gateway listener | `443` |
| `GATEWAY_NAME` | string | Name of Gateway API Gateway resource | `berg-gateway` |
| `GATEWAY_NAMESPACE` | string | Namespace of Gateway resource | `berg` |
| `CHALLENGE_HTTP_LISTENER_NAME` | string | HTTP listener name in Gateway | `http` |
| `CHALLENGE_TLS_LISTENER_NAME` | string | TLS listener name in Gateway | `tls` |
| `CHALLENGE_INSTANCE_TIMEOUT` | duration | Default instance timeout | `2h` |
| `CHALLENGE_CPU_LIMIT` | string | Default CPU limit | `1000m` |
| `CHALLENGE_CPU_REQUEST` | string | Default CPU request | `100m` |
| `CHALLENGE_MEMORY_LIMIT` | string | Default memory limit | `512Mi` |
| `CHALLENGE_MEMORY_REQUEST` | string | Default memory request | `128Mi` |
| `CHALLENGE_EGRESS_BANDWIDTH` | string | Default egress bandwidth | `10M` |
| `CHALLENGE_INGRESS_BANDWIDTH` | string | Default ingress bandwidth | `10M` |
| `CHALLENGE_IMAGE_PULL_POLICY` | string | Image pull policy | `IfNotPresent` |
| `CHALLENGE_RUNTIME_CLASS_NAME` | string | Default runtime class | `""` |
| `PULL_SECRET_NAME` | string | Docker registry pull secret | `""` |
| `CHALLENGE_ADDITIONAL_HEADLESS_SERVICE` | bool | Create headless service | `false` |

### 5.2 RBAC Requirements

```yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: berg-challenge-instance-controller
rules:
  # ChallengeInstance management
  - apiGroups: ["berg.norelect.ch"]
    resources: ["challengeinstances"]
    verbs: ["get", "list", "watch", "update", "patch"]
  - apiGroups: ["berg.norelect.ch"]
    resources: ["challengeinstances/status"]
    verbs: ["get", "update", "patch"]
  - apiGroups: ["berg.norelect.ch"]
    resources: ["challengeinstances/finalizers"]
    verbs: ["update"]

  # Challenge read access
  - apiGroups: ["berg.norelect.ch"]
    resources: ["challenges"]
    verbs: ["get", "list", "watch"]

  # Namespace management
  - apiGroups: [""]
    resources: ["namespaces"]
    verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]

  # Core resources
  - apiGroups: [""]
    resources: ["services", "configmaps", "secrets", "pods"]
    verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]

  # Deployments
  - apiGroups: ["apps"]
    resources: ["deployments"]
    verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]

  # PodDisruptionBudgets
  - apiGroups: ["policy"]
    resources: ["poddisruptionbudgets"]
    verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]

  # Gateway API
  - apiGroups: ["gateway.networking.k8s.io"]
    resources: ["httproutes", "tlsroutes"]
    verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]

  # Cilium
  - apiGroups: ["cilium.io"]
    resources: ["ciliumnetworkpolicies"]
    verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]

  # Events
  - apiGroups: [""]
    resources: ["events"]
    verbs: ["create", "patch"]
```

---

## 6. Flag Injection

### 6.1 Flag Source

**The flag is provided in `spec.flag` of the ChallengeInstance CR**, pre-generated by the Berg API. The controller does NOT generate flags.

### 6.2 Flag Injection Modes

The controller reads the Challenge CR to determine how to inject the flag from `spec.flag`:

#### 6.2.1 Environment Variable

**Configuration** (in Challenge CR):
```yaml
spec:
  containers:
    - hostname: web
      dynamicFlag:
        env:
          name: FLAG
```

**Implementation**:
- Add environment variable to container:
  ```yaml
  env:
    - name: FLAG
      value: <spec.flag>
  ```

#### 6.2.2 File Content

**Configuration** (in Challenge CR):
```yaml
spec:
  containers:
    - hostname: web
      dynamicFlag:
        content:
          path: /home/ctf/flag.txt
          mode: 0444
```

**Implementation**:
1. Create ConfigMap with `spec.flag` + `\n`
2. Mount as volume
3. Path supports `{entropy}` placeholder

#### 6.2.3 Executable

**Configuration** (in Challenge CR):
```yaml
spec:
  containers:
    - hostname: pwn
      dynamicFlag:
        executable:
          path: /usr/bin/get_flag
          mode: 0555
```

**Implementation**:
1. Generate ELF binary embedding `spec.flag`
2. Store in ConfigMap
3. Mount as volume
4. Drop `CAP_DAC_OVERRIDE`

**Executable Requirements**:
- Architecture: x86_64 ELF
- Behavior: Print `spec.flag` to stdout and exit
- Security: No symbols, obfuscated flag string

---

## 7. Integration with Berg API

### 7.1 API Responsibilities

The Berg API handles:
1. Authentication and authorization
2. **Flag generation** (dynamic flags with suffix/leetify modes)
3. Creating ChallengeInstance CRs with pre-generated flags
4. Watching ChallengeInstance status
5. Synchronizing status to database
6. Exposing REST API endpoints

### 7.2 Instance Creation Flow

**New Flow** (Declarative):
```
Player → POST /api/instances/current
         ↓
      1. Authenticate player
      2. Check if player has existing instance
      3. Generate dynamic flag (if challenge supports it)
         - Suffix mode: flag{original_a3f2c8d91e47}
         - Leetify mode: flag{0r161n4l}
      4. Create DB record (id, owner_id, challenge_name, flag, started_at)
      5. Create ChallengeInstance CR:
         apiVersion: berg.norelect.ch/v1
         kind: ChallengeInstance
         metadata:
           name: owner-<ownerId>
           namespace: berg
         spec:
           challengeRef:
             name: <challengeName>
             namespace: berg  # Optional, defaults to controller's challenge namespace
           ownerId: <ownerId>
           flag: <generated-flag>
           timeout: 2h
         ↓
      [Controller reconciles asynchronously]
         ↓
      6. Return immediately with instanceId from CR
```

### 7.3 Database Synchronization

**Database serves as cache and historical record**.

**Sync via Watch**:
```
Watch ChallengeInstance resources in namespace "berg"
On ADDED event:
    - Already created in step 4 above
On MODIFIED event (status updates):
    - Update DB with ready_at, terminated_at from status
    - Publish WebSocket notification
On DELETED event:
    - Update DB with terminated_at if not already set
```

**Database Schema**:
```sql
CREATE TABLE instances (
    id UUID PRIMARY KEY,
    owner_id UUID NOT NULL,
    challenge_name VARCHAR(64) NOT NULL,
    flag VARCHAR(1024),
    started_at TIMESTAMP NOT NULL,
    ready_at TIMESTAMP,
    terminated_at TIMESTAMP,
    termination_reason VARCHAR(20)
);
```

### 7.4 API Endpoints

| Endpoint | Behavior |
|----------|----------|
| `POST /api/instances/current` | Generate flag, create CR, return instanceId |
| `GET /api/instances/current` | Read ChallengeInstance CR status |
| `DELETE /api/instances/current` | Delete ChallengeInstance CR |
| `GET /api/instances` (admin) | List all ChallengeInstance CRs |
| `GET /api/instances/historic` | Query database for terminated instances |

---

## 8. Observability

### 8.1 Status Conditions

| Condition Type | Description |
|---------------|-------------|
| `ChallengeFound` | Referenced Challenge CR exists |
| `FlagValidation` | Flag provided when required |
| `NamespaceCreated` | Instance namespace created |
| `NetworkPolicyCreated` | NetworkPolicy created |
| `ServicesCreated` | All Services created |
| `RoutesCreated` | All Routes created |
| `DeploymentsCreated` | All Deployments created |
| `PodsReady` | All pods Running and Ready |
| `Healthy` | Pods remain healthy |
| `NamespaceDeleted` | Cleanup complete |

### 8.2 Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `berg_challengeinstance_total` | Counter | Total instances created |
| `berg_challengeinstance_active` | Gauge | Active instances by phase |
| `berg_challengeinstance_duration_seconds` | Histogram | Instance lifetime |
| `berg_challengeinstance_reconcile_duration_seconds` | Histogram | Reconciliation duration |
| `berg_challengeinstance_reconcile_errors_total` | Counter | Reconciliation errors |
| `berg_challengeinstance_timeouts_total` | Counter | Timeout terminations |

### 8.3 Events

| Type | Reason | Message |
|------|--------|---------|
| Normal | InstanceCreating | "Creating challenge instance" |
| Normal | InstanceReady | "Instance ready and accessible" |
| Normal | InstanceTerminating | "Terminating due to %s" |
| Warning | ChallengeMissing | "Challenge %s not found" |
| Warning | FlagMissing | "Flag required but not provided" |

---

## 9. Security Considerations

### 9.1 Controller Security
- Dedicated ServiceAccount with minimal RBAC
- No database credentials (stateless)
- Distroless container image
- Network policy to only API server

### 9.2 Instance Isolation
- Per-owner namespace isolation
- Cilium network policies
- Runtime isolation (gVisor support)
- Pod Security Standards

### 9.3 Flag Security
- Flags provided by API, not generated by controller
- Executable flags with CAP_DAC_OVERRIDE dropped
- Flags not logged or exposed in metrics

---

## 10. Testing Strategy

### 10.1 Unit Tests
- Reconciler logic with mocked Kubernetes client
- Resource builders verify correct manifests
- Flag injection logic (env, content, executable)

### 10.2 Integration Tests
- End-to-end instance lifecycle
- Timeout enforcement
- Flag injection modes
- Network policy validation

### 10.3 Load Tests
- Concurrent instance creation (100+)
- Sustained load (10 instances/sec)
- Timeout cleanup at scale

---

## 11. Acceptance Criteria

- [ ] ChallengeInstance CRD defined and registered
- [ ] Controller reconciles all phases correctly
- [ ] Flag injection works for all three modes (env, content, executable)
- [ ] Timeout-based termination functions
- [ ] Finalizer ensures cleanup
- [ ] Status conditions accurate
- [ ] No database connectivity required
- [ ] Integration tests pass
- [ ] Metrics exported
- [ ] Security review passed
- [ ] Documentation complete

---

**Specification Version**: 1.0
**Last Updated**: 2025-12-09
**Author**: Berg Development Team
**Status**: Draft for Review
