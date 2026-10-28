//! Le moteur de présentation : slides (horizontales et verticales), étapes, mise à l'échelle,
//! adresse, clavier.
//!
//! Même modèle que reveal.js : une slide `present`, les précédentes `past` (étapes toutes
//! visibles), les suivantes `future` (étapes masquées) ; une pile de slides verticales est une
//! `section.stack`. Les `.fragment` d'une slide sont groupés par `data-fragment-index`, puis
//! numérotés à partir de 0 ; l'index écrit par l'auteur reste lisible dans `data-sk-index` :
//! c'est le « clic » que les schémas suivent.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use web_sys::{Element, KeyboardEvent};

use crate::dom::{self, add, data, has, qsa, remove, style, toggle};

/// Réglages du thème (glitu.json), transmis par la page
#[derive(Clone)]
pub struct Config {
    pub width: f64,
    pub height: f64,
    pub margin: f64,
    pub center: bool,
    pub controls: bool,
    pub progress: bool,
    pub slide_number: bool,
    pub transition: String,
    /// Après la dernière slide, on revient à la première
    pub looped: bool,
    /// La molette de la souris fait avancer et reculer
    pub mouse_wheel: bool,
    /// Défilement automatique : délai entre deux étapes, en ms (0 : aucun)
    pub auto_slide: f64,
    /// Bornes de la mise à l'échelle dans la fenêtre (comme reveal.js : 0,2 et 2) ; un thème
    /// vectoriel peut monter plus haut, pour remplir un grand écran
    pub min_scale: f64,
    pub max_scale: f64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            width: 960.0,
            height: 700.0,
            margin: 0.04,
            center: true,
            controls: true,
            progress: true,
            slide_number: false,
            transition: "slide".into(),
            looped: false,
            mouse_wheel: false,
            auto_slide: 0.0,
            min_scale: 0.2,
            max_scale: 2.0,
        }
    }
}

/// Slides voisines gardées affichées de part et d'autre de la slide courante
pub const VIEW_DISTANCE: usize = 3;
/// Largeur de fenêtre (px) sous laquelle la présentation se lit en défilant, comme reveal.js
const SCROLL_BELOW: f64 = 435.0;

pub enum Ev<'a> {
    Ready,
    SlideChanged { previous: Option<&'a Element> },
    FragmentShown(&'a [Element]),
    FragmentHidden(&'a [Element]),
    Resize,
}

type Listener = Rc<dyn Fn(&Ev)>;
type KeyHandler = Rc<dyn Fn()>;
/// Lit une touche avant le moteur ; `true` : la touche est prise (vue d'ensemble, pause…)
type Interceptor = Rc<dyn Fn(&KeyboardEvent) -> bool>;

struct State {
    config: Config,
    /// Slides horizontales (une pile verticale est une seule entrée)
    slides: Vec<Element>,
    /// Par slide horizontale : ses slides verticales (elle-même si ce n'est pas une pile)
    leaves: Vec<Vec<Element>>,
    /// Par slide (h, v) : groupes de fragments dans l'ordre des clics
    groups: Vec<Vec<Vec<Vec<Element>>>>,
    h: usize,
    v: usize,
    /// Groupe courant de la slide courante (-1 : aucun)
    f: i32,
    navigated_h: bool,
    navigated_v: bool,
    listeners: Vec<Listener>,
    keys: Vec<(String, KeyHandler)>,
    interceptors: Vec<Interceptor>,
    /// Transformation ajoutée à la mise à l'échelle des slides (vue d'ensemble)
    extra_transform: String,
    /// `?print-pdf` : toutes les slides l'une sous l'autre, pour l'impression
    /// Les slides en pages, toutes étapes montrées : impression, ou vue défilante
    printing: bool,
    /// … pour être lues en défilant (voir `print`)
    scrolling: bool,
    /// Vue choisie d'après la largeur de la fenêtre (ni `?view=scroll`, ni impression, ni
    /// télécommande) : elle change quand la fenêtre franchit `SCROLL_BELOW`
    by_width: bool,
    /// `?receiver` : aperçu dans la vue de l'orateur, piloté par messages, sans clavier
    receiver: bool,
    view_distance: usize,
    /// Vue d'ensemble : toutes les slides affichées, chacune complète (étapes comprises)
    overview: bool,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(s.borrow_mut().as_mut().expect("moteur non démarré")))
}

pub fn root() -> Element {
    dom::qs(&dom::document().document_element().unwrap(), ".reveal").expect(".reveal")
}

pub fn slides_element() -> Element {
    dom::qs(&root(), ".slides").expect(".slides")
}

pub fn config() -> Config {
    with(|s| s.config.clone())
}

/// Prépare les slides ; les plugins s'abonnent ensuite, puis `ready()`.
pub fn init(config: Config) {
    // Attributs écrits en commentaire d'abord : ils peuvent masquer une slide
    for leaf in qsa(&slides_element(), ":scope > section").iter().flat_map(leaves_of) {
        crate::attributes::apply(&leaf);
    }
    // Slides masquées (`data-visibility="hidden"`) : retirées, comme reveal.js
    for hidden in qsa(&slides_element(), "section[data-visibility=\"hidden\"]") {
        hidden.remove();
    }
    let search = dom::window().location().search().unwrap_or_default();
    let receiver = search.contains("receiver");
    // Vue défilante : demandée (`?view=scroll`), ou d'office sur un écran de téléphone, sauf
    // pour la télécommande
    let by_width = !search.contains("view=scroll") && !receiver && !search.contains("remote") && !search.contains("print-pdf");
    let scrolling = search.contains("view=scroll") || (by_width && narrow());
    let printing = search.contains("print-pdf") || scrolling;
    let slides = qsa(&slides_element(), ":scope > section");
    let leaves: Vec<Vec<Element>> = slides.iter().map(leaves_of).collect();
    for (s, l) in slides.iter().zip(&leaves) {
        if l.first() != Some(s) {
            add(s, "stack");
        }
    }
    let groups = leaves.iter().map(|l| l.iter().map(sort_fragments).collect()).collect();
    let r = root();
    add(&r, &config.transition);
    if config.center {
        add(&r, "center");
    }
    add(&r, "has-horizontal-slides");
    if leaves.iter().any(|l| l.len() > 1) {
        add(&r, "has-vertical-slides");
    }
    let _ = r.set_attribute("data-transition-speed", "default");
    let _ = r.set_attribute("data-background-transition", "fade");
    STATE.with(|s| {
        *s.borrow_mut() = Some(State {
            config,
            slides,
            leaves,
            groups,
            h: 0,
            v: 0,
            f: -1,
            navigated_h: false,
            navigated_v: false,
            listeners: Vec::new(),
            keys: Vec::new(),
            interceptors: Vec::new(),
            extra_transform: String::new(),
            printing,
            scrolling,
            by_width,
            receiver,
            view_distance: VIEW_DISTANCE,
            overview: false,
        });
    });
}

/// Fenêtre de téléphone (en portrait) : la vue défilante convient mieux
fn narrow() -> bool {
    dom::window().inner_width().ok().and_then(|w| w.as_f64()).is_some_and(|w| w < SCROLL_BELOW)
}

/// Slides d'une slide horizontale : ses slides verticales, ou elle-même
fn leaves_of(s: &Element) -> Vec<Element> {
    let inner = qsa(s, ":scope > section");
    if inner.is_empty() { vec![s.clone()] } else { inner }
}

/// Groupes de fragments d'une slide, comme reveal.js : par index écrit, puis ceux sans index
/// dans l'ordre du document ; on renumérote ensuite à partir de 0.
fn sort_fragments(slide: &Element) -> Vec<Vec<Element>> {
    let mut ordered: Vec<(u32, Vec<Element>)> = Vec::new();
    let mut unordered: Vec<Vec<Element>> = Vec::new();
    for f in qsa(slide, ".fragment") {
        match data(&f, "fragment-index").and_then(|i| i.parse::<u32>().ok()) {
            Some(i) => {
                let _ = f.set_attribute("data-sk-index", &i.to_string());
                match ordered.iter_mut().find(|(k, _)| *k == i) {
                    Some((_, g)) => g.push(f),
                    None => ordered.push((i, vec![f])),
                }
            }
            None => unordered.push(vec![f]),
        }
    }
    ordered.sort_by_key(|(k, _)| *k);
    let groups: Vec<Vec<Element>> = ordered.into_iter().map(|(_, g)| g).chain(unordered).collect();
    for (i, g) in groups.iter().enumerate() {
        for f in g {
            let _ = f.set_attribute("data-fragment-index", &i.to_string());
        }
    }
    groups
}

pub fn on(f: impl Fn(&Ev) + 'static) {
    with(|s| s.listeners.push(Rc::new(f)));
}

/// Raccourci clavier d'un plugin (touche telle que `KeyboardEvent.key`, insensible à la casse)
pub fn on_key(key: &str, f: impl Fn() + 'static) {
    with(|s| s.keys.push((key.to_lowercase(), Rc::new(f))));
}

/// Un plugin lit les touches avant le moteur, le dernier inscrit d'abord
pub fn intercept(f: impl Fn(&KeyboardEvent) -> bool + 'static) {
    with(|s| s.interceptors.push(Rc::new(f)));
}

/// Mode impression (`?print-pdf`)
pub fn scrolling() -> bool {
    with(|s| s.scrolling)
}

pub fn printing() -> bool {
    with(|s| s.printing)
}

/// Aperçu de la vue de l'orateur (`?receiver`)
pub fn receiver() -> bool {
    with(|s| s.receiver)
}

/// L'aperçu « Ensuite » de la vue de l'orateur (`?receiver&upcoming`) : il montre l'état d'arrivée
/// de l'étape suivante, sans ses animations
pub fn upcoming() -> bool {
    receiver() && dom::window().location().search().unwrap_or_default().contains("upcoming")
}

/// Transformation ajoutée à la mise à l'échelle des slides ; vide : aucune
pub fn set_extra_transform(t: &str) {
    with(|s| s.extra_transform = t.to_string());
    layout();
}

/// Vue d'ensemble : toutes les slides affichées, quelle que soit leur distance (un chapitre
/// voisin peut être long), chacune dans son état final comme à l'impression ; en sortant, la
/// slide courante retrouve ses étapes
pub fn set_overview(on: bool) {
    with(|s| {
        s.overview = on;
        s.view_distance = if on { usize::MAX } else { VIEW_DISTANCE };
    });
    let (h, v, f) = position();
    show(h, v, Some(f), false);
    // En sortant, la slide choisie est annoncée comme si on y arrivait : schémas, terminal et
    // effets reprennent ses étapes (en vue d'ensemble, ils la montraient complète)
    if !on {
        emit(Ev::SlideChanged { previous: None });
    }
}

/// Rang de la slide courante dans l'ordre de lecture (slides verticales comprises)
pub fn leaf_index() -> usize {
    with(|s| s.leaves[..s.h].iter().map(Vec::len).sum::<usize>() + s.v)
}

/// Slides (h, v) dans l'ordre de lecture
pub fn all_leaves() -> Vec<(usize, usize, Element)> {
    with(|s| {
        s.leaves.iter().enumerate().flat_map(|(h, l)| l.iter().enumerate().map(move |(v, e)| (h, v, e.clone()))).collect()
    })
}


fn emit(ev: Ev) {
    let listeners = with(|s| s.listeners.clone());
    for l in listeners {
        l(&ev);
    }
}

/// Slides horizontales
pub fn slides() -> Vec<Element> {
    with(|s| s.slides.clone())
}

/// Slides verticales de chaque slide horizontale
pub fn leaves() -> Vec<Vec<Element>> {
    with(|s| s.leaves.clone())
}

/// (h, v, f)
pub fn position() -> (usize, usize, i32) {
    with(|s| (s.h, s.v, s.f))
}

pub fn current_slide() -> Element {
    with(|s| s.leaves[s.h][s.v].clone())
}

/// L'auteur a-t-il déjà navigué horizontalement, verticalement ?
pub fn navigated() -> (bool, bool) {
    with(|s| (s.navigated_h, s.navigated_v))
}

/// Clic courant de la slide, tel qu'écrit par l'auteur (0 : aucun)
pub fn current_step(slide: &Element) -> u32 {
    qsa(slide, ".fragment.visible[data-sk-index]")
        .iter()
        .filter_map(|f| data(f, "sk-index")?.parse().ok())
        .max()
        .unwrap_or(0)
}

/// Lance la présentation : position lue dans l'adresse, puis événement `Ready`.
pub fn ready() {
    layout();
    let (h, v, f) = read_hash();
    show(h, v, f, false);
    for c in data(&current_slide(), "state").unwrap_or_default().split_whitespace() {
        add(&dom::document().document_element().unwrap(), c);
    }
    add(&root(), "ready");
    emit(Ev::Ready);
    bind();
}

fn read_hash() -> (usize, usize, Option<i32>) {
    let hash = dom::window().location().hash().unwrap_or_default();
    let parts: Vec<&str> = hash.trim_start_matches('#').trim_start_matches('/').split('/').collect();
    let h = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
    let v = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
    let f = parts.get(2).and_then(|p| p.parse().ok());
    let n = with(|s| s.slides.len());
    (h.min(n.saturating_sub(1)), v, f)
}

fn write_hash() {
    let (h, v, f) = position();
    let hash = if f >= 0 {
        format!("#/{h}/{v}/{f}")
    } else if v > 0 {
        format!("#/{h}/{v}")
    } else if h > 0 {
        format!("#/{h}")
    } else {
        "#/".into()
    };
    let _ = dom::window().history().and_then(|hist| hist.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&hash)));
    // Pour la page : `glitu serve` synchronise ainsi les fenêtres ouvertes ; pas pendant la vue
    // d'ensemble, les autres fenêtres ne suivent que la slide choisie, en sortant
    if with(|s| s.overview) {
        return;
    }
    if let Ok(ev) = web_sys::Event::new("glitu:position") {
        let _ = dom::window().dispatch_event(&ev);
    }
}

fn set_state(el: &Element, state: &str) {
    for c in ["past", "present", "future"] {
        remove(el, c);
    }
    add(el, state);
}

fn state_of(i: usize, current: usize) -> &'static str {
    if i < current {
        "past"
    } else if i > current {
        "future"
    } else {
        "present"
    }
}

/// Distance d'une slide (x, y) à la slide courante, comme reveal.js
pub fn distance(x: usize, y: usize, h: usize, v: usize) -> usize {
    x.abs_diff(h) + if x == h { y.abs_diff(v) } else { y }
}

/// Affiche la slide (h, v), ses étapes jusqu'au groupe `f` (None : selon le sens d'arrivée)
pub fn show(h: usize, v: usize, f: Option<i32>, events: bool) {
    let (previous, slides, leaves, prev, prev_f) = with(|s| (s.leaves[s.h][s.v].clone(), s.slides.clone(), s.leaves.clone(), (s.h, s.v), s.f));
    let view = with(|s| s.view_distance);
    let h = h.min(slides.len().saturating_sub(1));
    let v = v.min(leaves[h].len() - 1);
    for (x, slide) in slides.iter().enumerate() {
        set_state(slide, state_of(x, h));
        let _ = slide.set_attribute("aria-hidden", if x == h { "false" } else { "true" });
        style(slide, "display", if x.abs_diff(h) < view { "block" } else { "none" });
        let stack = leaves[x].len() > 1;
        for (y, leaf) in leaves[x].iter().enumerate() {
            if stack {
                if x == h {
                    set_state(leaf, state_of(y, v));
                }
                style(leaf, "display", if distance(x, y, h, v) < view { "block" } else { "none" });
            }
            // Étapes : toutes visibles avant la slide courante, masquées après
            if (x, y) < (h, v) {
                set_fragments(x, y, i32::MAX);
            } else if (x, y) > (h, v) {
                set_fragments(x, y, -1);
            }
        }
    }
    let count = with(|s| s.groups[h][v].len() as i32);
    let f = match f {
        Some(f) => f.clamp(-1, count - 1),
        None if (h, v) < prev && events => count - 1,
        None => -1,
    };
    with(|s| {
        s.h = h;
        s.v = v;
        s.f = f;
    });
    set_fragments(h, v, f);
    toggle(&root(), "is-vertical-slide", v > 0);
    layout_centers();
    write_hash();
    if (h, v) != prev {
        mark_entry(&leaves, &previous, &leaves[h][v]);
        apply_state(&previous, &leaves[h][v]);
    }
    if events && (h, v) != prev {
        emit(Ev::SlideChanged { previous: Some(&previous) });
    } else if events && f != prev_f {
        // même slide, autre étape (fenêtre synchronisée, adresse) : comme autant de clics, un
        // groupe à la fois (terminal, code animé, schémas)
        let groups = with(|s| s.groups[h][v].clone());
        if f > prev_f {
            ((prev_f + 1)..=f).for_each(|i| emit(Ev::FragmentShown(&groups[i as usize])));
        } else {
            ((f + 1)..=prev_f).rev().for_each(|i| emit(Ev::FragmentHidden(&groups[i as usize])));
        }
    }
}

/// `data-state="a b"` de la slide courante : classes de la page, comme reveal.js
fn apply_state(previous: &Element, current: &Element) {
    let html = dom::document().document_element().unwrap();
    for c in data(previous, "state").unwrap_or_default().split_whitespace() {
        remove(&html, c);
    }
    for c in data(current, "state").unwrap_or_default().split_whitespace() {
        add(&html, c);
    }
}

/// La slide qui arrive garde la mise en page de celle qu'elle remplace (`data-layout` posé par
/// le compilateur) : le thème choisit sa transition selon le couple, en CSS
fn mark_entry(leaves: &[Vec<Element>], previous: &Element, current: &Element) {
    for leaf in leaves.iter().flatten() {
        let _ = leaf.remove_attribute("data-from-layout");
    }
    if let Some(layout) = data(previous, "layout") {
        let _ = current.set_attribute("data-from-layout", &layout);
    }
}

/// Étapes visibles d'une slide : groupes 0..=f ; le groupe f est le « courant »
fn set_fragments(h: usize, v: usize, f: i32) {
    // À l'impression et en vue d'ensemble, chaque slide se montre complète
    let f = if printing() || with(|s| s.overview) { i32::MAX } else { f };
    let groups = with(|s| s.groups[h][v].clone());
    for (i, g) in groups.iter().enumerate() {
        for el in g {
            toggle(el, "visible", (i as i32) <= f);
            toggle(el, "current-fragment", i as i32 == f && f != i32::MAX);
        }
    }
}

/// Fragment suivant de la slide courante ; false s'il n'y en a plus
fn fragment_next() -> bool {
    step_fragments(1)
}

/// Fragment précédent ; false si la slide en est à son début
fn fragment_prev() -> bool {
    step_fragments(-1)
}

/// Avance (`by` = 1) ou recule (-1) d'un groupe de fragments dans la slide courante, et
/// annonce le groupe montré ou masqué ; false au bout
fn step_fragments(by: i32) -> bool {
    let (h, v, f) = position();
    let count = with(|s| s.groups[h][v].len() as i32);
    let target = f + by;
    if target < -1 || target >= count {
        return false;
    }
    // Le groupe qui change : celui qui paraît en avançant, celui qui part en reculant
    let changed = with(|s| s.groups[h][v][f.max(target) as usize].clone());
    with(|s| s.f = target);
    set_fragments(h, v, target);
    write_hash();
    emit(if by > 0 { Ev::FragmentShown(&changed) } else { Ev::FragmentHidden(&changed) });
    true
}

/// Routes disponibles : gauche, droite, haut, bas
pub fn routes() -> (bool, bool, bool, bool) {
    with(|s| (s.h > 0, s.h + 1 < s.slides.len(), s.v > 0, s.v + 1 < s.leaves[s.h].len()))
}

/// Étapes restantes de part et d'autre dans la slide courante : précédente, suivante
pub fn fragment_routes() -> (bool, bool) {
    with(|s| (s.f >= 0, s.f + 1 < s.groups[s.h][s.v].len() as i32))
}

pub fn right() {
    with(|s| s.navigated_h = true);
    if fragment_next() {
        return;
    }
    if routes().1 {
        show(position().0 + 1, 0, None, true);
    } else if config().looped {
        goto(0);
    }
}

pub fn left() {
    with(|s| s.navigated_h = true);
    if !fragment_prev() && routes().0 {
        show(position().0 - 1, 0, None, true);
    }
}

pub fn down() {
    with(|s| s.navigated_v = true);
    if !fragment_next() && routes().3 {
        let (h, v, _) = position();
        show(h, v + 1, None, true);
    }
}

pub fn up() {
    with(|s| s.navigated_v = true);
    if !fragment_prev() && routes().2 {
        let (h, v, _) = position();
        show(h, v - 1, None, true);
    }
}

/// Haut de la colonne suivante (le chapitre suivant en navigation 2D), sans ses étapes
pub fn next_column() {
    with(|s| s.navigated_h = true);
    if routes().1 {
        show(position().0 + 1, 0, None, true);
    }
}

/// Haut de la colonne précédente (le chapitre précédent en navigation 2D)
pub fn previous_column() {
    with(|s| s.navigated_h = true);
    if routes().0 {
        show(position().0 - 1, 0, None, true);
    }
}

/// Étape suivante, slides verticales comprises
pub fn next() {
    if fragment_next() {
        return;
    }
    if routes().3 { down() } else { right() }
}

/// Étape précédente ; en revenant sur une pile, on arrive en bas de la pile
pub fn prev() {
    if fragment_prev() {
        return;
    }
    let (left_ok, _, up_ok, _) = routes();
    if up_ok {
        up();
    } else if left_ok {
        with(|s| s.navigated_h = true);
        let h = position().0 - 1;
        let bottom = with(|s| s.leaves[h].len() - 1);
        show(h, bottom, None, true);
    }
}

pub fn goto(h: usize) {
    show(h, 0, None, true);
}

/// Dernière étape de la dernière slide ?
pub fn at_end() -> bool {
    let (_, right, _, down) = routes();
    !right && !down && !fragment_routes().1
}

/// Mise à l'échelle dans la fenêtre, centrée (marge comprise)
pub fn layout() {
    let c = config();
    let reveal = dom::html(&root());
    let (w, h) = (reveal.offset_width() as f64, reveal.offset_height() as f64);
    let slides = slides_element();
    style(&slides, "width", &format!("{}px", c.width));
    style(&slides, "height", &format!("{}px", c.height));
    let (pw, ph) = (w - w * c.margin, h - h * c.margin);
    let scale = (pw / c.width).min(ph / c.height).clamp(c.min_scale, c.max_scale.max(c.min_scale));
    let extra = with(|s| s.extra_transform.clone());
    if printing() {
        return;
    }
    if extra.is_empty() && ((scale - 1.0).abs() < f64::EPSILON || w == 0.0) {
        for p in ["left", "top", "bottom", "right", "transform"] {
            dom::unstyle(&slides, p);
        }
    } else {
        style(&slides, "left", "50%");
        style(&slides, "top", "50%");
        style(&slides, "bottom", "auto");
        style(&slides, "right", "auto");
        style(&slides, "transform", format!("translate(-50%, -50%) scale({scale}) {extra}").trim_end());
    }
    if let Some(body) = dom::document().body() {
        let b: Element = body.into();
        style(&b, "--slide-width", &format!("{}px", c.width));
        style(&b, "--slide-height", &format!("{}px", c.height));
        style(&b, "--slide-scale", &scale.to_string());
        style(&b, "--viewport-width", &format!("{w}px"));
        style(&b, "--viewport-height", &format!("{h}px"));
    }
    layout_centers();
}

/// Slides centrées verticalement (les piles, elles, restent en haut)
fn layout_centers() {
    let c = config();
    for s in qsa(&slides_element(), "section") {
        if !c.center {
            dom::unstyle(&s, "top");
        } else if has(&s, "stack") {
            style(&s, "top", "0px");
        } else {
            let sh = dom::html(&s).scroll_height() as f64;
            style(&s, "top", &format!("{}px", ((c.height - sh) / 2.0).max(0.0)));
        }
    }
}

fn bind() {
    bind_window();
    if receiver() {
        receive();
        return;
    }
    bind_keys();
    // La vue défilante se lit en faisant défiler la page : le doigt et la molette lui restent
    if scrolling() {
        return;
    }
    if config().mouse_wheel {
        bind_wheel();
    }
    bind_touch();
}

/// Taille de la fenêtre, polices chargées, adresse changée à la main
fn bind_window() {
    let win = dom::window();
    dom::on::<web_sys::Event>(&win, "resize", |_| {
        // Un téléphone qu'on tourne passe d'une vue à l'autre ; la vue défilante range les
        // slides en pages : on recharge, la position est dans l'adresse
        if with(|s| s.by_width && s.scrolling != narrow()) {
            let _ = dom::window().location().reload();
            return;
        }
        layout();
        emit(Ev::Resize);
    });
    // Les polices changent la hauteur des slides : on recentre une fois tout chargé
    dom::on::<web_sys::Event>(&win, "load", |_| layout());
    if let Ok(ready) = dom::document().fonts().ready() {
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut(wasm_bindgen::JsValue)>::new(|_| layout());
        let _ = ready.then(&cb);
        cb.forget();
    }
    dom::on::<web_sys::Event>(&win, "hashchange", |_| {
        let (h, v, f) = read_hash();
        if (h, v, f.unwrap_or(-1)) != position() {
            show(h, v, f, true);
        }
    });
}

/// Clavier : plugins d'abord (interceptions, raccourcis), puis la navigation
fn bind_keys() {
    dom::on::<KeyboardEvent>(&dom::document(), "keydown", |e| {
        if e.alt_key() || e.ctrl_key() || e.meta_key() {
            return;
        }
        if let Some(t) = e.target().and_then(|t| t.dyn_into::<Element>().ok())
            && (matches!(t.tag_name().as_str(), "INPUT" | "TEXTAREA") || t.has_attribute("contenteditable"))
        {
            return;
        }
        let interceptors = with(|s| s.interceptors.clone());
        if interceptors.iter().rev().any(|f| f(&e)) {
            e.prevent_default();
            return;
        }
        let key = e.key();
        // Vue défilante : les flèches, Espace et les pages font défiler la page
        if scrolling() && matches!(key.as_str(), " " | "PageDown" | "PageUp" | "ArrowRight" | "ArrowLeft" | "ArrowDown" | "ArrowUp" | "Home" | "End") {
            return;
        }
        match key.as_str() {
            // les flèches suivent l'ordre de lecture ; Page↓ Page↑ sautent d'une colonne à l'autre
            // (un chapitre en navigation 2D) ; H J K L : gauche, bas, haut, droite
            " " if e.shift_key() => prev(),
            "ArrowRight" | " " | "n" | "N" => next(),
            "ArrowLeft" | "p" | "P" => prev(),
            "PageDown" => next_column(),
            "PageUp" => previous_column(),
            "l" | "L" => right(),
            "h" | "H" => left(),
            "ArrowDown" | "j" | "J" => down(),
            "ArrowUp" | "k" | "K" => up(),
            "Home" => goto(0),
            "End" => goto(with(|s| s.slides.len()) - 1),
            "f" | "F" => fullscreen(),
            _ => {
                let handlers: Vec<KeyHandler> =
                    with(|s| s.keys.iter().filter(|(k, _)| *k == key.to_lowercase()).map(|(_, f)| f.clone()).collect());
                if handlers.is_empty() {
                    return;
                }
                for f in handlers {
                    f();
                }
            }
        }
        e.prevent_default();
    });
}

/// La molette avance et recule (`"mouseWheel": true`)
fn bind_wheel() {
    let last = Rc::new(RefCell::new(0.0));
    dom::on::<web_sys::WheelEvent>(&dom::document(), "wheel", move |e| {
        // Une molette envoie une rafale d'événements : un pas toutes les 600 ms au plus
        let now = dom::now();
        // La vue d'ensemble a sa propre lecture de la molette
        if has(&root(), "overview") || now - *last.borrow() < 600.0 || e.delta_y() == 0.0 {
            return;
        }
        *last.borrow_mut() = now;
        if e.delta_y() > 0.0 { next() } else { prev() }
    });
}

/// Glisser d'un doigt : gauche / droite, haut / bas. À deux doigts, le geste reste au
/// navigateur (`touch-action: pinch-zoom` sur `.reveal`) : zoomer, puis déplacer la vue
fn bind_touch() {
    let start = Rc::new(RefCell::new(None::<(i32, i32)>));
    let s2 = Rc::clone(&start);
    dom::on::<web_sys::TouchEvent>(&dom::document(), "touchstart", move |e| {
        // En mode dessin, le doigt dessine (voir `draw`)
        if has(&root(), "drawing") {
            return;
        }
        // Un deuxième doigt annule le glissement en cours, jusqu'au prochain geste
        *s2.borrow_mut() = match (e.touches().length(), e.touches().get(0)) {
            (1, Some(t)) => Some((t.client_x(), t.client_y())),
            _ => None,
        };
    });
    // Un seul doigt ne déplace pas la vue : touch-action (glitu.css) ne suffit pas partout
    // (Firefox sur Android, au-dessus des schémas)
    dom::on_active::<web_sys::TouchEvent>(&dom::document(), "touchmove", |e| {
        if e.touches().length() == 1 && !has(&root(), "drawing") {
            e.prevent_default();
        }
    });
    dom::on::<web_sys::TouchEvent>(&dom::document(), "touchend", move |e| {
        let Some((x0, y0)) = start.borrow_mut().take() else { return };
        let Some(t) = e.changed_touches().get(0) else { return };
        let (dx, dy) = (t.client_x() - x0, t.client_y() - y0);
        if dx.abs() > 40 && dx.abs() > dy.abs() {
            if dx < 0 { next() } else { prev() }
        } else if dy.abs() > 40 {
            if dy < 0 { down() } else { up() }
        }
    });
}

/// Aperçu de la vue de l'orateur : il va où on lui dit (`{glitu: "show", h, v, f, ahead}`,
/// `ahead` : une étape plus loin), et annonce qu'il est prêt à sa fenêtre
fn receive() {
    let win = dom::window();
    dom::on::<web_sys::MessageEvent>(&win, "message", |e| {
        let msg = e.data();
        let get = |k: &str| js_sys::Reflect::get(&msg, &k.into()).ok();
        if get("glitu").and_then(|v| v.as_string()).as_deref() != Some("show") {
            return;
        }
        let num = |k: &str| get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
        show(num("h") as usize, num("v") as usize, Some(num("f") as i32), true);
        if get("ahead").and_then(|v| v.as_bool()) == Some(true) {
            next();
        }
    });
    if let Ok(Some(parent)) = win.parent() {
        let ready = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&ready, &"glitu".into(), &"ready".into());
        let _ = parent.post_message(&ready, "*");
    }
}

fn fullscreen() {
    let doc = dom::document();
    if doc.fullscreen_element().is_some() {
        doc.exit_fullscreen();
    } else if let Some(el) = doc.body() {
        let _ = el.request_fullscreen();
    }
}

/// Nombre de groupes d'étapes d'une slide, et combien sont affichés
pub fn fragment_counts(slide: &Element) -> (usize, usize) {
    let groups = with(|s| {
        s.leaves
            .iter()
            .enumerate()
            .find_map(|(h, l)| l.iter().position(|x| x == slide).map(|v| s.groups[h][v].clone()))
            .unwrap_or_default()
    });
    let shown = groups.iter().filter(|g| g.first().is_some_and(|f| has(f, "visible"))).count();
    (groups.len(), shown)
}
