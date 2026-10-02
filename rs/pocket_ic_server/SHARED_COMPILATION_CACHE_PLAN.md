# PocketIC server-lifetime compilation cache

## Goal

Warm a fixture once, then reuse its compiled Wasm across the private instances
created by tests on the same PocketIC server. Reuse should be automatic for both
fresh instances and restored state, including after the original instance has
been deleted.

The server-owned pool and runtime plumbing described below are implemented.
Verification results are recorded at the end; suite-level performance still needs
to be measured with the Liquidium fixture.

## Design

```text
ApiState: server-owned compilation-cache pool
  ├── Effective embedder configuration A → Arc<CompilationCache>
  │     ├── Instance 1 / subnet runtime
  │     └── Instance 2 / subnet runtime
  └── Effective embedder configuration B → Arc<CompilationCache>
        └── Instance 3 / subnet runtime
```

The pool owns a strong reference to each cache for the server's lifetime. Each
Hypervisor owns another reference to its selected cache. Canister state, registry
providers, clocks, execution controllers and sandbox processes belong to their
individual instances.

Share `CompilationCache`, not a canister's `WasmBinary::embedder_cache`: the latter
contains handles associated with a particular sandbox process. The existing
`open_wasm()` cache-hit path already loads shared serialised code into a fresh
sandbox.

### Configuration matching

Select the cache in `Hypervisor::new()`, after applying the scheduler's
`page_overhead` to the embedder configuration. Use exact equality of the complete
effective `ic_config::embedders::Config`. This includes validation limits,
instrumentation settings and compilation-cost parameters; using only the public
PocketIC configuration would miss derived values.

The config already implements `Eq`. Start with a mutex-protected vector of
configuration/cache pairs: the expected number of configurations per CI server is
small, and lookup occurs only during runtime construction. Compilation and cache
reads must happen after releasing the pool mutex.

Within each selected cache, retain the existing Wasm-hash key. Compiler and host
compatibility are implicit in this process-local pool. Matching configurations
can share across subnets as well as instances.

### Storage and ownership

Each shared cache owns an independent `TempDir`, outside all instance state
directories. The existing cache uses temporary files and open file descriptors;
retaining the existing live cache requires no persistent artefact format.

Preserve the existing memory, disk and entry-count limits on each cache. Total
retained capacity scales with the number of distinct configurations, rather than
the number of test instances. Cache-size metrics from different runtimes may
refer to the same cache and must not be summed as independent allocations.

## Implementation steps

1. **Add a concrete cache pool alongside `CompilationCache`.**
   Provide a get-or-create operation keyed by the effective embedder config.
   Pool-created caches use independently owned temporary directories and the
   current Hypervisor cache capacities.

2. **Thread an optional pool through runtime construction.**
   Pass it through `StateMachineBuilder`, `StateMachine::setup_from_dir`,
   `ExecutionServices::setup_execution`, its helper, and `Hypervisor::new`.
   The Hypervisor selects its cache using the final effective config. Callers
   without a pool continue to construct a private cache. Update production,
   replay, determinism and execution-test call sites as required by the internal
   signature changes.

3. **Create one pool in the PocketIC server's `ApiState`.**
   Supply it to instance creation and retain it in `PocketIcSubnets`, so both
   initial and dynamically created subnets use it. Wire restoration through the
   same construction path, before startup rounds can execute restored canisters.
   Ordinary client APIs and existing fixture save/restore calls suffice.

4. **Keep transient failures instance-local.**
   The current cache can retain compilation errors, including compiler RPC
   failures. A server-lifetime cache must not let one failed compiler process
   poison later instances. Permit shared deterministic validation/instrumentation
   failures under the matching configuration, but exclude transient engine/RPC
   failures from shared negative caching.

5. **Add focused tests and benchmark a real fixture.**
   Update the affected Cargo/Bazel dependency declarations where the pool type
   crosses crate boundaries. No Wasmtime dependency feature change is needed.

## Expected patch locations

| Location | Change |
| --- | --- |
| `rs/embedders/src/compilation_cache.rs`, `src/lib.rs` | Pool, ownership, shared-error policy and tests |
| `rs/execution_environment/src/lib.rs`, `src/hypervisor.rs` | Optional pool plumbing and final-config cache selection |
| `rs/state_machine_tests/src/lib.rs` | Builder option and construction plumbing |
| `rs/pocket_ic_server/src/state_api/{state,routes}.rs` | Server ownership and instance construction |
| `rs/pocket_ic_server/src/pocket_ic.rs` | Retain the pool for all subnet creation paths |
| Affected `Cargo.toml` and `BUILD.bazel` files | Direct dependency declarations and test wiring |

## Acceptance tests

1. **Warm reuse:** initialise one fixture, restore several instances on the same
   server and execute every warmed module. Each restored runtime records zero
   additional successful compilations of retained modules and records compiled
   cache hits. Include both queries and updates.
2. **Lifetime:** delete the original instance and its state directory before
   creating another instance. Cached code still executes, and compiling a new
   module succeeds. Deleting one child does not affect another.
3. **Configuration isolation:** warm the same Wasm under relaxed validation,
   then use stricter validation and verify rejection. Exercise a difference in
   scheduler-derived page overhead as well as equality of ordinary configs.
4. **State and accounting isolation:** run identical operations against cold
   and warm instances and compare replies, cycle charges and instruction
   accounting. Mutating one instance's balances, heap/stable memory or clock
   must not affect a sibling. Replicated `expected_compiled_wasms` stays private;
   it must not be replaced by shared physical-cache contents.
5. **Error recovery:** a transient compiler failure is not returned from the
   shared cache to a subsequent healthy instance.
6. **Parallel warm restores:** restore and execute several children concurrently
   after warming the fixture; verify hits and successful independent execution.
7. **Server boundary and eviction:** a new server starts cold. Existing cache
   eviction remains bounded and evicted modules can be compiled again.

Use `hypervisor_wasm_compile_time_seconds` histogram sample counts and
`sandboxed_execution_replica_cache_lookups`, read through each runtime's metrics
registry in server-side tests. Include construction-time compilation in the
counts. For successful warmed modules, the existing full compilation pipeline
should be bypassed, not merely made faster.

## CI integration and performance check

Run one PocketIC server per CI shard and direct all test clients to that server.
Keep it alive for the shard, including gaps between test processes. Initialise
the fixture on that server before starting parallel tests; restored instances
use their usual private state copies.

Compare the current implementation and the shared-cache implementation using the
same fixture, runner and test concurrency. Record initial warm-up, restore plus
first-call time, total suite wall time, compilation counts and peak resources.

The existing cache does not coordinate concurrent cold misses: two runtimes can
compile the same previously unseen module before either inserts it. Warming the
fixture before parallel tests avoids that race for fixture modules. Measure
cold-start contention separately; add per-module in-flight compilation
coordination only if that remains a material source of repeated work.

## Verification

Native ARM64 macOS build of `//rs/pocket_ic_server:pocket-ic-server` passed, using
the existing simulator Bazel cache. The output identifies itself as PocketIC
16.0.0 and is a Mach-O ARM64 executable.

Passed checks:

- All 44 tests in `//rs/embedders:embedders_test`, including pool lifetime,
  configuration partitioning, concurrent insertions and transient-error filtering.
- The state-machine effective-page-overhead test: compilation counts are
  `1, 0, 1, 0` when constructing runtimes with overheads `100, 100, 101, 101`.
- The server API lifecycle test: deleting the last instance retains compiled code
  for the next instance created through `ApiState::add_instance`.
- PocketIC parallel-restoration and validation-isolation tests. Three restored
  instances each record zero new compilations and compiled-cache hits after the
  original instance and directory are deleted. Heap, stable memory, balances and
  clocks remain independent; a fresh warm install has the same cycle balance as
  a cold install. Insertion of new code still succeeds after deletion.
- All 16 existing execution-environment tests matching `compilation`, covering
  private caches, cached errors, metrics and compilation charging.
- Builds of `//rs/replica:replica_lib`, `//rs/replay:replay` and
  `//rs/determinism_test:determinism_test` with the updated construction signature.
- Rust formatting, Buildifier and `git diff --check`.

Repository-wide `bazel build //... --nobuild` is blocked on this host by the
existing `//third_party:libvirt` dependency's missing headers. PocketIC target
analysis and the native build pass.

The full Liquidium fixture timing comparison remains outstanding. These checks
verify avoided compilations rather than claiming a measured CI-suite speedup.
