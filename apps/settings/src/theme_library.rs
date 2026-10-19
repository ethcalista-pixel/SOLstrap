use eframe::egui;
use solstrap_core::{
    data_dir,
    interface_colors::Palette,
    themes::{self, Theme},
};
use std::path::PathBuf;

pub struct Editor {
    dir: Option<PathBuf>,
    themes: Vec<Theme>,
    selected: Option<Theme>,
    name: String,
    error: Option<String>,
    notice: Option<String>,
}
impl Editor {
    pub fn new() -> Self {
        let mut e = Self {
            dir: None,
            themes: vec![],
            selected: None,
            name: "Solar flare".into(),
            error: None,
            notice: None,
        };
        match data_dir() {
            Ok(dir) => {
                e.dir = Some(dir);
                e.reload();
            }
            Err(error) => e.error = Some(format!("{error:#}")),
        }
        e
    }
    fn reload(&mut self) {
        if let Some(dir) = &self.dir {
            match themes::load(dir) {
                Ok(themes) => {
                    self.selected = self
                        .selected
                        .as_ref()
                        .and_then(|selected| {
                            themes
                                .iter()
                                .find(|t| t.name.to_lowercase() == selected.name.to_lowercase())
                        })
                        .cloned();
                    self.themes = themes;
                    self.error = None;
                }
                Err(error) => {
                    self.error = Some(format!("{error:#}"));
                }
            }
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, draft: Option<Palette>) -> Option<Palette> {
        let mut load = None;
        ui.label("Select a saved theme to load its colors, gradients, and animation into the editor. Apply GUI palette installs it after Ferns is fully closed.");
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("saved-palette-themes")
                .selected_text(
                    self.selected
                        .as_ref()
                        .map(|t| t.name.as_str())
                        .unwrap_or("Choose a saved theme"),
                )
                .width(240.0)
                .show_ui(ui, |ui| {
                    if self.themes.is_empty() {
                        ui.label("No saved themes yet");
                    }
                    for t in &self.themes {
                        if ui
                            .selectable_label(self.selected.as_ref() == Some(t), &t.name)
                            .clicked()
                        {
                            self.selected = Some(t.clone());
                            self.name = t.name.clone();
                            load = Some(t.palette);
                            self.notice = Some(format!(
                                "{} loaded into the editor. Apply to install it.",
                                t.name
                            ));
                        }
                    }
                });
            if ui.button("Reload themes").clicked() {
                self.reload();
            }
        });
        ui.horizontal(|ui| {
            ui.label("Theme name");
            ui.add(
                egui::TextEdit::singleline(&mut self.name)
                    .desired_width(260.0)
                    .char_limit(48),
            );
        });
        let document = draft.and_then(|p| Theme::new(&self.name, p).ok());
        ui.horizontal_wrapped(|ui|{
            if ui.add_enabled(document.is_some()&&self.dir.is_some(),egui::Button::new("Save as new")).clicked(){
                let theme=document.clone().unwrap();let result=themes::save_new(self.dir.as_ref().unwrap(),theme.clone());
                match result{Ok(())=>{self.selected=Some(theme);self.reload();self.notice=Some("Theme saved locally.".into());},Err(error)=>self.error=Some(format!("{error:#}"))}
            }
            if ui.add_enabled(document.is_some()&&self.selected.is_some()&&self.dir.is_some(),egui::Button::new("Update selected")).clicked(){
                let theme=document.clone().unwrap();let result=themes::update(self.dir.as_ref().unwrap(),self.selected.as_ref().unwrap(),theme.clone());
                match result{Ok(())=>{self.selected=Some(theme);self.reload();self.notice=Some("Selected theme updated locally.".into());},Err(error)=>self.error=Some(format!("{error:#}"))}
            }
            if ui.add_enabled(self.dir.is_some(),egui::Button::new("Import theme…")).clicked(){
                if let Some(path)=rfd::FileDialog::new().set_title("Import SOLstrap palette theme").add_filter("SOLstrap theme JSON",&["json"]).pick_file(){
                    let result=Theme::read(&path).and_then(|t|themes::import(self.dir.as_ref().unwrap(),t));
                    match result{Ok(theme)=>{load=Some(theme.palette);self.name=theme.name.clone();self.notice=Some(format!("{} imported and loaded. Apply to install it.",theme.name));self.selected=Some(theme);self.reload();},Err(error)=>self.error=Some(format!("{error:#}"))}
                }
            }
            if ui.add_enabled(document.is_some(),egui::Button::new("Export editor theme…")).clicked(){
                if let Some(path)=rfd::FileDialog::new().set_title("Export SOLstrap palette theme").set_file_name("solstrap-theme.json").add_filter("SOLstrap theme JSON",&["json"]).save_file(){
                    match document.as_ref().unwrap().export(&path){Ok(())=>{self.error=None;self.notice=Some("Theme exported. This file contains only its name, colors, gradients, and animation settings.".into());},Err(error)=>self.error=Some(format!("{error:#}"))}
                }
            }
        });
        if draft.is_none() {
            ui.small("Finish the color fields below before saving or exporting.");
        } else if document.is_none() {
            ui.small("Enter a theme name (1–48 characters).");
        }
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        if let Some(notice) = &self.notice {
            ui.label(notice);
        }
        load
    }
}
