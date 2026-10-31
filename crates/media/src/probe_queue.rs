//! Bounded background probe requests used for import, relink and library
//! validation.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

use crate::error::MediaError;
use crate::probe::{probe_with_tools, MediaProbe};
use crate::process::MediaTools;

const MAX_PROBE_QUEUE: usize = 64;
const MAX_PROBE_RESULTS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeRequest {
    pub request_id: u64,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct ProbeEvent {
    pub request_id: u64,
    pub path: PathBuf,
    pub elapsed_ms: u64,
    pub result: Result<MediaProbe, MediaError>,
}

pub trait ProbeExecutor: Send + 'static {
    fn probe(&self, path: &std::path::Path) -> Result<MediaProbe, MediaError>;
}

struct FilesystemProbeExecutor {
    tools: MediaTools,
}

impl ProbeExecutor for FilesystemProbeExecutor {
    fn probe(&self, path: &std::path::Path) -> Result<MediaProbe, MediaError> {
        probe_with_tools(path, &self.tools)
    }
}

struct Shared {
    requests: Mutex<VecDeque<ProbeRequest>>,
    results: Mutex<VecDeque<ProbeEvent>>,
    wake: Condvar,
    stop: AtomicBool,
    in_flight: AtomicBool,
    submitted: AtomicU64,
    published: AtomicU64,
}

pub struct ProbeQueue {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}

impl ProbeQueue {
    pub fn spawn() -> Self {
        Self::spawn_with_executor(Box::new(FilesystemProbeExecutor {
            tools: MediaTools::discover(),
        }))
    }

    pub fn spawn_with_executor(executor: Box<dyn ProbeExecutor>) -> Self {
        let shared = Arc::new(Shared {
            requests: Mutex::new(VecDeque::new()),
            results: Mutex::new(VecDeque::new()),
            wake: Condvar::new(),
            stop: AtomicBool::new(false),
            in_flight: AtomicBool::new(false),
            submitted: AtomicU64::new(0),
            published: AtomicU64::new(0),
        });
        let worker_shared = Arc::clone(&shared);
        let worker = thread::Builder::new()
            .name("cutcraft-probe".to_owned())
            .spawn(move || worker_loop(worker_shared, executor))
            .ok();
        if worker.is_none() {
            shared.stop.store(true, Ordering::SeqCst);
        }
        Self { shared, worker }
    }

    pub fn submit(&self, request: ProbeRequest) -> Result<(), MediaError> {
        if self.shared.stop.load(Ordering::SeqCst) {
            return Err(MediaError::EngineClosed);
        }
        let mut requests = self
            .shared
            .requests
            .lock()
            .map_err(|_| MediaError::EngineClosed)?;
        if requests.len() >= MAX_PROBE_QUEUE {
            return Err(MediaError::InvalidInput(
                "media probe queue is full".to_owned(),
            ));
        }
        requests.push_back(request);
        self.shared.submitted.fetch_add(1, Ordering::Relaxed);
        self.shared.wake.notify_one();
        Ok(())
    }

    pub fn poll(&self) -> Option<ProbeEvent> {
        self.shared
            .results
            .lock()
            .ok()
            .and_then(|mut results| results.pop_front())
    }

    pub fn is_pending(&self) -> bool {
        self.shared.in_flight.load(Ordering::SeqCst)
            || self
                .shared
                .requests
                .lock()
                .map(|requests| !requests.is_empty())
                .unwrap_or(false)
            || self
                .shared
                .results
                .lock()
                .map(|results| !results.is_empty())
                .unwrap_or(false)
    }

    pub fn metrics(&self) -> ProbeQueueMetrics {
        ProbeQueueMetrics {
            worker_count: usize::from(
                self.worker.is_some() && !self.shared.stop.load(Ordering::SeqCst),
            ),
            in_flight: self.shared.in_flight.load(Ordering::SeqCst),
            queued: self
                .shared
                .requests
                .lock()
                .map(|requests| requests.len())
                .unwrap_or(0),
            results: self
                .shared
                .results
                .lock()
                .map(|results| results.len())
                .unwrap_or(0),
            submitted: self.shared.submitted.load(Ordering::Relaxed),
            published: self.shared.published.load(Ordering::Relaxed),
        }
    }
}

impl Drop for ProbeQueue {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        if let Ok(mut requests) = self.shared.requests.lock() {
            requests.clear();
        }
        self.shared.wake.notify_all();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeQueueMetrics {
    pub worker_count: usize,
    pub in_flight: bool,
    pub queued: usize,
    pub results: usize,
    pub submitted: u64,
    pub published: u64,
}

fn worker_loop(shared: Arc<Shared>, executor: Box<dyn ProbeExecutor>) {
    loop {
        let request = {
            let mut requests = match shared.requests.lock() {
                Ok(requests) => requests,
                Err(_) => return,
            };
            while requests.is_empty() && !shared.stop.load(Ordering::SeqCst) {
                requests = match shared.wake.wait(requests) {
                    Ok(requests) => requests,
                    Err(_) => return,
                };
            }
            if shared.stop.load(Ordering::SeqCst) {
                return;
            }
            requests.pop_front()
        };
        let Some(request) = request else {
            return;
        };
        shared.in_flight.store(true, Ordering::SeqCst);
        let started = std::time::Instant::now();
        let result = executor.probe(&request.path);
        let elapsed_ms = started.elapsed().as_millis() as u64;
        shared.in_flight.store(false, Ordering::SeqCst);
        if let Ok(mut results) = shared.results.lock() {
            if results.len() >= MAX_PROBE_RESULTS {
                results.pop_front();
            }
            results.push_back(ProbeEvent {
                request_id: request.request_id,
                path: request.path,
                elapsed_ms,
                result,
            });
        }
        shared.published.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{self, Sender};
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::{Duration, Instant};

    use crate::probe::{MediaRational, MediaTimebase, MediaTimestamp};

    use super::*;

    struct FakeExecutor {
        started: Sender<u64>,
    }

    impl ProbeExecutor for FakeExecutor {
        fn probe(&self, path: &std::path::Path) -> Result<MediaProbe, MediaError> {
            let request_id = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(|stem| stem.parse::<u64>().ok())
                .unwrap_or(0);
            let _ = self.started.send(request_id);
            let timebase = MediaTimebase::new(1, 1_000).expect("timebase");
            Ok(MediaProbe {
                path: path.to_path_buf(),
                container: "mp4".to_owned(),
                video_stream_index: 0,
                codec: "h264".to_owned(),
                width: 320,
                height: 180,
                pixel_format: Some("yuv420p".to_owned()),
                timebase,
                nominal_frame_rate: MediaRational::new(30, 1),
                average_frame_rate: MediaRational::new(30, 1),
                duration: Some(MediaTimestamp::new(1_000, timebase)),
                stream_start: MediaTimestamp::new(0, timebase),
                rotation_degrees: 0,
                has_audio: false,
                frame_count_hint: Some(30),
                probe_elapsed_ms: 1,
            })
        }
    }

    #[test]
    fn probe_queue_returns_request_ids_and_has_one_worker() {
        let (tx, rx) = mpsc::channel();
        let queue = ProbeQueue::spawn_with_executor(Box::new(FakeExecutor { started: tx }));
        queue
            .submit(ProbeRequest {
                request_id: 7,
                path: PathBuf::from("7.mp4"),
            })
            .expect("submit");
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), 7);
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(event) = queue.poll() {
                assert_eq!(event.request_id, 7);
                assert!(event.result.is_ok());
                break;
            }
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        let metrics = queue.metrics();
        assert_eq!(metrics.worker_count, 1);
        assert!(metrics.queued <= MAX_PROBE_QUEUE);
        assert!(metrics.results <= MAX_PROBE_RESULTS);
    }

    struct BlockingExecutor {
        started: Sender<()>,
        gate: Arc<(Mutex<bool>, Condvar)>,
        probe: FakeExecutor,
    }

    impl ProbeExecutor for BlockingExecutor {
        fn probe(&self, path: &std::path::Path) -> Result<MediaProbe, MediaError> {
            let _ = self.started.send(());
            let (lock, wake) = &*self.gate;
            let mut released = lock.lock().expect("gate");
            while !*released {
                released = wake.wait(released).expect("gate wait");
            }
            self.probe.probe(path)
        }
    }

    #[test]
    fn full_probe_queue_rejects_without_unbounded_growth() {
        let (started_tx, started_rx) = mpsc::channel();
        let (fake_tx, _fake_rx) = mpsc::channel();
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let queue = ProbeQueue::spawn_with_executor(Box::new(BlockingExecutor {
            started: started_tx,
            gate: Arc::clone(&gate),
            probe: FakeExecutor { started: fake_tx },
        }));
        queue
            .submit(ProbeRequest {
                request_id: 0,
                path: PathBuf::from("0.mp4"),
            })
            .expect("first request");
        started_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("executor started");
        for id in 1..=MAX_PROBE_QUEUE {
            queue
                .submit(ProbeRequest {
                    request_id: id as u64,
                    path: PathBuf::from(format!("{id}.mp4")),
                })
                .expect("queue capacity");
        }
        assert!(queue
            .submit(ProbeRequest {
                request_id: 999,
                path: PathBuf::from("overflow.mp4"),
            })
            .is_err());
        assert_eq!(queue.metrics().queued, MAX_PROBE_QUEUE);
        let (lock, wake) = &*gate;
        *lock.lock().expect("gate") = true;
        wake.notify_all();
    }
}
