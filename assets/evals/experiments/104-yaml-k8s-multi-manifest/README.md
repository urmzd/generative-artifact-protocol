# Experiment: yaml-k8s-multi-manifest

**Format:** text/x-yaml | **Size:** large | **Edits:** 5

**Multi-page design:** 40 resources (Deployments/Services/ConfigMaps) separated by `---`, grouped into 4 logical "pages" (apps `auth`, `catalog`, `orders`, `billing`) of 10 resources each.

## Protocol cost (the only difference from base)

| Prompt | Chars | ~Tokens |
|---|---|---|
| Base system | 92 | 23 |
| GAP init system | 1217 | 304 |
| GAP maintain system | 267 | 67 |
| **Protocol overhead** | | **~348 tokens** |

## Turns
| Turn | Edit |
|---|---|
| 0 | (creation) |
| 1 | Single field on a non-first page: set `orders-worker` Deployment replicas to 6 |
| 2 | Insert a new Service `catalog-scheduler-svc` after `catalog-gateway-svc` |
| 3 | Delete the middle ConfigMap `billing-worker-config` |
| 4 | Bulk relabel `tier: backend` -> `tier: platform` across ALL resources |
| 5 | Add a whole new 10-resource app group `notifications` |

## Multi-page targeting coverage

| Pattern | Turn | Item count after |
|---|---|---|
| Single field, single item, non-first page | 1 | 40 |
| Insert item at a position/page | 2 | 41 |
| Delete a specific middle item | 3 | 40 |
| Bulk change one field across all items | 4 | 40 |
| Add a whole new page/section | 5 | 50 |

Item count is asserted every turn via `regex_count` on `(?m)^kind:` (one match per manifest document), so a flow that "succeeds" on the targeted edit but silently drops the other resources fails the oracle.
