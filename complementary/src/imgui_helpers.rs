use crate::math::{FVec2, FVec3, IVec2, IVec3};

#[derive(Default)]
pub struct ImGuiSettings {
    read_only: bool,
}

impl ImGuiSettings {
    pub fn new() -> Self {
        ImGuiSettings::default()
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}

pub trait ImGui {
    fn draw_gui_with_settings(&mut self, label: &str, gui: &imgui::Ui, settings: &ImGuiSettings);
    fn draw_gui(&mut self, label: &str, gui: &imgui::Ui) {
        self.draw_gui_with_settings(label, gui, &ImGuiSettings::default());
    }
}

impl ImGui for f32 {
    fn draw_gui_with_settings(&mut self, label: &str, gui: &imgui::Ui, settings: &ImGuiSettings) {
        gui.input_float(label, self)
            .read_only(settings.read_only)
            .build();
    }
}

impl ImGui for i32 {
    fn draw_gui_with_settings(&mut self, label: &str, gui: &imgui::Ui, settings: &ImGuiSettings) {
        gui.input_int(label, self)
            .read_only(settings.read_only)
            .build();
    }
}

impl ImGui for bool {
    fn draw_gui_with_settings(&mut self, label: &str, gui: &imgui::Ui, _settings: &ImGuiSettings) {
        gui.checkbox(label, self);
    }
}

impl ImGui for FVec2 {
    fn draw_gui_with_settings(&mut self, label: &str, gui: &imgui::Ui, settings: &ImGuiSettings) {
        let mut arr = &mut [self.x, self.y];
        gui.input_float2(label, &mut arr)
            .read_only(settings.read_only)
            .build();
        let [x, y] = arr;
        self.x = *x;
        self.y = *y;
    }
}

impl ImGui for FVec3 {
    fn draw_gui_with_settings(&mut self, label: &str, gui: &imgui::Ui, settings: &ImGuiSettings) {
        let mut arr = &mut [self.x, self.y, self.z];
        gui.input_float3(label, &mut arr)
            .read_only(settings.read_only)
            .build();
        let [x, y, z] = arr;
        self.x = *x;
        self.y = *y;
        self.z = *z;
    }
}

impl ImGui for IVec2 {
    fn draw_gui_with_settings(&mut self, label: &str, gui: &imgui::Ui, settings: &ImGuiSettings) {
        let mut arr = &mut [self.x, self.y];
        gui.input_int2(label, &mut arr)
            .read_only(settings.read_only)
            .build();
        let [x, y] = arr;
        self.x = *x;
        self.y = *y;
    }
}

impl ImGui for IVec3 {
    fn draw_gui_with_settings(&mut self, label: &str, gui: &imgui::Ui, settings: &ImGuiSettings) {
        let mut arr = &mut [self.x, self.y, self.z];
        gui.input_int3(label, &mut arr)
            .read_only(settings.read_only)
            .build();
        let [x, y, z] = arr;
        self.x = *x;
        self.y = *y;
        self.z = *z;
    }
}

impl ImGui for String {
    fn draw_gui_with_settings(&mut self, label: &str, gui: &imgui::Ui, settings: &ImGuiSettings) {
        gui.input_text(label, self)
            .read_only(settings.read_only)
            .build();
    }
}

impl ImGui for dyn AsRef<str> {
    fn draw_gui_with_settings(&mut self, label: &str, gui: &imgui::Ui, _: &ImGuiSettings) {
        gui.label_text(label, self);
    }
}
