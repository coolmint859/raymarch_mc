#![allow(dead_code)]
use std::{
    cell::Cell, collections::HashMap, fmt::{Debug, Display}, hash::Hash, marker::PhantomData, println, sync::mpsc, time::{Duration, Instant}
};

use tokio::{runtime::Handle, task::JoinHandle};

/// Represents the duration a resource is kept alive in a `ResourceHandler`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HoldPolicy {
    /// Resource persists for the duration of program unless manually removed
    Indefinite,
    /// Resource is kept alive for the provided duration since last accessed.
    WithDuration(Duration),
}

/// Specifies whether a `Task` is io-bound (Non-Blocking) or cpu-bound (Blocking)
#[derive(Debug)]
pub enum TaskType {
    /// task is cpu-bound
    CpuBound,
    /// task is io-bound
    IoBound
}

/// An operation performed by a `Future F`, where the result `R` is stored in a
/// `ResourceHandler` of the matching type.
#[derive(Debug)]
pub struct Task<F, R, E> {
    pub fut: F,
    pub ty: TaskType,
    pub hold_policy: HoldPolicy,
    _rsc: PhantomData<(R, E)>,
}

impl<F, R, E> Task<F, R, E>
where
    F: Future<Output = Result<R, E>> + Send + 'static,
    R: Send + Debug + 'static,
    E: Send + Debug + Display + From<&'static str> + 'static
{
    /// Create a io-bound task (non-blocking)
    pub fn io_bound(fut: F) -> Self {
        Self {
            fut,
            ty: TaskType::IoBound,
            hold_policy: HoldPolicy::Indefinite,
            _rsc: PhantomData,
        }
    }

    /// Create a cpu-bound task (blocking)
    pub fn cpu_bound(fut: F) -> Self {
        Self {
            fut,
            ty: TaskType::CpuBound,
            hold_policy: HoldPolicy::Indefinite,
            _rsc: PhantomData,
        }
    }

    /// Set the hold duration for the resource created from the task. 
    /// 
    /// If the hold timer exceeds this value after not being accessed, the resource is removed.
    pub fn with_hold_policy(mut self, policy: HoldPolicy) -> Self {
        self.hold_policy = policy;
        self
    }
}

/// The ready state of a resource, containing the resource `R` as well as it's hold policy and accessed timestamp.
pub struct Ready<R> {
    /// The stored resource
    pub rsc: R,
    /// The time in seconds before this resource should be deallocated.
    pub hold_policy: HoldPolicy,
    /// The time stamp for when this resource was last accessed.
    pub accessed: Cell<Instant>,
}

/// The failed state of a resource, containing the error `E` and the time the resource failed.
pub struct Failed<E> {
    /// The Error for why the resource failed
    pub err: E,
    /// The time stamp for when the resource failed.
    pub failed_at: Instant
}

/// The pending state of a resource, containing the handle to the worker thread and the time the resource was requested.
pub struct Pending {
    /// A handle to the tokio thread responsible for creating the resource
    pub thread_handle: JoinHandle<()>,
    /// The time stamp for when the resource was requested.
    pub requested_at: Instant,
}

/// Represents the state of a resource requested by the user of a handler instance.
pub enum ResourceState<R, E> {
    /// Resource has been requested but is not yet ready
    Pending(Pending),
    /// Resource is ready for retrieval
    Ready(Ready<R>),
    /// Resource failed to complete
    Failed(Failed<E>),
}

impl<R, E> ResourceState<R, E> {
    /// Retrieve the time the resource was requested if available
    pub fn requested_at(&self) -> Option<&Instant>{
        if let ResourceState::Pending(pending) = self { Some(&pending.requested_at) } else { None }
    }

    /// Retreive a reference to the `Ready` state of a resource, if available.
    pub fn ready_state(&self) -> Option<&Ready<R>> {
        if let ResourceState::Ready(ready) = self { Some(ready) } else { None }
    }

    /// Retreive a mutable reference to the `Ready` state of the resource, if available.
    pub fn ready_state_mut(&mut self) -> Option<&mut Ready<R>> {
        if let ResourceState::Ready(ready) = self { Some(ready) } else { None }
    }

    /// If the state is `Ready`, this takes ownership of the raw resource and passes it to the caller.
    pub fn take(self) -> Option<R> {
        if let ResourceState::Ready(ready) = self { Some(ready.rsc) } else { None }
    }

    /// Retreive the error message if the stored resource failed to complete.
    pub fn error(&self) -> Option<&E> {
        if let ResourceState::Failed(failed) = self { Some(&failed.err) } else { None }
    }

    /// Check if this resource is ready (has completed loading/creation)
    pub fn is_ready(&self) -> bool { self.ready_state().is_some() }

    /// Check if this resource is pending (not yet loaded/created)
    pub fn is_pending(&self) -> bool { self.requested_at().is_some() }

    /// Check if this resource is failed (didn't load/wasn't created)
    pub fn is_failed(&self) -> bool { self.error().is_some() }
}

/// A stored resource
pub struct Resource<R, E> {
    pub state: ResourceState<R, E>,
    pub generation: u64
}

/// The result of the worker thread spawned with a `Task` future
struct TaskResult<K, R, E> {
    key: K,
    result: Result<R, E>,
    policy: HoldPolicy,
    generation: u64,
}

/// Manages and stores memory resources with concurrent creation through futures.
/// Allows any future as long as the output type is a `Result<R, E>`.
/// 
/// * `K`: The key type to store resources with
/// * `R`: the resource type that will be stored
/// * `E`: the error type in case a resource fails completion
pub struct ResourceHandler<K, R, E> {
    resource_map: HashMap<K, Resource<R, E>>,

    tx: mpsc::Sender<TaskResult<K, R, E>>,
    rx: mpsc::Receiver<TaskResult<K, R, E>>,

    thread_timeout: Duration, // time before a worker thread is considered 'dead' by the main thread
    failed_timeout: Duration, // time before a failed resource is removed from the map
}

impl<K, R, E> ResourceHandler<K, R, E> 
where
    K: Send + Debug + Hash + Eq + PartialEq + Clone + 'static,
    R: Send + Debug + 'static,
    E: Send + Debug + Display + From<&'static str> + 'static
{
    /// Create a new `ResourceHandler`.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            resource_map: HashMap::new(),
            thread_timeout: Duration::from_secs(5),
            failed_timeout: Duration::from_secs(3),
            tx, rx,
        }
    }

    /// Set the resource timeout for worker threads, in seconds. The default is 5 seconds.
    /// 
    /// This is the amount of time before a thread is considered 'dead' and is told to stop executing.
    /// 
    /// For blocking tokio threads, generation counters are used to prevent multiple threads under the same key from overwriting eachother.
    pub fn set_thread_tmt(&mut self,  timeout: u64) {
        self.thread_timeout = Duration::from_secs(timeout);
    }

    /// Set the timeout for failed resources, in seconds. The default is 3 seconds.
    /// 
    /// This is the amount of time before a failed resource is removed from the internal map. 
    pub fn set_failed_tmt(&mut self, timeout: u64) {
        self.failed_timeout = Duration::from_secs(timeout);
    }

    /// Returns a resource if it is in the `Ready` state. If the resource has not yet been requested or previously failed, 
    /// a worker thread is spawned to execute the `Future F`, and `None` is returned.
    /// 
    /// * 'key' - A key of type `K` used query the handler for the resource
    /// * 'task' - A `Task` whose `Future` resolves with the resource
    pub fn get_or_request<F>(&mut self, key: K, task: Task<F, R, E>) -> Option<&R> 
    where 
        F: Future<Output = Result<R, E>> + Send + 'static
    {
        if !self.is_requested(&key) {
            self.request(key, task);
            return None;
        }

        self.get(&key)
    }

    /// Request a worker thread to create a resource via a Future.
    /// 
    /// If the resource is brand new or previously requested but failed, this method spawns a background tokio thread.
    /// If the resource exists but is pending or ready, this method does nothing.
    /// 
    /// * 'key' - A key of type `K` used query the handler for the resource
    /// * 'task' - A `Task` whose `Future` resolves with the resource
    pub fn request<F>(&mut self, key: K, task: Task<F, R, E>) 
    where 
        F: Future<Output = Result<R, E>> + Send + 'static
    {
        let key_cpy = key.clone();
        let next_gen = match self.resource_map.get(&key) {
            Some(rsc) => match &rsc.state {
                ResourceState::Failed(_) => rsc.generation + 1,
                _ => {
                    println!("Early return due to duplicate request for key {:?}", key);
                    return
                } // early return if the resource exists and is ready/pending
            },
            None => 1
        };

        let tx = self.tx.clone();

        let tokio_handle = match task.ty {
            TaskType::IoBound => {
                tokio::task::spawn( async move {
                    let task_result = TaskResult {
                        key: key_cpy,
                        result: task.fut.await,
                        policy: task.hold_policy,
                        generation: next_gen
                    };
                    let _ = tx.send(task_result);
                })
            },
            TaskType::CpuBound => {
                let handle = Handle::current();
                tokio::task::spawn_blocking(move || {
                    let task_result = TaskResult {
                        key: key_cpy,
                        result: handle.block_on(task.fut),
                        policy: task.hold_policy,
                        generation: next_gen
                    };
                    let _ = tx.send(task_result);
                })
            }
        };

        let resource = Resource {
            state: ResourceState::Pending(Pending {
                thread_handle: tokio_handle,
                requested_at: Instant::now(),
            }),
            generation: next_gen
        };

        self.resource_map.insert(key.clone(), resource);
    }

    /// Requests a new resource and waits for it's completion, then stores it in the internal map. 
    /// 
    /// Note that because it waits for the resource to finish, this makes it a blocking method; use with caution.
    /// 
    /// Returns a result object containing the completed resource, or an error message if failed.
    /// 
    /// * 'key' - A key of type `K` used query the handler for the resource
    /// * 'task' - A `Task` whose `Future` resolves with the resource
    pub fn request_wait<F>(&mut self, key: &K, task: Task<F, R, E>) -> Result<Option<&R>, E>
    where 
        F: Future<Output = Result<R, E>> + Send + 'static
    {
        if self.resource_map.contains_key(key) {
            return Ok(self.get(key));
        }
        
        let result = pollster::block_on(task.fut);
            
        result.map(|rsc| {
            self.store(key, task.hold_policy, rsc);
            self.get(key)
        })
    }

    /// Store a preloaded resource into the internal map
    /// 
    /// * 'key' - A key of type `K` used query the handler for the resource
    /// * 'hold_time' - The time in seconds before a resource is considered 'dead' and is removed from the handler
    /// * 'resource' - An instance of the expected Resource this handler stores
    pub fn store(&mut self, key: &K, hold_policy: HoldPolicy, resource: R) {
        let resource = Resource {
            state: ResourceState::Ready(Ready { 
                rsc: resource, 
                hold_policy, 
                accessed: Cell::new(Instant::now())
            }),
            generation: 1
        };

        self.resource_map.insert(key.clone(), resource);
    }

    /// Remove a resource from the internal map (in any state). Returns the resource if found.
    pub fn remove(&mut self, key: &K) -> Option<R> {
        self.resource_map
            .remove(key)
            .and_then(|rsc| rsc.state.take())
    }

    /// Removes a resource from internal map if it's in the `Ready` state, and returns it.
    pub fn remove_ready(&mut self, key: &K) -> Option<R> {
        if self.is_ready(key) { self.remove(key) } else { None }
    }

    /// Removes a resource from internal map if it's in the `Failed` state.
    pub fn remove_failed(&mut self, key: &K) {
        if self.is_failed(key) { self.remove(key); }
    }

    /// Removes all resources.
    pub fn clear(&mut self) {
        self.resource_map.clear();
    }

    /// Removes all resources that are in the `Failed` state.
    pub fn clear_failed(&mut self) {
        self.resource_map.retain(|_, rsc| {
            rsc.state.is_pending() || rsc.state.is_ready()
        });
    }

    /// Syncronize the resource threads with the main thread, making available any completed resources. 
    ///
    /// Should be called regularly (i.e. every frame)
    pub fn sync(&mut self) {
        while let Ok(task_result) = self.rx.try_recv() {
            let status = match task_result.result {
                Ok(rsc) => ResourceState::Ready(Ready {
                    rsc,
                    hold_policy: task_result.policy,
                    accessed: Cell::new(Instant::now())
                }),
                Err(e) => ResourceState::Failed(Failed { 
                    err: e, 
                    failed_at: Instant::now() 
                }),
            };

            let matching_generation = match self.resource_map.get(&task_result.key) {
                Some(rsc) => rsc.generation == task_result.generation,
                None => false
            };
            // skip mismatched generations - this prevents stale resources from overwriting more recent ones.
            if !matching_generation {
                println!("Mismatched generations for key {:?}, resource not inserted.", task_result.key);
                continue; 
            }

            self.resource_map.insert(task_result.key, Resource { 
                state: status, 
                generation: task_result.generation
            });
        }

        self.evaluate_rsc_statuses();
    }

    /// Evaluate the statuses of known resources, determining whether to mark as failed or remove from the map
    fn evaluate_rsc_statuses(&mut self) {
        let now = Instant::now();
        self.resource_map.retain(|key, resource| {
            match &mut resource.state {
                ResourceState::Ready(ready_state) => {
                    match ready_state.hold_policy {
                        HoldPolicy::WithDuration(duration) => {
                            let accessed = ready_state.accessed.get();

                            if now.saturating_duration_since(accessed) > duration {
                                println!("[ResourceHandler] Removed resource with key {:?} from handler due to hold timeout.", key);
                                
                                return false; // resource is considered 'dead', remove it from the handler
                            }
                        },
                        HoldPolicy::Indefinite => {}
                    }
                },
                ResourceState::Pending(pending_state) => {
                    if now.saturating_duration_since(pending_state.requested_at) > self.thread_timeout {
                        println!("[ResourceHandler] Aborted builder thread for resource with key {:?} due to thread timeout.", key);
                        
                        pending_state.thread_handle.abort();

                        resource.state = ResourceState::Failed(Failed { 
                            err: E::from("Worker thread lost or stalled execution."), 
                            failed_at: now 
                        });
                    }
                },
                ResourceState::Failed(failed_state) => {
                    if now.saturating_duration_since(failed_state.failed_at) > self.failed_timeout {
                        println!("[ResourceHandler] Removed resource with key {:?} from handler due to fail status timeout.", key);
                        
                        return false;
                    }
                }
            }
            return true; // resource is still active
        });
    }

    /// Returns `true` if a resource with `key` is stored in the map and is in the `Ready` state, `false` otherwise.
    pub fn is_ready(&self, key: &K) -> bool {
        self.resource_map.get(key).is_some_and(|rsc| rsc.state.is_ready())
    }

    /// Returns `true` if a resource with `key` is stored in the map and is in the `Pending` state, `false` otherwise.
    pub fn is_pending(&self, key: &K) -> bool {
        self.resource_map.get(key).is_some_and(|rsc| rsc.state.is_pending())
    }

    /// Returns `true` if a resource with `key` is stored in the map and is in the `Failed` state, `false` otherwise.
    pub fn is_failed(&self, key: &K) -> bool {
        self.resource_map.get(key).is_some_and(|rsc| rsc.state.is_failed())
    }

    /// Returns `true` if a resource is in either the `Pending` or `Ready` states, `false` otherwise.
    pub fn is_requested(&self, key: &K) -> bool {
        self.resource_map.get(key).is_some_and(|rsc| {
            return rsc.state.is_pending() || rsc.state.is_ready()
        })
    }

    /// Get the error `E` of a resource with `key` if in the `Failed` state.
    pub fn get_err_of(&self, key: &K) -> Option<&E> {
        return self.resource_map.get(key)?.state.error();
    }

    /// Get the state of a resource. `None` is returned if the resource does not exist.
    pub fn get_state_of(&self, key: &K) -> Option<&ResourceState<R, E>> {
        self.resource_map.get(key).map(|rsc| &rsc.state)
    }

    /// Returns a reference to a resource if in the `Ready` state, else `None`. Marks the resource as accessed.
    pub fn get(&self, key: &K) -> Option<&R> {
        match self.resource_map.get(key)?.state {
            ResourceState::Ready(ref ready) => {
                ready.accessed.set(Instant::now());
                Some(&ready.rsc)
            },
            _ => None
        }
    }

    /// Returns a mutable reference to a resource if in the `Ready` state, else `None`. Marks the resource as accessed.
    /// 
    /// Note: This locks the handler from retreival of other resources.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut R> {
        match self.resource_map.get_mut(key)?.state {
            ResourceState::Ready(ref mut ready) => {
                ready.accessed.set(Instant::now());
                Some(&mut ready.rsc)
            },
            _ => None
        }
    }

    /// Mark a `Ready` resource as accessed.
    /// 
    /// This is useful in the case the resource is a dependency to some other resource, but you don't need a reference to it.
    pub fn mark_accessed(&self, key: &K) {
        self.resource_map.get(key).map(|rsc| {
            match &rsc.state {
                ResourceState::Ready(ready) => {
                    ready.accessed.set(Instant::now())
                },
                _ => {}
            }
        });
    }

    /// Returns `true` if the internal map contains a resource with the specified `key` (in any state), `false` otherwise.
    /// 
    /// Note: This does not mark the resource as accessed if exists.
    pub fn contains(&self, key: &K) -> bool {
        self.resource_map.contains_key(key)
    }

    /// Returns `true` if the internal map contains a `Ready` resource with the specified `key`, `false` otherwise.
    /// 
    /// Note: This does not mark the resource as accessed if exists.
    pub fn contains_ready(&self, key: &K) -> bool {
        if let Some(rsc) = self.resource_map.get(key) {
            match rsc.state {
                ResourceState::Ready(_) => { return true; },
                _ => { return false; }
            }
        }
        false
    }

    /// Returns `true` if the internal map contains a `Pending` resource with the specified `key`, `false` otherwise.
    /// 
    /// Note: This does not mark the resource as accessed if exists.
    pub fn contains_pending(&self, key: &K) -> bool {
        if let Some(rsc) = self.resource_map.get(key) {
            match rsc.state {
                ResourceState::Pending(_) => { return true; },
                _ => { return false; }
            }
        }
        false
    }

    /// Returns `true` if the internal map contains a `Failed` resource with the specified `key`, `false` otherwise.
    /// 
    /// Note: This does not mark the resource as accessed if exists.
    pub fn contains_failed(&self, key: &K) -> bool {
        if let Some(rsc) = self.resource_map.get(key) {
            match rsc.state {
                ResourceState::Failed(_) => { return true; },
                _ => { return false; }
            }
        }
        false
    }
    
    /// Get an iterator of the keys of all known resources, regardless of status
    pub fn keys(&self) -> impl IntoIterator<Item = &K> {
        self.resource_map.keys()
    }

    /// Get an iterator of the keys of all `Ready` resources.
    pub fn ready_keys(&self) -> impl Iterator<Item = &K> {
        self.resource_map.iter().filter_map(|(key, rsc)| {
            if rsc.state.is_ready() { Some(key) } else { None }
        })
    }

    /// Get an iterator of the keys of all `Pending` resources.
    pub fn pending_keys(&self) -> impl IntoIterator<Item = &K> {
        self.resource_map.iter().filter_map(|(key, rsc)| {
            if rsc.state.is_pending() { Some(key) } else { None }
        })
    }

    /// Get an iterator of the keys of all `Failed` resources.
    pub fn failed_keys(&self) -> impl IntoIterator<Item = &K> {
        self.resource_map.iter().filter_map(|(key, rsc)| {
            if rsc.state.is_failed() { Some(key) } else { None }
        })
    }

    /// Get a vector of known resource keys mapped to strings of their statuses in the form of a tuple. Useful for debugging purposes.
    pub fn status_of_all(&self) -> Vec<(&K, String)> {
        self.resource_map.iter().map(|(key, resource)| {
            let status = match &resource.state {
                ResourceState::Failed(failed) => format!("[FAILED] Error Message: {}", failed.err),
                ResourceState::Pending(pending) => format!("[PENDING] Time since requested: {:?}s.", pending.requested_at.elapsed()),
                ResourceState::Ready(ready) => format!("[READY] Time since last accessed: {:?}s", ready.accessed.get().elapsed())
            };

            (key, status)
        }).collect()
    }
}
