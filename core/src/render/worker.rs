// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/render/worker.rs
//
// Single pdfium worker with a priority queue. Pdfium is not thread-safe,
// so every pdfium access (rendering, thumbnails, PDF operations) goes
// through this worker. Visible pages have priority over thumbnails.

use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::pdfium_ops::{Command, CommandResult, PdfOpsManager};
use crate::storage::thumbcache::ThumbSize;

/// Priority of a job. Higher values run first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// Thumbnails and other background work.
    Low = 0,
    /// Editing commands issued by the user.
    Command = 10,
    /// The currently visible page — must never wait long.
    VisiblePage = 20,
}

/// A job for the pdfium worker.
pub enum Job {
    /// Execute a PDF editing command on the shared document.
    Op(Command),
    /// Render a page of an arbitrary PDF file at the given zoom.
    RenderFilePage { path: PathBuf, page: u32, zoom: f32 },
    /// Render the first page of a PDF file as a thumbnail and store it
    /// in the freedesktop thumbnail cache.
    RenderThumb { path: PathBuf, size: ThumbSize },
}

/// Result of a finished job.
#[derive(Debug)]
pub enum JobResult {
    /// Result of a PDF editing command.
    Op(CommandResult),
    /// A rendered page as RGBA pixels.
    Rendered {
        width: u32,
        height: u32,
        rgba_data: Vec<u8>,
    },
    /// A failed job.
    Error(String),
}

struct QueuedJob {
    seq: u64,
    priority: Priority,
    job: Job,
    reply: Sender<JobResult>,
}

impl PartialEq for QueuedJob {
    fn eq(&self, other: &Self) -> bool {
        self.seq == other.seq
    }
}
impl Eq for QueuedJob {}
impl PartialOrd for QueuedJob {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for QueuedJob {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // BinaryHeap pops the greatest element first.
        self.priority
            .cmp(&other.priority)
            .then(other.seq.cmp(&self.seq))
    }
}

/// Handle to the running pdfium worker.
pub struct Worker {
    sender: Option<Sender<QueuedJob>>,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    /// Spawn the worker thread and return a handle.
    pub fn spawn() -> Self {
        let (sender, receiver) = mpsc::channel::<QueuedJob>();
        let handle = thread::Builder::new()
            .name("noctua-pdfium-worker".to_string())
            .spawn(move || run(receiver))
            .expect("failed to spawn pdfium worker");
        Self {
            sender: Some(sender),
            handle: Some(handle),
        }
    }

    /// Submit a job and block until the result arrives.
    pub fn execute(&self, priority: Priority, job: Job) -> JobResult {
        let (reply, result) = mpsc::channel();
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let queued = QueuedJob {
            seq,
            priority,
            job,
            reply,
        };
        match &self.sender {
            Some(sender) => {
                if sender.send(queued).is_err() {
                    return JobResult::Error("pdfium worker is not running".to_string());
                }
            }
            None => {
                return JobResult::Error("pdfium worker is shut down".to_string());
            }
        }
        result
            .recv()
            .unwrap_or_else(|_| JobResult::Error("pdfium worker vanished".to_string()))
    }

    /// Shut down the worker thread.
    pub fn shutdown(mut self) {
        // Dropping the sender signals the worker loop to exit.
        self.sender.take();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Convenience wrapper shared by UIs: a Worker handle plus a manager lock
/// so the current open-document state stays queryable without a pdfium call.
#[derive(Clone)]
pub struct SharedWorker {
    inner: Arc<WorkerShared>,
}

struct WorkerShared {
    worker: Worker,
    dirty: Mutex<bool>,
}

impl SharedWorker {
    /// Spawn the worker thread.
    pub fn spawn() -> Self {
        Self {
            inner: Arc::new(WorkerShared {
                worker: Worker::spawn(),
                dirty: Mutex::new(false),
            }),
        }
    }

    /// Submit a job and block until the result arrives.
    pub fn execute(&self, priority: Priority, job: Job) -> JobResult {
        if matches!(
            job,
            Job::Op(Command::Save) | Job::Op(Command::SaveAs { .. })
        ) && let Ok(mut dirty) = self.inner.dirty.lock()
        {
            *dirty = false;
        }
        self.inner.worker.execute(priority, job)
    }

    /// Whether the open document has unsaved changes.
    pub fn dirty(&self) -> bool {
        self.inner.dirty.lock().map(|d| *d).unwrap_or(false)
    }
}

fn run(receiver: Receiver<QueuedJob>) {
    let mut manager = PdfOpsManager::new();

    loop {
        // Wait for at least one job; then drain all currently queued jobs
        // in priority order before waiting again.
        let first = match receiver.recv() {
            Ok(job) => job,
            Err(_) => return, // channel closed — shut down
        };

        let mut heap: BinaryHeap<QueuedJob> = BinaryHeap::new();
        heap.push(first);
        loop {
            match receiver.try_recv() {
                Ok(job) => heap.push(job),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Finish remaining jobs, then exit.
                    while let Some(job) = heap.pop() {
                        run_job(&mut manager, job);
                    }
                    return;
                }
            }
        }

        while let Some(job) = heap.pop() {
            run_job(&mut manager, job);
        }
    }
}

fn run_job(manager: &mut PdfOpsManager, job: QueuedJob) {
    let result = match job.job {
        Job::Op(command) => JobResult::Op(manager.execute(command)),
        Job::RenderFilePage { path, page, zoom } => render_file_page(&path, page, zoom),
        Job::RenderThumb { path, size } => render_thumb(&path, size),
    };
    let _ = job.reply.send(result);
}

/// Render a page of an arbitrary PDF file. Opens the file, renders,
/// closes it again — the shared open document stays untouched.
fn render_file_page(path: &std::path::Path, page: u32, zoom: f32) -> JobResult {
    // A fresh manager instance keeps the worker's open document untouched.
    let mut scratch = PdfOpsManager::new();
    match scratch.execute(Command::Open {
        path: path.to_path_buf(),
    }) {
        CommandResult::Ok => match scratch.execute(Command::RenderPage { page, zoom }) {
            CommandResult::Rendered {
                width,
                height,
                rgba_data,
            } => JobResult::Rendered {
                width,
                height,
                rgba_data,
            },
            CommandResult::Error(e) => JobResult::Error(format!("{e:?}")),
            _ => JobResult::Error("unexpected render result".to_string()),
        },
        CommandResult::Error(e) => JobResult::Error(format!("{e:?}")),
        _ => JobResult::Error("unexpected open result".to_string()),
    }
}

/// Render the first page of a PDF file as a thumbnail-sized image and
/// store it in the freedesktop thumbnail cache.
fn render_thumb(path: &std::path::Path, size: ThumbSize) -> JobResult {
    // Serve from cache when a valid entry already exists.
    match crate::storage::thumbcache::lookup(path, size) {
        Ok(Some((width, height, rgba_data))) => {
            return JobResult::Rendered {
                width,
                height,
                rgba_data,
            };
        }
        Ok(None) => {}
        Err(e) => return JobResult::Error(format!("{e:?}")),
    }

    let mut scratch = PdfOpsManager::new();
    match scratch.execute(Command::Open {
        path: path.to_path_buf(),
    }) {
        CommandResult::Ok => {
            let result = scratch.execute(Command::RenderPage {
                page: 1,
                zoom: 0.25, // first page at reduced size; the cache entry is the source of truth
            });
            match result {
                CommandResult::Rendered {
                    width,
                    height,
                    rgba_data,
                } => {
                    let thumb = (width, height, rgba_data);
                    if let Err(e) = crate::storage::thumbcache::store(path, size, &thumb) {
                        return JobResult::Error(format!("{e:?}"));
                    }
                    JobResult::Rendered {
                        width: thumb.0,
                        height: thumb.1,
                        rgba_data: thumb.2,
                    }
                }
                CommandResult::Error(e) => JobResult::Error(format!("{e:?}")),
                _ => JobResult::Error("unexpected render result".to_string()),
            }
        }
        CommandResult::Error(e) => JobResult::Error(format!("{e:?}")),
        _ => JobResult::Error("unexpected open result".to_string()),
    }
}

/// Block until the worker has processed all queued jobs. Helper for tests.
pub fn drain(worker: &Worker, timeout: Duration) -> bool {
    let (reply, result) = mpsc::channel();
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1 << 60);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let queued = QueuedJob {
        seq,
        priority: Priority::Low,
        job: Job::Op(Command::PageCount),
        reply,
    };
    match &worker.sender {
        Some(sender) => {
            if sender.send(queued).is_err() {
                return false;
            }
        }
        None => return false,
    }
    match result.recv_timeout(timeout) {
        Ok(_) => true,
        Err(RecvTimeoutError::Timeout) => false,
        Err(RecvTimeoutError::Disconnected) => false,
    }
}
