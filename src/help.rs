//! Page Aide : guide des fonctionnalités, dans la langue de l'interface, avec des captures
//! d'écran génériques (images de démonstration, voir le scénario H1 de examples/validate.rs).
//!
//! Le texte est dans `help/<langue>.md`, dans un format simple :
//! `# Section` (sommaire), `## Sous-titre`, `- puce`, `> remarque`, `![image]`,
//! `**gras**` et `` `touche` `` dans le texte.

use std::collections::HashMap;

use eframe::egui::{self, vec2, Color32, CornerRadius, Margin, RichText, Ui};
use egui_material_icons::icons::*;

use crate::i18n::Language;
use crate::theme::{self, palette};

const TEXTS: [(Language, &str); 4] = [
    (Language::Fr, include_str!("../help/fr.md")),
    (Language::En, include_str!("../help/en.md")),
    (Language::Es, include_str!("../help/es.md")),
    (Language::De, include_str!("../help/de.md")),
];

macro_rules! shots {
    ($($lang:literal),*) => {
        &[$(
            ($lang, "accueil", include_bytes!(concat!("../help/img/", $lang, "/accueil.jpg")).as_slice()),
            ($lang, "selection", include_bytes!(concat!("../help/img/", $lang, "/selection.jpg")).as_slice()),
            ($lang, "editeur", include_bytes!(concat!("../help/img/", $lang, "/editeur.jpg")).as_slice()),
            ($lang, "parametres", include_bytes!(concat!("../help/img/", $lang, "/parametres.jpg")).as_slice()),
        )*]
    };
}

const SHOTS: &[(&str, &str, &[u8])] = shots!("fr", "en", "es", "de");

fn lang_code(l: Language) -> &'static str {
    match l {
        Language::En => "en",
        Language::Es => "es",
        Language::De => "de",
        _ => "fr",
    }
}

fn text_for(l: Language) -> &'static str {
    TEXTS.iter().find(|(x, _)| *x == l).map(|(_, t)| *t).unwrap_or(TEXTS[0].1)
}

/// Titres de section (`# …`) du texte.
pub fn sections(text: &str) -> Vec<&str> {
    text.lines().filter_map(|l| l.strip_prefix("# ")).collect()
}

#[derive(Default)]
pub struct Help {
    textures: HashMap<(&'static str, &'static str), egui::TextureHandle>,
    /// Section demandée dans le sommaire (défilement au prochain affichage).
    goto: Option<usize>,
}

impl Help {
    fn texture(&mut self, ctx: &egui::Context, lang: &'static str, name: &str) -> Option<egui::TextureHandle> {
        let (l, n, bytes) = SHOTS.iter().find(|(l, n, _)| *l == lang && *n == name)?;
        if let Some(t) = self.textures.get(&(*l, *n)) {
            return Some(t.clone());
        }
        let img = image::load_from_memory(bytes).ok()?.to_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        let tex = ctx.load_texture(format!("aide_{l}_{n}"), egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()), egui::TextureOptions::LINEAR);
        self.textures.insert((*l, *n), tex.clone());
        Some(tex)
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        let p = palette(ui.ctx());
        let lang = crate::i18n::current();
        let code = lang_code(lang);
        let text = text_for(lang);
        let titles = sections(text);

        ui.label(RichText::new(crate::i18n::tr("Aide")).font(theme::medium(28.0)).color(p.on_surface));
        ui.add_space(12.0);
        ui.horizontal_top(|ui| {
            // Sommaire.
            ui.vertical(|ui| {
                ui.set_width(210.0);
                for (i, t) in titles.iter().enumerate() {
                    let r = ui.add(egui::Button::new(RichText::new(*t).color(p.on_surface_variant)).frame(false).min_size(vec2(200.0, 30.0)));
                    if r.clicked() {
                        self.goto = Some(i);
                    }
                }
            });
            ui.add_space(16.0);
            egui::ScrollArea::vertical().id_salt("aide").auto_shrink([false; 2]).show(ui, |ui| ui.vertical(|ui| {
                // Largeur de lecture : la zone visible (moins la barre de défilement), 780 points au plus.
                ui.set_width((ui.available_width() - 16.0).clamp(200.0, 780.0));
                let goto = self.goto.take();
                let mut section = 0usize;
                for line in text.lines() {
                    let line = line.trim_end();
                    if let Some(t) = line.strip_prefix("# ") {
                        if section > 0 {
                            ui.add_space(22.0);
                        }
                        let r = ui.label(RichText::new(t).font(theme::medium(22.0)).color(p.on_surface));
                        if goto == Some(section) {
                            r.scroll_to_me(Some(egui::Align::TOP));
                        }
                        section += 1;
                        ui.add_space(6.0);
                    } else if let Some(t) = line.strip_prefix("## ") {
                        ui.add_space(10.0);
                        ui.label(RichText::new(t).font(theme::medium(16.0)).color(p.primary));
                        ui.add_space(2.0);
                    } else if let Some(t) = line.strip_prefix("- ") {
                        ui.horizontal_top(|ui| {
                            ui.add_space(6.0);
                            ui.label(RichText::new("•").color(p.primary));
                            rich_line(ui, t);
                        });
                    } else if let Some(t) = line.strip_prefix("> ") {
                        egui::Frame::new()
                            .fill(p.secondary_container)
                            .corner_radius(CornerRadius::same(12))
                            .inner_margin(Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                ui.horizontal_top(|ui| {
                                    ui.label(RichText::new(ICON_LIGHTBULB).size(18.0).color(p.on_secondary_container));
                                    rich_line(ui, t);
                                });
                            });
                        ui.add_space(4.0);
                    } else if let Some(name) = line.strip_prefix("![").and_then(|s| s.strip_suffix(']')) {
                        if let Some(tex) = self.texture(ui.ctx(), code, name) {
                            ui.add_space(6.0);
                            let s = tex.size_vec2();
                            let w = ui.available_width().min(s.x).min(760.0);
                            let size = s * (w / s.x);
                            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                            ui.painter().rect_filled(rect.translate(vec2(0.0, 3.0)), 10.0, Color32::from_black_alpha(30));
                            ui.painter().add(egui::epaint::RectShape::filled(rect, 10.0, Color32::WHITE).with_texture(tex.id(), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))));
                            ui.add_space(10.0);
                        }
                    } else if !line.is_empty() {
                        rich_line(ui, line);
                        ui.add_space(2.0);
                    }
                }
                ui.add_space(24.0);
            }));
        });
    }
}

/// Ligne de texte avec `**gras**` et `` `touche` ``, renvoyée à la ligne.
fn rich_line(ui: &mut Ui, text: &str) {
    let p = palette(ui.ctx());
    let mut job = egui::text::LayoutJob::default();
    let body = egui::FontId::proportional(14.5);
    let normal = egui::TextFormat { font_id: body.clone(), color: p.on_surface, ..Default::default() };
    let bold = egui::TextFormat { font_id: theme::medium(14.5), color: p.on_surface, ..Default::default() };
    let key = egui::TextFormat {
        font_id: egui::FontId::monospace(13.0),
        color: p.on_secondary_container,
        background: p.secondary_container,
        ..Default::default()
    };
    // Découpage : `…` d'abord (le code n'est jamais en gras), puis **…**.
    for (i, part) in text.split('`').enumerate() {
        if i % 2 == 1 {
            job.append(&format!("\u{2009}{part}\u{2009}"), 0.0, key.clone());
            continue;
        }
        for (j, seg) in part.split("**").enumerate() {
            job.append(seg, 0.0, if j % 2 == 1 { bold.clone() } else { normal.clone() });
        }
    }
    ui.add(egui::Label::new(job).wrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Même structure dans les quatre langues : sections, images, raccourcis.
    #[test]
    fn aide_complete() {
        let fr = text_for(Language::Fr);
        let count = |t: &str, pre: &str| t.lines().filter(|l| l.starts_with(pre)).count();
        let images = |t: &str| t.lines().filter(|l| l.starts_with("![")).map(str::to_string).collect::<Vec<_>>();
        for (l, t) in TEXTS {
            assert_eq!(count(t, "# "), count(fr, "# "), "{l:?} : sections");
            assert_eq!(count(t, "## "), count(fr, "## "), "{l:?} : sous-titres");
            assert_eq!(images(t), images(fr), "{l:?} : images");
            assert_eq!(t.matches('`').count() % 2, 0, "{l:?} : `");
            for line in t.lines() {
                assert_eq!(line.matches("**").count() % 2, 0, "{l:?} : ** dans « {line} »");
            }
            for img in images(t) {
                let name = img.trim_start_matches("![").trim_end_matches(']');
                assert!(SHOTS.iter().any(|(lang, n, _)| *lang == lang_code(l) && *n == name), "{l:?} : image {name}");
            }
        }
    }
}
