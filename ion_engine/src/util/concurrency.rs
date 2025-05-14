use std::{
    future::Future,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicI32, AtomicUsize, Ordering},
        mpsc,
    },
    task::{Context, Poll, Wake, Waker},
    time::Duration,
};

use ion_common::wasm_bindgen;
use ion_common::wasm_bindgen::prelude::*;
use ion_common::web_sys::Worker;
use ion_common::web_sys::WorkerOptions;

use ion_common::Instant;

/// Thread counter for thread default naming.
const THREADS_STARTED: AtomicI32 = AtomicI32::new(0);

/// Spawns a new thread of execution that works on both native and WebAssembly platforms.
///
/// This function provides a unified threading abstraction that works across platforms:
/// - On native platforms, it uses standard Rust threads via `std::thread::spawn`
/// - On WebAssembly, it uses Web Workers to achieve similar functionality
///
/// # Platform-specific behavior
///
/// ## Native
/// - Uses standard Rust threads
/// - Thread runs until the closure completes
///
/// ## WebAssembly
/// - Uses Web Workers
/// - Worker is automatically terminated after the closure completes
/// - Shares memory with the main thread for efficient communication
///
/// # Communication
///
/// The function does not return a handle to the spawned thread. Instead, communication
/// should be done using Rust's standard multithreading tools:
/// - `mpsc` channels for message passing
/// - `Arc<Mutex<T>>` for shared state
/// - Other Rust concurrency primitives
pub fn spawn_thread(name: Option<&str>, f: impl FnOnce() + Send + 'static) {
    let thread_id = THREADS_STARTED.fetch_add(1, Ordering::Relaxed);
    let default_name = format!("worker_thread_{thread_id}");
    let thread_name = name.unwrap_or(&default_name);

    if cfg!(not(target_arch = "wasm32")) {
        std::thread::Builder::new()
            .name(thread_name.to_string())
            .spawn(f)
            .unwrap();
    } else {
        #[wasm_bindgen]
        // This function is here for `worker.js` to call.
        pub fn worker_entry_point(ptr: u32) {
            // SAFETY: Pointer is previosly leaked by `Box::into_raw` so it stays valid until reclaimed here.
            let closure = unsafe { Box::from_raw(ptr as *mut Box<dyn FnOnce()>) };
            (*closure)()
        }

        let worker_options = WorkerOptions::new();
        worker_options.set_name(thread_name);

        let worker = Worker::new_with_options("worker.js", &worker_options).expect("Worker creation must succeed");

        // Double-boxing because `dyn FnOnce` is unsized and so `Box<dyn FnOnce()>` is a fat pointer.
        // But `Box<Box<dyn FnOnce()>>` is just a plain pointer, and since wasm has 32-bit pointers,
        // we can cast it to a `u32` and back.
        let ptr = Box::into_raw(Box::new(Box::new(f) as Box<dyn FnOnce()>));
        let msg = ion_common::js_sys::Array::new();

        // Send the worker a reference to our memory chunk, so it can initialize a wasm module
        // using the same memory.
        msg.push(&wasm_bindgen::memory());
        // Also send the worker the pointer to the closure we want to execute.
        msg.push(&JsValue::from(ptr as u32));

        worker.post_message(&msg).unwrap();
    }
}

/// Spawns a new thread of execution that works on both native and WebAssembly platforms.
/// Returns a handle to the thread that can be used to join the thread.
///
/// This is a version of `spawn_thread` that can return a value from the thread.
/// For more details on the behaviour of this function, see `spawn_thread`.
pub fn spawn_thread_with_handle<T: Send + 'static>(
    name: Option<&str>,
    f: impl FnOnce() -> T + Send + 'static,
) -> JoinHandle<T> {
    let thread_id = THREADS_STARTED.fetch_add(1, Ordering::Relaxed);
    let default_name = format!("worker_thread_{thread_id}");
    let thread_name = name.unwrap_or(&default_name).to_string();

    let (sender, receiver) = mpsc::channel();
    spawn_thread(Some(&thread_name), {
        let thread_name = thread_name.clone();
        move || {
            let result = f();
            sender.send(result).expect(
                format!("Failed to return result from thread {thread_name}. Did the original spawning thread die?")
                    .as_str(),
            );
        }
    });

    JoinHandle { receiver }
}

/// An instant that can be used to store time in a thread-safe manner.
/// Works the same way as other atomic types, but contains an [`Instant`] instead of a primitive.
pub struct AtomicInstant {
    base: Instant,
    offset: AtomicUsize,
}

impl AtomicInstant {
    pub fn new(base: Instant) -> AtomicInstant {
        AtomicInstant {
            base,
            offset: AtomicUsize::new(0),
        }
    }

    pub fn load(&self, ordering: Ordering) -> Instant {
        let offset_nanos = self.offset.load(ordering) as u64;
        let secs = offset_nanos / 1_000_000_000;
        let subsec_nanos = (offset_nanos % 1_000_000_000) as u32;
        let offset = Duration::new(secs, subsec_nanos);
        self.base + offset
    }

    pub fn store(&self, val: Instant, ordering: Ordering) {
        let offset = val - self.base;
        let offset_nanos = offset.as_secs() * 1_000_000_000 + offset.subsec_nanos() as u64;
        self.offset.store(offset_nanos as usize, ordering);
    }
}

/// A handle to a thread that can be joined.
/// Supports both native threads and WebAssembly workers.
pub struct JoinHandle<T> {
    receiver: mpsc::Receiver<T>,
}

impl<T> JoinHandle<T> {
    /// Blocks until the thread has finished and returns the result.
    pub fn join(self) -> T {
        self.receiver.recv().unwrap()
    }

    /// Returns `Some(result)` if the thread has finished, `None` if it is still running.
    /// If the thread panicked, this will panic too.
    pub fn try_join(&self) -> Option<T> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => panic!("Thread panicked"),
        }
    }
}

/// Polls a future to completion on current thread.
/// Simply blocks until future is completed.
pub fn block_on<F: Future>(mut future: F) -> F::Output {
    let mut future = unsafe { std::pin::Pin::new_unchecked(&mut future) };
    let future_state = Arc::new(FutureState::new());

    let waker = Waker::from(Arc::clone(&future_state));
    let mut context = Context::from_waker(&waker);

    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Pending => future_state.wait(),
            Poll::Ready(item) => break item,
        }
    }
}

struct FutureState {
    waiting: Mutex<bool>,
    cond_var: Condvar,
}

impl FutureState {
    fn new() -> Self {
        Self {
            waiting: Mutex::new(false),
            cond_var: Condvar::new(),
        }
    }

    fn wait(&self) {
        let mut is_waiting = self.waiting.lock().unwrap();
        if *is_waiting {
            *is_waiting = false
        } else {
            while *is_waiting {
                is_waiting = self.cond_var.wait(is_waiting).unwrap();
            }
        }
    }

    fn notify(&self) {
        let mut is_waiting = self.waiting.lock().unwrap();
        if *is_waiting {
            *is_waiting = false;
            self.cond_var.notify_one();
        }
    }
}

impl Wake for FutureState {
    fn wake(self: Arc<Self>) {
        self.notify();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_on_polls_futures_to_completion() {
        // Works on immediately ready future
        assert_eq!(block_on(std::future::ready(42)), 42);

        use std::future::Future;
        use std::pin::Pin;
        use std::task::{Context, Poll};
        use std::time::{Duration, Instant};

        struct Delay {
            start: Instant,
            duration: Duration,
        }

        impl Future for Delay {
            type Output = ();

            fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
                if Instant::now() - self.start >= self.duration {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }
        }

        // Ready after a timeout
        let then = Instant::now();
        let delaying_future = Delay {
            start: then,
            duration: Duration::from_millis(100),
        };
        block_on(delaying_future);
        assert!(Instant::now().duration_since(then) > Duration::from_millis(100));
    }
}
