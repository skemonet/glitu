//! `glitu init`: a sample presentation (`slides.md`), and with `--css` a `style.css` that lists
//! the variables of its theme, commented out: it changes nothing until a line is uncommented.
//! A file that exists is never overwritten.

use std::fs;
use std::path::{Path, PathBuf};

use glitu_core::Theme;

use crate::build::{self, Project};

/// The sample presentation
const SLIDES: &str = include_str!("../init/slides.md");

/// Writes what is missing in `dir`: `slides.md`, and `style.css` with `css`. Its theme is the
/// one of the existing `slides.md`, otherwise `theme`. Returns the files written.
pub fn run(dir: &Path, theme: &str, css: bool) -> Result<Vec<PathBuf>, String> {
    let (slides, style) = (dir.join("slides.md"), dir.join("style.css"));
    let write_slides = !slides.exists();
    let write_style = css && !style.exists();
    if !write_slides && !write_style {
        let existing = if css { format!("{} and {}", slides.display(), style.display()) } else { slides.display().to_string() };
        return Err(format!("{existing} already exist(s): nothing was written"));
    }
    if write_slides && !crate::themes::names().contains(&theme) && !theme.starts_with('.') {
        let names = crate::themes::names().join(", ");
        return Err(format!("unknown theme: {theme} (embedded themes: {names})"));
    }
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut written = Vec::new();
    if write_slides {
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
        assert!(run(&temp("unknown").0, "nope", false).unwrap_err().contains("unknown theme"));
    }
}
