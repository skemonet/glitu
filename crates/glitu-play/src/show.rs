//! What the slides' window shows, whatever draws it (winit + softbuffer in `player`, Wayland in
//! `wayland`): the images fitted to the window, the navigation, the animation of the clicks
//! forward (back, the image changes at once), and the frames sent through buffers that come
//! back in any order.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::deck::{Deck, Draw, Fade, Film, Image};
use crate::loader::{Job, Kind, Loader, Worker, load};
use crate::resample::resample;
use crate::scene::Transition;

/// Cross-fade of a click on the same slide, and from one slide to the next, in ms
const FRAGMENT_MS: f64 = 300.0;
const SLIDE_MS: f64 = 400.0;

/// Where the slide goes in the window: its scale, its size and its place
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Fit {
    pub scale: f32,
    pub size: (u32, u32),
    pub offset: (usize, usize),
    pub window: (u32, u32),
}

impl Fit {
    pub fn new(image: (u32, u32), window: (u32, u32)) -> Fit {
        let s = (window.0 as f32 / image.0 as f32).min(window.1 as f32 / image.1 as f32);
        // the exported width is the screen's: no resampling for a pixel of difference
        let s =
            if (s - 1.0).abs() * image.0 as f32 <= 2.0 && image.0 <= window.0 && image.1 <= window.1 { 1.0 } else { s };
        let size = (((image.0 as f32 * s).round() as u32).max(1), ((image.1 as f32 * s).round() as u32).max(1));
        let offset = (window.0.saturating_sub(size.0) as usize / 2, window.1.saturating_sub(size.1) as usize / 2);
        Fit { scale: s, size, offset, window }
    }

    /// With `small`, also the fit of the animations drawn at the slide's own size, in a window
    /// as much smaller as the slide is enlarged (the compositor enlarges it back): only when
    /// the slide is enlarged
    pub fn both(image: (u32, u32), window: (u32, u32), small: bool) -> (Fit, Option<Fit>) {
        let fit = Fit::new(image, window);
        let s = fit.scale;
        let little = (small && s > 1.0)
            .then(|| {
                let w = ((window.0 as f32 / s).round() as u32).max(image.0);
                let h = ((window.1 as f32 / s).round() as u32).max(image.1);
                Fit::new(image, (w, h))
            })
            .filter(|f| f.scale == 1.0);
        (fit, little)
    }
}

/// What a key does
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Next,
    Previous,
    First,
    Last,
    Fullscreen,
    LeaveFullscreen,
    Quit,
    Speaker,
    Timer,
    ResetTimer,
    Bigger,
    Smaller,
}

impl Command {
    /// The command of a character typed (with the keyboard's layout)
    pub fn of_char(c: &str) -> Option<Command> {
        Some(match c {
            "n" => Command::Next,
            "p" => Command::Previous,
            "f" => Command::Fullscreen,
            "q" => Command::Quit,
            "s" => Command::Speaker,
            "t" => Command::Timer,
            "r" => Command::ResetTimer,
            "+" | "=" => Command::Bigger,
            "-" => Command::Smaller,
            _ => return None,
        })
    }
}

/// A recorded animation playing (a terminal typing, animated code, an animated slide): its film,
/// the image shown and where
struct Playing {
    film: Film,
    shown: Option<usize>,
    image: Option<Image>,
}

/// What waits for its images: the current click, shown still, or the animation to it from
/// the previous one
#[derive(Clone, Copy, Debug, PartialEq)]
enum Pending {
    Still,
    Go(usize),
}

/// A transition running; its clock starts at its first frame, so that a slow preparation does
/// not eat it. Its frames are drawn straight into the window's buffer, `screen` keeping the
/// previous click (`id` tells them from those of another animation); the last one goes into
/// `screen`. `fit`: where it is drawn (the screen's, or the slide's own size)
struct Anim {
    id: u64,
    tr: Transition,
    start: Option<Instant>,
    fit: Fit,
    kind: Kind,
}

/// What a transition is built from (the arguments of `Transition::new`), to click `to` from the
/// one before
#[derive(Clone)]
struct Build {
    generation: u64,
    to: usize,
    kind: Kind,
    from: Arc<Image>,
    stages: Option<(Arc<Image>, Arc<Image>)>,
    image: Arc<Image>,
    draws: Vec<Draw>,
    fades: Vec<Fade>,
    global: f64,
}

/// A job is known by its click and scale (comparing the images would be slow)
impl PartialEq for Build {
    fn eq(&self, other: &Build) -> bool {
        (self.generation, self.to) == (other.generation, other.to)
    }
}

impl Build {
    fn build(&self) -> Transition {
        let from = Arc::clone(&self.from);
        Transition::new(from, self.stages.clone(), Arc::clone(&self.image), &self.draws, &self.fades, self.global)
    }
}

/// How to go from one click to the next: show the image (another size), or animate
enum Plan {
    Show,
    Animate(Build),
}

/// The measure of a click (`GLITU_PLAY_TRACE`): from the key to the first frame sent, the frames
/// and their spacing, the work per frame
struct Trace {
    click: usize,
    kind: &'static str,
    key: Instant,
    first: Option<Instant>,
    last: Option<Instant>,
    frames: u32,
    work: Duration,
    /// The transition was built before the key
    prepared: bool,
    /// Drawn at the slide's size, enlarged by the compositor
    small: bool,
}

impl Trace {
    fn line(&self) -> String {
        let ms = |d: Duration| d.as_secs_f64() * 1000.0;
        let Some((first, last)) = self.first.zip(self.last) else {
            return format!("click {} ({}): left before its first frame", self.click + 1, self.kind);
        };
        let spacing = if self.frames > 1 { ms(last - first) / f64::from(self.frames - 1) } else { 0.0 };
        format!(
            "click {} ({}): first frame {:.0} ms after the key, {} frames, {:.1} ms between frames, {:.1} ms of work per frame{}{}",
            self.click + 1,
            self.kind,
            ms(first - self.key),
            self.frames,
            spacing,
            ms(self.work) / f64::from(self.frames),
            if self.prepared { ", transition prepared ahead" } else { "" },
            if self.small { ", at the slide's size" } else { "" },
        )
    }
}

pub type Rect = (u32, u32, u32, u32);

/// A part of the window
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Region {
    Nothing,
    Rect(Rect),
    All,
}

impl Region {
    pub fn union(self, other: Region) -> Region {
        match (self, other) {
            (Region::All, _) | (_, Region::All) => Region::All,
            (Region::Nothing, r) | (r, Region::Nothing) => r,
            (Region::Rect(a), Region::Rect(b)) => Region::Rect(union(Some(a), b)),
        }
    }
}

/// A frame sent: what changed in `screen` with it, and what it shows that `screen` does not (a
/// frame of an animation drawn straight into the buffer: the animation, the part drawn)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sent {
    screen: Region,
    direct: Option<(u64, Rect)>,
}

/// The last frames sent, the latest first
pub type Log = VecDeque<Sent>;

/// What a buffer that shows the frame sent `age` frames ago (0: unknown) lacks of `screen`,
/// when a frame of the animation `drawing` is then drawn over it (the parts that animation
/// drew in it are drawn again)
fn stale(sent: &Log, age: usize, drawing: Option<u64>) -> Region {
    if age == 0 || age > sent.len() {
        return Region::All;
    }
    let since = sent.iter().take(age - 1).fold(Region::Nothing, |r, s| r.union(s.screen));
    match sent[age - 1].direct {
        Some((anim, rect)) if Some(anim) != drawing => since.union(Region::Rect(rect)),
        _ => since,
    }
}

/// The animation's frame to draw: its time in ms, and the animation's id (none: its last frame)
pub type Next = Option<(f64, Option<u64>)>;

pub struct Show {
    pub deck: Deck,
    pub at: usize,
    pub fit: Fit,
    /// The fit of the animations drawn at the slide's size (none: all at the screen's)
    small: Option<Fit>,
    /// Changes with the scale: the images of another one are dropped
    generation: u64,
    loader: Loader,
    /// The images fitted, around the current click, by name and size (an error: a damaged file)
    cache: HashMap<(String, Kind), Result<Arc<Image>, String>>,
    /// Builds the transitions of the next clicks while nothing moves
    builder: Worker<Build, Transition>,
    /// The transitions built, by click
    built: HashMap<usize, Transition>,
    pending: Option<Pending>,
    /// The click whose image the screen shows, whole (none during an animation)
    shown: Option<usize>,
    /// The window's pixels
    screen: Vec<u32>,
    /// `screen` shows something that may be sent (not after an animation at the slide's size
    /// whose last image is not read yet: the last frame sent stays)
    whole: bool,
    /// The previous click at the slide's size, under an animation drawn at that size
    small_base: Vec<u32>,
    anim: Option<Anim>,
    /// The id of the last animation
    anims: u64,
    film: Option<Playing>,
    /// What changed in `screen` since the last frame sent
    changed: Region,
    tracing: bool,
    trace: Option<Trace>,
}

impl Show {
    /// `small`: the animations without films drawn at the slide's size; `wake`: called from
    /// the worker threads when something is ready (then call `loaded`)
    pub fn new(
        deck: Deck,
        at: usize,
        window: (u32, u32),
        small: bool,
        wake: impl Fn() + Clone + Send + 'static,
    ) -> Show {
        let (fit, small) = Fit::both((deck.width, deck.height), window, small);
        let loader = Loader::new(deck.archive.clone(), wake.clone());
        let builder = Worker::spawn(Build::build, wake);
        let tracing = std::env::var_os("GLITU_PLAY_TRACE").is_some_and(|v| !v.is_empty());
        Show {
            deck,
            at,
            fit,
            small,
            generation: 0,
            loader,
            cache: HashMap::new(),
            builder,
            built: HashMap::new(),
            pending: Some(Pending::Still),
            shown: None,
            screen: Vec::new(),
            whole: true,
            small_base: Vec::new(),
            anim: None,
            anims: 0,
            film: None,
            changed: Region::All,
            tracing,
            trace: None,
        }
    }

    fn fit_of(&self, kind: Kind) -> Fit {
        match (kind, self.small) {
            (Kind::Small, Some(f)) => f,
            _ => self.fit,
        }
    }

    /// The size of the animation to click `to`: the slide's own if it has no film (a film
    /// lasts too long to be seen enlarged by the compositor)
    fn kind_of(&self, to: usize) -> Kind {
        if self.small.is_some() && self.deck.clicks[to].film.is_none() { Kind::Small } else { Kind::Screen }
    }

    fn job(&self, name: &str, kinds: &[Kind]) -> Job {
        // the decoded image goes to the last size: the slide's own, at scale 1
        let mut kinds = kinds.to_vec();
        kinds.sort_by_key(|k| *k == Kind::Small);
        let sizes = kinds
            .iter()
            .map(|&k| {
                let f = self.fit_of(k);
                (k, f.size, f.scale)
            })
            .collect();
        Job { generation: self.generation, name: name.to_string(), sizes }
    }

    /// The first image, read here: the window opens on it
    pub fn first(&mut self) -> Result<(), String> {
        let name = self.deck.clicks[self.at].image.clone();
        for (k, img) in load(&self.deck.archive, &self.job(&name, &[Kind::Screen]))? {
            self.cache.insert((name.clone(), k), Ok(Arc::new(img)));
        }
        self.settle().map(|_| ())
    }

    /// The image `name` at the size `kind`, if it is ready
    fn ready(&self, name: &str, kind: Kind) -> Result<Option<Arc<Image>>, String> {
        match self.cache.get(&(name.to_string(), kind)) {
            Some(Ok(img)) => Ok(Some(Arc::clone(img))),
            Some(Err(e)) => Err(e.clone()),
            None => Ok(None),
        }
    }

    /// Asks the loader for the images needed now, then for those of the next clicks and of the
    /// previous one; the others leave the cache
    fn want(&mut self) {
        let c = &self.deck.clicks;
        let at = self.at;
        let mut names: Vec<(&str, Kind)> = Vec::new();
        // the animations whose images are wanted, and the still images, most urgent first: the
        // one waiting, the click shown, then two clicks ahead (a second key pressed during the
        // animation finds its images ready, and the transitions to them are built ahead)
        let mut wants: Vec<(Option<usize>, usize)> = Vec::new();
        if let Some(Pending::Go(from)) = self.pending {
            wants.push((Some(from), at));
        }
        wants.push((None, at));
        for to in [at + 1, at + 2].into_iter().filter(|&to| to < c.len()) {
            wants.extend([(Some(to - 1), to), (None, to)]);
        }
        if at > 0 {
            wants.push((None, at - 1));
        }
        for (from, to) in wants {
            let Some(from) = from else {
                names.push((&c[to].image, Kind::Screen));
                continue;
            };
            let kind = self.kind_of(to);
            names.push((&c[from].image, kind));
            names.push((&c[to].image, kind));
            if let Some((x, y)) = c[to].stages.as_ref().filter(|_| c[from].slide == c[to].slide) {
                names.extend([(x.as_str(), kind), (y.as_str(), kind)]);
            }
        }
        let mut wanted: Vec<(String, Kind)> = Vec::new();
        for (n, k) in names {
            if !wanted.iter().any(|(w, wk)| w == n && *wk == k) {
                wanted.push((n.to_string(), k));
            }
        }
        self.cache.retain(|key, _| wanted.contains(key));
        // one job per image, for the sizes missing
        let mut jobs: Vec<(String, Vec<Kind>)> = Vec::new();
        for key in wanted.iter().filter(|key| !self.cache.contains_key(*key)) {
            match jobs.iter_mut().find(|(n, _)| *n == key.0) {
                Some((_, kinds)) => kinds.push(key.1),
                None => jobs.push((key.0.clone(), vec![key.1])),
            }
        }
        let jobs = jobs.iter().map(|(n, kinds)| self.job(n, kinds)).collect();
        self.loader.want(jobs);
    }

    /// Asks for the transitions to the next two clicks, once their images are ready; none while
    /// an animation runs (they would slow it down)
    fn prepare(&mut self) {
        let next: Vec<usize> = (self.at + 1..self.at + 3).filter(|&i| i < self.deck.clicks.len()).collect();
        self.built.retain(|to, _| next.contains(to));
        let mut jobs = Vec::new();
        if self.anim.is_none() {
            for to in next {
                if !self.built.contains_key(&to)
                    && let Ok(Some(Plan::Animate(b))) = self.plan(to - 1, to)
                {
                    jobs.push(b);
                }
            }
        }
        self.builder.want(jobs);
    }

    /// The images finished by the loader, the transitions by the builder; has the screen
    /// changed?
    pub fn loaded(&mut self) -> Result<bool, String> {
        for (job, images) in self.loader.take() {
            if job.generation != self.generation {
                continue;
            }
            match images {
                Ok(images) => {
                    for (k, img) in images {
                        self.cache.insert((job.name.clone(), k), Ok(Arc::new(img)));
                    }
                }
                Err(e) => {
                    for (k, ..) in job.sizes {
                        self.cache.insert((job.name.clone(), k), Err(e.clone()));
                    }
                }
            }
        }
        for (b, tr) in self.builder.take() {
            if b.generation == self.generation && (b.to == self.at + 1 || b.to == self.at + 2) {
                self.built.insert(b.to, tr);
            }
        }
        self.settle()
    }

    fn paint(&mut self, img: &Image) {
        self.changed = Region::All;
        self.whole = true;
        let (w, h) = (self.fit.window.0 as usize, self.fit.window.1 as usize);
        self.screen.clear();
        self.screen.resize(w * h, 0);
        blit(&mut self.screen, self.fit.window, img, self.fit.offset);
    }

    /// The window has another size; `small`: as in `new`. Has the screen changed?
    pub fn resize(&mut self, window: (u32, u32), small: bool) -> Result<bool, String> {
        let (fit, small) = Fit::both((self.deck.width, self.deck.height), window, small);
        if fit == self.fit && small == self.small && !self.screen.is_empty() {
            return Ok(false);
        }
        if fit.scale != self.fit.scale || small != self.small {
            self.generation += 1;
            self.cache.clear();
            self.built.clear();
        }
        self.fit = fit;
        self.small = small;
        self.anim = None;
        self.film = None;
        self.shown = None;
        self.screen.clear();
        self.screen.resize(window.0 as usize * window.1 as usize, 0);
        self.changed = Region::All;
        self.pending.get_or_insert(Pending::Still);
        self.settle()?;
        Ok(true)
    }

    /// Goes to click `to`: animated if it is the next one; has the screen changed?
    pub fn go(&mut self, to: usize) -> Result<bool, String> {
        let to = to.min(self.deck.clicks.len() - 1);
        self.film = None;
        if self.anim.take().is_some() {
            // a click during the animation ends it
            self.shown = None;
            self.pending = Some(Pending::Still);
        }
        if to != self.at {
            let from = self.at;
            self.at = to;
            self.pending = Some(if to == from + 1 { Pending::Go(from) } else { Pending::Still });
            if self.tracing {
                let kind = match self.pending {
                    Some(Pending::Go(from)) if self.deck.clicks[from].slide == self.deck.clicks[to].slide => "fragment",
                    Some(Pending::Go(_)) => "slide",
                    _ => "jump",
                };
                self.end_trace();
                let key = Instant::now();
                self.trace = Some(Trace {
                    click: to,
                    kind,
                    key,
                    first: None,
                    last: None,
                    frames: 0,
                    work: Duration::ZERO,
                    prepared: false,
                    small: false,
                });
            }
        }
        self.settle()
    }

    /// Does what waits once its images are ready; has the screen changed?
    fn settle(&mut self) -> Result<bool, String> {
        self.want();
        let changed = match self.pending {
            None => false,
            Some(Pending::Still) => self.still()?,
            Some(Pending::Go(from)) => self.start(from)?,
        };
        self.prepare();
        Ok(changed)
    }

    /// The current click, still, if its image is ready
    fn still(&mut self) -> Result<bool, String> {
        self.pending = Some(Pending::Still);
        let Some(img) = self.ready(&self.deck.clicks[self.at].image, Kind::Screen)? else { return Ok(false) };
        self.paint(&img);
        self.pending = None;
        self.shown = Some(self.at);
        Ok(true)
    }

    /// How to go from click `from` to click `to`, if their images are ready
    fn plan(&self, from: usize, to: usize) -> Result<Option<Plan>, String> {
        let c = &self.deck.clicks[to];
        let same = self.deck.clicks[from].slide == c.slide;
        let kind = self.kind_of(to);
        let (Some(a), Some(b)) = (self.ready(&self.deck.clicks[from].image, kind)?, self.ready(&c.image, kind)?) else {
            return Ok(None);
        };
        // the states on the way, for this animation only
        let stages = match (&c.stages, same) {
            (Some((base, drawn)), true) => match (self.ready(base, kind)?, self.ready(drawn, kind)?) {
                (Some(x), Some(y)) => Some((x, y)),
                _ => return Ok(None),
            },
            _ => None,
        };
        // the transition works on the slide's pixels; an image of another size is just shown
        if a.width != b.width || a.height != b.height {
            return Ok(Some(Plan::Show));
        }
        let s = self.fit_of(kind).scale;
        let draws: Vec<Draw> = if same { c.draws.iter().map(|d| scaled_draw(d, s)).collect() } else { Vec::new() };
        let fades: Vec<Fade> = if same { c.fades.iter().map(|f| scaled_fade(f, s)).collect() } else { Vec::new() };
        let global = if same { FRAGMENT_MS } else { SLIDE_MS };
        let stages = stages.filter(|(x, y)| x.pixels.len() == a.pixels.len() && y.pixels.len() == a.pixels.len());
        Ok(Some(Plan::Animate(Build {
            generation: self.generation,
            to,
            kind,
            from: a,
            stages,
            image: b,
            draws,
            fades,
            global,
        })))
    }

    /// The animation from `from` to the current click, if its images are ready
    fn start(&mut self, from: usize) -> Result<bool, String> {
        let to = self.at;
        let Some(plan) = self.plan(from, to)? else { return Ok(false) };
        let build = match plan {
            Plan::Show => return self.still(),
            Plan::Animate(b) => b,
        };
        self.pending = None;
        let fit = self.fit_of(build.kind);
        match build.kind {
            Kind::Screen if self.shown != Some(from) => self.paint(&build.from),
            Kind::Screen => {}
            Kind::Small => {
                self.small_base.clear();
                self.small_base.resize(fit.window.0 as usize * fit.window.1 as usize, 0);
                blit(&mut self.small_base, fit.window, &build.from, fit.offset);
            }
        }
        // built ahead, the key only starts the clock
        let prepared = self.built.remove(&to).filter(|_| from + 1 == to);
        if let Some(t) = &mut self.trace {
            t.prepared = prepared.is_some();
            t.small = build.kind == Kind::Small;
        }
        let tr = prepared.unwrap_or_else(|| build.build());
        self.anims += 1;
        self.anim = Some(Anim { id: self.anims, tr, start: None, fit, kind: build.kind });
        self.shown = None;
        self.film = self.deck.clicks[to].film.clone().map(|film| Playing { film, shown: None, image: None });
        Ok(true)
    }

    pub fn animating(&self) -> bool {
        self.anim.is_some()
    }

    /// The animation running at the slide's size: its id, and where it is drawn
    pub fn small_anim(&self) -> Option<(u64, Fit)> {
        self.anim.as_ref().filter(|a| a.kind == Kind::Small).map(|a| (a.id, a.fit))
    }

    /// `screen` may be sent
    // the Wayland front-end only: unused where it is not built (macOS, Windows)
    #[cfg_attr(
        not(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd")),
        allow(dead_code)
    )]
    pub fn whole(&self) -> bool {
        self.whole
    }

    /// The frame of the animation to draw now: its time in ms (the clock starts at the first
    /// frame), and the animation's id, none for its last frame (drawn into `screen` by `finish`)
    pub fn next_frame(&mut self) -> Next {
        let anim = self.anim.as_mut()?;
        let t = anim.start.get_or_insert_with(Instant::now).elapsed().as_secs_f64() * 1000.0;
        let mut end = anim.tr.duration();
        if let Some(p) = &self.film {
            end = end.max(p.film.frames.last().map_or(0.0, |f| f.0));
        }
        Some((t, (t < end).then_some(anim.id)))
    }

    /// Draws the animation at `t` ms into `out`, the window's pixels (`screen` if none); the
    /// part drawn
    fn draw(&mut self, t: f64, out: Option<&mut [u32]>) -> Result<Option<Rect>, String> {
        let mut screen = std::mem::take(&mut self.screen);
        let drawn = self.render(t, out.unwrap_or(&mut screen[..]));
        self.screen = screen;
        drawn
    }

    fn render(&mut self, t: f64, out: &mut [u32]) -> Result<Option<Rect>, String> {
        let Some(Anim { tr, fit, .. }) = &mut self.anim else { return Ok(None) };
        let fit = *fit;
        let stride = fit.window.0 as usize;
        tr.render(t.min(tr.duration()), out, stride, fit.offset);
        let (ox, oy) = (fit.offset.0 as u32, fit.offset.1 as u32);
        let mut drawn = tr.area.map(|(x, y, w, h)| (x + ox, y + oy, w, h));
        if let Some(p) = &mut self.film {
            if let Some(i) = p.film.frames.iter().rposition(|f| f.0 <= t)
                && p.shown != Some(i)
            {
                let s = fit.scale;
                let (_, _, w, h) = p.film.rect;
                let size = ((w as f32 * s).round() as u32, (h as f32 * s).round() as u32);
                p.image = Some(resample(self.deck.file(&p.film.frames[i].1)?, size, s));
                p.shown = Some(i);
            }
            if let Some(img) = &p.image {
                let s = fit.scale;
                let (x, y) =
                    (ox + (p.film.rect.0 as f32 * s).round() as u32, oy + (p.film.rect.1 as f32 * s).round() as u32);
                blit(out, fit.window, img, (x as usize, y as usize));
                drawn = Some(union(drawn, (x, y, img.width, img.height)));
            }
        }
        Ok(drawn.map(|r| clip(r, fit.window)))
    }

    /// The end of the animation, into `screen`: its last frame, or the click's image after a
    /// film or an animation at the slide's size
    pub fn finish(&mut self, t: f64) -> Result<(), String> {
        let small = self.anim.as_ref().is_some_and(|a| a.kind == Kind::Small);
        if !small && let Some(r) = self.draw(t, None)? {
            self.changed = self.changed.union(Region::Rect(r));
        }
        self.anim = None;
        let film = self.film.take().is_some();
        self.shown = Some(self.at);
        if small || film {
            match self.ready(&self.deck.clicks[self.at].image, Kind::Screen)? {
                Some(img) => self.paint(&img),
                // shown once read
                None if small => {
                    self.whole = false;
                    self.shown = None;
                    self.pending = Some(Pending::Still);
                }
                None => {}
            }
        }
        self.prepare();
        Ok(())
    }

    /// Brings `buf`, a buffer of the window that shows the frame sent `age` frames ago (0:
    /// unknown), to the frame to show: copies what it lacks of `screen`, then draws over it the
    /// animation's frame `next` when it is not the last (call `finish` before for that one).
    /// The frame is logged in `sent`; returns what changed since the last frame sent.
    pub fn compose(&mut self, sent: &mut Log, buf: &mut [u32], age: usize, next: Next) -> Result<Region, String> {
        let w = self.fit.window.0 as usize;
        let fits = buf.len() == self.screen.len();
        let drawing = next.and_then(|(_, id)| id).filter(|_| fits && self.small_anim().is_none());
        let changed = std::mem::replace(&mut self.changed, Region::Nothing);
        if fits {
            match stale(sent, age, drawing).union(changed) {
                Region::Nothing => {}
                Region::All => buf.copy_from_slice(&self.screen),
                Region::Rect((x, y, cw, ch)) => {
                    for row in y as usize..(y + ch) as usize {
                        let i = row * w + x as usize;
                        buf[i..i + cw as usize].copy_from_slice(&self.screen[i..i + cw as usize]);
                    }
                }
            }
        }
        let direct = match (next, drawing) {
            (Some((t, _)), Some(id)) => self.draw(t, Some(buf))?.map(|r| (id, r)),
            _ => None,
        };
        // what the last frame showed that `screen` does not, and this one
        let shown = |d: Option<(u64, Rect)>| d.map_or(Region::Nothing, |d| Region::Rect(d.1));
        let previous = sent.front().map_or(Region::Nothing, |s| shown(s.direct));
        sent.push_front(Sent { screen: changed, direct });
        sent.truncate(4);
        Ok(changed.union(shown(direct)).union(previous))
    }

    /// Draws a frame of the animation running at the slide's size into `buf`, a buffer of that
    /// size; `holds`: it shows a frame of this animation already (only the moving pixels are
    /// drawn again)
    // the Wayland front-end only: unused where it is not built (macOS, Windows)
    #[cfg_attr(
        not(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "netbsd")),
        allow(dead_code)
    )]
    pub fn compose_small(&mut self, buf: &mut [u32], holds: bool, t: f64) -> Result<(), String> {
        if !holds || buf.len() != self.small_base.len() {
            if buf.len() != self.small_base.len() {
                return Ok(());
            }
            buf.copy_from_slice(&self.small_base);
        }
        self.draw(t, Some(buf)).map(|_| ())
    }

    /// A frame was sent, after `work` spent on it
    pub fn presented(&mut self, work: Duration) {
        let Some(t) = &mut self.trace else { return };
        let now = Instant::now();
        t.first.get_or_insert(now);
        t.last = Some(now);
        t.frames += 1;
        t.work += work;
        if self.anim.is_none() && self.pending.is_none() {
            self.end_trace();
        }
    }

    pub fn end_trace(&mut self) {
        if let Some(t) = self.trace.take() {
            eprintln!("glitu-play: {}", t.line());
        }
    }
}

/// `img` copied into the window's pixels at `at`, cut at the window's edges
pub fn blit(screen: &mut [u32], window: (u32, u32), img: &Image, at: (usize, usize)) {
    let (w, h) = (window.0 as usize, window.1 as usize);
    let iw = img.width as usize;
    for (y, row) in img.pixels.chunks_exact(iw).enumerate() {
        if at.1 + y >= h || at.0 >= w {
            break;
        }
        let n = iw.min(w - at.0);
        screen[(at.1 + y) * w + at.0..][..n].copy_from_slice(&row[..n]);
    }
}

fn union(a: Option<Rect>, b: Rect) -> Rect {
    let Some(a) = a else { return b };
    let (x0, y0) = (a.0.min(b.0), a.1.min(b.1));
    let (x1, y1) = ((a.0 + a.2).max(b.0 + b.2), (a.1 + a.3).max(b.1 + b.3));
    (x0, y0, x1 - x0, y1 - y0)
}

/// `r` cut at the window's edges
fn clip(r: Rect, window: (u32, u32)) -> Rect {
    let (x, y) = (r.0.min(window.0), r.1.min(window.1));
    (x, y, r.2.min(window.0 - x), r.3.min(window.1 - y))
}

fn scaled_draw(d: &Draw, s: f32) -> Draw {
    Draw { points: d.points.iter().map(|&(x, y)| (x * s, y * s)).collect(), width: d.width * s, ..d.clone() }
}

fn scaled_fade(f: &Fade, s: f32) -> Fade {
    let (x, y, w, h) = f.rect;
    Fade { rect: (x * s, y * s, w * s, h * s), ..f.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_slide_fits_the_window() {
        // exported at the screen's width: shown as is, centred
        let f = Fit::new((1024, 576), (1024, 600));
        assert_eq!((f.scale, f.size, f.offset), (1.0, (1024, 576), (0, 12)));
        let f = Fit::new((1920, 1080), (1024, 600));
        assert_eq!((f.size, f.offset), ((1024, 576), (0, 12)));
        let f = Fit::new((960, 700), (1024, 600));
        assert_eq!(f.size.1, 600);
        assert!(f.offset.0 > 0 && f.offset.1 == 0);
        let fit = Fit::new((4, 2), (2, 1));
        let img = resample(Image { width: 4, height: 2, pixels: vec![0x808080; 8] }, fit.size, fit.scale);
        assert_eq!((img.width, img.height, img.pixels[0]), (2, 1, 0x808080));
        // an image at the window's edge is cut
        let mut screen = vec![0; 6];
        blit(&mut screen, (3, 2), &Image { width: 2, height: 2, pixels: vec![7; 4] }, (2, 1));
        assert_eq!(screen, [0, 0, 0, 0, 0, 7]);
        assert_eq!(union(Some((0, 0, 2, 2)), (3, 1, 1, 4)), (0, 0, 4, 5));
        assert_eq!(clip((10, 5, 30, 30), (20, 20)), (10, 5, 10, 15));
        // the animations at the slide's size: in a window as much smaller as the slide is enlarged
        let (fit, small) = Fit::both((1024, 576), (1920, 1080), true);
        assert_eq!(
            (fit.size, small.map(|f| (f.scale, f.window, f.offset))),
            ((1920, 1080), Some((1.0, (1024, 576), (0, 0))))
        );
        let (_, small) = Fit::both((1024, 576), (1920, 1200), true);
        assert_eq!(small.map(|f| (f.window, f.offset)), Some(((1024, 640), (0, 32))));
        // not when the slide is not enlarged
        assert_eq!(Fit::both((1024, 576), (1024, 600), true).1, None);
        assert_eq!(Fit::both((1920, 1080), (1024, 600), true).1, None);
        assert_eq!(Fit::both((1024, 576), (1920, 1080), false).1, None);
    }

    /// Frames sent through buffers that come back in any order: each one, after the copy of
    /// what it lacks and the frame drawn over it, shows what it should
    #[test]
    fn the_buffers_show_the_frames_whatever_their_age() {
        let w = 16usize;
        let rect = |x: u32, len: u32| (x, 0, len, 1);
        let fill = |img: &mut [u32], (x, _, len, _): Rect, v: u32| img[x as usize..][..len as usize].fill(v);
        // what happens, frame by frame: the screen changes, or an animation draws into the buffer
        enum Step {
            Paint(Region, u32),
            Draw(u64, Rect, u32),
        }
        use Step::{Draw, Paint};
        let steps = [
            Paint(Region::All, 1),
            Draw(1, rect(2, 6), 10),
            Draw(1, rect(2, 6), 11),
            Draw(1, rect(2, 8), 12),
            Paint(Region::Rect(rect(2, 8)), 2),
            Paint(Region::Nothing, 0),
            Draw(2, rect(9, 4), 20),
            Draw(2, rect(9, 4), 21),
            Paint(Region::All, 3),
            Draw(3, rect(0, 3), 30),
            Draw(4, rect(12, 4), 40),
            Draw(4, rect(12, 4), 41),
            Paint(Region::Rect(rect(12, 4)), 4),
            Paint(Region::Nothing, 0),
            Paint(Region::Nothing, 0),
        ];
        for order in [[0usize, 1, 0, 1].as_slice(), &[0, 1, 2], &[0, 0, 1], &[2, 1, 0, 0, 1], &[0, 1, 2, 2, 1, 0, 0]] {
            let mut buffers = vec![(vec![99u32; w], 0usize); 3];
            let (mut screen, mut sent) = (vec![0u32; w], VecDeque::new());
            for (n, step) in steps.iter().enumerate() {
                let b = order[n % order.len()];
                for (i, buffer) in buffers.iter_mut().enumerate() {
                    if i != b && buffer.1 > 0 {
                        buffer.1 += 1;
                    }
                }
                let (buf, age) = &mut buffers[b];
                let (changed, drawing) = match *step {
                    Paint(region, v) => {
                        match region {
                            Region::All => screen.fill(v),
                            Region::Rect(r) => fill(&mut screen, r, v),
                            Region::Nothing => {}
                        }
                        (region, None)
                    }
                    Draw(id, ..) => (Region::Nothing, Some(id)),
                };
                match stale(&sent, *age, drawing).union(changed) {
                    Region::All => buf.copy_from_slice(&screen),
                    Region::Rect((x, _, len, _)) => {
                        buf[x as usize..][..len as usize].copy_from_slice(&screen[x as usize..][..len as usize]);
                    }
                    Region::Nothing => {}
                }
                let mut want = screen.clone();
                let direct = match *step {
                    Draw(id, r, v) => {
                        fill(buf, r, v);
                        fill(&mut want, r, v);
                        Some((id, r))
                    }
                    Paint(..) => None,
                };
                assert!(*buf == want, "order {order:?}, step {n}: {buf:?} ≠ {want:?}");
                *age = 1;
                sent.push_front(Sent { screen: changed, direct });
                sent.truncate(4);
            }
        }
        assert_eq!(Region::Rect((0, 0, 2, 2)).union(Region::Nothing), Region::Rect((0, 0, 2, 2)));
        assert_eq!(Region::Rect((0, 0, 2, 2)).union(Region::All), Region::All);
    }

    /// The costs that matter on a slow computer, at 1920 × 1080:
    /// `cargo test --release -p glitu-play -- --ignored --nocapture timings`
    #[test]
    #[ignore]
    fn timings() {
        let mut seed = 1u32;
        let mut noise = |n: usize| {
            (0..n)
                .map(|_| {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    seed >> 8
                })
                .collect::<Vec<u32>>()
        };
        let img = Image { width: 1024, height: 576, pixels: noise(1024 * 576) };
        let fit = Fit::new((1024, 576), (1920, 1080));
        let best = |f: &mut dyn FnMut()| {
            (0..10)
                .map(|_| {
                    let t = Instant::now();
                    f();
                    t.elapsed()
                })
                .min()
                .unwrap_or_default()
        };
        let d = best(&mut || drop(resample(img.clone(), fit.size, fit.scale)));
        eprintln!("resample 1024×576 → 1920×1080: {d:?}");
        let a = resample(img.clone(), fit.size, fit.scale);
        let b = Image { pixels: noise(a.pixels.len()), ..a.clone() };
        let t = Instant::now();
        let mut tr = Transition::new(Arc::new(a.clone()), None, Arc::new(b), &[], &[], 400.0);
        eprintln!("cross-fade, preparation: {:?}", t.elapsed());
        let mut screen = a.pixels.clone();
        let t = Instant::now();
        for i in 0..20 {
            tr.render(f64::from(i) * 15.0, &mut screen, 1920, (0, 0));
        }
        eprintln!("cross-fade, one frame: {:?}", t.elapsed() / 20);
    }
}
