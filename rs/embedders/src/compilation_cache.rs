use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use tempfile::TempDir;

use crate::{OnDiskSerializedModule, SerializedModule};
use ic_config::{embedders::Config, execution_environment::MAX_COMPILATION_CACHE_SIZE};
use ic_heap_bytes::HeapBytes;
use ic_interfaces::execution_environment::{HypervisorError, HypervisorResult};
use ic_types::{DiskBytes, NumBytes};
use ic_utils_lru_cache::LruCache;
use ic_wasm_types::{CanisterModule, WasmHash};

const GB: u64 = 1024 * 1024 * 1024;

/// Each entry holds two file descriptors, so this will limit us to 1M total
/// descriptors used by the cache.
const DEFAULT_MAX_ENTRIES: usize = 500_000;

/// 100 GiB should be plenty. Otherwise a large portion of subnet memory would
/// be in Wasm code even though the x86 is a bit larger.
const DEFAULT_DISK_CAPACITY: NumBytes = NumBytes::new(100 * GB);

/// 10 GiB is already more than we can support with the entry count limit anyway.
const DEFAULT_MEMORY_CAPACITY: NumBytes = NumBytes::new(10 * GB);

/// Retains compiled Wasm across runtimes with identical effective embedder configs.
/// The owner must outlive individual runtimes to allow reuse after their deletion.
#[derive(Default)]
pub struct CompilationCachePool {
    caches: Mutex<Vec<(Config, Arc<CompilationCache>)>>,
}

impl CompilationCachePool {
    pub fn get_or_insert(&self, config: &Config) -> Arc<CompilationCache> {
        let mut caches = self.caches.lock().unwrap();
        // ponytail: few configs per server; use a map if construction-time lookup becomes costly.
        if let Some((_, cache)) = caches.iter().find(|(key, _)| key == config) {
            return Arc::clone(cache);
        }

        // Do not put this directory under a runtime's state directory: deleting
        // that runtime would prevent subsequent insertions into the shared cache.
        let mut cache = CompilationCacheBuilder::new()
            .with_memory_capacity(MAX_COMPILATION_CACHE_SIZE)
            .build();
        cache.shared = true;
        let cache = Arc::new(cache);
        caches.push((config.clone(), Arc::clone(&cache)));
        cache
    }
}

/// Stores the serialized modules of wasm code that has already been compiled so
/// that it can be used again without recompiling.
#[derive(HeapBytes)]
pub struct CompilationCache {
    /// Directory holding all the temporary files. It will be deleted on
    /// drop.
    dir: TempDir,
    /// Map from wasm hash to an open fd with the serialized Module result.
    cache: Mutex<LruCache<WasmHash, HypervisorResult<Arc<OnDiskSerializedModule>>>>,
    /// Atomic counter to deduplicate files in the case of concurrent compilations of the same module.
    counter: AtomicU64,
    /// Limit on the total number of entries in the cache.
    max_entries: usize,
    shared: bool,
}

impl DiskBytes for CompilationCache {
    fn disk_bytes(&self) -> usize {
        self.cache.lock().unwrap().disk_bytes()
    }
}

pub struct CompilationCacheBuilder {
    memory_capacity: NumBytes,
    disk_capacity: NumBytes,
    dir: Option<TempDir>,
    max_entries: usize,
}

impl Default for CompilationCacheBuilder {
    fn default() -> Self {
        Self::new()
    }
}
impl CompilationCacheBuilder {
    pub fn new() -> Self {
        Self {
            memory_capacity: DEFAULT_MEMORY_CAPACITY,
            disk_capacity: DEFAULT_DISK_CAPACITY,
            dir: None,
            max_entries: DEFAULT_MAX_ENTRIES,
        }
    }

    pub fn with_memory_capacity(mut self, memory_capacity: NumBytes) -> Self {
        self.memory_capacity = memory_capacity;
        self
    }

    pub fn with_disk_capacity(mut self, disk_capacity: NumBytes) -> Self {
        self.disk_capacity = disk_capacity;
        self
    }

    pub fn with_dir(mut self, dir: TempDir) -> Self {
        self.dir = Some(dir);
        self
    }

    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = max_entries;
        self
    }

    pub fn build(self) -> CompilationCache {
        let dir = self.dir.unwrap_or_else(|| tempfile::tempdir().unwrap());
        CompilationCache {
            dir,
            cache: Mutex::new(LruCache::new(self.memory_capacity, self.disk_capacity)),
            counter: AtomicU64::new(0),
            max_entries: self.max_entries,
            shared: false,
        }
    }
}

impl CompilationCache {
    pub fn insert_err(&self, canister_module: &CanisterModule, err: HypervisorError) {
        // Compiler RPC/engine failures can be transient. Sharing those errors
        // would poison later instances even after the failed compiler has exited.
        if self.shared
            && !matches!(
                err,
                HypervisorError::InvalidWasm(_) | HypervisorError::InstrumentationFailed(_)
            )
        {
            return;
        }
        let mut cache = self.cache.lock().unwrap();
        if cache.len() >= self.max_entries {
            let _ = cache.pop_lru();
        }
        let _ = cache.push(WasmHash::from(canister_module), Err(err));
    }

    pub fn insert_ok(
        &self,
        canister_module: &CanisterModule,
        serialized_module: SerializedModule,
    ) -> Arc<OnDiskSerializedModule> {
        // The file paths must not have existing files. To ensure this
        // we add a unique counter - otherwise concurent insertions for
        // the same Wasm would use the same file.
        let hash = WasmHash::from(canister_module);
        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        let mut bytes_path: PathBuf = self.dir.path().into();
        bytes_path.push(format!("{hash}-{id}.module_bytes"));
        let mut initial_state_path: PathBuf = self.dir.path().into();
        initial_state_path.push(format!("{hash}-{id}.initial_data"));

        let on_disk = Arc::new(OnDiskSerializedModule::from_serialized_module(
            serialized_module,
            &bytes_path,
            &initial_state_path,
        ));

        let mut cache = self.cache.lock().unwrap();
        if cache.len() >= self.max_entries {
            let _ = cache.pop_lru();
        }
        let _ = cache.push(hash, Ok(Arc::clone(&on_disk)));
        on_disk
    }

    pub fn get(
        &self,
        canister_module: &CanisterModule,
    ) -> Option<HypervisorResult<Arc<OnDiskSerializedModule>>> {
        self.cache
            .lock()
            .unwrap()
            .get(&WasmHash::from(canister_module))
            .map(|o| match o {
                Ok(m) => Ok(Arc::clone(m)),
                Err(e) => Err(e.clone()),
            })
    }

    #[doc(hidden)]
    pub fn clear_for_testing(&self) {
        self.cache.lock().unwrap().clear()
    }
}

/// Check that multiple threads compiling the same wasm won't interfere with
/// each other if they all try to insert in the cache at the same time.
#[test]
fn concurrent_insertions() {
    let cache = CompilationCacheBuilder::new().build();
    let wasm = wat::parse_str("(module)").unwrap();
    let canister_module = CanisterModule::new(wasm.clone());
    let binary = ic_wasm_types::BinaryEncodedWasm::new(wasm.clone());
    let config = ic_config::embedders::Config::default();
    let embedder = crate::WasmtimeEmbedder::new(config, ic_logger::no_op_logger());
    let (_, result) = crate::wasm_utils::compile(&embedder, &binary);
    let serialized_module = result.unwrap().1;

    std::thread::scope(|s| {
        let mut threads = vec![];
        for _ in 0..100 {
            let serialized_module = serialized_module.clone();
            threads.push(s.spawn(|| {
                let _ = cache.insert_ok(&canister_module, serialized_module);
            }));
        }
        for t in threads {
            t.join().unwrap();
        }
    })
}

#[test]
fn pool_retains_and_partitions_caches() {
    let pool = CompilationCachePool::default();
    let config = Config::default();
    let cache = pool.get_or_insert(&config);
    let weak = Arc::downgrade(&cache);
    let dir = cache.dir.path().to_path_buf();
    drop(cache);

    let cache = weak.upgrade().expect("pool must retain an unused cache");
    assert!(dir.exists());
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| assert!(Arc::ptr_eq(&cache, &pool.get_or_insert(&config))));
        }
    });

    let mut different_cost = config.clone();
    different_cost.page_overhead += ic_types::NumInstructions::new(1);
    assert!(!Arc::ptr_eq(&cache, &pool.get_or_insert(&different_cost)));
    let mut different_validation = config;
    different_validation.max_number_exported_functions += 1;
    assert!(!Arc::ptr_eq(
        &cache,
        &pool.get_or_insert(&different_validation)
    ));

    drop(cache);
    drop(pool);
    assert!(weak.upgrade().is_none());
    assert!(!dir.exists());
}

#[test]
fn shared_cache_does_not_retain_transient_errors() {
    let pool = CompilationCachePool::default();
    let cache = pool.get_or_insert(&Config::default());
    let wasm = CanisterModule::new(wat::parse_str("(module)").unwrap());
    let transient = HypervisorError::WasmEngineError(ic_wasm_types::WasmEngineError::Unexpected(
        "compiler RPC failed".to_string(),
    ));
    cache.insert_err(&wasm, transient.clone());
    assert!(cache.get(&wasm).is_none());

    let invalid =
        HypervisorError::InvalidWasm(ic_wasm_types::WasmValidationError::TooManyExports {
            defined: 2,
            allowed: 1,
        });
    cache.insert_err(&wasm, invalid.clone());
    assert_eq!(cache.get(&wasm).unwrap().err(), Some(invalid));

    // Private caches retain their existing negative-caching behaviour.
    let private = CompilationCacheBuilder::new().build();
    private.insert_err(&wasm, transient.clone());
    assert_eq!(private.get(&wasm).unwrap().err(), Some(transient));
}
