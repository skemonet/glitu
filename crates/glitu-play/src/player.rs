//! The windows of winit and their pixels (softbuffer): the slides (what they show: `show`) and
//! the speaker view (see `speaker`). On Wayland with `wp_viewporter`, `wayland` is used instead.

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, NamedKey};
use winit::monitor::MonitorHandle;
use winit::window::{Fullscreen, Window, WindowId};

use crate::Options;
use crate::deck::Deck;
use crate::show::{Command, Log, Region, Show};
use crate::speaker::{Speaker, Timer};

/// About 60 frames per second during an animation, where the compositor does not pace them
const FRAME: Duration = Duration::from_millis(16);

/// The loader has finished an image
pub struct Wake;

struct App {
    options: Options,
    show: Option<Show>,
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    speaker: Option<Speaker>,
    timer: Timer,
    error: Option<String>,
    proxy: EventLoopProxy<Wake>,
    /// The frames are paced by the compositor (Wayland's frame callbacks), not by a timer
    paced: bool,
    /// When the last frame of the slides began: the next one, `FRAME` after it (timer pacing)
    frame: Instant,
    /// The last frames sent, the latest first
    sent: Log,
    /// Frames still to send at start-up, so that the buffers are first written while nothing moves
    /// (the first writes into a new shared memory pool are slow)
    warm: u8,
}

/// Is the compositor pacing the frames? On Wayland, `pre_present_notify` then a redraw asked
/// after presenting wait for its frame callback
fn paced(el: &ActiveEventLoop) -> bool {
    #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
    {
        use winit::platform::wayland::ActiveEventLoopExtWayland;
        el.is_wayland()
    }
    #[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd")))]
    {
        let _ = el;
        false
    }
}

pub fn title(deck: &Deck) -> String {
    if deck.title.is_empty() { "glitu-play".to_string() } else { deck.title.clone() }
}

impl App {
    fn fail(&mut self, el: &ActiveEventLoop, e: String) {
        self.error = Some(e);
        el.exit();
    }

    /// Draws and sends a frame; is an animation still running? A frame of an animation is drawn
    /// straight into the buffer (every pixel of its area is drawn anew each frame, so `screen`
    /// keeps the previous click and is not copied); its last frame and the still images go into
    /// `screen`. Only what this buffer lacks of `screen` is copied, and the compositor is told
    /// what changed since the last frame.
    fn send(&mut self) -> Result<bool, String> {
        let (Some(show), Some(surface), Some(window)) = (&mut self.show, &mut self.surface, &self.window) else {
            return Ok(false);
        };
        let next = show.next_frame();
        if let Some((t, None)) = next {
            show.finish(t)?;
        }
        let (w, h) = show.fit.window;
        let (Some(nw), Some(nh)) = (NonZeroU32::new(w), NonZeroU32::new(h)) else { return Ok(show.animating()) };
        surface.resize(nw, nh).map_err(|e| e.to_string())?;
        let mut buf = surface.buffer_mut().map_err(|e| e.to_string())?;
        let age = usize::from(buf.age());
        let damage = show.compose(&mut self.sent, &mut buf, age, next)?;
        window.pre_present_notify();
        match damage {
            Region::Rect((x, y, w, h)) => match (NonZeroU32::new(w), NonZeroU32::new(h)) {
                (Some(width), Some(height)) => buf.present_with_damage(&[softbuffer::Rect { x, y, width, height }]),
                _ => buf.present(),
            },
            Region::Nothing | Region::All => buf.present(),
        }
        .map_err(|e| e.to_string())?;
        Ok(show.animating())
    }

    fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    /// Goes to click `to`, in both windows; the timer starts at the first click forward
    fn go(&mut self, to: usize) -> Result<(), String> {
        let Some(show) = &mut self.show else { return Ok(()) };
        if to > show.at {
            self.timer.moved_forward(Instant::now());
        }
        if show.go(to)? {
            self.redraw();
        }
        self.redraw_speaker();
        Ok(())
    }

    fn redraw_speaker(&mut self) {
        if let Some(s) = &mut self.speaker {
            s.ask();
        }
    }

    /// Opens or closes the speaker view: full screen on `screen`, or on another screen than the
    /// slides' if there is one
    fn toggle_speaker(&mut self, el: &ActiveEventLoop, screen: Option<MonitorHandle>) -> Result<(), String> {
        if self.speaker.take().is_some() {
            return Ok(());
        }
        let (Some(show), Some(window)) = (&self.show, &self.window) else { return Ok(()) };
        let screen = screen.or_else(|| {
            let current = window.current_monitor();
            el.available_monitors().find(|m| Some(m) != current.as_ref()).filter(|_| !self.options.window)
        });
        let mut speaker = Speaker::open(el, &title(&show.deck), screen)?;
        speaker.ask();
        self.speaker = Some(speaker);
        Ok(())
    }

    fn key(&mut self, el: &ActiveEventLoop, id: WindowId, key: &Key) -> Result<(), String> {
        let command = match key {
            Key::Named(n) => match n {
                NamedKey::ArrowRight | NamedKey::ArrowDown | NamedKey::Space | NamedKey::Enter | NamedKey::PageDown => {
                    Some(Command::Next)
                }
                NamedKey::ArrowLeft | NamedKey::ArrowUp | NamedKey::Backspace | NamedKey::PageUp => {
                    Some(Command::Previous)
                }
                NamedKey::Home => Some(Command::First),
                NamedKey::End => Some(Command::Last),
                NamedKey::F11 => Some(Command::Fullscreen),
                NamedKey::Escape => Some(Command::LeaveFullscreen),
                _ => None,
            },
            Key::Character(c) => Command::of_char(c),
            _ => None,
        };
        match command {
            Some(c) => self.command(el, id, c),
            None => Ok(()),
        }
    }

    fn command(&mut self, el: &ActiveEventLoop, id: WindowId, command: Command) -> Result<(), String> {
        let Some(show) = &self.show else { return Ok(()) };
        let (at, last) = (show.at, show.deck.clicks.len() - 1);
        match command {
            Command::Next => return self.go(at + 1),
            Command::Previous => return self.go(at.saturating_sub(1)),
            Command::First => return self.go(0),
            Command::Last => return self.go(last),
            _ => {}
        }
        // full screen: of the window that has the key; the cursor is hidden over the slides only
        let speaker = self.speaker.as_ref().filter(|s| s.window.id() == id).map(|s| Rc::clone(&s.window));
        let target = speaker.clone().or_else(|| self.window.clone());
        match command {
            Command::Fullscreen => {
                if let Some(w) = &target {
                    toggle_fullscreen(w, speaker.is_none());
                }
            }
            Command::LeaveFullscreen => {
                if let Some(w) = &target {
                    w.set_fullscreen(None);
                    w.set_cursor_visible(true);
                }
            }
            Command::Quit => el.exit(),
            Command::Speaker => return self.toggle_speaker(el, None),
            Command::Timer => self.timer.toggle(Instant::now()),
            Command::ResetTimer => self.timer.reset(),
            Command::Bigger | Command::Smaller => {
                if let Some(s) = &mut self.speaker {
                    s.view.zoom_by(if command == Command::Bigger { 1 } else { -1 });
                }
            }
            Command::Next | Command::Previous | Command::First | Command::Last => {}
        }
        self.redraw_speaker();
        Ok(())
    }

    fn paint_speaker(&mut self) -> Result<(), String> {
        let (Some(speaker), Some(show)) = (&mut self.speaker, &self.show) else { return Ok(()) };
        speaker.paint(&show.deck, show.at, &self.timer, Instant::now())
    }
}

fn toggle_fullscreen(window: &Window, hide_cursor: bool) {
    let full = window.fullscreen().is_none();
    window.set_fullscreen(full.then_some(Fullscreen::Borderless(None)));
    window.set_cursor_visible(!(full && hide_cursor));
}

/// With the speaker view and two screens: the slides on the secondary screen, the view on the
/// primary one (the laptop's, usually)
fn screens(el: &ActiveEventLoop) -> Option<(MonitorHandle, MonitorHandle)> {
    let all: Vec<MonitorHandle> = el.available_monitors().collect();
    let primary = el.primary_monitor().or_else(|| all.first().cloned())?;
    let other = all.into_iter().find(|m| *m != primary)?;
    Some((other, primary))
}

impl ApplicationHandler<Wake> for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let deck = match Deck::open(&self.options.deck) {
            Ok(d) => d,
            Err(e) => return self.fail(el, e),
        };
        let mut attrs = Window::default_attributes()
            .with_title(title(&deck))
            .with_inner_size(winit::dpi::PhysicalSize::new(deck.width, deck.height));
        let screens = screens(el).filter(|_| self.options.speaker && !self.options.window);
        if !self.options.window {
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(screens.as_ref().map(|s| s.0.clone()))));
        }
        let window = match el.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(e) => return self.fail(el, e.to_string()),
        };
        window.set_cursor_visible(self.options.window);
        let surface = Context::new(Rc::clone(&window)).and_then(|c| Surface::new(&c, Rc::clone(&window)));
        let surface = match surface {
            Ok(s) => s,
            Err(e) => return self.fail(el, e.to_string()),
        };
        let size = window.inner_size();
        let at = deck.start_click(self.options.start);
        let proxy = self.proxy.clone();
        let wake = move || {
            let _ = proxy.send_event(Wake);
        };
        let mut show = Show::new(deck, at, (size.width.max(1), size.height.max(1)), false, wake);
        if let Err(e) = show.first() {
            return self.fail(el, e);
        }
        self.paced = paced(el);
        self.show = Some(show);
        self.surface = Some(surface);
        self.window = Some(window);
        if self.options.speaker
            && let Err(e) = self.toggle_speaker(el, screens.map(|s| s.1))
        {
            self.fail(el, e);
        }
    }

    fn new_events(&mut self, _: &ActiveEventLoop, cause: StartCause) {
        if !matches!(cause, StartCause::ResumeTimeReached { .. }) {
            return;
        }
        if let (Some(show), Some(w)) = (&self.show, &self.window)
            && show.animating()
            && !self.paced
        {
            w.request_redraw();
        }
        if self.timer.running() {
            self.redraw_speaker();
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let on_speaker = self.speaker.as_ref().is_some_and(|s| s.window.id() == id);
        let result = match event {
            WindowEvent::CloseRequested if on_speaker => {
                self.speaker = None;
                Ok(())
            }
            WindowEvent::CloseRequested => {
                el.exit();
                Ok(())
            }
            WindowEvent::Resized(size) if on_speaker => {
                if let Some(s) = &mut self.speaker {
                    s.resized((size.width, size.height));
                }
                Ok(())
            }
            WindowEvent::Resized(size) => match &mut self.show {
                Some(show) if size.width > 0 && size.height > 0 => {
                    show.resize((size.width, size.height), false).map(|changed| {
                        if changed {
                            self.redraw();
                        }
                    })
                }
                _ => Ok(()),
            },
            WindowEvent::RedrawRequested if on_speaker => self.paint_speaker(),
            WindowEvent::RedrawRequested => {
                let begin = Instant::now();
                self.frame = begin;
                self.send().map(|running| {
                    if let Some(show) = &mut self.show {
                        show.presented(begin.elapsed());
                    }
                    // on Wayland, the next frame when the compositor asks for it
                    if running && self.paced {
                        self.redraw();
                    } else if self.warm > 0 {
                        self.warm -= 1;
                        self.redraw();
                    }
                })
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                self.key(el, id, &event.logical_key)
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                match &self.show {
                    Some(show) => self.go(show.at + 1),
                    None => Ok(()),
                }
            }
            _ => Ok(()),
        };
        if let Err(e) = result {
            self.fail(el, e);
        }
    }

    fn user_event(&mut self, el: &ActiveEventLoop, _: Wake) {
        let Some(show) = &mut self.show else { return };
        match show.loaded() {
            Ok(true) => self.redraw(),
            Ok(false) => {}
            Err(e) => self.fail(el, e),
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let now = Instant::now();
        let animating = self.show.as_ref().is_some_and(Show::animating) && !self.paced;
        // the next frame of the animation, the next second of the timer (a little after it)
        let tick = self.speaker.as_ref().and(self.timer.next_tick(now)).map(|t| now + t + Duration::from_millis(5));
        let wake = [animating.then(|| (self.frame + FRAME).max(now)), tick].into_iter().flatten().min();
        el.set_control_flow(wake.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        if let Some(show) = &mut self.show {
            show.end_trace();
        }
    }
}

pub fn run(options: Options) -> Result<(), String> {
    let el = EventLoop::<Wake>::with_user_event().build().map_err(|e| format!("no display: {e}"))?;
    let mut app = App {
        options,
        show: None,
        window: None,
        surface: None,
        speaker: None,
        timer: Timer::default(),
        error: None,
        proxy: el.create_proxy(),
        paced: false,
        frame: Instant::now(),
        sent: Log::new(),
        warm: 2,
    };
    el.run_app(&mut app).map_err(|e| e.to_string())?;
    match app.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
