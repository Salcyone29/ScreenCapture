//! Modèle des annotations de l'éditeur (coordonnées en pixels de l'image).

use eframe::egui::{pos2, Pos2, Rect, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tool {
    Select,
    Arrow,
    Line,
    Rect,
    Ellipse,
    Highlight,
    Blur,
    Text,
    Bubble,
    Number,
    Pen,
    Magnifier,
    Stamp,
    Crop,
    Cut,
    MoveArea,
    Picker,
}

impl Tool {
    pub const ALL: [Tool; 17] = [
        Tool::Select,
        Tool::Arrow,
        Tool::Line,
        Tool::Rect,
        Tool::Ellipse,
        Tool::Highlight,
        Tool::Blur,
        Tool::Text,
        Tool::Bubble,
        Tool::Number,
        Tool::Pen,
        Tool::Magnifier,
        Tool::Stamp,
        Tool::Crop,
        Tool::Cut,
        Tool::MoveArea,
        Tool::Picker,
    ];

    pub fn icon(self) -> &'static str {
        use egui_material_icons::icons::*;
        match self {
            Tool::Select => ICON_NEAR_ME,
            Tool::Arrow => ICON_NORTH_EAST,
            Tool::Line => ICON_HORIZONTAL_RULE,
            Tool::Rect => ICON_CHECK_BOX_OUTLINE_BLANK,
            Tool::Ellipse => ICON_PANORAMA_FISH_EYE,
            Tool::Highlight => ICON_INK_HIGHLIGHTER,
            Tool::Blur => ICON_BLUR_ON,
            Tool::Text => ICON_TEXT_FIELDS,
            Tool::Bubble => ICON_CHAT_BUBBLE,
            Tool::Number => ICON_FORMAT_LIST_NUMBERED,
            Tool::Pen => ICON_DRAW,
            Tool::Magnifier => ICON_LOUPE,
            Tool::Stamp => ICON_APPROVAL,
            Tool::Crop => ICON_CROP,
            Tool::Cut => ICON_CONTENT_CUT,
            Tool::MoveArea => ICON_OPEN_WITH,
            Tool::Picker => ICON_COLORIZE,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Tool::Select => crate::i18n::tr("Sélection (V)"),
            Tool::Arrow => crate::i18n::tr("Flèche (A)"),
            Tool::Line => crate::i18n::tr("Ligne (L)"),
            Tool::Rect => crate::i18n::tr("Rectangle (R)"),
            Tool::Ellipse => crate::i18n::tr("Ellipse (E)"),
            Tool::Highlight => crate::i18n::tr("Surligneur (H)"),
            Tool::Blur => crate::i18n::tr("Flouter / pixeliser (B)"),
            Tool::Text => crate::i18n::tr("Texte (T)"),
            Tool::Bubble => crate::i18n::tr("Bulle de texte (U)"),
            Tool::Number => crate::i18n::tr("Numérotation (N)"),
            Tool::Pen => crate::i18n::tr("Main levée (P)"),
            Tool::Magnifier => crate::i18n::tr("Loupe (M)"),
            Tool::Stamp => crate::i18n::tr("Tampon (S)"),
            Tool::Crop => crate::i18n::tr("Rogner (C)"),
            Tool::Cut => crate::i18n::tr("Découper une bande (X)"),
            Tool::MoveArea => crate::i18n::tr("Déplacer une zone (D)"),
            Tool::Picker => crate::i18n::tr("Pipette (I)"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub color: [u8; 4],
    pub width: f32,
    pub filled: bool,
    pub shadow: bool,
    pub dashed: bool,
    pub font_size: f32,
    /// Forme de la bulle de texte.
    pub bubble: BubbleKind,
    /// Pointe de la bulle, relative au coin haut gauche de la bulle (`None` : position par défaut).
    pub tail: Option<Vec2>,
}

/// Forme d'une bulle de texte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BubbleKind {
    #[default]
    Rounded,
    Rectangle,
    Oval,
    Thought,
}

impl BubbleKind {
    pub const ALL: [BubbleKind; 4] = [BubbleKind::Rounded, BubbleKind::Rectangle, BubbleKind::Oval, BubbleKind::Thought];

    pub fn label(self) -> &'static str {
        match self {
            BubbleKind::Rounded => crate::i18n::tr("Arrondie"),
            BubbleKind::Rectangle => crate::i18n::tr("Rectangulaire"),
            BubbleKind::Oval => crate::i18n::tr("Ovale"),
            BubbleKind::Thought => crate::i18n::tr("Pensée"),
        }
    }
}

impl Style {
    /// Style initial propre à chaque outil (mémorisé ensuite outil par outil).
    pub fn for_tool(t: Tool) -> Self {
        let d = Self::default();
        match t {
            Tool::Highlight => Self { color: [255, 235, 59, 255], width: 4.0, shadow: false, ..d },
            Tool::Bubble => Self { color: [30, 136, 229, 255], ..d },
            Tool::Magnifier => Self { color: [66, 66, 66, 255], width: 3.0, ..d },
            Tool::Blur => Self { width: 4.0, shadow: false, ..d },
            Tool::Text => Self { shadow: false, ..d },
            _ => d,
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Self { color: [229, 57, 53, 255], width: 4.0, filled: false, shadow: true, dashed: false, font_size: 24.0, bubble: BubbleKind::Rounded, tail: None }
    }
}

/// Style d'incrément de l'outil Numérotation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NumberFormat {
    #[default]
    Decimal,
    Padded,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
    Greek,
}

const GREEK: [char; 24] = [
    'α', 'β', 'γ', 'δ', 'ε', 'ζ', 'η', 'θ', 'ι', 'κ', 'λ', 'μ', 'ν', 'ξ', 'ο', 'π', 'ρ', 'σ', 'τ', 'υ', 'φ', 'χ', 'ψ', 'ω',
];
const ROMAN: [(u32, &str); 13] = [
    (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
    (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
];

impl NumberFormat {
    pub const ALL: [NumberFormat; 7] = [
        NumberFormat::Decimal,
        NumberFormat::Padded,
        NumberFormat::UpperAlpha,
        NumberFormat::LowerAlpha,
        NumberFormat::UpperRoman,
        NumberFormat::LowerRoman,
        NumberFormat::Greek,
    ];

    /// Aperçu affiché sur la puce de choix.
    pub fn sample(self) -> &'static str {
        match self {
            NumberFormat::Decimal => "1, 2, 3",
            NumberFormat::Padded => "01, 02, 03",
            NumberFormat::LowerAlpha => "a, b, c",
            NumberFormat::UpperAlpha => "A, B, C",
            NumberFormat::LowerRoman => crate::i18n::tr("i, ii, iii"),
            NumberFormat::UpperRoman => crate::i18n::tr("I, II, III"),
            NumberFormat::Greek => "α, β, γ",
        }
    }

    /// Libellé de la valeur `n` (≥ 1) : lettres en base bijective (z → aa), romain jusqu'à 3999.
    pub fn format(self, n: u32) -> String {
        let n = n.max(1);
        match self {
            NumberFormat::Decimal => n.to_string(),
            NumberFormat::Padded => format!("{n:02}"),
            NumberFormat::LowerAlpha | NumberFormat::UpperAlpha => {
                let s = bijective(n, 26, |d| char::from(b'a' + d as u8));
                if self == NumberFormat::UpperAlpha { s.to_uppercase() } else { s }
            }
            NumberFormat::Greek => bijective(n, 24, |d| GREEK[d as usize]),
            NumberFormat::LowerRoman | NumberFormat::UpperRoman if n < 4000 => {
                let mut s = String::new();
                let mut r = n;
                for (v, sym) in ROMAN {
                    while r >= v {
                        s.push_str(sym);
                        r -= v;
                    }
                }
                if self == NumberFormat::LowerRoman { s.to_lowercase() } else { s }
            }
            _ => n.to_string(),
        }
    }

    /// Inverse de `format` (saisie manuelle) ; accepte aussi un nombre décimal quel que soit le style.
    pub fn parse(self, s: &str) -> Option<u32> {
        let s = s.trim();
        if let Ok(n) = s.parse::<u32>() {
            return (n >= 1).then_some(n);
        }
        let lower = s.to_lowercase();
        if lower.is_empty() {
            return None;
        }
        match self {
            NumberFormat::LowerAlpha | NumberFormat::UpperAlpha => {
                lower.chars().try_fold(0u32, |acc, c| {
                    c.is_ascii_lowercase().then_some(())?;
                    acc.checked_mul(26)?.checked_add(c as u32 - 'a' as u32 + 1)
                })
            }
            NumberFormat::Greek => lower.chars().try_fold(0u32, |acc, c| {
                let c = if c == 'ς' { 'σ' } else { c };
                let i = GREEK.iter().position(|g| *g == c)?;
                acc.checked_mul(24)?.checked_add(i as u32 + 1)
            }),
            NumberFormat::LowerRoman | NumberFormat::UpperRoman => {
                let up = lower.to_uppercase();
                let mut rest = up.as_str();
                let mut n = 0;
                for (v, sym) in ROMAN {
                    while let Some(r) = rest.strip_prefix(sym) {
                        n += v;
                        rest = r;
                    }
                }
                (rest.is_empty() && n > 0 && self.format(n).to_uppercase() == up).then_some(n)
            }
            _ => None,
        }
    }
}

fn bijective(mut n: u32, base: u32, digit: impl Fn(u32) -> char) -> String {
    let mut out = Vec::new();
    while n > 0 {
        n -= 1;
        out.push(digit(n % base));
        n /= base;
    }
    out.iter().rev().collect()
}

#[derive(Clone, Debug)]
pub enum Kind {
    Arrow,
    Line,
    Rect,
    Ellipse,
    Highlight,
    Blur,
    Magnifier,
    Text(String),
    Bubble(String),
    Number(u32, NumberFormat),
    Pen(Vec<Pos2>),
    Stamp(String),
    /// Zone de l'image déplacée : pixels copiés depuis `src` (vidée avec `fill`), affichés en a..b.
    Patch { img: std::sync::Arc<image::RgbaImage>, src: Rect, fill: [u8; 4] },
}

/// Sens de la bande retirée par l'outil Découper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutDir {
    Horizontal,
    Vertical,
}

/// Raccord entre les deux morceaux après découpe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutEdge {
    Straight,
    Zigzag,
}

#[derive(Clone, Debug)]
pub struct Shape {
    pub kind: Kind,
    pub a: Pos2,
    pub b: Pos2,
    pub style: Style,
}

impl Shape {
    pub fn rect(&self) -> Rect {
        match &self.kind {
            Kind::Pen(pts) => {
                let mut r = Rect::NOTHING;
                for p in pts {
                    r.extend_with(*p);
                }
                r
            }
            Kind::Number(n, f) => {
                let rad = number_radius(&self.style);
                Rect::from_center_size(self.a, Vec2::new(number_half_width(&self.style, &f.format(*n)) * 2.0, rad * 2.0))
            }
            _ => Rect::from_two_pos(self.a, self.b),
        }
    }

    /// Les formes à deux points exposent des poignées de redimensionnement.
    pub fn has_handles(&self) -> bool {
        !matches!(self.kind, Kind::Pen(_) | Kind::Number(..) | Kind::Text(_) | Kind::Stamp(_))
    }

    pub fn is_textual(&self) -> bool {
        matches!(self.kind, Kind::Text(_) | Kind::Bubble(_))
    }

    /// Formes dont le contenu se modifie par double-clic (texte, bulle, numéro, tampon).
    pub fn is_editable(&self) -> bool {
        matches!(self.kind, Kind::Text(_) | Kind::Bubble(_) | Kind::Number(..) | Kind::Stamp(_))
    }

    pub fn hit(&self, p: Pos2, tol: f32) -> bool {
        let tol = tol + self.style.width / 2.0;
        match &self.kind {
            Kind::Arrow | Kind::Line => seg_dist(p, self.a, self.b) <= tol,
            Kind::Pen(pts) => pts.windows(2).any(|w| seg_dist(p, w[0], w[1]) <= tol),
            Kind::Rect | Kind::Ellipse if !self.style.filled => {
                let r = self.rect();
                r.expand(tol).contains(p) && !r.shrink(tol).contains(p)
                    || (matches!(self.kind, Kind::Ellipse) && ellipse_border_hit(r, p, tol))
            }
            _ => self.rect().expand(tol).contains(p),
        }
    }

    pub fn translate(&mut self, d: Vec2) {
        self.a += d;
        self.b += d;
        if let Kind::Pen(pts) = &mut self.kind {
            for p in pts {
                *p += d;
            }
        }
    }

    pub fn map_points(&mut self, f: impl Fn(Pos2) -> Pos2) {
        self.a = f(self.a);
        self.b = f(self.b);
        if let Kind::Pen(pts) = &mut self.kind {
            for p in pts {
                *p = f(*p);
            }
        }
    }
}

pub fn number_radius(style: &Style) -> f32 {
    (style.font_size * 0.85).max(12.0)
}

/// Demi-largeur de la pastille : cercle pour un libellé court, gélule au-delà (ex. « VIII »).
pub fn number_half_width(style: &Style, label: &str) -> f32 {
    let rad = number_radius(style);
    let text_w = label.chars().count() as f32 * rad * 1.1 * 0.6;
    (text_w / 2.0 + rad * 0.3).max(rad)
}

fn seg_dist(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_sq();
    if len2 < 1e-6 {
        return (p - a).length();
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

fn ellipse_border_hit(r: Rect, p: Pos2, tol: f32) -> bool {
    let c = r.center();
    let (rx, ry) = (r.width() / 2.0, r.height() / 2.0);
    if rx < 1.0 || ry < 1.0 {
        return false;
    }
    let d = ((p.x - c.x) / rx).powi(2) + ((p.y - c.y) / ry).powi(2);
    let t = tol / rx.min(ry);
    (d.sqrt() - 1.0).abs() <= t
}

/// Contraint un segment à des multiples de 45° (Maj).
pub fn snap45(a: Pos2, b: Pos2) -> Pos2 {
    let d = b - a;
    let ang = d.y.atan2(d.x);
    let step = std::f32::consts::FRAC_PI_4;
    let ang = (ang / step).round() * step;
    let len = d.length();
    pos2(a.x + len * ang.cos(), a.y + len * ang.sin())
}

/// Contraint un rectangle à un carré (Maj).
pub fn snap_square(a: Pos2, b: Pos2) -> Pos2 {
    let d = b - a;
    let s = d.x.abs().max(d.y.abs());
    pos2(a.x + s * d.x.signum(), a.y + s * d.y.signum())
}

/// Tampons proposés, dans la langue de l'interface.
pub fn stamps() -> [&'static str; 10] {
    [
        crate::i18n::tr("APPROUVÉ"),
        crate::i18n::tr("VALIDÉ"),
        crate::i18n::tr("REFUSÉ"),
        crate::i18n::tr("CONFIDENTIEL"),
        crate::i18n::tr("BROUILLON"),
        crate::i18n::tr("À CORRIGER"),
        crate::i18n::tr("NOUVEAU"),
        crate::i18n::tr("IMPORTANT"),
        crate::i18n::tr("AVANT"),
        crate::i18n::tr("APRÈS"),
    ]
}

pub const COLORS: [[u8; 4]; 12] = [
    [229, 57, 53, 255],
    [251, 140, 0, 255],
    [253, 216, 53, 255],
    [67, 160, 71, 255],
    [0, 172, 193, 255],
    [30, 136, 229, 255],
    [103, 80, 164, 255],
    [216, 27, 96, 255],
    [0, 0, 0, 255],
    [117, 117, 117, 255],
    [255, 255, 255, 255],
    [0, 0, 0, 0],
];

#[cfg(test)]
mod tests {
    use super::NumberFormat::*;

    #[test]
    fn formats() {
        assert_eq!(Decimal.format(12), "12");
        assert_eq!(Padded.format(7), "07");
        assert_eq!(LowerAlpha.format(1), "a");
        assert_eq!(LowerAlpha.format(26), "z");
        assert_eq!(UpperAlpha.format(27), "AA");
        assert_eq!(UpperRoman.format(1994), "MCMXCIV");
        assert_eq!(LowerRoman.format(4), "iv");
        assert_eq!(Greek.format(3), "γ");
        assert_eq!(Greek.format(25), "αα");
    }

    #[test]
    fn round_trip() {
        for f in super::NumberFormat::ALL {
            for n in 1..=800 {
                assert_eq!(f.parse(&f.format(n)), Some(n), "{f:?} {n}");
            }
        }
        assert_eq!(UpperRoman.parse("IIII"), None);
        assert_eq!(LowerAlpha.parse("E"), Some(5));
        assert_eq!(UpperRoman.parse("9"), Some(9));
    }
}
