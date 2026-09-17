// SPDX-License-Identifier: GPL-3.0-or-later
// core/src/render/worker.rs
//
// Single pdfium worker with a priority queue. Pdfium is not thread-safe,
// so every pdfium access (rendering, thumbnails, PDF operations) goes
// through this worker. Visible pages have priority over thumbnails.

use std::collections::BinaryHeap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

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
#[derive(Debug)]
pub enum Job {
    /// Execute a PDF editing command on the shared document.
    Op(Command),
    /// Render a page of an arbitrary PDF file at the given zoom.
    RenderFilePage { path: PathBuf, page: u32, zoom: f32 },
    /// Render the given 1-based pages of a PDF file as thumbnails. The
    /// document is opened once and closed again — the shared open
    /// document stays untouched.
    RenderPageThumbs {
        path: PathBuf,
        pages: Vec<u32>,
        zoom: f32,
    },
    /// Count the pages of an arbitrary PDF file.
    FilePageCount { path: PathBuf },
    /// Width and height of every page of an arbitrary PDF file, in points.
    FilePageSizes { path: PathBuf },
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
    /// Rendered thumbnails as (page, width, height, rgba).
    RenderedThumbs(Vec<PageThumb>),
    /// Page count of a PDF file.
    PageCount(u32),
    /// Width and height of every page in points, in page order.
    PageSizes(Vec<(f32, f32)>),
    /// A failed job.
    Error(String),
}

/// A rendered page thumbnail: (page, width, height, rgba).
pub type PageThumb = (u32, u32, u32, Vec<u8>);

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

/// Convenience wrapper shared by UIs: a cloneable handle to the worker.
#[derive(Clone)]
pub struct SharedWorker {
    inner: Arc<WorkerShared>,
}

struct WorkerShared {
    worker: Worker,
}

impl SharedWorker {
    /// Spawn the worker thread.
    pub fn spawn() -> Self {
        Self {
            inner: Arc::new(WorkerShared {
                worker: Worker::spawn(),
            }),
        }
    }

    /// Submit a job and block until the result arrives.
    pub fn execute(&self, priority: Priority, job: Job) -> JobResult {
        self.inner.worker.execute(priority, job)
    }

    /// Thumbnail for a file: freedesktop cache first; on a miss, PDFs
    /// are rendered on the worker thread, raster/SVG in place.
    pub fn thumbnail(&self, path: &Path, size: ThumbSize) -> Option<(u32, u32, Vec<u8>)> {
        if let Ok(Some(thumb)) = crate::storage::thumbcache::lookup(path, size) {
            return Some(thumb);
        }
        if crate::storage::document::is_pdf(path).unwrap_or(false) {
            match self.execute(
                Priority::Low,
                Job::RenderThumb {
                    path: path.to_path_buf(),
                    size,
                },
            ) {
                JobResult::Rendered {
                    width,
                    height,
                    rgba_data,
                } => Some((width, height, rgba_data)),
                _ => None,
            }
        } else {
            crate::storage::thumbcache::get_or_create(path, size).ok()
        }
    }

    /// Render a page of an arbitrary PDF file on the worker thread.
    pub fn render_page(&self, path: &Path, page: u32, zoom: f32) -> Option<(u32, u32, Vec<u8>)> {
        match self.execute(
            Priority::VisiblePage,
            Job::RenderFilePage {
                path: path.to_path_buf(),
                page,
                zoom,
            },
        ) {
            JobResult::Rendered {
                width,
                height,
                rgba_data,
            } => Some((width, height, rgba_data)),
            _ => None,
        }
    }

    /// Render several pages of a PDF file with a single document open.
    /// The document is opened once on the worker; failed pages are
    /// skipped. Returns (page, width, height, rgba) tuples.
    pub fn render_pages(
        &self,
        path: &Path,
        pages: &[u32],
        zoom: f32,
        priority: Priority,
    ) -> Option<Vec<PageThumb>> {
        match self.execute(
            priority,
            Job::RenderPageThumbs {
                path: path.to_path_buf(),
                pages: pages.to_vec(),
                zoom,
            },
        ) {
            JobResult::RenderedThumbs(thumbs) => Some(thumbs),
            _ => None,
        }
    }

    /// Render several pages of a PDF file as thumbnails (low priority).
    pub fn page_thumbs(&self, path: &Path, pages: &[u32], zoom: f32) -> Option<Vec<PageThumb>> {
        self.render_pages(path, pages, zoom, Priority::Low)
    }

    /// Width and height of every page of a PDF file, in points.
    pub fn page_sizes(&self, path: &Path) -> Option<Vec<(f32, f32)>> {
        match self.execute(
            Priority::VisiblePage,
            Job::FilePageSizes {
                path: path.to_path_buf(),
            },
        ) {
            JobResult::PageSizes(sizes) => Some(sizes),
            _ => None,
        }
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
    // The span carries the job description and its duration; the worker
    // is a single thread, so its timeline shows up directly in traces.
    let _span = tracing::debug_span!("worker_job", ?job.job, priority = ?job.priority).entered();
    // A panicking job must not kill the worker: the shared Pdfium binding
    // panics when libpdfium.so is missing, and a dead worker would make
    // every later job fail silently.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match job.job {
        Job::Op(command) => JobResult::Op(manager.execute(command)),
        Job::RenderFilePage { path, page, zoom } => render_file_page(&path, page, zoom),
        Job::RenderPageThumbs { path, pages, zoom } => render_page_thumbs(&path, &pages, zoom),
        Job::FilePageCount { path } => file_page_count(&path),
        Job::FilePageSizes { path } => file_page_sizes(&path),
        Job::RenderThumb { path, size } => render_thumb(&path, size),
    }))
    .unwrap_or_else(|_| {
        tracing::warn!("worker job panicked; check libpdfium availability");
        JobResult::Error("worker job panicked".to_string())
    });
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

/// Render several pages of an arbitrary PDF file as thumbnails. Opens
/// the file once, renders every requested page, closes it again — the
/// shared open document stays untouched. Failed pages are skipped.
fn render_page_thumbs(path: &Path, pages: &[u32], zoom: f32) -> JobResult {
    let mut scratch = PdfOpsManager::new();
    match scratch.execute(Command::Open {
        path: path.to_path_buf(),
    }) {
        CommandResult::Ok => {
            let mut thumbs = Vec::new();
            for page in pages {
                if let CommandResult::Rendered {
                    width,
                    height,
                    rgba_data,
                } = scratch.execute(Command::RenderPage { page: *page, zoom })
                {
                    thumbs.push((*page, width, height, rgba_data));
                }
            }
            JobResult::RenderedThumbs(thumbs)
        }
        CommandResult::Error(e) => JobResult::Error(format!("{e:?}")),
        _ => JobResult::Error("unexpected open result".to_string()),
    }
}

/// Count the pages of an arbitrary PDF file. Opens the file, counts,
/// closes it again — the shared open document stays untouched.
fn file_page_count(path: &Path) -> JobResult {
    let mut scratch = PdfOpsManager::new();
    match scratch.execute(Command::Open {
        path: path.to_path_buf(),
    }) {
        CommandResult::Ok => match scratch.execute(Command::PageCount) {
            CommandResult::PageCount(count) => JobResult::PageCount(count),
            CommandResult::Error(e) => JobResult::Error(format!("{e:?}")),
            _ => JobResult::Error("unexpected page count result".to_string()),
        },
        CommandResult::Error(e) => JobResult::Error(format!("{e:?}")),
        _ => JobResult::Error("unexpected open result".to_string()),
    }
}

/// Read the size of every page of an arbitrary PDF file, in points.
/// Opens the file, reads, closes it again — the shared open document
/// stays untouched.
fn file_page_sizes(path: &Path) -> JobResult {
    let mut scratch = PdfOpsManager::new();
    match scratch.execute(Command::Open {
        path: path.to_path_buf(),
    }) {
        CommandResult::Ok => match scratch.execute(Command::PageSizes) {
            CommandResult::PageSizes(sizes) => JobResult::PageSizes(sizes),
            CommandResult::Error(e) => JobResult::Error(format!("{e:?}")),
            _ => JobResult::Error("unexpected page sizes result".to_string()),
        },
        CommandResult::Error(e) => JobResult::Error(format!("{e:?}")),
        _ => JobResult::Error("unexpected open result".to_string()),
    }
}

/// Render the first page of a PDF file as a thumbnail-sized image and
/// store it in the freedesktop thumbnail cache. The caller has already
/// checked the cache, so this always renders on a miss.
fn render_thumb(path: &std::path::Path, size: ThumbSize) -> JobResult {
    let mut scratch = PdfOpsManager::new();
    match scratch.execute(Command::Open {
        path: path.to_path_buf(),
    }) {
        CommandResult::Ok => {
            let result = scratch.execute(Command::RenderThumbnail {
                page: 1,
                max_px: size.max_px(),
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
