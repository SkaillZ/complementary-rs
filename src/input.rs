use complementary_macros::EnumCount;
use imgui::TreeNodeFlags;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::fmt::Debug;

use crate::imgui_helpers::ImGui;

#[derive(Clone, Copy, Debug, EnumCount, FromPrimitive)]
pub enum ButtonType {
    Jump,
    Switch,
    Ability,
    SwitchAndAbility,
    Left,
    Right,
    Up,
    Down,

    Pause,
    Confirm,
}

#[derive(Clone, Copy)]
pub struct Button {
    pressed_ticks: Option<i32>,
}

impl Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(pressed_ticks) = self.pressed_ticks {
            write!(f, "Pressed {pressed_ticks} ticks")
        } else {
            write!(f, "Not pressed")
        }
    }
}

impl Button {
    fn new() -> Self {
        Button {
            pressed_ticks: None,
        }
    }

    pub fn pressed(&self) -> bool {
        self.pressed_ticks.is_some()
    }

    pub fn pressed_first_frame(&self) -> bool {
        matches!(self.pressed_ticks, Some(1))
    }

    pub fn pressed_ticks(&self) -> Option<i32> {
        self.pressed_ticks
    }
}

#[derive(Debug)]
pub struct Input {
    buttons: [Button; ButtonType::COUNT],
}

impl Input {
    pub fn new() -> Self {
        Input {
            buttons: [Button::new(); ButtonType::COUNT],
        }
    }

    pub fn tick(&mut self) {
        for button in &mut self.buttons {
            if let Some(pressed_ticks) = button.pressed_ticks {
                button.pressed_ticks = Some(pressed_ticks + 1);
            }
        }
    }

    pub fn set_button_pressed(&mut self, typ: ButtonType) {
        if !self.buttons[typ as usize].pressed_ticks.is_some() {
            self.buttons[typ as usize].pressed_ticks = Some(0);
        }
    }

    pub fn set_button_released(&mut self, typ: ButtonType) {
        self.buttons[typ as usize].pressed_ticks = None;
    }

    pub fn get_button(&self, typ: ButtonType) -> &Button {
        &self.buttons[typ as usize]
    }

    pub fn ability_button_pressed_first_frame(&self) -> bool {
        self.get_button(ButtonType::Ability).pressed_first_frame()
            || self
                .get_button(ButtonType::SwitchAndAbility)
                .pressed_first_frame()
    }

    pub fn ability_button_pressed(&self) -> bool {
        self.get_button(ButtonType::Ability).pressed()
            || self.get_button(ButtonType::SwitchAndAbility).pressed()
    }
}

impl ImGui for Input {
    fn draw_gui_with_settings(
        &mut self,
        label: &str,
        gui: &imgui::Ui,
        _settings: &crate::imgui_helpers::ImGuiSettings,
    ) {
        if gui.collapsing_header(label, TreeNodeFlags::empty()) {
            for (index, button) in self.buttons.iter().enumerate() {
                gui.text(format!("{:?}", ButtonType::from_usize(index).unwrap()));
                gui.same_line();

                let _token = gui.begin_disabled(true);

                let mut pressed = button.pressed();
                gui.same_line();
                gui.checkbox("Pressed", &mut pressed);

                let mut pressed_first_frame = button.pressed_first_frame();
                gui.same_line();
                gui.checkbox("First frame", &mut pressed_first_frame);

                if pressed {
                    gui.same_line();
                    gui.text(button.pressed_ticks().unwrap().to_string());
                }
            }
        }
    }
}
