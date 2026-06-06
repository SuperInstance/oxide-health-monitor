# oxide-health-monitor

GPU health monitoring with ternary status. Nodes report Healthy(1)/Degraded(0)/Failed(-1). Auto-failover redistributes work from failed nodes. CRDT sync across monitoring agents.

## Why This Matters

# oxide-health-monitor
GPU health monitoring with ternary status.
Healthy(1) / Degraded(0) / Failed(-1). Auto-failover + CRDT sync.

## The Five-Layer Stack

This crate is part of the **Oxide Stack** — a distributed GPU runtime built on five layers:

```
┌─────────────────┐
│  cudaclaw        │  Persistent GPU kernels, warp consensus, SmartCRDT
├─────────────────┤
│  cuda-oxide      │  Flux → MIR → Pliron → NVVM → PTX compiler
├─────────────────┤
│  flux-core       │  Bytecode VM + A2A agent protocol
├─────────────────┤
│  pincher         │  "Vector DB as runtime, LLM as compiler"
├─────────────────┤
│  open-parallel   │  Async runtime (tokio fork)
└─────────────────┘
```

The key insight: **ternary values {-1, 0, +1} map directly to GPU compute**. They pack 16× denser than FP32, enable XNOR+popcount matmul, and conservation laws become compile-time checks.

## Design

Every value in this crate follows **ternary algebra** (Z₃):

| Value | Meaning | GPU Analog |
|-------|---------|------------|
| +1 | Positive / Active / Healthy | Warp vote yes |
| 0 | Neutral / Pending / Balanced | Warp vote abstain |
| -1 | Negative / Failed / Overloaded | Warp vote no |

This isn't arbitrary — ternary is the natural encoding for:
1. **BitNet b1.58** (Microsoft) — ternary LLMs at 60% less power
2. **GPU warp voting** — hardware ballot returns ternary consensus
3. **Conservation laws** — {-1, 0, +1} preserves quantity

## Key Types

```rust
pub enum Health
pub fn val
pub struct GpuNode
pub fn new
pub fn heartbeat
pub fn fail
pub fn degrade
pub fn recover
pub struct HealthMonitor
pub fn new
pub fn add_node
pub fn heartbeat
```

## Usage

```toml
[dependencies]
oxide-health-monitor = "0.1.0"
```

```rust
use oxide_health_monitor::*;
// See src/lib.rs tests for complete working examples
```

## Testing

```bash
git clone https://github.com/SuperInstance/oxide-health-monitor.git
cd oxide-health-monitor
cargo test    # 8 tests
```

## Stats

| Metric | Value |
|--------|-------|
| Tests | 8 |
| Lines of Rust | 208 |
| Public API | 21 items |

## License

Apache-2.0
