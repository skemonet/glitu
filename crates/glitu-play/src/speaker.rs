//! Speaker view (S key, `--speaker`): a second window, full screen on the other screen when
//! there are two. Over it, a bar with the slide, its click and title, and the timer; under it, the
//! current click and the next one on the left, the notes on the right.
//!
//! It is painted by software too, and only when something changes: the body at each click,
//! the bar alone at each second of the timer.

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use softbuffer::{Context, Surface};
use winit::event_loop::ActiveEventLoop;
use winit::monitor::MonitorHandle;
use winit::window::{Fullscreen, Window};

use crate::deck::{Deck, Image};
use crate::resample::resample;
use crate::show::{Fit, blit};
use crate::text::{Canvas, Text};

const BACKGROUND: u32 = 0x1b1d23;
const BAR: u32 = 0x111318;
const INK: u32 = 0xe8e8ea;
const DIM: u32 = 0x8a8f99;
const RUNNING: u32 = 0x4fc3f7;
const PAUSED: u32 = 0xf0b400;
const FRAME: u32 = 0x3a3e48;

/// Time of the talk: it starts at the first click forward, `t` pauses it, `r` sets it back to
/// zero (it starts again at the next click)
#[derive(Default)]
pub struct Timer {
    banked: Duration,
    since: Option<Instant>,
}

impl Timer {
    pub fn elapsed(&self, now: Instant) -> Duration {
        self.banked + self.since.map_or(Duration::ZERO, |s| now - s)
    }

    pub fn running(&self) -> bool {
        self.since.is_some()
    }

    /// Not started yet (or reset)
    pub fn idle(&self) -> bool {
        self.since.is_none() && self.banked.is_zero()
    }

    pub fn moved_forward(&mut self, now: Instant) {
        if self.idle() {
            self.since = Some(now);
        }
    }

    pub fn toggle(&mut self, now: Instant) {
        match self.since.take() {
            Some(s) => self.banked += now - s,
            None => self.since = Some(now),
        }
    }

    pub fn reset(&mut self) {
        *self = Timer::default();
    }

    /// Until the next second shown
    pub fn next_tick(&self, now: Instant) -> Option<Duration> {
        self.running().then(|| Duration::from_millis(1000 - (self.elapsed(now).as_millis() % 1000) as u64))
    }
}

/// `12:05`, `1:02:09`
fn clock(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{:02}:{:02}", s / 60, s % 60)
    }
}

pub type Rect = (i32, i32, i32, i32);

/// Where the parts go in a window of `size`
#[derive(Debug, PartialEq)]
struct Layout {
    bar: Rect,
    current: Rect,
    next_label: (i32, i32),
    next: Rect,
    notes: Rect,
    /// The size of the bar's text, of the notes' text before the zoom
    text: f32,
    notes_text: f32,
}

fn layout((w, h): (u32, u32)) -> Layout {
    let (w, h) = (w as i32, h as i32);
    let pad = (h / 40).max(8);
    let bar_h = (h / 13).max(28);
    let top = bar_h + pad;
    let left = ((w - 3 * pad) as f32 * 0.55) as i32;
    let body = h - top - pad;
    let current_h = (body as f32 * 0.6) as i32;
    let text = bar_h as f32 * 0.45;
    let label_h = (text * 1.4) as i32;
    let next_y = top + current_h + pad / 2 + label_h;
    let notes_x = 2 * pad + left;
    Layout {
        bar: (0, 0, w, bar_h),
        current: (pad, top, left, current_h),
        next_label: (pad, top + current_h + pad / 2),
        next: (pad, next_y, (left as f32 * 0.75) as i32, (h - pad - next_y).max(1)),
        notes: (notes_x, top, (w - notes_x - pad).max(1), body),
        text,
        notes_text: (h as f32 / 26.0).max(12.0),
    }
}

/// What the window shows, to paint only what changed
#[derive(Clone, PartialEq)]
struct Painted {
    at: usize,
    size: (u32, u32),
    zoom: i32,
}

/// The view's pixels, whatever window shows them
pub struct SpeakerView {
    pub screen: Vec<u32>,
    pub size: (u32, u32),
    text: Text,
    /// The previews, by click and size of their box
    thumbs: HashMap<(usize, (i32, i32)), Rc<Image>>,
    body: Option<Painted>,
    /// The text of the bar painted
    bar: Option<String>,
    /// Size of the notes: each step is 10 %
    pub zoom: i32,
    /// The redraw was asked by the player (else by the system: the window must be sent whole)
    pub asked: bool,
}

/// The title of the view's window
pub fn title(deck_title: &str) -> String {
    format!("Speaker view · {deck_title}")
}

/// The speaker view in a window of winit
pub struct Speaker {
    pub window: Rc<Window>,
    surface: Surface<Rc<Window>, Rc<Window>>,
    pub view: SpeakerView,
}

impl Speaker {
    /// `screen`: full screen on this monitor, else a window
    pub fn open(el: &ActiveEventLoop, deck_title: &str, screen: Option<MonitorHandle>) -> Result<Speaker, String> {
        let mut attrs = Window::default_attributes()
            .with_title(title(deck_title))
            .with_inner_size(winit::dpi::PhysicalSize::new(1024, 640));
        if let Some(m) = screen {
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(Some(m))));
        }
        let window = Rc::new(el.create_window(attrs).map_err(|e| e.to_string())?);
        let surface = Context::new(Rc::clone(&window))
            .and_then(|c| Surface::new(&c, Rc::clone(&window)))
            .map_err(|e| e.to_string())?;
        let size = window.inner_size();
        Ok(Speaker { window, surface, view: SpeakerView::new((size.width, size.height)) })
    }

    pub fn resized(&mut self, size: (u32, u32)) {
        self.view.resize(size);
        self.window.request_redraw();
    }

    /// Asks for a redraw: what changed only
    pub fn ask(&mut self) {
        self.view.asked = true;
        self.window.request_redraw();
    }

    /// Paints what changed, and shows it
    pub fn paint(&mut self, deck: &Deck, at: usize, timer: &Timer, now: Instant) -> Result<(), String> {
        let Some(damage) = self.view.paint(deck, at, timer, now)? else { return Ok(()) };
        let (w, h) = self.view.size;
        let (Some(nw), Some(nh)) = (NonZeroU32::new(w), NonZeroU32::new(h)) else { return Ok(()) };
        self.surface.resize(nw, nh).map_err(|e| e.to_string())?;
        let mut buf = self.surface.buffer_mut().map_err(|e| e.to_string())?;
        if buf.len() == self.view.screen.len() {
            buf.copy_from_slice(&self.view.screen);
        }
        let rect = damage.and_then(|(x, y, w, h)| {
            Some(softbuffer::Rect {
                x: x as u32,
                y: y as u32,
                width: NonZeroU32::new(w as u32)?,
                height: NonZeroU32::new(h as u32)?,
            })
        });
        match rect {
            Some(r) => buf.present_with_damage(&[r]),
            None => buf.present(),
        }
        .map_err(|e| e.to_string())
    }
}

impl SpeakerView {
    pub fn new(size: (u32, u32)) -> SpeakerView {
        SpeakerView {
            screen: Vec::new(),
            size: (size.0.max(1), size.1.max(1)),
            text: Text::new(),
            thumbs: HashMap::new(),
            body: None,
            bar: None,
            zoom: 0,
            asked: false,
        }
    }

    pub fn resize(&mut self, size: (u32, u32)) {
        self.size = (size.0.max(1), size.1.max(1));
    }

    /// The size of the notes, by steps of 10 %
    pub fn zoom_by(&mut self, by: i32) {
        self.zoom = (self.zoom + by).clamp(-5, 15);
    }

    /// Paints what changed since the last time: what to send (none: nothing; else the
    /// rectangle that changed, none for all)
    pub fn paint(
        &mut self,
        deck: &Deck,
        at: usize,
        timer: &Timer,
        now: Instant,
    ) -> Result<Option<Option<Rect>>, String> {
        let asked = std::mem::take(&mut self.asked);
        let l = layout(self.size);
        let painted = Painted { at, size: self.size, zoom: self.zoom };
        let full = self.body.as_ref() != Some(&painted);
        if full {
            self.screen.clear();
            self.screen.resize(self.size.0 as usize * self.size.1 as usize, BACKGROUND);
            self.paint_body(deck, at, &l)?;
            self.body = Some(painted);
            self.bar = None;
        }
        let bar = self.bar_text(deck, at, timer, now);
        let changed = self.bar.as_ref() != Some(&bar);
        if changed {
            self.paint_bar(&bar, timer, &l);
            self.bar = Some(bar);
        } else if asked && !full {
            return Ok(None);
        }
        Ok(Some((asked && !full).then_some(l.bar)))
    }

    /// The left of the bar, and the timer
    fn bar_text(&self, deck: &Deck, at: usize, timer: &Timer, now: Instant) -> String {
        let c = &deck.clicks[at];
        let total = deck.clicks.iter().map(|c| c.slide).max().unwrap_or(1);
        let clicks: Vec<usize> = (0..deck.clicks.len()).filter(|&i| deck.clicks[i].slide == c.slide).collect();
        let mut left = format!("Slide {} / {total}", c.slide);
        if clicks.len() > 1 {
            let k = clicks.iter().position(|&i| i == at).unwrap_or(0) + 1;
            left += &format!("   ·   click {k} / {}", clicks.len());
        }
        if !c.title.is_empty() {
            left += &format!("   ·   {}", c.title);
        }
        let state = if timer.running() {
            ""
        } else if timer.idle() {
            "starts at the next click   "
        } else {
            "paused   "
        };
        format!("{left}\t{state}{}", clock(timer.elapsed(now)))
    }

    fn paint_bar(&mut self, bar: &str, timer: &Timer, l: &Layout) {
        let (w, _) = self.size;
        let mut c = Canvas { pixels: &mut self.screen, width: w as usize, clip: (0, 0, l.bar.2, l.bar.3) };
        c.fill(l.bar, BAR);
        let (left, right) = bar.split_once('\t').unwrap_or((bar, ""));
        let pad = l.current.0 as f32;
        let y = (l.bar.3 as f32 - self.text.line(l.text)) / 2.0;
        let (state, time) = right.rsplit_once("   ").unwrap_or(("", right));
        // the timer is larger, at the right; the rest gives way to it
        let big = l.text * 1.3;
        let tw = self.text.width(time, big);
        let tx = w as f32 - pad - tw;
        let sw = if state.is_empty() { 0.0 } else { self.text.width(state, l.text) + pad };
        let color = if timer.running() {
            RUNNING
        } else if timer.idle() {
            DIM
        } else {
            PAUSED
        };
        self.text.draw(&mut c, time, big, color, (tx, (l.bar.3 as f32 - self.text.line(big)) / 2.0));
        if !state.is_empty() {
            self.text.draw(&mut c, state, l.text, color, (tx - sw, y));
        }
        c.clip.2 = (tx - sw - pad) as i32;
        self.text.draw(&mut c, left, l.text, INK, (pad, y));
    }

    fn thumb(&mut self, deck: &Deck, i: usize, (w, h): (i32, i32)) -> Result<Rc<Image>, String> {
        if let Some(t) = self.thumbs.get(&(i, (w, h))) {
            return Ok(Rc::clone(t));
        }
        let fit = Fit::new((deck.width, deck.height), (w.max(1) as u32, h.max(1) as u32));
        let t = Rc::new(resample(deck.image(i)?, fit.size, fit.scale));
        // the click shown, the next, and the one before (for going back)
        self.thumbs.retain(|&(k, _), _| k + 1 >= i && k <= i + 1);
        self.thumbs.insert((i, (w, h)), Rc::clone(&t));
        Ok(t)
    }

    fn paint_body(&mut self, deck: &Deck, at: usize, l: &Layout) -> Result<(), String> {
        let current = self.thumb(deck, at, (l.current.2, l.current.3))?;
        self.preview(&current, l.current);
        let next = at + 1;
        let next = if next < deck.clicks.len() { Some(self.thumb(deck, next, (l.next.2, l.next.3))?) } else { None };
        let (w, h) = self.size;
        let mut c = Canvas { pixels: &mut self.screen, width: w as usize, clip: (0, 0, w as i32, h as i32) };
        let label = if next.is_some() { "Next" } else { "End of the presentation" };
        self.text.draw(&mut c, label, l.text, DIM, (l.next_label.0 as f32, l.next_label.1 as f32));
        if let Some(next) = next {
            self.preview(&next, l.next);
        }
        self.paint_notes(&deck.clicks[at].notes, l);
        Ok(())
    }

    /// An image at the top left of its box, framed
    fn preview(&mut self, img: &Image, (ox, oy, _, _): Rect) {
        let (sw, sh) = self.size;
        let mut c = Canvas { pixels: &mut self.screen, width: sw as usize, clip: (0, 0, sw as i32, sh as i32) };
        c.fill((ox - 1, oy - 1, img.width as i32 + 2, img.height as i32 + 2), FRAME);
        blit(&mut self.screen, self.size, img, (ox as usize, oy as usize));
    }

    fn paint_notes(&mut self, notes: &[String], l: &Layout) {
        let (x, y, w, h) = l.notes;
        let mut c = Canvas { pixels: &mut self.screen, width: self.size.0 as usize, clip: (x, y, x + w, y + h) };
        let size = l.notes_text * (1.0 + 0.1 * self.zoom as f32);
        if notes.is_empty() {
            self.text.draw(&mut c, "No notes", size, DIM, (x as f32, y as f32));
            return;
        }
        let line = self.text.line(size);
        let mut top = y as f32;
        for para in notes {
            for l in self.text.wrap(para, size, w as f32) {
                self.text.draw(&mut c, &l, size, INK, (x as f32, top));
                top += line;
            }
            top += line * 0.6;
            if top > (y + h) as f32 {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_timer() {
        let t0 = Instant::now();
        let s = |n| t0 + Duration::from_secs(n);
        let mut t = Timer::default();
        assert!(t.idle() && t.next_tick(t0).is_none());
        t.moved_forward(s(1));
        assert_eq!(t.elapsed(s(4)), Duration::from_secs(3));
        t.moved_forward(s(4)); // already running: unchanged
        t.toggle(s(5));
        assert!(!t.running() && !t.idle());
        assert_eq!(t.elapsed(s(100)), Duration::from_secs(4));
        t.moved_forward(s(100)); // paused by hand: a click does not start it again
        assert!(!t.running());
        t.toggle(s(100));
        assert_eq!(t.elapsed(s(102)), Duration::from_secs(6));
        assert_eq!(t.next_tick(s(102) + Duration::from_millis(300)), Some(Duration::from_millis(700)));
        t.reset();
        assert!(t.idle());
        assert_eq!(
            (clock(Duration::from_secs(725)), clock(Duration::from_secs(3729))),
            ("12:05".into(), "1:02:09".into())
        );
    }

    #[test]
    fn the_layout() {
        for size in [(1024, 600), (1366, 768), (1920, 1080), (640, 480)] {
            let l = layout(size);
            let inside = |(x, y, w, h): Rect| {
                x >= 0 && y >= 0 && w > 0 && h > 0 && x + w <= size.0 as i32 && y + h <= size.1 as i32
            };
            assert!([l.bar, l.current, l.next, l.notes].into_iter().all(inside), "{size:?}: {l:?}");
            // the previews on the left, the notes on the right, all under the bar
            assert!(l.current.0 + l.current.2 < l.notes.0 && l.next.0 + l.next.2 < l.notes.0);
            assert!(l.current.1 >= l.bar.3 && l.next.1 > l.current.1 + l.current.3 && l.notes.1 >= l.bar.3);
        }
    }
}
