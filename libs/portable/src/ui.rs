//! A small branded preparation window; close it before Windows launch dialogs.
use native_windows_gui as nwg;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub struct Splash {
    done: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Splash {
    pub fn close(mut self) -> bool {
        self.done.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.cancelled.load(Ordering::SeqCst)
    }
}

pub fn setup() -> Splash {
    let done = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::new(AtomicBool::new(false));
    let finish = done.clone();
    let cancel = cancelled.clone();
    let thread = std::thread::spawn(move || {
        if let Err(error) = show(finish, cancel) {
            eprintln!("Merdian-Desk preparation window: {error}");
        }
    });
    Splash {
        done,
        cancelled,
        thread: Some(thread),
    }
}

fn show(done: Arc<AtomicBool>, cancelled: Arc<AtomicBool>) -> Result<(), nwg::NwgError> {
    nwg::init()?;
    let mut window = nwg::Window::default();
    let mut label = nwg::Label::default();
    let mut timer = nwg::AnimationTimer::default();
    nwg::Window::builder()
        .title("Merdian-Desk")
        .size((350, 100))
        .center(true)
        .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
        .build(&mut window)?;
    nwg::Label::builder()
        .parent(&window)
        .text("Merdian-Desk\r\nPreparing attended support. No installation.")
        .position((18, 18))
        .size((310, 58))
        .build(&mut label)?;
    nwg::AnimationTimer::builder()
        .parent(&window)
        .interval(std::time::Duration::from_millis(50))
        .build(&mut timer)?;
    let handler = nwg::full_bind_event_handler(&window.handle, move |event, _, _| match event {
        nwg::Event::OnTimerTick if done.load(Ordering::SeqCst) => nwg::stop_thread_dispatch(),
        nwg::Event::OnWindowClose => {
            if !done.load(Ordering::SeqCst) {
                cancelled.store(true, Ordering::SeqCst);
            }
            nwg::stop_thread_dispatch();
        }
        _ => {}
    });
    timer.start();
    nwg::dispatch_thread_events();
    timer.stop();
    nwg::unbind_event_handler(&handler);
    Ok(())
}
