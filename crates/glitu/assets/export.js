// Relevé de l'écran pour l'export PowerPoint modifiable : chaque élément visible devient un
// objet (forme, texte, chemin, image) en pixels de la fenêtre, dans l'ordre où il est peint.
// Injecté par `glitu export` ; `__gx.collect()` relève l'état affiché, `__gx.isolate(ids)`
// ne laisse peindre que certains éléments (backdrops, images, ce qui ne se convertit pas) pour
// les capturer, `__gx.clear()` rend la page telle qu'elle était.
window.__gx = (() => {
  const W = innerWidth, H = innerHeight;
  const DEBUG = !!window.__gxDebug;
  const round = (v) => Math.round(v * 100) / 100;
  const cs = (el, p) => getComputedStyle(el, p);
  const sheet = document.createElement('style');
  sheet.textContent = '.gx-nop::before,.gx-nop::after{content:none!important}';
  let isoSheet = null;

  // ------------------------------------------------------------ couleurs
  const probe = document.createElement('canvas');
  probe.width = probe.height = 1;
  const pctx = probe.getContext('2d', { willReadFrequently: true });
  const colors = new Map();
  function color(s) {
    if (!s || s === 'none' || s === 'transparent' || s.startsWith('url')) return null;
    if (colors.has(s)) return colors.get(s);
    let c = null;
    const m = /^rgba?\(([^)]*)\)$/.exec(s);
    if (m) {
      const p = m[1].split(/[\s,/]+/).filter(Boolean).map(parseFloat);
      c = [p[0], p[1], p[2], p.length > 3 ? p[3] : 1];
    } else if (CSS.supports('color', s)) {
      pctx.clearRect(0, 0, 1, 1);
      pctx.fillStyle = s;
      pctx.fillRect(0, 0, 1, 1);
      const d = pctx.getImageData(0, 0, 1, 1).data;
      c = [d[0], d[1], d[2], d[3] / 255];
    }
    if (c && c[3] <= 0.004) c = null;
    colors.set(s, c);
    return c;
  }
  const fade = (c, o) => (c ? [c[0], c[1], c[2], c[3] * o] : null);

  // ------------------------------------------------------------ opacité, découpe, échelle
  const opacities = new Map();
  function opacity(el) {
    if (!el || el.nodeType !== 1) return 1;
    if (opacities.has(el)) return opacities.get(el);
    const o = parseFloat(cs(el).opacity) * opacity(el.parentElement);
    opacities.set(el, o);
    return o;
  }
  const clips = new Map();
  const VIEW = { l: 0, t: 0, r: W, b: H };
  function clipOf(el) {
    const parent = el.parentElement;
    if (!parent) return VIEW;
    if (clips.has(el)) return clips.get(el);
    let c = clipOf(parent);
    const s = cs(parent);
    if ((s.overflowX !== 'visible' || s.overflowY !== 'visible') && cs(el).position !== 'fixed') {
      const r = parent.getBoundingClientRect();
      c = { l: Math.max(c.l, r.left), t: Math.max(c.t, r.top), r: Math.min(c.r, r.right), b: Math.min(c.b, r.bottom) };
    }
    if (cs(el).position === 'fixed') c = VIEW;
    clips.set(el, c);
    return c;
  }
  const inside = (c, x, y) => x >= c.l - 0.5 && x <= c.r + 0.5 && y >= c.t - 0.5 && y <= c.b + 0.5;
  const meets = (c, r) => r.right > c.l && r.left < c.r && r.bottom > c.t && r.top < c.b;
  const within = (c, r) => r.left >= c.l - 1 && r.right <= c.r + 1 && r.top >= c.t - 1 && r.bottom <= c.b + 1;
  // Partie visible d'un élément coupé par un ancêtre (overflow) : capturée en image
  const cut = (el, c, r) => ({ t: 'pic', iso: mark(el, 'all'), x: Math.max(r.left, c.l), y: Math.max(r.top, c.t), w: Math.min(r.right, c.r) - Math.max(r.left, c.l), h: Math.min(r.bottom, c.b) - Math.max(r.top, c.t) });
  // Pixels de la fenêtre par pixel CSS de l'élément (transformations de mise à l'échelle)
  const scales = new Map();
  function scale(el) {
    if (!el || el.nodeType !== 1) return 1;
    if (scales.has(el)) return scales.get(el);
    let k = el.offsetWidth > 0 ? el.getBoundingClientRect().width / el.offsetWidth : scale(el.parentElement);
    if (!isFinite(k) || k <= 0) k = 1;
    scales.set(el, k);
    return k;
  }

  // Ordre de peinture approché : z-index des contextes d'empilement, puis ordre du document
  const zs = new Map();
  function zkey(el) {
    if (!el || el.nodeType !== 1) return [];
    if (zs.has(el)) return zs.get(el);
    const s = cs(el);
    const k = zkey(el.parentElement).slice();
    if (s.zIndex !== 'auto' && (s.position !== 'static' || el.parentElement && /flex|grid/.test(cs(el.parentElement).display))) k.push(parseInt(s.zIndex));
    else if (s.position !== 'static') k.push(0.5);
    zs.set(el, k);
    return k;
  }

  // ------------------------------------------------------------ pseudo-éléments
  // ::before / ::after deviennent de vrais éléments, au même style : le relevé les traite alors
  // comme les autres (puces carrées, numéros de ligne…)
  const made = [];
  function content(value, el) {
    let out = '';
    const re = /"((?:[^"\\]|\\.)*)"|'((?:[^'\\]|\\.)*)'|attr\(\s*([\w-]+)\s*\)/g;
    let m;
    while ((m = re.exec(value))) {
      if (m[3]) out += el.getAttribute(m[3]) || '';
      else out += (m[1] ?? m[2]).replace(/\\([0-9a-fA-F]{1,6})\s?/g, (_, h) => String.fromCodePoint(parseInt(h, 16))).replace(/\\(.)/g, '$1');
    }
    return out;
  }
  function materialize(root) {
    const els = [...root.querySelectorAll('*')].filter((el) => !(el instanceof SVGElement) && !el.closest('aside.notes'));
    const todo = [];
    for (const el of els) {
      for (const p of ['::before', '::after']) {
        const s = cs(el, p);
        if (!s.content || s.content === 'none' || s.content === 'normal' || s.display === 'none') continue;
        const copy = {};
        for (const prop of s) copy[prop] = s.getPropertyValue(prop);
        todo.push([el, p, copy, content(s.content, el)]);
      }
    }
    for (const [el, p, copy, text] of todo) {
      const span = document.createElement('gx-pseudo');
      for (const prop in copy) {
        if (prop === 'content' || prop.startsWith('transition') || prop.startsWith('animation')) continue;
        span.style.setProperty(prop, copy[prop]);
      }
      span.textContent = text;
      if (p === '::before') el.prepend(span); else el.append(span);
      el.classList.add('gx-nop');
      made.push(span);
    }
  }

  // ------------------------------------------------------------ backdrops et bordures
  // Découpe aux virgules de premier niveau (couches d'un backdrop, ombres, arrêts d'un dégradé)
  function splitTop(v) {
    const out = [];
    let depth = 0, cur = '';
    for (const ch of v) {
      if (ch === '(') depth++;
      else if (ch === ')') depth--;
      if (ch === ',' && depth === 0) { out.push(cur.trim()); cur = '' } else cur += ch;
    }
    if (cur.trim()) out.push(cur.trim());
    return out;
  }
  function gradient(img) {
    const m = /^(repeating-)?(linear|radial)-gradient\((.*)\)$/.exec(img.trim());
    if (!m || m[1]) return null;
    const parts = splitTop(img.slice(img.indexOf('(') + 1, -1));
    let angle = 180;
    if (m[2] === 'linear') {
      const a = /^(-?[\d.]+)deg$/.exec(parts[0]);
      const sides = { 'to top': 0, 'to right': 90, 'to bottom': 180, 'to left': 270, 'to top right': 45, 'to right top': 45, 'to bottom right': 135, 'to right bottom': 135, 'to bottom left': 225, 'to left bottom': 225, 'to top left': 315, 'to left top': 315 };
      if (a) { angle = parseFloat(a[1]); parts.shift() } else if (parts[0] in sides) { angle = sides[parts.shift()] }
    } else if (!/^(rgb|#|[a-z]+$)/.test(parts[0]) || /circle|ellipse|at /.test(parts[0])) {
      parts.shift();
    }
    const stops = [];
    for (const p of parts) {
      const mm = /^(.*?\))\s*([\d.]+%)?(?:\s+([\d.]+%))?$/.exec(p) || /^(\S+)\s*([\d.]+%)?$/.exec(p);
      if (!mm) continue;
      const c = color(mm[1]) || [0, 0, 0, 0];
      stops.push([mm[2] ? parseFloat(mm[2]) / 100 : null, c]);
      if (mm[3]) stops.push([parseFloat(mm[3]) / 100, c]);
    }
    if (stops.length < 2) return null;
    if (stops[0][0] === null) stops[0][0] = 0;
    if (stops[stops.length - 1][0] === null) stops[stops.length - 1][0] = 1;
    for (let i = 1; i < stops.length; i++) {
      if (stops[i][0] !== null) continue;
      let j = i;
      while (stops[j][0] === null) j++;
      for (let k = i; k < j; k++) stops[k][0] = stops[i - 1][0] + (stops[j][0] - stops[i - 1][0]) * (k - i + 1) / (j - i + 1);
    }
    return { angle, radial: m[2] === 'radial', stops };
  }
  // Ombres du `box-shadow` / `drop-shadow()` qui ne sont pas intérieures
  function shadows(s) {
    if (!s || s === 'none') return [];
    const out = [];
    for (const part of splitTop(s)) {
      if (/inset/.test(part)) continue;
      const c = color((/rgba?\([^)]*\)|#\w+/.exec(part) || [])[0]);
      const nums = part.replace(/rgba?\([^)]*\)/, '').match(/-?[\d.]+px/g) || [];
      const [dx, dy, blur, spread] = nums.map(parseFloat);
      if (c) out.push({ dx: dx || 0, dy: dy || 0, blur: blur || 0, spread: spread || 0, c });
    }
    return out;
  }
  const shadow = (s) => shadows(s).find((x) => x.blur > 0) || null;
  function boxes(el, s, r, o, k, out) {
    const bg = fade(color(s.backgroundColor), o);
    let fill = bg ? { c: bg } : null;
    const images = s.backgroundImage !== 'none' ? splitTop(s.backgroundImage) : [];
    let raster = false, best = -1;
    // Plusieurs couches : la plus couvrante (PowerPoint n'en peint qu'une)
    for (const img of images) {
      if (/^url/.test(img)) { raster = true; continue }
      const g = gradient(img);
      if (!g) continue;
      const cover = g.stops.reduce((a, [, c]) => a + c[3], 0) / g.stops.length;
      if (cover > best) { best = cover; g.stops = g.stops.map(([p, c]) => [p, fade(c, o)]); fill = { grad: g } }
    }
    if (s.backgroundClip === 'text' || s.webkitBackgroundClip === 'text') fill = null, raster = false;
    const c = clipOf(el);
    if (!within(c, r)) {
      r = { left: Math.max(r.left, c.l), top: Math.max(r.top, c.t), right: Math.min(r.right, c.r), bottom: Math.min(r.bottom, c.b) };
      r.width = r.right - r.left; r.height = r.bottom - r.top;
    }
    if (raster) out.push({ t: 'pic', iso: mark(el, 'own'), x: r.left, y: r.top, w: r.width, h: r.height });
    const sides = ['Top', 'Right', 'Bottom', 'Left'].map((n) => ({
      w: s['border' + n + 'Style'] === 'none' || s['border' + n + 'Style'] === 'hidden' ? 0 : parseFloat(s['border' + n + 'Width']) || 0,
      c: fade(color(s['border' + n + 'Color']), o),
      dash: /dash|dot/.test(s['border' + n + 'Style']),
    }));
    const radius = ['TopLeft', 'TopRight', 'BottomRight', 'BottomLeft'].map((n) => Math.min(parseFloat(s['border' + n + 'Radius']) || 0, r.width / 2, r.height / 2) * k);
    const uniform = sides.every((d) => d.w === sides[0].w && d.w > 0 && d.c && sides[0].c && d.c.join() === sides[0].c.join());
    const sh = shadow(s.boxShadow);
    if (sh) sh.c = fade(sh.c, o);
    // Ombres sans flou : des copies de la forme, décalées (pastilles, soulignés)
    for (const d of shadows(s.boxShadow).filter((x) => x.blur === 0).reverse()) {
      out.push({ t: 'box', x: r.left + d.dx * k - d.spread * k, y: r.top + d.dy * k - d.spread * k, w: r.width + 2 * d.spread * k, h: r.height + 2 * d.spread * k, fill: { c: fade(d.c, o) }, r: radius });
    }
    const line = uniform ? { w: sides[0].w * k, c: sides[0].c, dash: sides[0].dash } : null;
    if (raster) fill = null;
    if (fill || line) {
      out.push({ t: 'box', x: r.left, y: r.top, w: r.width, h: r.height, fill, line, r: radius, shadow: sh });
    }
    if (!uniform) {
      const [t, ri, b, l] = sides;
      const side = (d, x, y, w, h) => d.w > 0 && d.c && out.push({ t: 'box', x, y, w, h, fill: { c: d.c }, r: [0, 0, 0, 0] });
      side(t, r.left, r.top, r.width, t.w * k);
      side(b, r.left, r.bottom - b.w * k, r.width, b.w * k);
      side(l, r.left, r.top, l.w * k, r.height);
      side(ri, r.right - ri.w * k, r.top, ri.w * k, r.height);
    }
  }

  // ------------------------------------------------------------ texte
  const INLINE = new Set(['inline', 'inline-block', 'contents']);
  const REPLACED = new Set(['IMG', 'SVG', 'svg', 'CANVAS', 'VIDEO', 'IFRAME', 'MATH', 'math', 'OBJECT', 'EMBED', 'BR', 'INPUT', 'TEXTAREA', 'SELECT', 'AUDIO']);
  const isInline = (el) => !REPLACED.has(el.tagName) && !(el instanceof SVGElement) && INLINE.has(cs(el).display);
  function family(f) {
    const first = f.split(',')[0].trim().replace(/^["']|["']$/g, '');
    return { monospace: 'Consolas', 'sans-serif': 'Arial', serif: 'Times New Roman', 'system-ui': 'Segoe UI' }[first] || first;
  }
  function runStyle(el) {
    const s = cs(el);
    const k = scale(el);
    // -webkit-text-fill-color vaut la couleur du texte, sauf texte évidé (contour, dégradé)
    let c = color(s.webkitTextFillColor), grad = null;
    if (s.backgroundClip === 'text' || s.webkitBackgroundClip === 'text') {
      const g = s.backgroundImage !== 'none' && gradient(splitTop(s.backgroundImage)[0]);
      if (g) { c = g.stops[0][1]; grad = g }
    }
    let hl = null;
    for (let e = el; e && isInline(e); e = e.parentElement) {
      const b = color(cs(e).backgroundColor);
      if (b) { hl = b; break }
    }
    const deco = s.textDecorationLine;
    const a = el.closest('a[href]');
    return {
      font: family(s.fontFamily),
      size: round(parseFloat(s.fontSize) * k),
      b: parseInt(s.fontWeight) >= 600,
      i: s.fontStyle !== 'normal',
      u: /underline/.test(deco),
      s: /line-through/.test(deco),
      cap: s.textTransform === 'uppercase' ? 'all' : s.fontVariantCaps === 'small-caps' ? 'small' : null,
      spc: s.letterSpacing === 'normal' ? 0 : round(parseFloat(s.letterSpacing) * k),
      color: fade(c, opacity(el)),
      grad: grad && { ...grad, stops: grad.stops.map(([p, x]) => [p, fade(x, opacity(el))]) },
      // Texte en contour (`-webkit-text-stroke`)
      stroke: parseFloat(s.webkitTextStrokeWidth) > 0 && color(s.webkitTextStrokeColor)
        ? { w: parseFloat(s.webkitTextStrokeWidth) * k, c: fade(color(s.webkitTextStrokeColor), opacity(el)) } : null,
      hl: hl && hl[3] > 0.15 ? hl : null,
      href: a && /^https?:|^mailto:/.test(a.getAttribute('href')) ? a.href : null,
      mono: /mono|courier|consolas/i.test(s.fontFamily),
    };
  }
  const preformatted = (s) => /^pre/.test(s.whiteSpace) || s.whiteSpace === 'break-spaces';

  // Mots d'un nœud texte, chacun mesuré à l'écran ; en `pre`, des lignes et leurs sauts
  function words(node, pre, clip, out) {
    const el = node.parentElement;
    if (cs(el).visibility !== 'visible' || opacity(el) < 0.02) return;
    const st = runStyle(el);
    // Mots, coupés aussi après un trait d'union : le navigateur peut y passer à la ligne
    const re = pre ? /\n|[^\n]+/g : /\s+|[^\s-]+-*|-+/g;
    let m;
    while ((m = re.exec(node.data))) {
      const tok = m[0];
      if (pre && tok === '\n') { out.push({ br: true }); continue }
      if (!pre && /^\s+$/.test(tok)) { out.push({ sp: true }); continue }
      const range = document.createRange();
      range.setStart(node, m.index);
      range.setEnd(node, m.index + tok.length);
      const rects = [...range.getClientRects()].filter((r) => r.width > 0 || r.height > 0);
      if (!rects.length) continue;
      const r = rects[0];
      if (!inside(clip, r.left + r.width / 2, r.top + r.height / 2)) continue;
      out.push({ text: tok, st, top: r.top, bottom: r.bottom, left: r.left, right: rects[rects.length - 1].right });
    }
  }

  // Mots du texte d'un bloc (`out`) et éléments qui se relèvent à part (`escaped`) : blocs,
  // images, SVG… dans le flux du texte
  function collect(owner, clip, out, escaped) {
    const visit = (node, pre) => {
      for (const n of node.childNodes) {
        if (n.nodeType === 3) { words(n, pre || preformatted(cs(n.parentElement)), clip, out); continue }
        if (n.nodeType !== 1 || cs(n).display === 'none') continue;
        if (n.tagName === 'BR') out.push({ br: true });
        else if (isInline(n)) visit(n, pre || preformatted(cs(n)));
        else escaped.push(n);
      }
    };
    visit(owner, preformatted(cs(owner)));
  }

  // Mots → paragraphes (sauts forcés) faits de morceaux de même style ; les lignes, repérées
  // par la position des mots, gardent les runs à la ligne du navigateur
  function paragraphs(toks) {
    const paras = [];
    let para = { runs: [], lines: [] }, space = false, line = null;
    for (const t of toks) {
      if (t.br) { paras.push(para); para = { runs: [], lines: [] }; line = null; space = false; continue }
      if (t.sp) { space = true; continue }
      const mid = (t.top + t.bottom) / 2;
      const newLine = !line || mid > line.bottom + 1 || mid < line.top - 1;
      if (newLine) {
        line = { top: t.top, bottom: t.bottom, left: t.left, right: t.right };
        para.lines.push(line);
      } else {
        line.top = Math.min(line.top, t.top); line.bottom = Math.max(line.bottom, t.bottom);
        line.left = Math.min(line.left, t.left); line.right = Math.max(line.right, t.right);
      }
      // Deux éléments en ligne sans espace entre eux mais écartés (marge) : une espace
      const gapped = !newLine && para.runs.length && t.left - line.prevRight > t.st.size * 0.2;
      // Passage à la ligne du navigateur : gardé tel quel, PowerPoint ne coupe pas au même endroit
      if (newLine && para.lines.length > 1) { para.runs.push({ br: true }); space = false }
      const last = para.runs[para.runs.length - 1];
      const text = ((space || gapped) && last && !last.br ? ' ' : '') + t.text;
      line.prevRight = t.right;
      if (last && !last.br && JSON.stringify(last.st) === JSON.stringify(t.st)) last.text += text;
      else para.runs.push({ text, st: t.st });
      space = false;
    }
    paras.push(para);
    return paras;
  }

  // Cadre horizontal de la zone de texte : la boîte du bloc (le navigateur y a coupé les
  // lignes), ou juste le texte s'il ne passe pas à la ligne
  function textFrame(owner, s, lines, nowrap, algn) {
    const k = scale(owner);
    const r = owner.getBoundingClientRect();
    const pad = (n) => (parseFloat(s['padding' + n]) + parseFloat(s['border' + n + 'Width'])) * k;
    const box = { l: r.left + pad('Left'), r: r.right - pad('Right') };
    const L = Math.min(...lines.map((l) => l.left)), R = Math.max(...lines.map((l) => l.right));
    if (!nowrap) return { x: Math.min(box.l, L), w: Math.max(box.r, R) - Math.min(box.l, L) };
    if (algn === 'l' || box.r - box.l < R - L) return { x: L, w: R - L };
    return { x: box.l, w: box.r - box.l };
  }

  // Puce native (`list-style`) d'un élément de liste
  function bulletOf(owner, s, fontSize) {
    if (s.display !== 'list-item' || s.listStyleType === 'none') return null;
    const marker = cs(owner, '::marker');
    return { type: s.listStyleType, color: color(marker.color), size: parseFloat(marker.fontSize) * scale(owner), indent: fontSize * 1.1 };
  }

  function textItem(owner, s, clip, out, escaped) {
    const toks = [];
    collect(owner, clip, toks, escaped);
    if (!toks.some((t) => t.text)) return;
    const k = scale(owner);
    const fs = parseFloat(s.fontSize) * k;
    const lh = s.lineHeight === 'normal' ? fs * 1.2 : parseFloat(s.lineHeight) * k;
    const paras = paragraphs(toks);
    const lines = paras.flatMap((p) => p.lines);
    if (!lines.length) return;
    const nowrap = /nowrap|^pre$/.test(s.whiteSpace) || lines.length === paras.length;
    const algn = { center: 'ctr', right: 'r', end: 'r', justify: 'just' }[s.textAlign] || 'l';
    const { x, w } = textFrame(owner, s, lines, nowrap, algn);
    const centers = lines.map((l) => (l.top + l.bottom) / 2);
    const top = Math.min(...centers) - lh / 2, bottom = Math.max(...centers) + lh / 2;
    const kept = paras.filter((p, i) => p.runs.length || (i > 0 && i < paras.length - 1));
    out.push({
      t: 'text', x: round(x), y: round(top), w: round(w), h: round(bottom - top), wrap: !nowrap, algn, lh: round(lh),
      bullet: bulletOf(owner, s, fs),
      paras: kept.map((p) => p.runs.map((ru) => (ru.br ? { br: true, size: p.runs[0].st.size } : { text: ru.text, ...ru.st }))),
    });
  }

  // ------------------------------------------------------------ SVG
  function pathData(d) {
    const toks = d.match(/[a-zA-Z]|-?(?:\d*\.\d+|\d+\.?)(?:e[-+]?\d+)?/g) || [];
    const out = [];
    let i = 0, cmd = '', x = 0, y = 0, sx = 0, sy = 0, cx = 0, cy = 0, qx = 0, qy = 0, prev = '';
    const num = () => parseFloat(toks[i++]);
    const flag = () => {
      // drapeaux d'arc collés (« 0 01 ») : un chiffre à la fois
      const t = toks[i];
      if (t.length > 1 && (t[0] === '0' || t[0] === '1') && !t.includes('.')) { toks[i] = t.slice(1); return +t[0] }
      i++; return +t;
    };
    while (i < toks.length) {
      if (/[a-zA-Z]/.test(toks[i])) cmd = toks[i++];
      const rel = cmd === cmd.toLowerCase(), C = cmd.toUpperCase();
      const ox = rel ? x : 0, oy = rel ? y : 0;
      if (C === 'Z') { out.push(['Z']); x = sx; y = sy; prev = 'Z'; continue }
      if (C === 'M') { x = ox + num(); y = oy + num(); sx = x; sy = y; out.push(['M', x, y]); cmd = rel ? 'l' : 'L' }
      else if (C === 'L') { x = ox + num(); y = oy + num(); out.push(['L', x, y]) }
      else if (C === 'H') { x = ox + num(); out.push(['L', x, y]) }
      else if (C === 'V') { y = (rel ? y : 0) + num(); out.push(['L', x, y]) }
      else if (C === 'C' || C === 'S') {
        let x1, y1;
        if (C === 'C') { x1 = ox + num(); y1 = oy + num() } else { x1 = /[CS]/.test(prev) ? 2 * x - cx : x; y1 = /[CS]/.test(prev) ? 2 * y - cy : y }
        const x2 = ox + num(), y2 = oy + num(), ex = ox + num(), ey = oy + num();
        out.push(['C', x1, y1, x2, y2, ex, ey]); cx = x2; cy = y2; x = ex; y = ey;
      } else if (C === 'Q' || C === 'T') {
        let x1, y1;
        if (C === 'Q') { x1 = ox + num(); y1 = oy + num() } else { x1 = /[QT]/.test(prev) ? 2 * x - qx : x; y1 = /[QT]/.test(prev) ? 2 * y - qy : y }
        const ex = ox + num(), ey = oy + num();
        out.push(['C', x + 2 / 3 * (x1 - x), y + 2 / 3 * (y1 - y), ex + 2 / 3 * (x1 - ex), ey + 2 / 3 * (y1 - ey), ex, ey]);
        qx = x1; qy = y1; x = ex; y = ey;
      } else if (C === 'A') {
        const rx = num(), ry = num(), rot = num(), large = flag(), sweep = flag(), ex = ox + num(), ey = oy + num();
        out.push(...arc(x, y, rx, ry, rot, large, sweep, ex, ey));
        x = ex; y = ey;
      } else { i++; continue }
      prev = C;
    }
    return out;
  }
  // Arc elliptique → courbes de Bézier (SVG 1.1, annexe F.6)
  function arc(x1, y1, rx, ry, phi, fa, fs, x2, y2) {
    if (!rx || !ry || (x1 === x2 && y1 === y2)) return x1 === x2 && y1 === y2 ? [] : [['L', x2, y2]];
    rx = Math.abs(rx); ry = Math.abs(ry);
    const p = phi * Math.PI / 180, cp = Math.cos(p), sp = Math.sin(p);
    const dx = (x1 - x2) / 2, dy = (y1 - y2) / 2;
    const x1p = cp * dx + sp * dy, y1p = -sp * dx + cp * dy;
    const lam = x1p * x1p / (rx * rx) + y1p * y1p / (ry * ry);
    if (lam > 1) { rx *= Math.sqrt(lam); ry *= Math.sqrt(lam) }
    const num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
    const den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
    const co = (fa === fs ? -1 : 1) * Math.sqrt(Math.max(0, num / den));
    const cxp = co * rx * y1p / ry, cyp = -co * ry * x1p / rx;
    const cx = cp * cxp - sp * cyp + (x1 + x2) / 2, cy = sp * cxp + cp * cyp + (y1 + y2) / 2;
    const ang = (ux, uy, vx, vy) => Math.atan2(ux * vy - uy * vx, ux * vx + uy * vy);
    let t1 = ang(1, 0, (x1p - cxp) / rx, (y1p - cyp) / ry);
    let dt = ang((x1p - cxp) / rx, (y1p - cyp) / ry, (-x1p - cxp) / rx, (-y1p - cyp) / ry);
    if (!fs && dt > 0) dt -= 2 * Math.PI;
    if (fs && dt < 0) dt += 2 * Math.PI;
    const n = Math.ceil(Math.abs(dt) / (Math.PI / 2));
    const out = [];
    const d = dt / n, a = 4 / 3 * Math.tan(d / 4);
    const pt = (t) => [cx + rx * Math.cos(t) * cp - ry * Math.sin(t) * sp, cy + rx * Math.cos(t) * sp + ry * Math.sin(t) * cp];
    const der = (t) => [-rx * Math.sin(t) * cp - ry * Math.cos(t) * sp, -rx * Math.sin(t) * sp + ry * Math.cos(t) * cp];
    for (let i = 0; i < n; i++) {
      const ta = t1 + i * d, tb = ta + d;
      const [ax, ay] = pt(ta), [bx, by] = pt(tb), [dax, day] = der(ta), [dbx, dby] = der(tb);
      out.push(['C', ax + a * dax, ay + a * day, bx - a * dbx, by - a * dby, bx, by]);
    }
    return out;
  }
  function geometry(el) {
    const n = (a) => parseFloat(el.getAttribute(a)) || 0;
    switch (el.tagName) {
      case 'path': return pathData(el.getAttribute('d') || '');
      case 'line': return [['M', n('x1'), n('y1')], ['L', n('x2'), n('y2')]];
      case 'polyline': case 'polygon': {
        const p = (el.getAttribute('points') || '').trim().split(/[\s,]+/).map(parseFloat);
        const out = [];
        for (let i = 0; i + 1 < p.length; i += 2) out.push([i ? 'L' : 'M', p[i], p[i + 1]]);
        if (el.tagName === 'polygon') out.push(['Z']);
        return out;
      }
      case 'rect': {
        const x = n('x'), y = n('y'), w = n('width'), h = n('height');
        let rx = el.hasAttribute('rx') ? n('rx') : n('ry'), ry = el.hasAttribute('ry') ? n('ry') : rx;
        rx = Math.min(rx, w / 2); ry = Math.min(ry, h / 2);
        if (!rx) return [['M', x, y], ['L', x + w, y], ['L', x + w, y + h], ['L', x, y + h], ['Z']];
        return [['M', x + rx, y], ['L', x + w - rx, y], ...arc(x + w - rx, y, rx, ry, 0, 0, 1, x + w, y + ry), ['L', x + w, y + h - ry],
          ...arc(x + w, y + h - ry, rx, ry, 0, 0, 1, x + w - rx, y + h), ['L', x + rx, y + h], ...arc(x + rx, y + h, rx, ry, 0, 0, 1, x, y + h - ry),
          ['L', x, y + ry], ...arc(x, y + ry, rx, ry, 0, 0, 1, x + rx, y), ['Z']];
      }
      case 'circle': case 'ellipse': {
        const cx = n('cx'), cy = n('cy'), rx = el.tagName === 'circle' ? n('r') : n('rx'), ry = el.tagName === 'circle' ? n('r') : n('ry');
        return [['M', cx + rx, cy], ...arc(cx + rx, cy, rx, ry, 0, 0, 1, cx - rx, cy), ...arc(cx - rx, cy, rx, ry, 0, 0, 1, cx + rx, cy), ['Z']];
      }
    }
    return [];
  }
  const filterShadows = new Map();
  function filterShadow(el, s) {
    const f = s.filter;
    if (!f || f === 'none') return null;
    if (/drop-shadow/.test(f)) return shadow(f.replace(/drop-shadow\(([^()]*(\([^)]*\))?[^()]*)\)/g, '$1'));
    const id = /url\("?#([^")]+)"?\)/.exec(f);
    if (!id) return null;
    if (filterShadows.has(id[1])) return filterShadows.get(id[1]);
    const fe = el.ownerSVGElement && el.ownerSVGElement.getElementById(id[1]);
    let sh = null;
    if (fe) {
      const blurs = [...fe.querySelectorAll('feGaussianBlur')].map((b) => parseFloat(b.getAttribute('stdDeviation')) || 0);
      const offs = [...fe.querySelectorAll('feOffset')].map((o) => parseFloat(o.getAttribute('dy')) || 0);
      const flood = fe.querySelector('feFlood');
      const i = blurs.length ? blurs.indexOf(Math.max(...blurs)) : 0;
      const c = color(flood ? flood.getAttribute('flood-color') : '#000') || [0, 0, 0, 1];
      c[3] = flood ? parseFloat(flood.getAttribute('flood-opacity') || 1) : 0.4;
      sh = { dx: 0, dy: offs[i] || 0, blur: (blurs[i] || 0) * 2, c };
    }
    filterShadows.set(id[1], sh);
    return sh;
  }
  function paint(el, value, svgOpacity) {
    if (!value || value === 'none') return null;
    const g = /url\("?#([^")]+)"?\)/.exec(value);
    if (g) {
      const grad = el.ownerSVGElement && el.ownerSVGElement.getElementById(g[1]);
      const stops = grad ? [...grad.querySelectorAll('stop')] : [];
      if (!stops.length) return null;
      const list = stops.map((st) => {
        const ss = cs(st);
        const c = color(ss.stopColor) || [0, 0, 0, 1];
        return [parseFloat(st.getAttribute('offset')) / (String(st.getAttribute('offset')).includes('%') ? 100 : 1) || 0, fade(c, parseFloat(ss.stopOpacity) * svgOpacity)];
      });
      const x1 = parseFloat(grad.getAttribute('x1') || 0), y1 = parseFloat(grad.getAttribute('y1') || 0);
      const x2 = parseFloat(grad.getAttribute('x2') ?? 1), y2 = parseFloat(grad.getAttribute('y2') || 0);
      const angle = 90 + Math.atan2(y2 - y1, x2 - x1) * 180 / Math.PI;
      return { grad: { angle, radial: grad.tagName === 'radialGradient', stops: list } };
    }
    const c = color(value);
    return c ? { c: fade(c, svgOpacity) } : null;
  }
  const SVG_SKIP = new Set(['defs', 'title', 'desc', 'metadata', 'style', 'marker', 'symbol', 'clipPath', 'mask', 'pattern', 'linearGradient', 'radialGradient', 'filter']);
  const SVG_GROUPS = new Set(['g', 'a', 'svg']);

  // Un SVG (schéma skemo, icône, marque) : un groupe d'objets, ses `<g>` en sous-groupes
  function svgItems(svg, clip, out) {
    const items = [];
    for (const el of svg.children) svgNode(el, svg, clip, items);
    if (items.length) out.push({ t: 'group', name: svg.getAttribute('class') || 'svg', items });
  }

  function svgNode(el, svg, clip, items) {
    const s = cs(el);
    if (s.display === 'none' || SVG_SKIP.has(el.tagName) || opacity(el) < 0.02) return;
    const r = el.getBoundingClientRect();
    const sized = r.width + r.height > 0;
    if (sized && !meets(clip, r)) return;
    if (SVG_GROUPS.has(el.tagName)) {
      const g = { t: 'group', name: el.getAttribute('data-id') || el.getAttribute('class') || 'g', items: [] };
      for (const c of el.children) svgNode(c, svg, clip, g.items);
      if (g.items.length > 1) items.push(g); else items.push(...g.items);
      return;
    }
    if (el.tagName === 'foreignObject') { for (const c of el.children) walk(c, items); return }
    if (s.visibility !== 'visible') return;
    if (sized && !within(clip, r)) items.push(cut(el, clip, r));
    else if (el.tagName === 'text') svgText(el, s, opacity(el), items);
    else if (el.tagName === 'image' || el.tagName === 'use') items.push({ t: 'pic', iso: mark(el, 'all'), x: r.left, y: r.top, w: r.width, h: r.height });
    else { const path = svgShape(el, s, svg); if (path) items.push(path) }
  }

  // Une forme SVG → un tracé en pixels de la fenêtre (transformations appliquées)
  function svgShape(el, s, svg) {
    const m = el.getScreenCTM();
    if (!m) return null;
    const k = Math.sqrt(Math.abs(m.a * m.d - m.b * m.c));
    const tr = (x, y) => [round(m.a * x + m.c * y + m.e), round(m.b * x + m.d * y + m.f)];
    const d = geometry(el).map(([c, ...p]) => {
      const q = [];
      for (let i = 0; i < p.length; i += 2) q.push(...tr(p[i], p[i + 1]));
      return [c, ...q];
    });
    if (d.length < 2) return null;
    const o = opacity(el);
    const open = el.tagName === 'line' || el.tagName === 'polyline';
    const fill = open && s.fill === 'none' ? null : paint(el, s.fill, o * parseFloat(s.fillOpacity));
    const stroke = paint(el, s.stroke, o * parseFloat(s.strokeOpacity));
    // Trait d'épaisseur fixe (`vector-effect: non-scaling-stroke`) : déjà en pixels de l'écran
    const sw = parseFloat(s.strokeWidth) * (s.vectorEffect === 'non-scaling-stroke' ? scale(svg) : k);
    const line = stroke && stroke.c && sw > 0 ? { w: sw, c: stroke.c, dash: s.strokeDasharray !== 'none', cap: s.strokeLinecap } : null;
    if (!fill && !line) return null;
    return { t: 'path', d, fill: el.tagName === 'line' ? null : fill, line, head: s.markerStart !== 'none', tail: s.markerEnd !== 'none', shadow: filterShadow(el, s) };
  }

  function svgText(el, s, o, items) {
    const parts = el.querySelector('tspan[x], tspan[y], tspan[dy]') ? [...el.querySelectorAll('tspan')] : [el];
    for (const part of parts) {
      const r = part.getBoundingClientRect();
      if (!r.width) continue;
      const ps = cs(part);
      const m = el.getScreenCTM();
      const k = m ? Math.sqrt(Math.abs(m.a * m.d - m.b * m.c)) : 1;
      const runs = [];
      const add = (node) => {
        for (const n of node.childNodes) {
          if (n.nodeType === 3 && n.data.trim()) {
            const ns = cs(n.parentElement);
            const c = color(ns.fill);
            runs.push({
              text: n.data.replace(/\s+/g, ' '), font: family(ns.fontFamily), size: round(parseFloat(ns.fontSize) * k),
              b: parseInt(ns.fontWeight) >= 600, i: ns.fontStyle !== 'normal', u: false, s: false, cap: null,
              spc: ns.letterSpacing === 'normal' ? 0 : round(parseFloat(ns.letterSpacing) * k),
              color: fade(c, o * parseFloat(ns.fillOpacity)), hl: null, href: null, mono: /mono/i.test(ns.fontFamily),
            });
          } else if (n.nodeType === 1) add(n);
        }
      };
      add(part);
      if (!runs.length) continue;
      runs[0].text = runs[0].text.replace(/^\s+/, '');
      const anchor = ps.textAnchor;
      items.push({ t: 'text', x: round(r.left), y: round(r.top), w: round(r.width), h: round(r.height), wrap: false,
        algn: anchor === 'middle' ? 'ctr' : anchor === 'end' ? 'r' : 'l', lh: round(r.height), bullet: null, paras: [runs] });
    }
  }

  // ------------------------------------------------------------ images et éléments à capturer
  let ids = 0;
  // Marque un élément à capturer par `isolate` : 'own' (son backdrop seul) ou 'all' (avec son contenu)
  function mark(el, how) {
    const id = ++ids;
    el.setAttribute('data-gx-' + how, (el.getAttribute('data-gx-' + how) || '') + ' ' + id);
    return id;
  }
  function picture(img, r, out) {
    const s = cs(img);
    const src = img.currentSrc || img.src || '';
    if (s.objectFit === 'fill' || s.objectFit === '' ) {
      if (/^data:image\/(png|jpeg|gif);base64,/.test(src)) return out.push({ t: 'pic', src, x: r.left, y: r.top, w: r.width, h: r.height });
      try {
        const c = document.createElement('canvas');
        c.width = Math.max(1, Math.round(r.width * devicePixelRatio));
        c.height = Math.max(1, Math.round(r.height * devicePixelRatio));
        c.getContext('2d').drawImage(img, 0, 0, c.width, c.height);
        return out.push({ t: 'pic', src: c.toDataURL('image/png'), x: r.left, y: r.top, w: r.width, h: r.height });
      } catch (e) { /* image d'une autre origine : capturée */ }
    }
    out.push({ t: 'pic', iso: mark(img, 'all'), x: r.left, y: r.top, w: r.width, h: r.height });
  }

  // ------------------------------------------------------------ parcours
  const SKIP = new Set(['SCRIPT', 'STYLE', 'TEMPLATE', 'HEAD', 'LINK', 'META', 'NOSCRIPT', 'ASIDE']);
  let background = [];
  function walk(el, out) {
    if (SKIP.has(el.tagName) && (el.tagName !== 'ASIDE' || el.classList.contains('notes'))) return;
    const s = cs(el);
    if (s.display === 'none') return;
    if (s.display === 'contents') { for (const c of el.children) walk(c, out); return }
    const o = opacity(el);
    if (o < 0.02) return;
    const r = el.getBoundingClientRect();
    const clip = clipOf(el);
    const z = zkey(el);
    const items = [];
    const own = s.visibility === 'visible' && meets(clip, r);
    const full = r.width >= W * 0.97 && r.height >= H * 0.97;
    const tag = el.tagName.toUpperCase();
    // Élément tourné ou incliné : PowerPoint ne le reproduirait pas, il est capturé tel quel
    const m = /^matrix\(([^)]*)\)/.exec(s.transform);
    const turned = m && m[1].split(',').map(parseFloat).some((v, i) => (i === 1 || i === 2) && Math.abs(v) > 0.01);
    if (turned) {
      if (own && r.width > 0 && r.height > 0) items.push(cut(el, clip, r));
    } else if (tag === 'SVG') {
      svgItems(el, clip, items);
    } else if (['IMG', 'CANVAS', 'VIDEO', 'IFRAME', 'MATH', 'OBJECT', 'EMBED'].includes(tag)) {
      if (own && r.width > 0 && r.height > 0) {
        if (full && tag !== 'IFRAME') background.push(mark(el, 'all'));
        else if (!within(clip, r)) items.push(cut(el, clip, r));
        else if (tag === 'IMG') picture(el, r, items);
        else items.push({ t: 'pic', iso: mark(el, 'all'), x: r.left, y: r.top, w: r.width, h: r.height });
      }
    } else {
      if (own) {
        if (full) {
          if (color(s.backgroundColor) || s.backgroundImage !== 'none') background.push(mark(el, 'own'));
        } else {
          boxes(el, s, r, o, scale(el), items);
        }
      }
      const escaped = [];
      textItem(el, s, clip, items, escaped);
      // Le backdrop et le texte de l'élément sous ceux de ses enfants
      emit(el, z, items, out);
      items.length = 0;
      for (const c of escaped) walk(c, out);
    }
    emit(el, z, items, out);
  }
  function emit(el, z, items, out) {
    for (const it of items) {
      it.z = z;
      it.seq = seq++;
      if (DEBUG) it.who = el.tagName + '.' + [...el.classList].join('.');
      out.push(it);
    }
  }
  let seq = 0;

  function cmp(a, b) {
    const n = Math.max(a.z.length, b.z.length);
    for (let i = 0; i < n; i++) {
      const x = a.z[i] ?? 0, y = b.z[i] ?? 0;
      if (x !== y) return x - y;
    }
    return a.seq - b.seq;
  }

  function collectAll() {
    document.head.appendChild(sheet);
    materialize(document.body);
    opacities.clear(); clips.clear(); scales.clear(); zs.clear(); filterShadows.clear();
    background = [];
    seq = 0;
    const items = [];
    walk(document.body, items);
    items.sort(cmp);
    const strip = (it) => { delete it.z; delete it.seq; if (it.items) it.items.forEach(strip); for (const k of ['x', 'y', 'w', 'h']) if (k in it) it[k] = round(it[k]) };
    items.forEach(strip);
    if (color(cs(document.documentElement).backgroundColor)) background.push(mark(document.documentElement, 'own'));
    const page = [document.documentElement, document.body, ...document.querySelectorAll('.reveal, .reveal-viewport')]
      .map((e) => color(cs(e).backgroundColor)).filter(Boolean).pop() || [255, 255, 255, 1];
    return { items, background, color: page, width: W, height: H };
  }

  // Ne laisse peindre que les éléments marqués `ids` : leur backdrop seul pour 'own', tout pour 'all'
  function isolate(list, transparent) {
    unisolate();
    const sel = list.flatMap((id) => [`[data-gx-own~="${id}"]`, `[data-gx-all~="${id}"]`, `[data-gx-all~="${id}"] *`]).join(',');
    isoSheet = document.createElement('style');
    const own = list.map((id) => `[data-gx-own~="${id}"]`).join(',');
    isoSheet.textContent = `*{visibility:hidden!important;transition:none!important}${sel}{visibility:visible!important}` +
      `${own}{color:transparent!important;-webkit-text-fill-color:transparent!important;border-color:transparent!important;box-shadow:none!important}` +
      (transparent ? 'html,body{background:transparent!important}' : '');
    document.head.appendChild(isoSheet);
    const r = list.length === 1 && document.querySelector(`[data-gx-own~="${list[0]}"],[data-gx-all~="${list[0]}"]`);
    return r ? (({ left, top, width, height }) => ({ x: left, y: top, w: width, h: height }))(r.getBoundingClientRect()) : null;
  }
  function unisolate() {
    if (isoSheet) { isoSheet.remove(); isoSheet = null }
  }
  function clear() {
    unisolate();
    for (const m of made) m.remove();
    made.length = 0;
    for (const el of document.querySelectorAll('.gx-nop')) el.classList.remove('gx-nop');
    for (const el of document.querySelectorAll('[data-gx-own],[data-gx-all]')) { el.removeAttribute('data-gx-own'); el.removeAttribute('data-gx-all') }
    sheet.remove();
  }
  return { collect: collectAll, isolate, unisolate, clear, gradient, color };
})();
