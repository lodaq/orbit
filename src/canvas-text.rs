use crate::{CanvasText, Launcher};
use cosmic_text::{
    Attrs, Buffer, Color, Cursor, Edit, Editor, Family, FontSystem, Metrics, Selection, Shaping,
    SwashCache, UnderlineStyle, Weight, Wrap,
};
use serde::{Deserialize, Serialize};
use slint::{ComponentHandle, Rgba8Pixel, SharedPixelBuffer};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Format {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub highlight: bool,
    pub code: bool,
    pub font: u8,
    pub heading: u8,
    pub link: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Span {
    pub text: String,
    pub format: Format,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Document {
    pub spans: Vec<Span>,
    pub width: f32,
    pub scale: f32,
    pub rotation: f32,
    pub alignment: u8,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            spans: Vec::new(),
            width: 0.0,
            scale: 1.0,
            rotation: 0.0,
            alignment: 0,
        }
    }
}
impl Document {
    pub fn plain(text: &str) -> Self {
        Self {
            spans: vec![Span {
                text: normalize_text(text),
                format: Format::default(),
            }],
            ..Self::default()
        }
    }
    pub fn text(&self) -> String {
        self.spans.iter().map(|span| span.text.as_str()).collect()
    }
    pub fn valid(&self) -> bool {
        self.width.is_finite()
            && (0.0..=16000.0).contains(&self.width)
            && self.scale.is_finite()
            && (0.1..=20.0).contains(&self.scale)
            && self.rotation.is_finite()
            && (-360.0..=360.0).contains(&self.rotation)
            && self.alignment <= 3
            && self.spans.len() <= 10000
            && self
                .spans
                .iter()
                .map(|s| s.text.chars().count())
                .sum::<usize>()
                <= 10000
            && self.spans.iter().all(|s| {
                !s.text.contains(['\r', '\u{0085}', '\u{2028}', '\u{2029}'])
                    && s.format.font <= 3
                    && s.format.heading <= 3
                    && (s.format.link.is_empty() || valid_link(&s.format.link))
            })
    }
    fn characters(&self) -> Vec<(char, Format)> {
        self.spans
            .iter()
            .flat_map(|s| s.text.chars().map(|c| (c, s.format.clone())))
            .collect()
    }
    fn set_characters(&mut self, chars: Vec<(char, Format)>) {
        self.spans.clear();
        for (c, format) in chars {
            if let Some(span) = self.spans.last_mut()
                && span.format == format
            {
                span.text.push(c);
            } else {
                self.spans.push(Span {
                    text: c.to_string(),
                    format,
                });
            }
        }
    }
    fn replace(&mut self, start: usize, end: usize, insert: &str, format: &Format) {
        let text = self.text();
        let from = text[..start].chars().count();
        let to = from + text[start..end].chars().count();
        let mut chars = self.characters();
        chars.splice(from..to, insert.chars().map(|c| (c, format.clone())));
        self.set_characters(chars);
    }
}
fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace(['\r', '\u{0085}', '\u{2028}', '\u{2029}'], "\n")
}

pub fn valid_link(url: &str) -> bool {
    url.len() <= 2048
        && !url.chars().any(char::is_control)
        && (url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:"))
}
struct Renderer {
    fonts: FontSystem,
    cache: SwashCache,
}
impl Renderer {
    fn new() -> Self {
        let mut fonts = FontSystem::new();
        fonts
            .db_mut()
            .load_font_data(include_bytes!("../ui/fonts/inter.ttf").to_vec());
        fonts
            .db_mut()
            .load_font_data(include_bytes!("../ui/fonts/amiri.ttf").to_vec());
        fonts
            .db_mut()
            .load_font_data(include_bytes!("../ui/fonts/shantell-regular.ttf").to_vec());
        fonts
            .db_mut()
            .load_font_data(include_bytes!("../ui/fonts/shantell-bold.ttf").to_vec());
        fonts
            .db_mut()
            .load_font_data(include_bytes!("../ui/fonts/shantell-italic.ttf").to_vec());
        fonts
            .db_mut()
            .load_font_data(include_bytes!("../ui/fonts/shantell-bold-italic.ttf").to_vec());
        Self {
            fonts,
            cache: SwashCache::new(),
        }
    }
    fn layout(&mut self, doc: &Document, size: f32, color: &str) -> Editor<'static> {
        let size = size.clamp(1.0, 64.0);
        let mut buffer = Buffer::new(&mut self.fonts, Metrics::new(size, size * 1.4));
        buffer.set_size((doc.width > 0.0).then_some(doc.width.max(16.0)), None);
        buffer.set_wrap(if doc.width > 0.0 {
            Wrap::WordOrGlyph
        } else {
            Wrap::None
        });
        let default = Attrs::new()
            .family(Family::Name("Inter"))
            .color(parse_color(color));
        buffer.set_rich_text(
            doc.spans.iter().enumerate().map(|(index, span)| {
                let f = &span.format;
                let family = if f.code {
                    Family::Monospace
                } else {
                    match f.font {
                        1 => Family::Serif,
                        2 => Family::Monospace,
                        3 => Family::Name("Shantell Sans Informal"),
                        _ => Family::Name("Inter"),
                    }
                };
                let heading = match f.heading {
                    1 => 2.0,
                    2 => 1.5,
                    3 => 1.25,
                    _ => 1.0,
                };
                let mut attrs = default
                    .clone()
                    .family(family)
                    .metadata(index)
                    .weight(match (f.font == 3 && !f.code, f.bold || f.heading > 0) {
                        // The reference's static Informal faces use these actual weights.
                        (true, false) => Weight(611),
                        (true, true) => Weight(800),
                        (false, true) => Weight::BOLD,
                        (false, false) => Weight::NORMAL,
                    })
                    .style(if f.italic {
                        cosmic_text::Style::Italic
                    } else {
                        cosmic_text::Style::Normal
                    })
                    .metrics(Metrics::new(size * heading, size * heading * 1.4));
                if f.underline || !f.link.is_empty() {
                    attrs = attrs.underline(UnderlineStyle::Single);
                }
                if f.strike {
                    attrs = attrs.strikethrough();
                }
                if !f.link.is_empty() {
                    attrs = attrs.color(Color::rgb(182, 190, 255));
                }
                (span.text.as_str(), attrs)
            }),
            &default,
            Shaping::Advanced,
            match doc.alignment {
                1 => Some(cosmic_text::Align::Left),
                2 => Some(cosmic_text::Align::Center),
                3 => Some(cosmic_text::Align::Right),
                _ => None,
            },
        );
        // Rich-text paragraph iteration omits a trailing empty paragraph.
        // The editing cursor still needs a line after every explicit newline.
        while buffer.lines.len() < doc.text().split('\n').count() {
            buffer.lines.push(cosmic_text::BufferLine::new(
                "",
                cosmic_text::LineEnding::None,
                cosmic_text::AttrsList::new(&default),
                Shaping::Advanced,
            ));
        }
        buffer.shape_until_scroll(&mut self.fonts, false);
        Editor::new(buffer)
    }
    fn draw(
        &mut self,
        editor: &mut Editor<'static>,
        doc: &Document,
        color: &str,
        editing: bool,
    ) -> Rendered {
        editor.shape_as_needed(&mut self.fonts, false);
        let (width, height) = editor.with_buffer(|buffer| {
            buffer
                .layout_runs()
                .fold((16.0_f32, 1.0_f32), |(w, h), run| {
                    (
                        w.max(run.line_w + 2.0),
                        h.max(run.line_top + run.line_height),
                    )
                })
        });
        let width = if doc.width > 0.0 {
            doc.width.max(16.0)
        } else {
            width
        };
        // Keep raster allocation bounded for long documents; logical layout remains complete.
        let raster_scale = (4096.0 / width.max(height)).min(doc.scale.max(1.0) * 2.0);
        let mut pixels = SharedPixelBuffer::<Rgba8Pixel>::new(
            (width * raster_scale).ceil().max(1.0) as u32,
            (height * raster_scale).ceil().max(1.0) as u32,
        );
        editor.with_buffer(|buffer| {
            for run in buffer.layout_runs() {
                for glyph in run.glyphs {
                    if doc
                        .spans
                        .get(glyph.metadata)
                        .is_some_and(|s| s.format.highlight)
                    {
                        paint(
                            &mut pixels,
                            raster_scale,
                            glyph.x as i32,
                            run.line_top as i32,
                            glyph.w.ceil() as u32,
                            run.line_height.ceil() as u32,
                            Color::rgba(180, 137, 30, 150),
                        );
                    }
                }
            }
        });
        let transparent = Color::rgba(0, 0, 0, 0);
        editor.render(
            &mut TextRaster {
                fonts: &mut self.fonts,
                cache: &mut self.cache,
                pixels: &mut pixels,
                scale: raster_scale,
            },
            parse_color(color),
            if editing {
                Color::rgb(239, 240, 255)
            } else {
                transparent
            },
            Color::rgba(89, 102, 210, 170),
            Color::rgb(255, 255, 255),
        );
        Rendered {
            pixels,
            width,
            height,
        }
    }
}
// Rasterize glyphs at display resolution while keeping selection geometry in Canvas units.
struct TextRaster<'a> {
    fonts: &'a mut FontSystem,
    cache: &'a mut SwashCache,
    pixels: &'a mut SharedPixelBuffer<Rgba8Pixel>,
    scale: f32,
}
impl cosmic_text::Renderer for TextRaster<'_> {
    fn rectangle(&mut self, x: i32, y: i32, w: u32, h: u32, color: Color) {
        paint(self.pixels, self.scale, x, y, w, h, color);
    }
    fn glyph(&mut self, glyph: cosmic_text::PhysicalGlyph, color: Color) {
        let key = glyph.cache_key;
        let (key, x, y) = cosmic_text::CacheKey::new(
            key.font_id,
            key.glyph_id,
            f32::from_bits(key.font_size_bits) * self.scale,
            (
                (glyph.x as f32 + key.x_bin.as_float()) * self.scale,
                (glyph.y as f32 + key.y_bin.as_float()) * self.scale,
            ),
            key.font_weight,
            key.flags,
        );
        self.cache
            .with_pixels(self.fonts, key, color, |dx, dy, color| {
                paint(self.pixels, 1.0, x + dx, y + dy, 1, 1, color);
            });
    }
}
fn paint(
    pixels: &mut SharedPixelBuffer<Rgba8Pixel>,
    scale: f32,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    color: Color,
) {
    if color.a() == 0 {
        return;
    }
    let w = pixels.width() as usize;
    let h = pixels.height() as usize;
    let x1 = (x as f32 * scale).floor().max(0.0) as usize;
    let y1 = (y as f32 * scale).floor().max(0.0) as usize;
    let x2 = ((x as f32 + width as f32) * scale).ceil().max(0.0) as usize;
    let y2 = ((y as f32 + height as f32) * scale).ceil().max(0.0) as usize;
    let data = pixels.make_mut_slice();
    for row in y1.min(h)..y2.min(h) {
        for col in x1.min(w)..x2.min(w) {
            let p = &mut data[row * w + col];
            let a = u32::from(color.a());
            let old = u32::from(p.a) * (255 - a) / 255;
            let out = a + old;
            p.r = ((u32::from(color.r()) * a + u32::from(p.r) * old) / out) as u8;
            p.g = ((u32::from(color.g()) * a + u32::from(p.g) * old) / out) as u8;
            p.b = ((u32::from(color.b()) * a + u32::from(p.b) * old) / out) as u8;
            p.a = out as u8;
        }
    }
}
fn parse_color(value: &str) -> Color {
    let rgb = u32::from_str_radix(value.trim_start_matches('#'), 16).unwrap_or(0xe8e5ef);
    Color::rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}
#[derive(Clone)]
pub struct Rendered {
    pub pixels: SharedPixelBuffer<Rgba8Pixel>,
    pub width: f32,
    pub height: f32,
}
thread_local! {
    static RENDERER: RefCell<Renderer> = RefCell::new(Renderer::new());
    static CACHE: RefCell<Vec<(String,Rendered)>> = const { RefCell::new(Vec::new()) };
}
pub fn cached_render(doc: &Document, size: f32, color: &str) -> Rendered {
    let key = format!(
        "{size}:{color}:{}",
        serde_json::to_string(doc).unwrap_or_default()
    );
    CACHE.with_borrow_mut(|cache| {
        if let Some((_, view)) = cache.iter().find(|(saved, _)| saved == &key) {
            return view.clone();
        }
        let view = render(doc, size, color);
        while !cache.is_empty()
            && (cache.len() >= 32
                || cache
                    .iter()
                    .map(|(_, image)| image.pixels.as_bytes().len())
                    .sum::<usize>()
                    + view.pixels.as_bytes().len()
                    > 32 * 1024 * 1024)
        {
            cache.remove(0);
        }
        cache.push((key, view.clone()));
        view
    })
}
pub fn render(doc: &Document, size: f32, color: &str) -> Rendered {
    RENDERER.with_borrow_mut(|r| {
        let mut editor = r.layout(doc, size, color);
        r.draw(&mut editor, doc, color, false)
    })
}
pub fn bounds(width: f32, height: f32, doc: &Document) -> (usize, usize) {
    let (s, c) = doc.rotation.to_radians().sin_cos();
    (
        (doc.scale * (width * c.abs() + height * s.abs()))
            .ceil()
            .max(1.0) as usize,
        (doc.scale * (width * s.abs() + height * c.abs()))
            .ceil()
            .max(1.0) as usize,
    )
}
#[derive(Clone)]
struct Snapshot {
    doc: Document,
    cursor: usize,
    anchor: usize,
}
struct Gesture {
    doc: Document,
    x: f32,
    y: f32,
    pointer_x: f32,
    pointer_y: f32,
    width: f32,
    height: f32,
}
struct Session {
    doc: Document,
    editor: Editor<'static>,
    cursor: usize,
    anchor: usize,
    format: Format,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    original: Document,
    gesture: Option<Gesture>,
    clipboard: Vec<(char, Format)>,
    pasting: bool,
}
impl Session {
    fn checkpoint(&mut self) {
        if self.undo.len() == 20 {
            self.undo.remove(0);
        }
        self.undo.push(Snapshot {
            doc: self.doc.clone(),
            cursor: self.cursor,
            anchor: self.anchor,
        });
        self.redo.clear();
    }
    fn selection(&self) -> (usize, usize) {
        (self.cursor.min(self.anchor), self.cursor.max(self.anchor))
    }
    fn refresh(&mut self, ui: &Launcher) {
        let state = ui.global::<CanvasText>();
        RENDERER.with_borrow_mut(|r| {
            self.editor = r.layout(&self.doc, state.get_size(), &state.get_color());
            let text = self.doc.text();
            self.cursor = self.cursor.min(text.len());
            self.anchor = self.anchor.min(text.len());
            self.editor.set_cursor(to_cursor(&text, self.cursor));
            self.editor.set_selection(if self.cursor == self.anchor {
                Selection::None
            } else {
                Selection::Normal(to_cursor(&text, self.anchor))
            });
            let view = r.draw(
                &mut self.editor,
                &self.doc,
                &state.get_color(),
                state.get_editing(),
            );
            state.set_picture(slint::Image::from_rgba8(view.pixels));
            state.set_text_width(view.width);
            state.set_text_height(view.height);
        });
        state.set_scale(self.doc.scale);
        state.set_rotation(self.doc.rotation);
        state.set_fixed_width(self.doc.width);
        state.set_document(serde_json::to_string(&self.doc).unwrap_or_default().into());
        state.set_text(self.doc.text().into());
        state.set_cursor(self.cursor as i32);
        state.set_anchor(self.anchor as i32);
        state.set_link(self.format.link.clone().into());
        state.set_bold(self.format.bold);
        state.set_italic(self.format.italic);
        state.set_underline(self.format.underline);
        state.set_strike(self.format.strike);
        state.set_highlight(self.format.highlight);
        state.set_font(i32::from(self.format.font));
        ui.invoke_sync_canvas_text();
    }
    fn sync_format(&mut self) {
        let text = self.doc.text();
        let index = text[..self.cursor.min(text.len())].chars().count();
        if let Some((_, format)) = self.doc.characters().get(index.saturating_sub(1)) {
            self.format = format.clone();
        }
    }
}
fn to_cursor(text: &str, offset: usize) -> Cursor {
    let prefix = &text[..offset];
    Cursor::new(
        prefix.bytes().filter(|b| *b == b'\n').count(),
        prefix.rsplit('\n').next().unwrap_or("").len(),
    )
}
fn from_cursor(text: &str, cursor: Cursor) -> usize {
    text.split_inclusive('\n')
        .take(cursor.line)
        .map(str::len)
        .sum::<usize>()
        + cursor.index
}

pub fn connect(
    ui: &Launcher,
    save: impl Fn(String, Document, crate::ink::Style, i32, i32) + 'static,
) {
    RENDERER.with(|_| {});
    let save = Rc::new(save);
    let session: Rc<RefCell<Option<Session>>> = Rc::new(RefCell::new(None));
    let weak = ui.as_weak();
    let current = session.clone();
    ui.global::<CanvasText>().on_open(
        move |id, json, text, size, color, opacity, x, y, editing| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let state = ui.global::<CanvasText>();
            let doc = serde_json::from_str::<Document>(&json)
                .ok()
                .filter(Document::valid)
                .unwrap_or_else(|| Document::plain(&text));
            let editor = RENDERER.with_borrow_mut(|r| r.layout(&doc, size, &color));
            let cursor = doc.text().len();
            let clipboard = current
                .borrow()
                .as_ref()
                .map(|session| session.clipboard.clone())
                .unwrap_or_default();
            *current.borrow_mut() = Some(Session {
                original: doc.clone(),
                doc,
                editor,
                cursor,
                anchor: cursor,
                format: Format {
                    font: if id.is_empty() && text.is_empty() {
                        3
                    } else {
                        0
                    },
                    ..Format::default()
                },
                undo: Vec::new(),
                redo: Vec::new(),
                gesture: None,
                clipboard,
                pasting: false,
            });
            state.set_moved(false);
            state.set_placement_valid(true);
            state.set_saving(false);
            state.set_id(id);
            state.set_size(size);
            state.set_color(color);
            state.set_opacity(opacity);
            state.set_x(x);
            state.set_y(y);
            state.set_editing(editing);
            state.set_active(true);
            state.set_error("".into());
            let mut current = current.borrow_mut();
            let s = current.as_mut().unwrap();
            s.sync_format();
            s.refresh(&ui);
        },
    );
    let weak = ui.as_weak();
    let current = session.clone();
    ui.global::<CanvasText>().on_edit(move || {
        if let Some(ui) = weak.upgrade()
            && let Some(s) = current.borrow_mut().as_mut()
        {
            ui.global::<CanvasText>().set_editing(true);
            s.refresh(&ui);
        }
    });
    let weak = ui.as_weak();
    let current = session.clone();
    ui.global::<CanvasText>()
        .on_changed(move |text, cursor, anchor| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let mut session = current.borrow_mut();
            let Some(s) = session.as_mut() else {
                return;
            };
            if text.chars().count() > 10000 {
                ui.global::<CanvasText>()
                    .set_error("Text is limited to 10,000 characters".into());
                s.refresh(&ui);
                return;
            }
            let cursor = text
                .get(..cursor.max(0) as usize)
                .map_or(0, |prefix| normalize_text(prefix).len()) as i32;
            let anchor = text
                .get(..anchor.max(0) as usize)
                .map_or(0, |prefix| normalize_text(prefix).len()) as i32;
            let text = normalize_text(&text);
            let before = s.doc.text();
            if before != text.as_str() {
                s.checkpoint();
                let prefix = before
                    .chars()
                    .zip(text.chars())
                    .take_while(|(a, b)| a == b)
                    .map(|(a, _)| a.len_utf8())
                    .sum::<usize>();
                let suffix = before[prefix..]
                    .chars()
                    .rev()
                    .zip(text[prefix..].chars().rev())
                    .take_while(|(a, b)| a == b)
                    .map(|(a, _)| a.len_utf8())
                    .sum::<usize>();
                let insert = &text[prefix..text.len() - suffix];
                s.doc
                    .replace(prefix, before.len() - suffix, insert, &s.format);
                if s.pasting && s.clipboard.iter().map(|(c, _)| *c).collect::<String>() == insert {
                    let from = text[..prefix].chars().count();
                    let mut characters = s.doc.characters();
                    characters.splice(from..from + insert.chars().count(), s.clipboard.clone());
                    s.doc.set_characters(characters);
                }
                s.pasting = false;
            }
            let cursor = cursor.max(0) as usize;
            let anchor = anchor.max(0) as usize;
            if text.is_char_boundary(cursor) && text.is_char_boundary(anchor) {
                s.cursor = cursor;
                s.anchor = anchor;
            }
            s.refresh(&ui);
        });
    let weak = ui.as_weak();
    let current = session.clone();
    ui.global::<CanvasText>()
        .on_selection(move |cursor, anchor| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let Ok(mut session) = current.try_borrow_mut() else {
                return;
            };
            let Some(s) = session.as_mut() else {
                return;
            };
            let text = s.doc.text();
            let cursor = cursor.max(0) as usize;
            let anchor = anchor.max(0) as usize;
            if (cursor, anchor) != (s.cursor, s.anchor)
                && text.is_char_boundary(cursor)
                && text.is_char_boundary(anchor)
            {
                s.cursor = cursor;
                s.anchor = anchor;
                s.sync_format();
                s.refresh(&ui);
            }
        });
    let weak = ui.as_weak();
    let current = session.clone();
    ui.global::<CanvasText>().on_pointer(move |kind, x, y| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let mut session = current.borrow_mut();
        let Some(s) = session.as_mut() else {
            return;
        };
        let action = match kind {
            1 => cosmic_text::Action::Drag {
                x: x as i32,
                y: y as i32,
            },
            2 => cosmic_text::Action::DoubleClick {
                x: x as i32,
                y: y as i32,
            },
            _ => cosmic_text::Action::Click {
                x: x as i32,
                y: y as i32,
            },
        };
        RENDERER.with_borrow_mut(|r| s.editor.action(&mut r.fonts, action));
        let text = s.doc.text();
        s.cursor = from_cursor(&text, s.editor.cursor());
        s.anchor = s.editor.selection_bounds().map_or(s.cursor, |(a, b)| {
            let a = from_cursor(&text, a);
            let b = from_cursor(&text, b);
            if a == s.cursor { b } else { a }
        });
        s.sync_format();
        if kind == 3
            && valid_link(&s.format.link)
            && let Err(error) = crate::platform::open(&s.format.link)
        {
            ui.global::<CanvasText>()
                .set_error(error.to_string().into());
        }
        s.refresh(&ui);
    });
    let weak = ui.as_weak();
    let current = session.clone();
    ui.global::<CanvasText>().on_command(move |command, value| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let mut session = current.borrow_mut();
        let Some(s) = session.as_mut() else {
            return;
        };
        command_session(s, &ui, &command, &value);
    });
    let weak = ui.as_weak();
    let current = session.clone();
    let save_drag = save.clone();
    ui.global::<CanvasText>()
        .on_drag(move |handle, phase, x, y, snap| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let mut current = current.borrow_mut();
            let Some(s) = current.as_mut() else {
                return;
            };
            drag_session(s, &ui, handle, phase, x, y, snap);
            if phase == 2 && ui.global::<CanvasText>().get_moved() {
                let state = ui.global::<CanvasText>();
                state.set_keep_selection(true);
                state.set_saving(true);
                save_drag(
                    state.get_id().to_string(),
                    s.doc.clone(),
                    crate::ink::Style {
                        size: state.get_size(),
                        color: state.get_color().to_string(),
                        opacity: state.get_opacity(),
                        ..Default::default()
                    },
                    state.get_x().round() as i32,
                    state.get_y().round() as i32,
                );
            }
        });
    let weak = ui.as_weak();
    let current = session.clone();
    ui.global::<CanvasText>()
        .on_navigate(move |key, shift, control| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let mut current = current.borrow_mut();
            let Some(s) = current.as_mut() else {
                return;
            };
            use cosmic_text::Motion;
            let key = key.chars().next().unwrap_or(' ');
            let motion = if key == char::from(slint::platform::Key::LeftArrow) {
                if control {
                    Motion::LeftWord
                } else {
                    Motion::Left
                }
            } else if key == char::from(slint::platform::Key::RightArrow) {
                if control {
                    Motion::RightWord
                } else {
                    Motion::Right
                }
            } else if key == char::from(slint::platform::Key::UpArrow) {
                Motion::Up
            } else if key == char::from(slint::platform::Key::DownArrow) {
                Motion::Down
            } else if key == char::from(slint::platform::Key::Home) {
                if control {
                    Motion::BufferStart
                } else {
                    Motion::Home
                }
            } else {
                if control {
                    Motion::BufferEnd
                } else {
                    Motion::End
                }
            };
            RENDERER.with_borrow_mut(|r| {
                s.editor
                    .action(&mut r.fonts, cosmic_text::Action::Motion(motion))
            });
            s.cursor = from_cursor(&s.doc.text(), s.editor.cursor());
            if !shift {
                s.anchor = s.cursor;
            }
            s.sync_format();
            s.refresh(&ui);
        });
    let weak = ui.as_weak();
    let current = session.clone();
    ui.global::<CanvasText>().on_open_link(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let current = current.borrow();
        let Some(s) = current.as_ref() else {
            return;
        };
        if valid_link(&s.format.link)
            && let Err(error) = crate::platform::open(&s.format.link)
        {
            ui.global::<CanvasText>()
                .set_error(error.to_string().into());
        }
    });
    let weak = ui.as_weak();
    let current = session.clone();
    ui.global::<CanvasText>().on_saved(move || {
        if let Some(ui) = weak.upgrade()
            && let Some(s) = current.borrow_mut().as_mut()
        {
            s.original = s.doc.clone();
            ui.global::<CanvasText>().set_moved(false);
        }
    });
    let weak = ui.as_weak();
    let current = session;
    ui.global::<CanvasText>().on_finish(move |cancel| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let state = ui.global::<CanvasText>();
        let mut session = current.borrow_mut();
        let Some(s) = session.as_mut() else {
            return;
        };
        if s.gesture.is_some() {
            drag_session(s, &ui, 0, 3, 0.0, 0.0, false);
            return;
        }
        if state.get_saving() {
            return;
        }
        state.set_keep_selection(false);
        if cancel {
            state.set_active(false);
            state.set_editing(false);
            return;
        }
        if !s.doc.valid() {
            state.set_error("Text formatting is invalid".into());
            return;
        }
        if state.get_id().is_empty() && s.doc.text().trim().is_empty() {
            state.set_active(false);
            state.set_editing(false);
            return;
        }
        if s.doc == s.original && !state.get_id().is_empty() && !state.get_moved() {
            state.set_active(false);
            state.set_editing(false);
            return;
        }
        state.set_saving(true);
        save(
            state.get_id().to_string(),
            s.doc.clone(),
            crate::ink::Style {
                size: state.get_size(),
                color: state.get_color().to_string(),
                opacity: state.get_opacity(),
                ..Default::default()
            },
            state.get_x().round() as i32,
            state.get_y().round() as i32,
        );
    });
}
fn command_session(s: &mut Session, ui: &Launcher, command: &str, value: &str) {
    if command == "copy" {
        let text = s.doc.text();
        let (from, to) = s.selection();
        s.clipboard = s
            .doc
            .characters()
            .into_iter()
            .skip(text[..from].chars().count())
            .take(text[from..to].chars().count())
            .collect();
        return;
    }
    if command == "paste" {
        s.pasting = true;
        return;
    }
    if command == "undo" || command == "redo" {
        let (source, target) = if command == "undo" {
            (&mut s.undo, &mut s.redo)
        } else {
            (&mut s.redo, &mut s.undo)
        };
        if let Some(previous) = source.pop() {
            target.push(Snapshot {
                doc: s.doc.clone(),
                cursor: s.cursor,
                anchor: s.anchor,
            });
            s.doc = previous.doc;
            s.cursor = previous.cursor;
            s.anchor = previous.anchor;
            s.sync_format();
        }
        s.refresh(ui);
        return;
    }
    let text = s.doc.text();
    let (mut start, mut end) = if ui.global::<CanvasText>().get_editing() {
        s.selection()
    } else {
        (0, text.len())
    };
    if command == "heading" {
        start = text[..start].rfind('\n').map_or(0, |i| i + 1);
        end = text[end..].find('\n').map_or(text.len(), |i| end + i);
    }
    if command == "link" && !value.is_empty() && !valid_link(value) {
        ui.global::<CanvasText>()
            .set_error("Use an https://, http:// or mailto: link".into());
        return;
    }
    s.checkpoint();
    match command {
        "enter" => {
            let line = &text[..start].rsplit('\n').next().unwrap_or("");
            let prefix = list_prefix(line);
            if !prefix.is_empty() && line.trim() == prefix.trim() {
                s.doc.replace(start - prefix.len(), end, "", &s.format);
                s.cursor = start - prefix.len();
            } else {
                let insert = format!("\n{}", next_prefix(&prefix));
                s.doc.replace(start, end, &insert, &s.format);
                s.cursor = start + insert.len();
            }
            s.anchor = s.cursor;
        }
        "bullet" | "number" | "indent" | "outdent" => {
            edit_list(s, &text, start, end, command);
        }
        "align" => s.doc.alignment = value.parse::<u8>().unwrap_or(0).min(3),
        "auto" => s.doc.width = 0.0,
        _ => {
            let apply = |format: &mut Format| match command {
                "bold" => format.bold = !s.format.bold,
                "italic" => format.italic = !s.format.italic,
                "underline" => format.underline = !s.format.underline,
                "strike" => format.strike = !s.format.strike,
                "highlight" => format.highlight = !s.format.highlight,
                "code" => format.code = !s.format.code,
                "font" => format.font = value.parse::<u8>().unwrap_or(0).min(3),
                "heading" => format.heading = value.parse::<u8>().unwrap_or(0).min(3),
                "link" => format.link = value.into(),
                "clear" => *format = Format::default(),
                _ => (),
            };
            let mut next = s.format.clone();
            apply(&mut next);
            let mut chars = s.doc.characters();
            let mut offset = 0;
            for (c, format) in &mut chars {
                if offset >= start && offset < end {
                    apply(format);
                }
                offset += c.len_utf8();
            }
            s.doc.set_characters(chars);
            s.format = next;
        }
    }
    s.refresh(ui);
}
fn list_prefix(line: &str) -> String {
    let indent = line.len() - line.trim_start_matches(' ').len();
    let rest = &line[indent..];
    if rest.starts_with("• ") || rest.starts_with("- ") || rest.starts_with("* ") {
        return line[..indent + rest.chars().next().unwrap().len_utf8() + 1].into();
    }
    if let Some((number, _)) = rest.split_once(". ")
        && !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
    {
        return line[..indent + number.len() + 2].into();
    }
    String::new()
}
fn next_prefix(prefix: &str) -> String {
    let trimmed = prefix.trim_start();
    if let Some(number) = trimmed
        .strip_suffix(". ")
        .and_then(|n| n.parse::<u64>().ok())
    {
        format!(
            "{}{}. ",
            &prefix[..prefix.len() - trimmed.len()],
            number.saturating_add(1)
        )
    } else {
        prefix.into()
    }
}

fn drag_session(
    s: &mut Session,
    ui: &Launcher,
    handle: i32,
    phase: i32,
    x: f32,
    y: f32,
    snap: bool,
) {
    let state = ui.global::<CanvasText>();
    if phase == 0 {
        s.gesture = Some(Gesture {
            doc: s.doc.clone(),
            x: state.get_x(),
            y: state.get_y(),
            pointer_x: x,
            pointer_y: y,
            width: state.get_text_width(),
            height: state.get_text_height(),
        });
        return;
    }
    let Some(g) = s.gesture.as_ref() else {
        return;
    };
    if phase == 3 {
        s.doc = g.doc.clone();
        state.set_x(g.x);
        state.set_y(g.y);
        s.gesture = None;
        s.refresh(ui);
        return;
    }
    let dx = x - g.pointer_x;
    let dy = y - g.pointer_y;
    let (sin, cos) = g.doc.rotation.to_radians().sin_cos();
    let local_x = dx * cos + dy * sin;
    let local_y = -dx * sin + dy * cos;
    let (bw, bh) = bounds(g.width, g.height, &g.doc);
    let mut cx = g.x + bw as f32 / 2.0;
    let mut cy = g.y + bh as f32 / 2.0;
    let mut doc = g.doc.clone();
    if handle == 0 {
        cx += dx;
        cy += dy;
    } else if handle == 9 {
        let gap = ui.get_grid_gap() as f32 / 2.0;
        let start = (g.pointer_y - cy - gap).atan2(g.pointer_x - cx - gap);
        let end = (y - cy - gap).atan2(x - cx - gap);
        doc.rotation = (g.doc.rotation + (end - start).to_degrees() + 540.0) % 360.0 - 180.0;
        if snap {
            doc.rotation = (doc.rotation / 15.0).round() * 15.0;
        }
    } else {
        let hx = if matches!(handle, 1 | 3 | 5) {
            -1.0
        } else if matches!(handle, 7 | 8) {
            0.0
        } else {
            1.0
        };
        let hy = if matches!(handle, 1 | 2 | 7) {
            -1.0
        } else if matches!(handle, 5 | 6) {
            0.0
        } else {
            1.0
        };
        let old_w = g.width * g.doc.scale;
        let old_h = g.height * g.doc.scale;
        let (new_w, new_h) = if matches!(handle, 5 | 6) {
            doc.width = ((old_w + hx * local_x) / doc.scale).clamp(16.0, 16000.0);
            let view = render(&doc, state.get_size(), &state.get_color());
            (view.width * doc.scale, view.height * doc.scale)
        } else {
            let denominator = (old_w * hx).powi(2) + (old_h * hy).powi(2);
            let factor = 1.0 + (local_x * old_w * hx + local_y * old_h * hy) / denominator.max(1.0);
            doc.scale = (g.doc.scale * factor).clamp(0.1, 20.0);
            (g.width * doc.scale, g.height * doc.scale)
        };
        let shift_x = (new_w - old_w) * hx / 2.0;
        let shift_y = (new_h - old_h) * if matches!(handle, 5 | 6) { 1.0 } else { hy } / 2.0;
        cx += shift_x * cos - shift_y * sin;
        cy += shift_x * sin + shift_y * cos;
    }
    s.doc = doc;
    s.refresh(ui);
    let (bw, bh) = bounds(state.get_text_width(), state.get_text_height(), &s.doc);
    state.set_x(cx - bw as f32 / 2.0);
    state.set_y(cy - bh as f32 / 2.0);
    state.set_moved(true);
    let valid = ui.invoke_preview_valid(
        ui.get_items(),
        state.get_id(),
        state.get_x().round() as i32,
        state.get_y().round() as i32,
        bw as i32 + ui.get_grid_gap(),
        bh as i32 + ui.get_grid_gap(),
    );
    state.set_placement_valid(valid);
    if phase == 2 {
        if !valid && let Some(g) = s.gesture.take() {
            s.doc = g.doc;
            state.set_x(g.x);
            state.set_y(g.y);
            state.set_moved(false);
            state.set_placement_valid(true);
            state.set_error("Text does not fit here".into());
            s.refresh(ui);
        }
        s.gesture = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_font_uses_bundled_shantell_face() {
        let mut doc = Document::plain("Hello");
        doc.spans[0].format.font = 3;
        let mut renderer = Renderer::new();
        let editor = renderer.layout(&doc, 24.0, "#ffffff");
        editor.with_buffer(|buffer| {
            for run in buffer.layout_runs() {
                for glyph in run.glyphs {
                    let face = renderer.fonts.db().face(glyph.font_id).unwrap();
                    assert!(
                        face.families
                            .iter()
                            .any(|(family, _)| family == "Shantell Sans Informal"),
                        "{:?}",
                        face.families
                    );
                }
            }
        });
    }

    #[test]
    fn trailing_blank_lines_and_unicode_cursor_remain_editable() {
        let doc = Document::plain("مرحبا\r\nOrbit\u{2028}\n");
        assert_eq!(doc.text(), "مرحبا\nOrbit\n\n");
        assert!(doc.valid());
        let mut renderer = Renderer::new();
        let mut editor = renderer.layout(&doc, 24.0, "#ffffff");
        editor.set_cursor(to_cursor(&doc.text(), doc.text().len()));
        editor.shape_as_needed(&mut renderer.fonts, false);
        assert_eq!(editor.cursor().line, 3);
        assert_eq!(from_cursor(&doc.text(), editor.cursor()), doc.text().len());
    }

    #[test]
    fn replacing_unicode_keeps_surrounding_formats() {
        let bold = Format {
            bold: true,
            ..Format::default()
        };
        let mut doc = Document {
            spans: vec![
                Span {
                    text: "مرحبا".into(),
                    format: bold.clone(),
                },
                Span {
                    text: " world".into(),
                    format: Format::default(),
                },
            ],
            ..Document::default()
        };
        doc.replace(
            "مرحبا ".len(),
            "مرحبا world".len(),
            "Orbit",
            &Format::default(),
        );
        assert_eq!(doc.text(), "مرحبا Orbit");
        assert_eq!(doc.spans[0].format, bold);
        let restored: Document =
            serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
        assert_eq!(doc, restored);
        assert!(restored.valid());
    }

    #[test]
    fn lists_continue_and_nested_lists_keep_their_indent() {
        assert_eq!(next_prefix(&list_prefix("  9. nine")), "  10. ");
        assert_eq!(next_prefix(&list_prefix("    • bullet")), "    • ");
        assert_eq!(list_prefix("plain text"), "");
        assert_eq!(list_prefix("10.5 decimals"), "");
    }

    #[test]
    fn invalid_document_geometry_and_unsafe_links_are_rejected() {
        let mut doc = Document::plain("Hello");
        doc.rotation = f32::NAN;
        assert!(!doc.valid());
        doc.rotation = 45.0;
        doc.scale = 0.0;
        assert!(!doc.valid());
        doc.scale = 1.0;
        doc.spans[0].format.link = "javascript:alert(1)".into();
        assert!(!doc.valid());
        doc.spans[0].format.link = "https://example.com".into();
        assert!(doc.valid());
        assert!(!valid_link("https://example.com\nfile:///tmp"));
    }

    #[test]
    fn rotated_bounds_cover_the_full_text() {
        let doc = Document {
            rotation: 90.0,
            scale: 2.0,
            ..Document::default()
        };
        let (width, height) = bounds(200.0, 40.0, &doc);
        assert!((80..=81).contains(&width));
        assert!((400..=401).contains(&height));
    }
}

fn edit_list(s: &mut Session, text: &str, start: usize, end: usize, command: &str) {
    let from = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let to = text[end..].find('\n').map_or(text.len(), |i| end + i);
    let mut offset = from;
    let mut edits = Vec::new();
    for (index, line) in text[from..to].split('\n').enumerate() {
        let prefix = list_prefix(line);
        let indent = line.len() - line.trim_start_matches(' ').len();
        let (removed, insert) = match command {
            "indent" => (0, "  ".into()),
            "outdent" => (indent.min(2), String::new()),
            "bullet" if prefix.trim() == "•" => (prefix.len(), line[..indent].into()),
            "bullet" => (prefix.len().max(indent), format!("{}• ", &line[..indent])),
            _ => (
                prefix.len().max(indent),
                format!("{}{}. ", &line[..indent], index + 1),
            ),
        };
        edits.push((offset, removed, insert));
        offset += line.len() + 1;
    }
    let delta = edits
        .iter()
        .map(|(_, removed, insert)| insert.len() as isize - *removed as isize)
        .sum::<isize>();
    for (offset, removed, insert) in edits.into_iter().rev() {
        s.doc.replace(offset, offset + removed, &insert, &s.format);
    }
    s.cursor = to.saturating_add_signed(delta);
    s.anchor = if start == end { s.cursor } else { from };
}
