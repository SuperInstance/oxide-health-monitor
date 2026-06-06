# oxide-health-monitor

GPU health monitoring with ternary status. Nodes report Healthy(1)/Degraded(0)/Failed(-1). Auto-failover redistributes work from failed nodes. CRDT sync across monitoring agents.

## Overview

# oxide-health-monitor

GPU health monitoring with ternary status.

## Stats

- **Tests**: 8
- **LOC**: 207
- **License**: Apache-2.0

## Part of the Oxide Stack

This crate is part of the [Flux→PTX](https://github.com/SuperInstance/cuda-oxide/blob/main/FLUX_TO_PTX.md) experimental suite, testing synergies between the five layers of the distributed GPU runtime:

1. **open-parallel** — async runtime (tokio fork)
2. **pincher** — "Vector DB as runtime, LLM as compiler"
3. **flux-core** — bytecode VM + A2A agent protocol
4. **cuda-oxide** — Flux→MIR→Pliron→NVVM→PTX compiler
5. **cudaclaw** — persistent GPU kernels, warp-level consensus, SmartCRDT

## Usage

```rust
use oxide_health_monitor::*;
// See tests in src/lib.rs for examples
```

## License

Apache-2.0
