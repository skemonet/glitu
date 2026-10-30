//! `glitu init`: a sample presentation (`slides.md`), and with `--css` a `style.css` that lists
//! the variables of its theme, commented out: it changes nothing until a line is uncommented.
//! `--theme <name>` with a name that is not an official theme creates the base of a theme of
//! one's own, `themes/<name>/`: a copy of crystal whose `theme.css` sets every variable.
//! A file that exists is never overwritten.

use std::fs;
use std::path::{Path, PathBuf};

use glitu_core::Theme;

use crate::build::{self, Project};

/// The sample presentation
const SLIDES: &str = include_str!("../init/slides.md");

/// Writes what is missing in `dir`: `slides.md`, `style.css` with `css`, and `themes/<theme>/`
/// if `theme` is a new theme. The theme of `slides.md` is `theme`, unless it exists. Returns the
/// files written.
pub fn run(dir: &Path, theme: &str, css: bool) -> Result<Vec<PathBuf>, String> {
    let (slides, style) = (dir.join("slides.md"), dir.join("style.css"));
    let own = own_theme(theme)?;
    let write_slides = !slides.exists();
    let write_theme = own.as_ref().is_some_and(|_| !dir.join("themes").join(theme).exists());
    let write_style = css && !style.exists();
    if !write_slides && !write_style && !write_theme {
        let existing = if css { format!("{} and {}", slides.display(), style.display()) } else { slides.display().to_string() };
        return Err(format!("{existing} already exist(s): nothing was written"));
    }
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut written = Vec::new();
    if write_theme {
        written.extend(new_theme(&dir.join("themes").join(theme))?);
    }
    if write_slides {
        let theme = own.as_deref().unwrap_or(theme);
        let text = SLIDES.replacen("theme: crystal\n", &format!("theme: {theme}\n"), 1);
        fs::write(&slides, text).map_err(|e| format!("{}: {e}", slides.display()))?;
        written.push(slides.clone());
    }
    if write_style {
        let project = Project::new(&slides);
        let theme = glitu_core::header(&project.source()?).theme;
        let text = style_css(&project, &theme)?;
        fs::write(&style, text).map_err(|e| format!("{}: {e}", style.display()))?;
        written.push(style);
    }
    Ok(written)
}

/// `Some(./themes/<name>)` if `theme` names a theme of one's own, to create; `None` for an
/// official theme or a path
fn own_theme(theme: &str) -> Result<Option<String>, String> {
    if crate::themes::names().contains(&theme) || theme.starts_with('.') || theme.contains("://") {
        return Ok(None);
    }
    if theme.is_empty() || !theme.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(format!("{theme}: a theme name is made of letters, digits, - and _"));
    }
    Ok(Some(format!("./themes/{theme}")))
}

/// A theme of one's own in `dest`, from crystal: its fonts, its diagram colours, its `glitu.json`
/// without a version (it is not published), and a `theme.css` that sets every variable of the
/// template, to change
fn new_theme(dest: &Path) -> Result<Vec<PathBuf>, String> {
    let crystal = crate::themes::dir("crystal")?.ok_or("crystal: theme not embedded")?;
    let base = build::LocalTheme::open(crystal.clone(), &Theme::Official { name: "crystal".into(), version: None })?;
    let mut written = Vec::new();
    let mut put = |rel: &str, bytes: &[u8]| -> Result<(), String> {
        let path = dest.join(rel);
        build::write_file(&path, bytes)?;
        written.push(path);
        Ok(())
    };
    let mut config = base.config.clone();
    if let Some(c) = config.as_object_mut() {
        c.remove("version");
    }
    put("glitu.json", (serde_json::to_string_pretty(&config).map_err(|e| e.to_string())? + "\n").as_bytes())?;
    let width = build::theme_variables(&base)?.iter().map(|(n, _)| n.len()).max().unwrap_or(0);
    let mut css = format!(
        "/* The \"{}\" theme, made from crystal by `glitu init`: the shared template\n   \
         (glitu/template.css) with these values. Change them, add rules; `glitu serve` reloads at\n   \
         every save. glitu.json sets the backdrop (\"backdrop\": crystal, constellation, paper,\n   \
         aurora, ascii, blueprint or iso), the transition and the size of the slides; skemo.css the\n   \
         colours of the diagrams. */\n\n",
        dest.file_name().unwrap_or_default().to_string_lossy()
    );
    css += TEMPLATE_GUIDE;
    css += ":root {\n";
    for (name, value) in build::theme_variables(&base)? {
        css += &format!("  {:<width$} {value};\n", format!("{name}:"), width = width + 1);
    }
    css += "}\n";
    css += &TEMPLATE_RULES.replace("Rules, for what the variables do not reach", "Rules of the theme, for what the variables do not reach");
    put("theme.css", css.as_bytes())?;
    for file in ["fonts.css", "skemo.css"] {
        put(file, &fs::read(crystal.join(file)).map_err(|e| format!("{file}: {e}"))?)?;
    }
    for font in fs::read_dir(crystal.join("fonts")).map_err(|e| format!("fonts: {e}"))?.flatten() {
        let name = format!("fonts/{}", font.file_name().to_string_lossy());
        put(&name, &fs::read(font.path()).map_err(|e| format!("{name}: {e}"))?)?;
    }
    Ok(written)
}

/// A `style.css` for the theme `theme`: every variable of the theme with its value, commented
/// out, then examples of rules
fn style_css(project: &Project, theme: &Theme) -> Result<String, String> {
    let local = project.theme(theme)?;
    let vars = build::theme_variables(&local)?;
    let width = vars.iter().map(|(n, _)| n.len()).max().unwrap_or(0);
    let mut css = format!(
        "/* style.css: the adjustments of this presentation, loaded after its theme (\"{}\").\n   \
         Nothing is active: every line is commented out, with the theme's value. Uncomment a line\n   \
         and change its value to adjust the presentation; `glitu serve` reloads at every save. */\n\n",
        local.name
    );
    if local.config["template"].as_bool() == Some(true) {
        css += TEMPLATE_GUIDE;
    }
    css += ":root {\n";
    for (name, value) in &vars {
        css += &format!("  /* {name:<width$} {value}; */\n", name = format!("{name}:"), width = width + 1);
    }
    css += "}\n";
    if local.config["template"].as_bool() == Some(true) {
        css += TEMPLATE_RULES;
    } else {
        css += REVEAL_RULES;
    }
    Ok(css)
}

/// What the variables of a template theme set
const TEMPLATE_GUIDE: &str = "\
/* The variables, by role:
   --accent, --accent-2       the two accent colours (rules, progress, links, highlights)
   --mark                     colour of the hand-drawn marks: [word]{.circle @2}, .highlight…
   --text-scale               multiplies every text size (1.1: 10 % larger)
   --font, --title-font, --mono, --label-font
                              fonts (a font named with `font:` in the front matter wins)
   --heading, --text, --text-dim, --ink
                              colours of titles, texts, dimmed texts, texts on light cards
   --panel-w, --tilt          width of the text panel, slant of its edge
   --header-h, --footer-h     heights of the header and of the progress footer
   --panel-bg, --chapter-bg, --cover-veil, --chapter-veil, --body-veil
                              backgrounds and veils laid over the animated backdrop
   --win-bg, --win-border     code windows */

";

/// Examples of rules for a template theme
const TEMPLATE_RULES: &str = "
/* Rules, for what the variables do not reach. Examples:

.reveal .tpl-progress { display: none; }                  no progress footer
.reveal .panel li { font-size: 1.05em; }                  larger bullets
.reveal section.big-code .window pre { font-size: 1.2em; } larger code on `## Topic {.big-code}`
.reveal .cover h1 { letter-spacing: -.02em; }             tighter cover title
*/
";

/// Examples of rules for a reveal.js theme (black, white)
const REVEAL_RULES: &str = "
/* Rules, for what the variables do not reach. Examples:

.reveal h1, .reveal h2 { text-transform: none; }          titles as written
.reveal .progress { color: #f59e0b; }                     progress bar colour
.reveal section.small { font-size: .8em; }                smaller text on `<!-- .slide: class=\"small\" -->`
*/
";

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn temp(name: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("glitu-init-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        TempDir(dir)
    }

    #[test]
    fn init_writes_the_markdown_then_the_css() {
        let d = temp("css");
        let names = |files: Vec<PathBuf>| files.iter().map(|f| f.file_name().unwrap().to_string_lossy().into_owned()).collect::<Vec<_>>();
        assert_eq!(names(run(&d.0, "paper", false).unwrap()), ["slides.md"]);
        assert!(run(&d.0, "paper", false).unwrap_err().contains("nothing was written"));
        // the markdown exists: only the css, for the markdown's theme
        assert_eq!(names(run(&d.0, "crystal", true).unwrap()), ["style.css"]);
        let css = fs::read_to_string(d.0.join("style.css")).unwrap();
        assert!(css.contains("(\"paper\")") && css.contains("/* --accent:"));
        // every variable is commented out: the css changes nothing
        let body = css.split(":root {").nth(1).unwrap().split('}').next().unwrap();
        assert!(body.lines().filter(|l| !l.trim().is_empty()).all(|l| l.trim().starts_with("/*")));
        assert!(run(&d.0, "crystal", true).unwrap_err().contains("nothing was written"));
    }

    #[test]
    fn init_with_css_writes_both_and_refuses_an_unknown_theme() {
        let d = temp("both");
        assert_eq!(run(&d.0, "black", true).unwrap().len(), 2);
        assert!(fs::read_to_string(d.0.join("style.css")).unwrap().contains("--r-"));
        assert!(run(&temp("bad").0, "no pe", false).unwrap_err().contains("a theme name"));
    }

    #[test]
    fn init_creates_a_theme_of_ones_own() {
        let d = temp("own");
        let written = run(&d.0, "mine", false).unwrap();
        assert!(written.contains(&d.0.join("themes/mine/theme.css")) && written.contains(&d.0.join("slides.md")));
        assert!(fs::read_to_string(d.0.join("slides.md")).unwrap().contains("theme: ./themes/mine\n"));
        let config = fs::read_to_string(d.0.join("themes/mine/glitu.json")).unwrap();
        assert!(config.contains("\"backdrop\": \"crystal\"") && !config.contains("version"));
        // every variable set, none commented out
        let css = fs::read_to_string(d.0.join("themes/mine/theme.css")).unwrap();
        assert!(css.contains("\n  --accent:") && css.contains("\"mine\" theme"));
        // the presentation builds with it; a second run creates nothing
        let site = build::site(&Project::new(&d.0.join("slides.md")), false).unwrap();
        assert!(site.iter().any(|f| f.path.starts_with("theme/fonts/")));
        assert!(run(&d.0, "mine", false).unwrap_err().contains("nothing was written"));
        // its style.css lists the variables of the new theme
        assert_eq!(run(&d.0, "mine", true).unwrap(), [d.0.join("style.css")]);
    }
}
