use crate::{Action, Application, Reply, Request};
use bobo_core::{Draft, Error, Id, Result};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        mpsc::{self, RecvTimeoutError, SyncSender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub const DRAFT_DEBOUNCE: Duration = Duration::from_millis(250);

pub enum Response {
    Updated {
        action: Action,
        result: Result<Box<Reply>>,
    },
    DraftSaved(Result<()>),
}

enum Work {
    Request(Request),
    DraftWake,
    Stop,
}

#[derive(Default)]
struct DraftInbox {
    changes: HashMap<Id, String>,
    deadline: Option<Instant>,
    wake_queued: bool,
}

pub struct Worker {
    sender: SyncSender<Work>,
    responses: async_channel::Receiver<Response>,
    drafts: Arc<Mutex<DraftInbox>>,
    thread: Option<JoinHandle<()>>,
}

fn open_application<'a>(
    application: &'a mut Option<Application>,
    path: &std::path::Path,
) -> Result<&'a mut Application> {
    if application.is_none() {
        *application = Some(Application::open(path)?);
    }
    application
        .as_mut()
        .ok_or_else(|| Error::Storage("Aplicação local indisponível.".into()))
}

fn flush_drafts(
    inbox: &Mutex<DraftInbox>,
    application: &mut Option<Application>,
    path: &std::path::Path,
) -> Result<()> {
    let changes = {
        let mut inbox = inbox
            .lock()
            .map_err(|_| Error::Storage("Fila de rascunhos indisponível.".into()))?;
        inbox.deadline = None;
        std::mem::take(&mut inbox.changes)
    };
    if changes.is_empty() {
        return Ok(());
    }
    let drafts: Vec<_> = changes
        .iter()
        .map(|(id, body)| Draft {
            channel_id: *id,
            body: body.clone(),
        })
        .collect();
    let result = open_application(application, path).and_then(|app| app.persist_drafts(&drafts));
    if result.is_err() {
        let mut inbox = inbox
            .lock()
            .map_err(|_| Error::Storage("Fila de rascunhos indisponível.".into()))?;
        for (id, body) in changes {
            inbox.changes.entry(id).or_insert(body);
        }
        // Retry on the next edit, explicit action, or shutdown, never a polling loop.
    }
    result
}

impl Worker {
    pub fn start(path: PathBuf) -> std::io::Result<(Self, async_channel::Receiver<Response>)> {
        // Reserve room for one wake and one foreground command. Typing immediately
        // followed by Enter must not fail just because a draft wake is queued.
        let (sender, requests) = mpsc::sync_channel(2);
        let (responses, receiver) = async_channel::unbounded();
        let drafts = Arc::new(Mutex::new(DraftInbox::default()));
        let thread_drafts = drafts.clone();
        let thread = thread::Builder::new()
            .name("bobocord-storage".into())
            .spawn(move || {
                let mut application = None;
                loop {
                    let deadline = thread_drafts.lock().ok().and_then(|inbox| inbox.deadline);
                    let work = match deadline {
                        Some(deadline) => match requests
                            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                        {
                            Ok(work) => work,
                            Err(RecvTimeoutError::Timeout) => {
                                let result = flush_drafts(&thread_drafts, &mut application, &path);
                                let _ = responses.send_blocking(Response::DraftSaved(result));
                                continue;
                            }
                            Err(RecvTimeoutError::Disconnected) => Work::Stop,
                        },
                        None => requests.recv().unwrap_or(Work::Stop),
                    };
                    match work {
                        Work::DraftWake => {
                            if let Ok(mut inbox) = thread_drafts.lock() {
                                inbox.wake_queued = false;
                            }
                        }
                        Work::Request(request) => {
                            let action = request.action.clone();
                            let result = flush_drafts(&thread_drafts, &mut application, &path)
                                .and_then(|()| {
                                    open_application(&mut application, &path)?
                                        .handle(request)
                                        .map(Box::new)
                                });
                            let _ = responses.send_blocking(Response::Updated { action, result });
                        }
                        Work::Stop => {
                            if let Err(error) =
                                flush_drafts(&thread_drafts, &mut application, &path)
                            {
                                eprintln!("Falha ao salvar estado local ao encerrar: {error}");
                            }
                            break;
                        }
                    }
                }
            })?;
        let worker = Self {
            sender,
            responses: receiver.clone(),
            drafts,
            thread: Some(thread),
        };
        let _ = worker.submit(Request {
            action: Action::Reload,
            draft: String::new(),
        });
        Ok((worker, receiver))
    }

    pub fn submit(&self, request: Request) -> Result<()> {
        self.sender.try_send(Work::Request(request)).map_err(|_| {
            Error::Storage("Armazenamento ocupado ou indisponível. Tente novamente.".into())
        })
    }

    pub fn edit_draft(&self, draft: Draft) -> Result<()> {
        if draft.body.chars().count() > bobo_core::DRAFT_LIMIT || draft.body.contains('\0') {
            return Err(Error::Validation(
                "Rascunho acima do limite local ou com caractere NUL.".into(),
            ));
        }
        let mut inbox = self
            .drafts
            .lock()
            .map_err(|_| Error::Storage("Fila de rascunhos indisponível.".into()))?;
        inbox.changes.insert(draft.channel_id, draft.body);
        inbox.deadline = Some(Instant::now() + DRAFT_DEBOUNCE);
        if !inbox.wake_queued {
            match self.sender.try_send(Work::DraftWake) {
                Ok(()) => inbox.wake_queued = true,
                Err(mpsc::TrySendError::Full(_)) => {} // Queued foreground action flushes pending drafts.
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    return Err(Error::Storage("Armazenamento indisponível.".into()));
                }
            }
        }
        Ok(())
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.responses.close();
        let _ = self.sender.send(Work::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
