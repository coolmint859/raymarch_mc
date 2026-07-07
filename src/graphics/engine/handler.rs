#![allow(dead_code)]
use std::{
    cell::Cell, collections::HashMap, fmt::Debug, hash::Hash, marker::PhantomData, sync::mpsc, time::{Duration, Instant}
};

use tokio::{runtime::Handle, task::JoinHandle};

/// Specfifies whether a task is io-bound (Non-Blocking) or cpu-bound (Blocking)
#[derive(Debug)]
pub enum TaskType {
    /// task is cpu-bound
    Blocking,
    /// task is io-bound
    NonBlocking
}

/// An operation performed by a Future F, where the result R is stored in a
/// ResourceHandler of the matching type.
#[derive(Debug)]
pub struct Task<F, R> {
    pub fut: F,
    pub ty: TaskType,
    pub hold_time: Option<u64>,
    _rsc: PhantomData<R>
}

impl<F, R> Task<F, R>
where
    F: Future<Output = Result<R, String>> + Send + 'static,
    R: Send + 'static
{
    /// Create a non-blocking task (useful for io-bound operations)
    pub fn non_blocking(fut: F) -> Self {
        Self {
            fut,
            ty: TaskType::NonBlocking,
            hold_time: None,
            _rsc: PhantomData
        }
    }

    /// Create a blocking task (useful for cpu-bound operations)
    pub fn blocking(fut: F) -> Self {
        Self {
            fut,
            ty: TaskType::Blocking,
            hold_time: None,
            _rsc: PhantomData
        }
    }

    /// Set the hold time for the resource created from the task after not being accessed
    pub fn with_hold_time(mut self, secs: u64) -> Self {
        self.hold_time = Some(secs);
        self
    }
}

/// Stores metadata about resources that finished completion
pub struct Ready<R> {
    /// The stored resource
    pub rsc: R,
    /// The time in seconds before this resource should be deallocated.
    pub hold_time: Option<u64>,
    /// The time stamp for when this resource was last accessed.
    pub accessed: Cell<Instant>,
}

/// Stores metadata about resources that failed completion
pub struct Failed {
    /// An error message indicating why the Resource failed
    pub err_msg: String,
    /// The time stamp for when the resource failed.
    pub failed_at: Instant
}

/// Stores metadata about resources that are pending completion
pub struct Pending {
    /// A handle to the tokio thread responsible for creating the resource
    pub thread_handle: JoinHandle<()>,
    /// The time stamp for when the resource was requested.
    pub requested_at: Instant,
}

/// Represents the state of a resource requested by the user of a handler instance.
pub enum ResourceStatus<R> {
    /// Resource has been requested but is not yet ready
    Pending(Pending),
    /// Resource is ready for retrieval
    Ready(Ready<R>),
    /// Resource failed to complete
    Failed(Failed),
}

impl<R> ResourceStatus<R> {
    /// Retrieve the time the resource was requested if available
    pub fn requested_at(&self) -> Option<&Instant>{
        if let ResourceStatus::Pending(pending) = self { Some(&pending.requested_at) } else { None }
    }

    /// Retreive a reference to the stored resource if available.
    pub fn value(&self) -> Option<&Ready<R>> {
        if let ResourceStatus::Ready(ready) = self { Some(ready) } else { None }
    }

    /// Retreive a mutable reference to the stored resource if available.
    pub fn value_mut(&mut self) -> Option<&mut Ready<R>> {
        if let ResourceStatus::Ready(ready) = self { Some(ready) } else { None }
    }

    /// If the status is Ready, this takes ownership of the raw resource and passes it to the caller.
    pub fn take(self) -> Option<R> {
        if let ResourceStatus::Ready(ready) = self { Some(ready.rsc) } else { None }
    }

    /// Retreive the error message if the stored resource failed to complete.
    pub fn error_msg(&self) -> Option<&str> {
        if let ResourceStatus::Failed(failed) = self { Some(&failed.err_msg) } else { None }
    }

    /// Check if this resource is ready (has completed loading/creation)
    pub fn is_ready(&self) -> bool { self.value().is_some() }

    /// Check if this resource is pending (not yet loaded/created)
    pub fn is_pending(&self) -> bool { self.requested_at().is_some() }

    /// Check if this resource is failed (didn't load/wasn't created)
    pub fn is_failed(&self) -> bool { self.error_msg().is_some() }
}

/// Manages and stores any memory resources with concurrent creation through futures.
/// Allows any future as long as the output type is a Result that wraps the resource type.
/// 
/// K: The key type to store resouces with
/// 
/// R: the resource type that will be stored
pub struct ResourceHandler<K, R> {
    resource_map: HashMap<K, ResourceStatus<R>>,

    tx: mpsc::Sender<(K, Result<Ready<R>, String>)>,
    rx: mpsc::Receiver<(K, Result<Ready<R>, String>)>,

    thread_timeout: Duration, // time before a worker thread is considered 'dead' by the main thread
    failed_timeout: Duration, // time before a failed resource is removed from the map
}

impl<K: Debug, R> ResourceHandler<K, R> 
where
    K: Hash + Eq + PartialEq + Clone + Send + 'static,
    R: Send + Debug + 'static,
{
    /// Create a new resource handler.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            resource_map: HashMap::new(),
            tx, rx,
            thread_timeout: Duration::from_secs(5),
            failed_timeout: Duration::from_secs(3)
        }
    }

    /// Set the resource timeout for worker threads, in seconds. The default is 5 seconds.
    /// 
    /// This is the amount of time before a thread is considered 'dead' and is told to stop executing.
    pub fn set_thread_tmt(&mut self,  timeout: u64) {
        self.thread_timeout = Duration::from_secs(timeout);
    }

    /// Set the timeout for failed resources, in seconds. The default is 3 seconds.
    /// 
    /// This is the amount of time before a failed resource is removed from the internal map. 
    pub fn set_failed_tmt(&mut self, timeout: u64) {
        self.failed_timeout = Duration::from_secs(timeout);
    }

    /// Retrieve a resource if is is ready. If the resource has not yet been requested, 
    /// a worker thread tracks its creation via a future, and None is returned.
    /// 
    /// * 'key' - A handle K to query the handler for the resource
    /// * 'task' - A Task instance whose Future resolves with the resource
    pub fn get_or_request<F>(&mut self, key: &K, task: Task<F, R>) -> Option<&R> 
    where 
        F: Future<Output = Result<R, String>> + Send + 'static
    {
        let needs_request = match self.resource_map.get(key) {
            None => true,                               // resource doesn't exist in map
            Some(ResourceStatus::Failed(_)) => true,    // resource exists but previously failed
            Some(_) => false                            // resource exists but is either pending or ready
        };

        if needs_request {
            self.remove(key);
            self.request_new(key, task);
            return None;
        }

        self.get(key)
    }

    /// Request a worker thread to create a resource via a Future if previously failed.
    /// 
    /// If the resource does not exist, this method still spawns a thread.
    /// If the resource exists and is pending or ready, no thread is spawned.
    /// 
    /// * 'key' - A handle K to query the handler for the resource
    /// * 'task' - A Task instance whose Future resolves with the resource
    pub fn request_retry<F>(&mut self, key: &K, task: Task<F, R>)
    where 
        F: Future<Output = Result<R, String>> + Send + 'static
    {
        let needs_retry = match self.resource_map.get(key) {
            None => true,
            Some(ResourceStatus::Failed(_)) => true,
            Some(_) => false
        };

        if needs_retry {
            self.remove(key);
            self.request_new(key, task);
        }
    }

    /// Request a new worker thread to create a resource via a Future.
    /// Does nothing if a resource with the matching key was already requested.
    /// 
    /// * 'key' - A handle K to query the handler for the resource
    /// * 'task' - A Task instance whose Future resolves with the resource
    pub fn request_new<F>(&mut self, key: &K, task: Task<F, R>) 
    where 
        F: Future<Output = Result<R, String>> + Send + 'static
    {
        let key_cpy = key.clone();
        if self.resource_map.contains_key(&key_cpy) {
            return;
        }

        let tx = self.tx.clone();

        let tokio_handle = match task.ty {
            TaskType::NonBlocking => {
                tokio::task::spawn( async move {
                    let result = task.fut.await;

                    let ready_result = result
                        .map(|rsc| { Ready { 
                            rsc, 
                            hold_time: task.hold_time, 
                            accessed: Cell::new(Instant::now()) 
                        }});

                    let _ = tx.send((key_cpy, ready_result));
                })
            },
            TaskType::Blocking => {
                let handle = Handle::current();
                tokio::task::spawn_blocking(move || {
                    let result = handle.block_on(task.fut);

                    let ready_result = result
                        .map(|rsc| { Ready { 
                            rsc, 
                            hold_time: task.hold_time, 
                            accessed: Cell::new(Instant::now()) 
                        }});

                    let _ = tx.send((key_cpy, ready_result));
                })
            }
        };

        let status = ResourceStatus::Pending(Pending {
            thread_handle: tokio_handle,
            requested_at: Instant::now(),
        });
        self.resource_map.insert(key.clone(), status);
    }

    /// Request a new resource and wait for it's completion.
    /// 
    /// Returns a result object containing the completed resource, or an error message if failed.
    /// 
    /// * 'key' - A handle K to query the handler for the resource
    /// * 'task' - A Task instance whose Future resolves with the resource
    pub fn request_wait<F>(&mut self, key: &K, task: Task<F, R>) -> Result<Option<&R>, String>
    where 
        F: Future<Output = Result<R, String>> + Send + 'static
    {
        if self.resource_map.contains_key(key) {
            return Ok(self.get(key));
        }
        
        let result = pollster::block_on(task.fut);
            
        result.map(|rsc| {
            self.store(key, task.hold_time, rsc);
            self.get(key)
        })
    }

    /// Store a preloaded resource into the internal map
    /// 
    /// * 'key' - A handle K to query the handler for the resource
    /// * 'hold_time' - The time in seconds before a resource is considered 'dead' and is removed from the handler
    /// * 'resource' - An instance of the expected Resource this handler stores
    pub fn store(&mut self, key: &K, hold_time: Option<u64>, resource: R) {
        let status = ResourceStatus::Ready(Ready { 
            rsc: resource, 
            hold_time, 
            accessed: Cell::new(Instant::now())
        });

        self.resource_map.insert(key.clone(), status);
    }

    /// Remove a resource from the internal map. Returns the resource if found.
    pub fn remove(&mut self, key: &K) -> Option<R> {
        self.resource_map
            .remove(key)
            .and_then(|status| status.take())
    }

    /// Syncronize the resource threads with the main thread, making available any completed resources. 
    ///
    /// Should be called regularly (i.e. every frame)
    pub fn sync(&mut self) {
        while let Ok((key, result)) = self.rx.try_recv() {
            let status = match result {
                Ok(rsc) => ResourceStatus::Ready(rsc),
                Err(e) => ResourceStatus::Failed(Failed { 
                    err_msg: e, failed_at: Instant::now() 
                }),
            };
            self.resource_map.insert(key, status);
        }

        self.evaluate_rsc_statuses();
    }

    /// Evaluate the statuses of known resources, determining whether to mark as failed or remove from the map
    fn evaluate_rsc_statuses(&mut self) {
        let now = Instant::now();
        self.resource_map.retain(|key, status| {
            match status {
                ResourceStatus::Ready(ready_rsc) => {
                    if let Some(hold_time) = ready_rsc.hold_time {
                        let hold_duration = Duration::from_secs(hold_time);
                        let accessed = ready_rsc.accessed.get();

                        if now.saturating_duration_since(accessed) > hold_duration {
                            println!("[ResourceHandler] Removed resource with key {:?} from handler due to hold timeout.", key);
                            
                            return false; // resource is considered 'dead', remove it from the handler
                        }
                    }
                },
                ResourceStatus::Pending(pending) => {
                    if now.saturating_duration_since(pending.requested_at) > self.thread_timeout {
                        println!("[ResourceHandler] Aborted builder thread for resource with key {:?} due to thread timeout.", key);
                        
                        pending.thread_handle.abort();

                        *status = ResourceStatus::Failed(Failed { 
                            err_msg: "Worker thread lost or stalled execution.".to_string(), 
                            failed_at: now 
                        });
                    }
                },
                ResourceStatus::Failed(failed_state) => {
                    if now.saturating_duration_since(failed_state.failed_at) > self.failed_timeout {
                        println!("[ResourceHandler] Removed resource with key {:?} from handler due to fail status timeout.", key);
                        
                        return false;
                    }
                }
            }
            return true; // resource is still active
        });
    }

    /// Check if a requested resource has finished completion and is stored in the map.
    pub fn is_ready(&self, key: &K) -> bool {
        self.resource_map.get(key).is_some_and(|rsc| rsc.is_ready())
    }

    /// Check if requested resource is still pending completion
    pub fn is_pending(&self, key: &K) -> bool {
        self.resource_map.get(key).is_some_and(|rsc| rsc.is_pending())
    }

    /// Check if a requested resource failed completion.
    pub fn is_failed(&self, key: &K) -> bool {
        self.resource_map.get(key).is_some_and(|rsc| rsc.is_failed())
    }

    /// Get the error message of a failed resource, if applicable.
    pub fn get_err(&self, key: &K) -> Option<&str> {
        return self.resource_map.get(key)?.error_msg();
    }

    /// Get the status of a resource. None is returned if the resource does not exist.
    pub fn status_of(&self, key: &K) -> Option<&ResourceStatus<R>> {
        self.resource_map.get(key)
    }

    /// Get a reference to a completed resource. Returns None if the resource does not exist/is unavailable.
    pub fn get(&self, key: &K) -> Option<&R> {
        match self.resource_map.get(key) {
            Some(ResourceStatus::Ready(ready)) => {
                ready.accessed.set(Instant::now());
                Some(&ready.rsc)
            },
            _ => None
        }
    }

    /// Get a mutable reference to a completed resource. Returns None if the resource does not exist/is unavailable.
    /// 
    /// Note: This locks the handler from retreival of other resources.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut R> {
        match self.resource_map.get_mut(key) {
            Some(ResourceStatus::Ready(ready)) => {
                ready.accessed.set(Instant::now());
                Some(&mut ready.rsc)
            },
            _ => None
        }
    }

    /// Mark a resource as accessed. 
    /// 
    /// This is useful in cases where a resource may have dependencies, but you don't need to access the dependencies directly.
    pub fn mark_accessed(&self, key: &K) {
        match self.resource_map.get(key) {
            Some(ResourceStatus::Ready(ready)) => {
                ready.accessed.set(Instant::now());
            },
            _ => {}
        }
    }

    /// Check if the internal map contains a resource with the specified key (in any state)
    pub fn contains(&self, key: &K) -> bool {
        self.resource_map.contains_key(key)
    }

    /// Get a vector of known resource keys mapped to their resource status' in the form of a tuple. Useful for debugging purposes.
    pub fn status_of_all(&self) -> Vec<(&K, String)> {
        self.resource_map.iter().map(|(key, resource)| {
            let status = match resource {
                ResourceStatus::Failed(failed) => format!("[FAILED] Error Message: {}", failed.err_msg),
                ResourceStatus::Pending(pending) => format!("[PENDING] Time since requested: {:?}s.", pending.requested_at.elapsed()),
                ResourceStatus::Ready(ready) => format!("[READY] Time since last accessed: {:?}s", ready.accessed.get().elapsed())
            };

            (key, status)
        }).collect()
    }
}
