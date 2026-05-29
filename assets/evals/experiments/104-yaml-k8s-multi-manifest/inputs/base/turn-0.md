Create a multi-document Kubernetes manifest set for a microservices platform with EXACTLY 40 resources, separated by `---` between every document.

Organize the 40 resources into 4 logical app groups ("pages"), with EXACTLY 10 resources per group:

- Group `auth` (namespace: auth)
- Group `catalog` (namespace: catalog)
- Group `orders` (namespace: orders)
- Group `billing` (namespace: billing)

Within EACH group emit, in this exact order, EXACTLY 10 resources:
- 4 Deployment resources, named `<group>-api`, `<group>-worker`, `<group>-scheduler`, `<group>-gateway`
- 3 Service resources, named `<group>-api-svc`, `<group>-worker-svc`, `<group>-gateway-svc`
- 3 ConfigMap resources, named `<group>-api-config`, `<group>-worker-config`, `<group>-gateway-config`

Every resource MUST include `apiVersion`, `kind`, and a `metadata:` block with `name`, `namespace`, and a `labels:` map containing `app: <resource-name>`, `group: <group>`, and `tier: backend`.

Every Deployment MUST include `spec.replicas` (set to 2), a `spec.selector.matchLabels`, a pod template with one container that has `image: registry.example.com/<resource-name>:v1.0.0`, `resources.requests` (cpu/memory), `resources.limits` (cpu/memory), a `livenessProbe`, a `readinessProbe`, and `env` entries.

Every Service MUST include `spec.type: ClusterIP`, a `selector`, and a `ports` list (port 80 targeting 8080).

Every ConfigMap MUST include a `data:` map with `LOG_LEVEL`, `CACHE_TTL`, and `FEATURE_FLAGS` keys.

Use stable, addressable resource names exactly as specified so individual resources can be located by name. Do not add any resources beyond the 40 specified. Separate each of the 40 documents with `---`.