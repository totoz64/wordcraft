//! Colour glyphs (#1269): emoji from colour fonts (bitmap strikes, COLR) are drawn in colour.

use super::*;
use wordcraft_layout::{LayoutCache, LayoutOptions, layout};

/// Colour emoji families the system fallback may pick.
const COLOR_EMOJI: &[&str] = &["Noto Color Emoji", "Apple Color Emoji", "Segoe UI Emoji"];

/// A strongly coloured (non-grey) pixel.
fn colourful(p: [u8; 4]) -> bool {
    let (hi, lo) = (p[0].max(p[1]).max(p[2]), p[0].min(p[1]).min(p[2]));
    hi - lo > 60
}

/// System-dependent: skips (passes) when no colour emoji font is installed. Each emoji glyph's box
/// on the rendered page holds coloured pixels.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn emoji_are_drawn_in_colour() {
    let Some((img, boxes)) = emoji_page(&RenderOptions::default()) else { return };
    for (x0, y0, x1, y1) in boxes {
        let n = (x0..x1).flat_map(|x| (y0..y1).map(move |y| (x, y))).filter(|(x, y)| colourful(img.pixel(*x, *y))).count();
        eprintln!("emoji_are_drawn_in_colour: validated_colour_pixels={n} box={x0},{y0}..{x1},{y1}");
        assert!(n > 20, "only {n} coloured pixels in emoji box {x0},{y0}..{x1},{y1}");
    }
}

/// #2534: text effects (drawn from outlines) leave colour emoji, which have none, drawn in colour.
/// Skips like [`emoji_are_drawn_in_colour`].
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn emoji_with_text_effects_stay_drawn() {
    let cp = wordcraft_doc::CharProps { outline: Some(true), shadow: Some(true), ..Default::default() };
    let Some((img, boxes)) = emoji_page_with(&RenderOptions::default(), cp) else { return };
    for (x0, y0, x1, y1) in boxes {
        let n = (x0..x1).flat_map(|x| (y0..y1).map(move |y| (x, y))).filter(|(x, y)| colourful(img.pixel(*x, *y))).count();
        eprintln!("emoji_with_text_effects_stay_drawn: validated_colour_pixels={n} box={x0},{y0}..{x1},{y1}");
        assert!(n > 20, "only {n} coloured pixels in emoji box {x0},{y0}..{x1},{y1}");
    }
}

/// A dark page (View › Switch Modes) doesn't invert the emoji: they keep the font's colours, the
/// same as pictures. The pixels well inside each emoji (coloured with coloured neighbours, so the
/// paper doesn't show through) match the light page's. Skips like [`emoji_are_drawn_in_colour`].
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn emoji_keep_their_colours_on_a_dark_page() {
    let Some((light, boxes)) = emoji_page(&RenderOptions::default()) else { return };
    let Some((dark, _)) = emoji_page(&RenderOptions { dark: true, ..Default::default() }) else { return };
    for (x0, y0, x1, y1) in boxes {
        let inside = |x: u32, y: u32| (x - 1..=x + 1).all(|i| (y - 1..=y + 1).all(|j| colourful(light.pixel(i, j))));
        let mut n = 0;
        for (x, y) in (x0 + 1..x1.saturating_sub(1)).flat_map(|x| (y0 + 1..y1.saturating_sub(1)).map(move |y| (x, y))) {
            if !inside(x, y) {
                continue;
            }
            let (l, d) = (light.pixel(x, y), dark.pixel(x, y));
            assert!(l.iter().zip(d).all(|(a, b)| a.abs_diff(b) <= 2), "emoji pixel {x},{y} changed on a dark page: {l:?} → {d:?}");
            n += 1;
        }
        eprintln!("emoji_keep_their_colours_on_a_dark_page: validated_colour_pixels={n} box={x0},{y0}..{x1},{y1}");
        assert!(n > 10, "only {n} pixels well inside emoji box {x0},{y0}..{x1},{y1}");
    }
}

/// "Emoji test: 🧠 brain, ✅ check." rendered at 2x with `opts`, and each emoji glyph's box in
/// pixels; None (the test skips) when no colour emoji font is installed.
#[cfg(not(target_arch = "wasm32"))]
fn emoji_page(opts: &RenderOptions) -> Option<(Rendered, Vec<(u32, u32, u32, u32)>)> {
    emoji_page_with(opts, wordcraft_doc::CharProps::default())
}

/// [`emoji_page`] with the text formatted `cp`.
#[cfg(not(target_arch = "wasm32"))]
fn emoji_page_with(opts: &RenderOptions, cp: wordcraft_doc::CharProps) -> Option<(Rendered, Vec<(u32, u32, u32, u32)>)> {
    let required = std::env::var("WORDCRAFT_REQUIRE_COLOUR_EMOJI").ok().as_deref() == Some("1");
    let colour = FontDb::global().fallback_for('🧠', u32::MAX).filter(|f| COLOR_EMOJI.contains(&f.family.as_str()));
    let Some(colour) = colour else {
        assert!(!required, "WORDCRAFT_REQUIRE_COLOUR_EMOJI=1 requires an installed colour font for the original brain/check renderer fixtures");
        eprintln!("skipped: no colour emoji font installed");
        return None;
    };
    eprintln!(
        "original emoji render: required={required} family={} brain_glyph={} usable_colour_data={} dark={}",
        colour.family,
        colour.glyph_for('🧠'),
        colour.color_glyph(colour.glyph_for('🧠'), 64.0).is_some(),
        opts.dark
    );
    let mut d = Document::from_text("x");
    d.body = vec![wordcraft_doc::para_block(wordcraft_doc::Paragraph::with_text("Emoji test: 🧠 brain, ✅ check.", cp))];
    let l = layout(&d, &mut LayoutCache::new(), &LayoutOptions::default());
    let page = &l.pages[0];
    let scale = 2.0;
    let img = render_page(&d, page, scale, opts);
    let mut boxes = vec![];
    for it in page_display(&d, page, &DisplayOptions::default()) {
        if let Draw::Glyphs { face, size, glyphs, .. } = it
            && COLOR_EMOJI.contains(&face.family.as_str())
        {
            for (_, x, y) in glyphs {
                boxes.push((((x * scale) as u32), ((y - size) * scale) as u32, ((x + size) * scale) as u32, (y * scale) as u32));
            }
        }
    }
    assert_eq!(boxes.len(), 2, "both emoji use the colour emoji font");
    Some((img, boxes))
}

/// A text symbol must not carry its monochrome fallback into the following default emoji.
#[cfg(windows)]
#[test]
fn emoji_after_a_text_symbol_chooses_colour_without_changing_explicit_fonts() {
    let required = std::env::var("WORDCRAFT_REQUIRE_COLOUR_EMOJI").ok().as_deref() == Some("1");
    let db = FontDb::global();
    if !db.has_family("Segoe UI Symbol") || !db.has_family("Segoe UI Emoji") {
        assert!(!required, "required Windows symbol and colour emoji fonts unavailable");
        eprintln!("skipped: Windows symbol and colour emoji fonts unavailable");
        return;
    }
    let mono = db.face("Segoe UI Symbol", "Regular");
    let colour = db.face("Segoe UI Emoji", "Regular");
    if !mono.covers('☒') || !mono.covers('✅') || colour.color_glyph(colour.glyph_for('✅'), 64.0).is_none() {
        assert!(!required, "required Windows symbol/colour glyph coverage unavailable");
        eprintln!("skipped: installed Windows fonts lack the required symbol/colour glyph coverage");
        return;
    }
    let segments = |text: &str, font: &str| {
        let cp = wordcraft_doc::CharProps { font: Some(font.into()), ..Default::default() };
        let mut doc = Document::from_text("x");
        doc.body = vec![wordcraft_doc::para_block(wordcraft_doc::Paragraph::with_text(text, cp))];
        let result = layout(&doc, &mut LayoutCache::new(), &LayoutOptions::default());
        page_display(&doc, &result.pages[0], &DisplayOptions::default())
            .into_iter()
            .filter_map(|d| if let Draw::Glyphs { face, text, .. } = d { Some((text, face.family.clone())) } else { None })
            .collect::<Vec<_>>()
    };
    for primary in ["Source Sans 3", "Source Serif 4"] {
        for text in ["☒✅", "✅☒", "☒ ✅", "A☒✅B"] {
            let drawn = segments(text, primary);
            let emoji_faces: Vec<_> = drawn.iter().filter(|(s, _)| s.contains('✅')).map(|(_, f)| f.as_str()).collect();
            assert!(!emoji_faces.is_empty(), "emoji has a displayed segment: {text:?}");
            assert!(emoji_faces.iter().all(|f| COLOR_EMOJI.contains(f)), "{primary}: {text:?} uses {drawn:?}");
            for letter in text.chars().filter(|c| matches!(*c, 'A' | 'B')) {
                assert!(drawn.iter().any(|(s, f)| s.contains(letter) && f == primary), "{letter} keeps {primary}: {drawn:?}");
            }
        }
    }
    let explicit = segments("☒✅", "Segoe UI Symbol");
    assert!(explicit.iter().any(|(s, f)| s.contains('✅') && f == "Segoe UI Symbol"), "an explicit covering font keeps priority: {explicit:?}");
}

/// The adjacent-fallback regression on every native platform, using only bundled text fonts.
/// Hosted validation requires a real colour font instead of accepting the portable local skip.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn emoji_after_a_bundled_text_fallback_uses_real_colour_on_native_platforms() {
    let db = FontDb::global();
    let required = std::env::var("WORDCRAFT_REQUIRE_COLOUR_EMOJI").ok().as_deref() == Some("1");
    let colour = db
        .fallback_for('☕', u32::MAX)
        .filter(|f| COLOR_EMOJI.contains(&f.family.as_str()) && f.glyph_for('☕') != 0 && f.color_glyph(f.glyph_for('☕'), 48.0).is_some());
    let Some(colour) = colour else {
        assert!(!required, "WORDCRAFT_REQUIRE_COLOUR_EMOJI=1 requires an installed colour font with usable coffee glyph data");
        eprintln!("skipped: no installed colour emoji font with a usable coffee glyph");
        return;
    };
    eprintln!("native emoji: required={required} family={} coffee_glyph={}", colour.family, colour.glyph_for('☕'));
    let mono = db.face("Source Sans 3", "Regular");
    assert!(mono.covers('☕') && !mono.has_color_glyphs(), "bundled text fallback has a real monochrome coffee glyph");
    let mut fixtures = Vec::new();
    for family in ["Source Serif 4", "Inter", "JetBrains Mono"] {
        let primary = db.face(family, "Regular");
        assert!(primary.named(family) && !primary.covers('☕'), "bundled {family} must require a coffee fallback");
        let control = (0x0100..=0x02AF).filter_map(char::from_u32).find(|c| {
            c.is_alphabetic() && !primary.covers(*c) && mono.covers(*c) && db.fallback_for(*c, primary.id()).is_some_and(|f| f.id() == mono.id())
        });
        if let Some(control) = control {
            fixtures.push((primary, control));
        }
    }
    assert!(!fixtures.is_empty(), "a bundled primary must exercise a true Source Sans 3 fallback before coffee");
    let fixture_count = fixtures.len();
    for (primary, control) in fixtures {
        assert_eq!(db.fallback_for(control, primary.id()).unwrap().id(), mono.id(), "first letter really chooses the monochrome fallback");
        eprintln!("native adjacency fixture: primary={} control=U+{:04X} fallback={}", primary.family, u32::from(control), mono.family);
        for text in [format!("{control}☕"), format!("☕{control}"), format!("{control} ☕"), format!("A{control}☕B")] {
            let cp = wordcraft_doc::CharProps { font: Some(primary.family.clone()), size: Some(24.0), ..Default::default() };
            let mut doc = Document::from_text("x");
            doc.body = vec![wordcraft_doc::para_block(wordcraft_doc::Paragraph::with_text(&text, cp))];
            let result = layout(&doc, &mut LayoutCache::new(), &LayoutOptions::default());
            let page = result.pages.first().expect("short original fixture has a page");
            let image = render_page(&doc, page, 2.0, &RenderOptions::default());
            let mut coffee = 0;
            let mut source_control = false;
            let mut ordinary = Vec::new();
            for draw in page_display(&doc, page, &DisplayOptions::default()) {
                if let Draw::Glyphs { face, text: segment, size, glyphs, .. } = draw {
                    source_control |= segment.contains(control) && face.id() == mono.id();
                    for letter in ['A', 'B'] {
                        if segment.contains(letter) && face.id() == primary.id() {
                            ordinary.push(letter);
                        }
                    }
                    if !segment.contains('☕') {
                        continue;
                    }
                    assert!(COLOR_EMOJI.contains(&face.family.as_str()), "{} {text:?}: coffee used {}", primary.family, face.family);
                    let gid = face.glyph_for('☕');
                    assert!(gid != 0 && face.color_glyph(gid, 48.0).is_some(), "displayed coffee has actual colour data");
                    for (_, x, y) in glyphs.into_iter().filter(|(g, _, _)| *g == gid) {
                        let (x0, y0, x1, y1) = ((x * 2.0) as u32, ((y - size) * 2.0) as u32, ((x + size) * 2.0) as u32, (y * 2.0) as u32);
                        let pixels = (x0..x1)
                            .flat_map(|px| (y0..y1).map(move |py| (px, py)))
                            .filter(|(px, py)| {
                                let p = image.pixel(*px, *py);
                                // The coffee artwork can be pastel; grey text has equal RGB channels.
                                p[0].max(p[1]).max(p[2]) - p[0].min(p[1]).min(p[2]) > 20
                            })
                            .count();
                        eprintln!(
                            "native coffee render: primary={} input={text:?} colour_family={} colourful_pixels={pixels}",
                            primary.family, face.family
                        );
                        assert!(pixels > 20, "{} {text:?}: real coloured coffee pixels required, got {pixels}", primary.family);
                        coffee += 1;
                    }
                }
            }
            assert_eq!(coffee, 1, "exactly one real coffee glyph is displayed: {text:?}");
            if text.starts_with(control) || text.starts_with('A') {
                assert!(source_control, "preceding letter really used the bundled monochrome fallback: {text:?}");
            }
            if text.starts_with('A') {
                assert!(ordinary.contains(&'A') && ordinary.contains(&'B'), "ordinary letters keep the requested primary font");
            }
        }
    }
    let mut explicit = Document::from_text("x");
    let cp = wordcraft_doc::CharProps { font: Some(mono.family.clone()), size: Some(24.0), ..Default::default() };
    explicit.body = vec![wordcraft_doc::para_block(wordcraft_doc::Paragraph::with_text("A☕B", cp))];
    let result = layout(&explicit, &mut LayoutCache::new(), &LayoutOptions::default());
    let drawn = page_display(&explicit, &result.pages[0], &DisplayOptions::default());
    assert!(
        drawn.into_iter().any(|d| matches!(d, Draw::Glyphs { face, text, .. } if text.contains('☕') && face.id() == mono.id())),
        "explicitly selected Source Sans 3 keeps its monochrome coffee"
    );
    eprintln!(
        "native adjacency success: required={required} fixtures={fixture_count} rendered_cases={} explicit_monochrome={}",
        fixture_count * 4,
        mono.family
    );
}
