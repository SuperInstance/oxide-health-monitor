# oxide-health-monitor

GPU health monitoring with ternary status signals and CRDT-based distributed sync.

## Why This Exists

GPU hardware fails. Memory errors, thermal throttling, PCIe link degradation, power supply flakiness. When a GPU starts failing, you need to detect it fast, route work away from it, and let the rest of the cluster absorb the load. But health isn't binary — a GPU can be fully operational (**Healthy**, +1), degraded but functional (**Degraded**, 0), or dead (**Failed**, -1). Treating everything as "up or down" means you either overreact to transient issues or underreact to slow degradation.

The CRDT merge strategy means multiple health monitors can observe the same fleet independently and merge their observations without coordination. `Failed` overrides everything (fail-fast). `Degraded` propagates but doesn't override `Failed`. This is the same lattice-merge pattern used in distributed databases, applied to GPU fleet management.

## Architecture

```
┌───────────────────────────────────────────────┐
│             HealthMonitor                      │
│                                               │
│  ┌───────┐  ┌───────┐  ┌───────┐             │
│  │ gpu-0 │  │ gpu-1 │  │ gpu-2 │             │
│  │Health.│  │Degrad.│  │Failed │             │
│  │heartb.│  │heartb.│  │failur.│             │
│  │= 42   │  │= 3    │  │= 2    │             │
│  │weight │  │weight │  │weight │             │
│  │= 1.0  │  │= 0.5  │  │= 0.0  │             │
│  └───────┘  └───────┘  └───────┘             │
│                                               │
│  fleet_health() → Health (aggregate)          │
│  fail(id)     → auto-redistribute weight      │
│  heartbeat(id)→ auto-recovery from Degraded   │
│  crdt_merge() → lattice merge with peer       │
└───────────────────────────────────────────────┘

Fleet Health Aggregation:
  > 50% failed    → Failed
  > 50% degraded  → Degraded
  otherwise       → Healthy

Auto-Recovery:
  Degraded node with > 5 consecutive heartbeats → Healthy

Redistribution on Failure:
  Failed node's weight distributed equally to healthy nodes
  healthy_node.weight += 1.0 / healthy_count

CRDT Merge Rules:
  other.Failed  → local.Failed  (fail-fast)
  other.Degraded → local.Degraded (unless local.Failed)
  other.Healthy  → no override
```

**Key types:**

- `Health` — `Healthy(+1)`, `Degraded(0)`, `Failed(-1)`
- `GpuNode` — id, health, heartbeat counter, failure count, workload weight
- `HealthMonitor` — the monitoring engine with CRDT merge support

## Usage

```rust
use oxide_health_monitor::HealthMonitor;

let mut monitor = HealthMonitor::new();
monitor.add_node("gpu-0");
monitor.add_node("gpu-1");
monitor.add_node("gpu-2");
monitor.add_node("gpu-3");

// Regular heartbeats
monitor.heartbeat("gpu-0");
monitor.heartbeat("gpu-1");

// Detect degradation
monitor.degrade("gpu-2"); // thermal throttling
assert_eq!(monitor.fleet_health(), Health::Healthy); // majority still healthy

// Auto-recovery: 5+ heartbeats while Degraded → Healthy
for _ in 0..6 { monitor.heartbeat("gpu-2"); }
// gpu-2 is back to Healthy

// Handle failure with automatic load redistribution
monitor.fail("gpu-3");
// gpu-3's weight redistributed to gpu-0, gpu-1, gpu-2
let healthy = monitor.healthy_nodes();
assert_eq!(healthy.len(), 3);

// CRDT merge with another monitor (distributed deployment)
let mut other_monitor = HealthMonitor::new();
other_monitor.add_node("gpu-0");
other_monitor.add_node("gpu-1");
other_monitor.fail("gpu-0"); // other partition saw gpu-0 fail
monitor.crdt_merge(&other_monitor);
// gpu-0 is now Failed in both monitors (fail-fast propagation)
```

## API Reference

### `Health`

```rust
pub enum Health {
    Healthy = 1,   // Fully operational
    Degraded = 0,  // Functional but impaired
    Failed = -1,   // Dead
}
```

- `val() -> i8` — numeric value

### `GpuNode`

```rust
pub struct GpuNode {
    pub id: String,
    pub health: Health,
    pub consecutive_heartbeats: u64,
    pub failures: u64,
    pub workload_weight: f64,  // 1.0 = normal, 0.5 = degraded, 0.0 = failed
}
```

- `new(id: &str) -> Self`
- `heartbeat()` — increment counter, auto-recover from Degraded after 5 beats
- `fail()` — mark Failed, zero weight
- `degrade()` — mark Degraded, halve weight
- `recover()` — mark Healthy, restore weight to 1.0

### `HealthMonitor`

- `new() -> Self` / `add_node(id: &str)`
- `heartbeat(id: &str)` / `fail(id: &str)` / `degrade(id: &str)` / `recover(id: &str)`
- `fleet_health() -> Health` — aggregate ternary signal
- `healthy_nodes() -> Vec<&str>` / `failed_nodes() -> Vec<&str>`
- `crdt_merge(&mut self, other: &HealthMonitor)` — lattice merge (Failed > Degraded > Healthy)
- `node_count() -> usize` / `alerts() -> &[String]`

## The Deeper Idea

This is the **health layer** in the oxide stack's operational architecture. The ternary health signal (Healthy/Degraded/Failed) is the primitive that every other system consumes. Federation uses it for quorum decisions. Tenancy uses it for quality classification. Capacity planning uses it for effective capacity calculations. Load shedding uses it for admission control.

The CRDT merge strategy makes distributed health monitoring possible without a central coordinator. Each monitor observes a subset of the fleet (e.g., same rack, same availability zone). Merges are commutative, associative, and idempotent — you can merge in any order, any number of times, and converge to the same state. The fail-fast rule (Failed overrides everything) ensures that worst-case information propagates immediately, which is the right bias for infrastructure monitoring.

## Related Crates

- **oxide-federation** — consumes health signals for cross-cluster routing and quorum
- **oxide-capacity** — adjusts effective capacity based on GPU health
- **oxide-tenancy** — uses health signals for tenant quality classification
- **oxide-loadshed** — uses fleet health to adjust admission thresholds
