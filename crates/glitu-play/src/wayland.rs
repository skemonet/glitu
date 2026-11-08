//! The windows on Wayland, without winit: our own shared-memory buffers, so that an animation
//! can be drawn at the slide's own size and enlarged to the screen by the compositor
//! (`wp_viewporter`). On a slow computer, 1024 × 576 pixels instead of 1920 × 1080 are 3.5
//! times fewer bytes to blend, copy and upload; the compositor's bilinear scaling is softer,
//! which nobody sees while it moves, and the click's last image comes back sharp, at full size.
//!
//! Used when the compositor has `wp_viewporter` (else, or with `GLITU_PLAY_WINIT=1`: winit;
//! `GLITU_PLAY_SMALL=0`: all frames at the screen's size, to compare). The keys are read with
//! the compositor's keymap (`keymap`), without xkbcommon.

use std::os::unix::fs::FileExt;
use std::time::{Duration, Instant};

use smithay_client_toolkit as sctk;

use sctk::compositor::{CompositorHandler, CompositorState};
use sctk::output::{OutputHandler, OutputState};
use sctk::reexports::calloop::EventLoop;
use sctk::reexports::calloop::channel::{self, Event};
use sctk::reexports::calloop_wayland_source::WaylandSource;
use sctk::reexports::client::globals::registry_queue_init;
use sctk::reexports::client::protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface};
use sctk::reexports::client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};
use sctk::reexports::csd_frame::WindowState;
use sctk::reexports::protocols::wp::viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter};
use sctk::registry::{ProvidesRegistryState, RegistryState, SimpleGlobal};
use sctk::seat::pointer::{CursorIcon, PointerEvent, PointerEventKind, PointerHandler, ThemeSpec, ThemedPointer};
use sctk::seat::{Capability, SeatHandler, SeatState};
use sctk::shell::WaylandSurface;
use sctk::shell::xdg::XdgShell;
use sctk::shell::xdg::window::{Window, WindowConfigure, WindowDecorations, WindowHandler};
use sctk::shm::slot::{Buffer, SlotPool};
use sctk::shm::{Shm, ShmHandler};
use sctk::{
    delegate_compositor, delegate_output, delegate_pointer, delegate_registry, delegate_seat, delegate_shm,
    delegate_simple, delegate_xdg_shell, delegate_xdg_window, registry_handlers,
};

use crate::Options;
use crate::deck::Deck;
use crate::keymap::{self, Keymap};
use crate::player::title;
use crate::show::{Command, Log, Region, Show};
use crate::speaker::{self, SpeakerView, Timer};

/// The left button (evdev)
const BTN_LEFT: u32 = 0x110;

/// A buffer and what it shows
struct Slot {
    buffer: Buffer,
    size: (u32, u32),
    /// The number of the frame it was last sent with (0: never)
    sent: u64,
    /// The animation whose frame it shows
    anim: Option<u64>,
}

/// The buffers of a window at one size, used in turn
#[derive(Default)]
struct Buffers {
    slots: Vec<Slot>,
    /// Frames sent through them
    frames: u64,
}

impl Buffers {
    /// A buffer of `size` that the compositor does not use (made if none, up to three)
    fn free(&mut self, pool: &mut SlotPool, size: (u32, u32)) -> Result<Option<usize>, String> {
        self.slots.retain(|s| s.size == size);
        if let Some(i) = self.slots.iter().position(|s| s.buffer.canvas(pool).is_some()) {
            return Ok(Some(i));
        }
        if self.slots.len() >= 3 {
            return Ok(None);
        }
        let (w, h) = (size.0 as i32, size.1 as i32);
        let (buffer, _) =
            pool.create_buffer(w, h, w * 4, wl_shm::Format::Xrgb8888).map_err(|e| format!("Wayland buffer: {e}"))?;
        self.slots.push(Slot { buffer, size, sent: 0, anim: None });
        Ok(Some(self.slots.len() - 1))
    }

    /// How many frames ago the buffer `i` was sent (0: never)
    fn age(&self, i: usize) -> usize {
        match self.slots[i].sent {
            0 => 0,
            n => (self.frames + 1 - n) as usize,
        }
    }

    fn sent(&mut self, i: usize) {
        self.frames += 1;
        self.slots[i].sent = self.frames;
    }

    /// The pixels of buffer `i`
    fn pixels<'a>(&self, i: usize, pool: &'a mut SlotPool) -> Result<&'a mut [u32], String> {
        let bytes = self.slots[i].buffer.canvas(pool).ok_or("Wayland buffer in use")?;
        bytemuck::try_cast_slice_mut(bytes).map_err(|e| format!("Wayland buffer: {e}"))
    }
}

/// Attaches `buffer`, marks `damage` (in its pixels), shows it at `size` (the window's, in the
/// compositor's units), asks for a frame callback and commits
fn commit(
    qh: &QueueHandle<App>,
    window: &Window,
    viewport: &WpViewport,
    buffer: &Buffer,
    damage: Region,
    size: (u32, u32),
) {
    let surface = window.wl_surface();
    if buffer.attach_to(surface).is_err() {
        return;
    }
    match damage {
        Region::Rect((x, y, w, h)) => surface.damage_buffer(x as i32, y as i32, w as i32, h as i32),
        Region::All | Region::Nothing => surface.damage_buffer(0, 0, i32::MAX, i32::MAX),
    }
    viewport.set_destination(size.0.max(1) as i32, size.1.max(1) as i32);
    surface.frame(qh, surface.clone());
    surface.commit();
}

/// A window: the slides' or the speaker view's
struct Win {
    window: Window,
    viewport: WpViewport,
    /// In the compositor's units, and the output's scale
    size: (u32, u32),
    scale: u32,
    configured: bool,
    fullscreen: bool,
    /// A frame callback is awaited (the compositor has not shown the last frame yet)
    waiting: bool,
    /// Something to draw when the compositor is ready
    dirty: bool,
    full: Buffers,
}

impl Win {
    fn open(app: &App, title: &str, size: (u32, u32), output: Option<&wl_output::WlOutput>, full: bool) -> Win {
        let qh = &app.qh;
        let surface = app.compositor.create_surface(qh);
        let window = app.xdg.create_window(surface, WindowDecorations::RequestServer, qh);
        window.set_title(title);
        window.set_app_id("glitu-play");
        if full {
            window.set_fullscreen(output);
        }
        let viewport = app.viewporter.get_viewport(window.wl_surface(), qh, ());
        window.commit();
        Win {
            window,
            viewport,
            size,
            scale: 1,
            configured: false,
            fullscreen: full,
            waiting: false,
            dirty: true,
            full: Buffers::default(),
        }
    }

    /// The size in pixels
    fn pixels(&self) -> (u32, u32) {
        (self.size.0.max(1) * self.scale, self.size.1.max(1) * self.scale)
    }

    fn is(&self, surface: &wl_surface::WlSurface) -> bool {
        self.window.wl_surface() == surface
    }
}

/// The slides' window
struct Slides {
    win: Win,
    /// The screen it is on
    output: Option<wl_output::WlOutput>,
    /// The buffers at the slide's size
    small: Buffers,
    /// The last frame sent was one of those
    showing_small: bool,
    sent: Log,
    /// Frames still to send at start-up, so that the buffers are first written while nothing
    /// moves (the first writes into a new shared memory pool are slow)
    warm: u8,
}

struct SpeakerWin {
    win: Win,
    view: SpeakerView,
}

struct App {
    conn: Connection,
    qh: QueueHandle<App>,
    registry: RegistryState,
    outputs: OutputState,
    seats: SeatState,
    compositor: CompositorState,
    xdg: XdgShell,
    shm: Shm,
    viewporter: WpViewporter,
    pool: SlotPool,
    options: Options,
    /// The animations drawn at the slide's size
    small: bool,
    title: String,
    /// The deck, until the window's size is known
    deck: Option<Deck>,
    show: Option<Show>,
    wake: channel::Sender<()>,
    slides: Option<Slides>,
    speaker: Option<SpeakerWin>,
    timer: Timer,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    keymap: Keymap,
    /// The surface that has the keyboard
    focus: Option<wl_surface::WlSurface>,
    pointer: Option<ThemedPointer>,
    exit: bool,
    error: Option<String>,
}

/// Runs the player on Wayland; none if it cannot be used here (then winit is used)
pub fn run(options: &Options) -> Option<Result<(), String>> {
    if std::env::var_os("GLITU_PLAY_WINIT").is_some_and(|v| !v.is_empty()) {
        return None;
    }
    let conn = Connection::connect_to_env().ok()?;
    let (globals, queue) = registry_queue_init(&conn).ok()?;
    let qh = queue.handle();
    let compositor = CompositorState::bind(&globals, &qh).ok()?;
    let xdg = XdgShell::bind(&globals, &qh).ok()?;
    let shm = Shm::bind(&globals, &qh).ok()?;
    let viewporter = SimpleGlobal::<WpViewporter, 1>::bind(&globals, &qh).ok()?.get().ok()?.clone();
    Some(play(options, conn, &globals, queue, compositor, xdg, shm, viewporter).and_then(|(mut el, mut app)| {
        while !app.exit {
            let timeout = app.timeout();
            el.dispatch(timeout, &mut app).map_err(|e| e.to_string())?;
            app.after();
        }
        if let Some(show) = &mut app.show {
            show.end_trace();
        }
        app.error.map_or(Ok(()), Err)
    }))
}

#[allow(clippy::too_many_arguments)]
fn play(
    options: &Options,
    conn: Connection,
    globals: &sctk::reexports::client::globals::GlobalList,
    mut queue: sctk::reexports::client::EventQueue<App>,
    compositor: CompositorState,
    xdg: XdgShell,
    shm: Shm,
    viewporter: WpViewporter,
) -> Result<(EventLoop<'static, App>, App), String> {
    let deck = Deck::open(&options.deck)?;
    let qh = queue.handle();
    let pool = SlotPool::new(1 << 20, &shm).map_err(|e| format!("Wayland shared memory: {e}"))?;
    let (wake, woken) = channel::channel::<()>();
    let mut app = App {
        conn: conn.clone(),
        qh: qh.clone(),
        registry: RegistryState::new(globals),
        outputs: OutputState::new(globals, &qh),
        seats: SeatState::new(globals, &qh),
        compositor,
        xdg,
        shm,
        viewporter,
        pool,
        options: Options { deck: options.deck.clone(), ..*options },
        small: std::env::var_os("GLITU_PLAY_SMALL").is_none_or(|v| v != "0"),
        title: title(&deck),
        deck: Some(deck),
        show: None,
        wake,
        slides: None,
        speaker: None,
        timer: Timer::default(),
        keyboard: None,
        keymap: Keymap::default(),
        focus: None,
        pointer: None,
        exit: false,
        error: None,
    };
    // the screens, before choosing where the windows go
    queue.roundtrip(&mut app).map_err(|e| e.to_string())?;
    let screens = app.screens().filter(|_| app.options.speaker && !app.options.window);
    let size = app.deck.as_ref().map_or((1024, 576), |d| (d.width, d.height));
    let win = Win::open(&app, &app.title, size, screens.as_ref().map(|s| &s.0), !app.options.window);
    app.slides =
        Some(Slides { win, output: None, small: Buffers::default(), showing_small: false, sent: Log::new(), warm: 2 });
    if app.options.speaker {
        app.open_speaker(screens.map(|s| s.1));
    }
    let el = EventLoop::try_new().map_err(|e| e.to_string())?;
    WaylandSource::new(conn, queue).insert(el.handle()).map_err(|e| e.to_string())?;
    el.handle()
        .insert_source(woken, |event, _, app: &mut App| {
            if let Event::Msg(()) = event {
                app.loaded();
            }
        })
        .map_err(|e| e.to_string())?;
    Ok((el, app))
}

impl App {
    fn fail(&mut self, e: String) {
        self.error.get_or_insert(e);
        self.exit = true;
    }

    /// With two screens: the slides on one, the speaker view on the other, the laptop's own
    /// screen if there is one, else the first by name (Wayland has no primary screen)
    fn screens(&self) -> Option<(wl_output::WlOutput, wl_output::WlOutput)> {
        let name = |o: &wl_output::WlOutput| self.outputs.info(o).and_then(|i| i.name).unwrap_or_default();
        let mut all: Vec<wl_output::WlOutput> = self.outputs.outputs().collect();
        all.sort_by_key(|o| name(o));
        let internal = |o: &&wl_output::WlOutput| ["eDP", "LVDS", "DSI"].iter().any(|p| name(o).starts_with(p));
        let speaker = all.iter().find(internal).or(all.first())?.clone();
        let slides = all.into_iter().find(|o| *o != speaker)?;
        Some((slides, speaker))
    }

    /// Opens the speaker view: full screen on `output`, or on another screen than the slides'
    fn open_speaker(&mut self, output: Option<wl_output::WlOutput>) {
        let slides = self.slides.as_ref().and_then(|s| s.output.clone());
        let other = self.outputs.outputs().find(|o| Some(o) != slides.as_ref());
        let output = output.or(other.filter(|_| !self.options.window));
        let title = speaker::title(&self.title);
        let win = Win::open(self, &title, (1024, 640), output.as_ref(), output.is_some());
        let view = SpeakerView::new(win.pixels());
        self.speaker = Some(SpeakerWin { win, view });
    }

    /// When to wake up without an event: the next second of the timer, in the speaker view
    fn timeout(&self) -> Option<Duration> {
        self.speaker.as_ref()?;
        self.timer.next_tick(Instant::now()).map(|t| t + Duration::from_millis(5))
    }

    /// After the events: draws what waits for it
    fn after(&mut self) {
        let result = self.draw_slides().and_then(|()| self.draw_speaker());
        if let Err(e) = result {
            self.fail(e);
        }
    }

    fn redraw(&mut self) {
        if let Some(s) = &mut self.slides {
            s.win.dirty = true;
        }
    }

    fn ask_speaker(&mut self) {
        if let Some(s) = &mut self.speaker {
            s.view.asked = true;
            s.win.dirty = true;
        }
    }

    fn loaded(&mut self) {
        let Some(show) = &mut self.show else { return };
        match show.loaded() {
            Ok(true) => self.redraw(),
            Ok(false) => {}
            Err(e) => self.fail(e),
        }
    }

    /// The slides' window has a new size (or its first)
    fn resized(&mut self) {
        let Some(s) = &mut self.slides else { return };
        let window = s.win.pixels();
        s.win.dirty = true;
        let result = match (&mut self.show, self.deck.take()) {
            (Some(show), _) => show.resize(window, self.small).map(|_| ()),
            (None, Some(deck)) => {
                let at = deck.clicks.iter().position(|c| c.slide >= self.options.start).unwrap_or(0);
                let wake = self.wake.clone();
                let mut show = Show::new(deck, at, window, self.small, move || {
                    let _ = wake.send(());
                });
                let first = show.first();
                self.show = Some(show);
                first
            }
            (None, None) => Ok(()),
        };
        if let Err(e) = result {
            self.fail(e);
        }
    }

    /// Draws and sends a frame of the slides when the compositor is ready for it: an animation
    /// at the slide's size into a buffer of that size (enlarged by the compositor), else as with
    /// winit (`Show::compose`)
    fn draw_slides(&mut self) -> Result<(), String> {
        let (Some(s), Some(show)) = (&mut self.slides, &mut self.show) else { return Ok(()) };
        if !s.win.dirty || s.win.waiting || !s.win.configured {
            return Ok(());
        }
        let begin = Instant::now();
        s.win.dirty = false;
        let next = show.next_frame();
        let size = s.win.size;
        match (next, show.small_anim()) {
            (Some((t, Some(_))), Some((id, fit))) => {
                let Some(i) = s.small.free(&mut self.pool, fit.window)? else {
                    s.win.dirty = true;
                    return Ok(());
                };
                let holds = s.small.slots[i].anim == Some(id);
                show.compose_small(s.small.pixels(i, &mut self.pool)?, holds, t)?;
                s.small.slots[i].anim = Some(id);
                s.small.sent(i);
                s.showing_small = true;
                commit(&self.qh, &s.win.window, &s.win.viewport, &s.small.slots[i].buffer, Region::All, size);
            }
            _ => {
                if let Some((t, None)) = next {
                    show.finish(t)?;
                }
                // after an animation at the slide's size, its last frame stays until the click's
                // image is read
                if !show.whole() {
                    return Ok(());
                }
                let Some(i) = s.win.full.free(&mut self.pool, show.fit.window)? else {
                    s.win.dirty = true;
                    return Ok(());
                };
                let age = s.win.full.age(i);
                let mut damage = show.compose(&mut s.sent, s.win.full.pixels(i, &mut self.pool)?, age, next)?;
                if std::mem::take(&mut s.showing_small) {
                    damage = Region::All;
                }
                s.win.full.sent(i);
                commit(&self.qh, &s.win.window, &s.win.viewport, &s.win.full.slots[i].buffer, damage, size);
            }
        }
        s.win.waiting = true;
        if show.animating() {
            s.win.dirty = true;
        } else if s.warm > 0 {
            s.warm -= 1;
            s.win.dirty = true;
        }
        show.presented(begin.elapsed());
        Ok(())
    }

    fn draw_speaker(&mut self) -> Result<(), String> {
        let (Some(sp), Some(show)) = (&mut self.speaker, &self.show) else { return Ok(()) };
        if !sp.win.configured || sp.win.waiting {
            return Ok(());
        }
        // each second of the timer, the bar
        if self.timer.running() {
            sp.view.asked = true;
            sp.win.dirty = true;
        }
        if !std::mem::take(&mut sp.win.dirty) {
            return Ok(());
        }
        let Some(damage) = sp.view.paint(&show.deck, show.at, &self.timer, Instant::now())? else { return Ok(()) };
        let Some(i) = sp.win.full.free(&mut self.pool, sp.view.size)? else {
            sp.win.dirty = true;
            sp.view.asked = false;
            return Ok(());
        };
        let pixels = sp.win.full.pixels(i, &mut self.pool)?;
        if pixels.len() == sp.view.screen.len() {
            pixels.copy_from_slice(&sp.view.screen);
        }
        sp.win.full.sent(i);
        let damage = damage.map_or(Region::All, |(x, y, w, h)| Region::Rect((x as u32, y as u32, w as u32, h as u32)));
        commit(&self.qh, &sp.win.window, &sp.win.viewport, &sp.win.full.slots[i].buffer, damage, sp.win.size);
        sp.win.waiting = true;
        Ok(())
    }

    /// Goes to click `to`, in both windows; the timer starts at the first click forward
    fn go(&mut self, to: usize) {
        let Some(show) = &mut self.show else { return };
        if to > show.at {
            self.timer.moved_forward(Instant::now());
        }
        match show.go(to) {
            Ok(true) => self.redraw(),
            Ok(false) => {}
            Err(e) => self.fail(e),
        }
        self.ask_speaker();
    }

    fn key(&mut self, key: u32) {
        let Some(command) = self.keymap.symbol(key).and_then(keymap::command) else { return };
        let Some(show) = &self.show else { return };
        let (at, last) = (show.at, show.deck.clicks.len() - 1);
        match command {
            Command::Next => return self.go(at + 1),
            Command::Previous => return self.go(at.saturating_sub(1)),
            Command::First => return self.go(0),
            Command::Last => return self.go(last),
            _ => {}
        }
        // full screen: of the window that has the key
        let on_speaker = self.speaker.as_ref().zip(self.focus.as_ref()).is_some_and(|(s, f)| s.win.is(f));
        let target =
            if on_speaker { self.speaker.as_ref().map(|s| &s.win) } else { self.slides.as_ref().map(|s| &s.win) };
        match command {
            Command::Fullscreen => {
                if let Some(w) = target {
                    if w.fullscreen {
                        w.window.unset_fullscreen();
                    } else {
                        w.window.set_fullscreen(None);
                    }
                }
            }
            Command::LeaveFullscreen => {
                if let Some(w) = target {
                    w.window.unset_fullscreen();
                }
            }
            Command::Quit => self.exit = true,
            Command::Speaker => {
                if self.speaker.take().is_none() {
                    self.open_speaker(None);
                }
            }
            Command::Timer => self.timer.toggle(Instant::now()),
            Command::ResetTimer => self.timer.reset(),
            Command::Bigger | Command::Smaller => {
                if let Some(s) = &mut self.speaker {
                    s.view.zoom_by(if command == Command::Bigger { 1 } else { -1 });
                }
            }
            Command::Next | Command::Previous | Command::First | Command::Last => {}
        }
        self.ask_speaker();
    }

    /// The cursor: hidden over the slides in full screen
    fn cursor(&self, surface: &wl_surface::WlSurface) {
        let Some(p) = &self.pointer else { return };
        let hidden = self.slides.as_ref().is_some_and(|s| s.win.is(surface) && s.win.fullscreen);
        let _ = if hidden { p.hide_cursor() } else { p.set_cursor(&self.conn, CursorIcon::Default) };
    }

    fn win_of(&mut self, surface: &wl_surface::WlSurface) -> Option<&mut Win> {
        if let Some(s) = self.slides.as_mut().filter(|s| s.win.is(surface)) {
            return Some(&mut s.win);
        }
        self.speaker.as_mut().filter(|s| s.win.is(surface)).map(|s| &mut s.win)
    }
}

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        factor: i32,
    ) {
        let scale = factor.max(1) as u32;
        let Some(w) = self.win_of(surface) else { return };
        if w.scale == scale {
            return;
        }
        w.scale = scale;
        w.dirty = true;
        if self.slides.as_ref().is_some_and(|s| s.win.is(surface)) {
            self.resized();
        } else if let Some(s) = &mut self.speaker {
            s.view.resize(s.win.pixels());
            s.view.asked = false;
        }
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, surface: &wl_surface::WlSurface, _: u32) {
        if let Some(w) = self.win_of(surface) {
            w.waiting = false;
        }
    }

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        output: &wl_output::WlOutput,
    ) {
        if let Some(s) = self.slides.as_mut().filter(|s| s.win.is(surface)) {
            s.output = Some(output.clone());
        }
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl WindowHandler for App {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, window: &Window) {
        if self.speaker.as_ref().is_some_and(|s| s.win.window == *window) {
            self.speaker = None;
        } else {
            self.exit = true;
        }
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        window: &Window,
        configure: WindowConfigure,
        _: u32,
    ) {
        let surface = window.wl_surface().clone();
        let Some(w) = self.win_of(&surface) else { return };
        let size = match configure.new_size {
            (Some(width), Some(height)) => (width.get(), height.get()),
            _ => w.size,
        };
        let fullscreen = configure.state.contains(WindowState::FULLSCREEN);
        let changed = !w.configured || size != w.size || fullscreen != w.fullscreen;
        (w.size, w.fullscreen, w.configured) = (size, fullscreen, true);
        if !changed {
            return;
        }
        w.dirty = true;
        // a new configuration is drawn at once, without waiting for the last frame to be shown
        w.waiting = false;
        if self.slides.as_ref().is_some_and(|s| s.win.is(&surface)) {
            self.resized();
        } else if let Some(s) = &mut self.speaker {
            s.view.resize(s.win.pixels());
            s.view.asked = false;
        }
        self.cursor(&surface);
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            self.keyboard = Some(seat.get_keyboard(qh, ()));
        }
        if capability == Capability::Pointer && self.pointer.is_none() {
            let surface = self.compositor.create_surface(qh);
            let themed = self.seats.get_pointer_with_theme(qh, &seat, self.shm.wl_shm(), surface, ThemeSpec::default());
            self.pointer = themed.ok();
        }
    }

    fn remove_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Keyboard
            && let Some(k) = self.keyboard.take()
        {
            k.release();
        }
        if capability == Capability::Pointer {
            self.pointer = None;
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for App {
    fn event(
        app: &mut App,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
        match event {
            wl_keyboard::Event::Keymap { format: WEnum::Value(wl_keyboard::KeymapFormat::XkbV1), fd, size } => {
                // read at its start: the compositor may share the file with other clients
                let mut text = vec![0; size as usize];
                if std::fs::File::from(fd).read_exact_at(&mut text, 0).is_ok() {
                    let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
                    app.keymap = Keymap::parse(&String::from_utf8_lossy(&text[..end]));
                }
            }
            wl_keyboard::Event::Enter { surface, .. } => app.focus = Some(surface),
            wl_keyboard::Event::Leave { .. } => app.focus = None,
            wl_keyboard::Event::Key { key, state: WEnum::Value(wl_keyboard::KeyState::Pressed), .. } => app.key(key),
            _ => {}
        }
    }
}

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for e in events {
            match e.kind {
                PointerEventKind::Enter { .. } => self.cursor(&e.surface),
                PointerEventKind::Press { button: BTN_LEFT, .. }
                    if self.slides.as_ref().is_some_and(|s| s.win.is(&e.surface)) =>
                {
                    if let Some(at) = self.show.as_ref().map(|s| s.at) {
                        self.go(at + 1);
                    }
                }
                _ => {}
            }
        }
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }

    registry_handlers![OutputState, SeatState];
}

/// The viewport has no events
impl Dispatch<WpViewport, ()> for App {
    fn event(
        _: &mut App,
        _: &WpViewport,
        _: <WpViewport as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
    }
}

delegate_compositor!(App);
delegate_output!(App);
delegate_shm!(App);
delegate_seat!(App);
delegate_pointer!(App);
delegate_xdg_shell!(App);
delegate_xdg_window!(App);
delegate_registry!(App);
delegate_simple!(App, WpViewporter, 1);
