//! The work done away from the window's thread: a worker thread takes the jobs in order of need
//! (for the images: the click shown, then the next one and its states on the way, then the
//! previous one; for the transitions: the next click, then the one after); the window takes the
//! results when it is woken up, and a key pressed meanwhile is never blocked.

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};

use crate::deck::{Archive, Image, decode};
use crate::resample::resample;

/// The size an image is wanted at: the screen's, or the slide's own (the animations that the
/// compositor scales to the screen)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Screen,
    Small,
}

/// An image of the archive, at one or two sizes (read and decoded once); `generation` changes
/// with the scale
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    pub generation: u64,
    pub name: String,
    pub sizes: Vec<(Kind, (u32, u32), f32)>,
}

struct Queue<J> {
    jobs: VecDeque<J>,
    busy: Option<J>,
    stop: bool,
}

/// A thread that does the jobs `J` into results `R`, the most urgent first
pub struct Worker<J, R> {
    shared: Arc<(Mutex<Queue<J>>, Condvar)>,
    results: Receiver<(J, R)>,
}

/// The images read and scaled
pub type Loader = Worker<Job, Result<Vec<(Kind, Image)>, String>>;

/// The work of a job: read, decode, scale
pub fn load(archive: &Archive, job: &Job) -> Result<Vec<(Kind, Image)>, String> {
    let bytes = archive.read(&job.name)?;
    let img = decode(&bytes).map_err(|e| format!("{}: {e}", job.name))?;
    let mut out = Vec::new();
    // the decoded image itself goes to the last size (at scale 1, it is that image)
    if let Some((&(kind, size, scale), first)) = job.sizes.split_last() {
        for &(k, sz, sc) in first {
            out.push((k, resample(img.clone(), sz, sc)));
        }
        out.push((kind, resample(img, size, scale)));
    }
    Ok(out)
}

fn work<J: Clone, R>(
    mut task: impl FnMut(&J) -> R,
    shared: Arc<(Mutex<Queue<J>>, Condvar)>,
    out: Sender<(J, R)>,
    wake: impl Fn(),
) {
    let (lock, ready) = &*shared;
    loop {
        let job = {
            let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
            loop {
                if q.stop {
                    return;
                }
                if let Some(job) = q.jobs.pop_front() {
                    q.busy = Some(job.clone());
                    break job;
                }
                q = ready.wait(q).unwrap_or_else(|e| e.into_inner());
            }
        };
        let result = task(&job);
        lock.lock().unwrap_or_else(|e| e.into_inner()).busy = None;
        if out.send((job, result)).is_err() {
            return;
        }
        wake();
    }
}

impl Loader {
    /// `wake`: called from the worker after each image, to wake the window's thread up
    pub fn new(archive: Archive, wake: impl Fn() + Send + 'static) -> Loader {
        Worker::spawn(move |job| load(&archive, job), wake)
    }
}

impl<J: Clone + PartialEq + Send + 'static, R: Send + 'static> Worker<J, R> {
    /// `task` does a job; `wake`: called from the worker after each one
    pub fn spawn(task: impl FnMut(&J) -> R + Send + 'static, wake: impl Fn() + Send + 'static) -> Worker<J, R> {
        let queue = Queue { jobs: VecDeque::new(), busy: None, stop: false };
        let shared = Arc::new((Mutex::new(queue), Condvar::new()));
        let (tx, results) = channel();
        let worker = Arc::clone(&shared);
        std::thread::spawn(move || work(task, worker, tx, wake));
        Worker { shared, results }
    }

    /// The jobs needed, most urgent first: they replace the queue (the one being done is not
    /// asked again)
    pub fn want(&self, jobs: Vec<J>) {
        let (lock, ready) = &*self.shared;
        let mut q = lock.lock().unwrap_or_else(|e| e.into_inner());
        let busy = q.busy.take();
        q.jobs = jobs.into_iter().filter(|j| busy.as_ref() != Some(j)).collect();
        q.busy = busy;
        ready.notify_one();
    }

    /// The jobs finished since the last call, with their results
    pub fn take(&self) -> Vec<(J, R)> {
        self.results.try_iter().collect()
    }
}

impl<J, R> Drop for Worker<J, R> {
    fn drop(&mut self) {
        let (lock, ready) = &*self.shared;
        lock.lock().unwrap_or_else(|e| e.into_inner()).stop = true;
        ready.notify_one();
    }
}
