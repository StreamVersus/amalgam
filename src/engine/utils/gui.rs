use egui::{Context, Key, Modifiers, MouseWheelUnit, PointerButton, Pos2, RawInput, Rect, Vec2, ViewportId, ViewportInfo};
use winit::cursor::{Cursor, CursorIcon};
use winit::event::{DeviceEvent, ElementState, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[derive(Default)]
pub struct EGuiMediator {
    raw_input: RawInput,
    scale: f32,
    pointer_position: Pos2,
    modifiers: Modifiers,
    pub ctx: Context,
}

impl EGuiMediator {
    pub fn init(screen: Vec2, scale: f32) -> EGuiMediator {
        let mut egui = EGuiMediator {
            raw_input: Default::default(),
            scale,
            pointer_position: Default::default(),
            modifiers: Default::default(),
            ctx: Default::default(),
        };
        egui.raw_input.viewport_id = ViewportId::ROOT;

        egui.raw_input.screen_rect = Some(Rect::from_min_size(
            Pos2::ZERO,
            screen / scale,
        ));

        egui.raw_input.viewports.insert(ViewportId::ROOT, ViewportInfo {
            native_pixels_per_point: Some(scale),
            inner_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                screen / scale
            )),
            ..Default::default()
        });

        egui.raw_input.screen_rect = Some(Rect::from_min_size(
            Pos2::ZERO,
            screen / scale
        ));

        egui.ctx.set_pixels_per_point(scale);

        egui
    }

    pub fn handle_window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::SurfaceResized(size) => {
                self.raw_input.screen_rect = Some(Rect::from_min_size(
                    Pos2::ZERO,
                    Vec2::new(size.width as f32, size.height as f32) / self.scale,
                ));
            },
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = scale_factor as f32;
                self.ctx.set_pixels_per_point(self.scale);
            },
            WindowEvent::PointerLeft { .. } => {
                self.raw_input.events.push(egui::Event::PointerGone);
            },
            WindowEvent::PointerButton { state, button, .. } => {
                let Some(button) = button.mouse_button() else { return };
                let Some(button) = winit_mouse_button(button) else { return };

                let pressed = state == ElementState::Pressed;
                self.raw_input.events.push(egui::Event::PointerButton {
                    pos: self.pointer_position,
                    button,
                    pressed,
                    modifiers: self.modifiers,
                });
            },
            WindowEvent::MouseWheel { delta, .. } => {
                let (delta, unit) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        (Vec2::new(x, y) * 20.0, MouseWheelUnit::Line)
                    }
                    MouseScrollDelta::PixelDelta(pos) => {
                        (Vec2::new(pos.x as f32, pos.y as f32) / self.scale, MouseWheelUnit::Point)
                    }
                };
                self.raw_input.events.push(egui::Event::MouseWheel { unit, delta, modifiers: self.modifiers});
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                self.modifiers = update_modifiers(self.modifiers, &event.physical_key, pressed);
                self.raw_input.modifiers = self.modifiers;

                if let Some(key) = winit_key(event.physical_key) {
                    self.raw_input.events.push(egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed,
                        repeat: event.repeat,
                        modifiers: self.modifiers,
                    });
                }

                // Text input — only on press, no ctrl combos
                if pressed && !self.modifiers.ctrl && !self.modifiers.alt {
                    if let Some(text) = &event.text {
                        let s = text.to_string();
                        if !s.is_empty() && !s.chars().all(|c| c.is_control()) {
                            self.raw_input.events.push(egui::Event::Text(s));
                        }
                    }
                }
            },
            WindowEvent::Focused(focused) => {
                self.raw_input.focused = focused;
                if !focused {
                    self.modifiers = Modifiers::default();
                    self.raw_input.events.push(egui::Event::PointerGone);
                }
            }
            _ => {}
        }
    }

    pub fn handle_device_event(&mut self, event: DeviceEvent) {
        match event {
            DeviceEvent::PointerMotion { delta: (x_delta, y_delta), .. } => {
                self.pointer_position += Vec2::new(x_delta as f32, y_delta as f32);
                self.raw_input.events.push(egui::Event::PointerMoved(self.pointer_position.clone()));
            }
            _ => {},
        }
    }

    pub fn take_egui_input(&mut self) -> RawInput {
        self.raw_input.time = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
        );
        self.raw_input.take()
    }
}

pub fn egui_to_winit_cursor(icon: egui::CursorIcon) -> Cursor {
    use egui::CursorIcon as E;
    let icon = match icon {
        E::Default               => CursorIcon::Default,
        E::PointingHand          => CursorIcon::Pointer,
        E::Text                  => CursorIcon::Text,
        E::Crosshair             => CursorIcon::Crosshair,
        E::Move                  => CursorIcon::Move,
        E::ResizeNorthWest       => CursorIcon::NwResize,
        E::ResizeSouthEast       => CursorIcon::SeResize,
        E::ResizeNorthEast       => CursorIcon::NeResize,
        E::ResizeSouthWest       => CursorIcon::SwResize,
        E::ResizeHorizontal      => CursorIcon::EwResize,
        E::ResizeVertical        => CursorIcon::NsResize,
        E::NotAllowed            => CursorIcon::NotAllowed,
        E::Grab                  => CursorIcon::Grab,
        E::Grabbing              => CursorIcon::Grabbing,
        E::Wait                  => CursorIcon::Wait,
        E::Progress              => CursorIcon::Progress,
        E::Help                  => CursorIcon::Help,
        E::ZoomIn                => CursorIcon::ZoomIn,
        E::ZoomOut               => CursorIcon::ZoomOut,
        _                        => CursorIcon::Default,
    };
    Cursor::Icon(icon)
}

// ── Mouse button mapping ──────────────────────────────────────────────────────

pub fn winit_mouse_button(button: winit::event::MouseButton) -> Option<PointerButton> {
    match button {
        winit::event::MouseButton::Left   => Some(PointerButton::Primary),
        winit::event::MouseButton::Right  => Some(PointerButton::Secondary),
        winit::event::MouseButton::Middle => Some(PointerButton::Middle),
        winit::event::MouseButton::Back   => Some(PointerButton::Extra1),
        winit::event::MouseButton::Forward => Some(PointerButton::Extra2),
        _ => None,
    }
}

// ── Modifier tracking ─────────────────────────────────────────────────────────

pub fn update_modifiers(mut mods: Modifiers, key: &PhysicalKey, pressed: bool) -> Modifiers {
    match key {
        PhysicalKey::Code(KeyCode::ShiftLeft | KeyCode::ShiftRight) => mods.shift = pressed,
        PhysicalKey::Code(KeyCode::ControlLeft | KeyCode::ControlRight) => {
            mods.ctrl = pressed;
            mods.command = pressed;
        }
        PhysicalKey::Code(KeyCode::AltLeft | KeyCode::AltRight) => mods.alt = pressed,
        PhysicalKey::Code(KeyCode::Super) => mods.mac_cmd = pressed,
        _ => {}
    }
    mods
}

// ── Key mapping ───────────────────────────────────────────────────────────────

pub fn winit_key(key: PhysicalKey) -> Option<Key> {
    let PhysicalKey::Code(code) = key else { return None };
    Some(match code {
        KeyCode::Escape       => Key::Escape,
        KeyCode::Tab          => Key::Tab,
        KeyCode::Backspace    => Key::Backspace,
        KeyCode::Delete       => Key::Delete,
        KeyCode::Enter        => Key::Enter,
        KeyCode::NumpadEnter  => Key::Enter,
        KeyCode::Space        => Key::Space,
        KeyCode::Insert       => Key::Insert,
        KeyCode::Home         => Key::Home,
        KeyCode::End          => Key::End,
        KeyCode::PageUp       => Key::PageUp,
        KeyCode::PageDown     => Key::PageDown,
        KeyCode::ArrowLeft    => Key::ArrowLeft,
        KeyCode::ArrowRight   => Key::ArrowRight,
        KeyCode::ArrowUp      => Key::ArrowUp,
        KeyCode::ArrowDown    => Key::ArrowDown,
        KeyCode::F1           => Key::F1,
        KeyCode::F2           => Key::F2,
        KeyCode::F3           => Key::F3,
        KeyCode::F4           => Key::F4,
        KeyCode::F5           => Key::F5,
        KeyCode::F6           => Key::F6,
        KeyCode::F7           => Key::F7,
        KeyCode::F8           => Key::F8,
        KeyCode::F9           => Key::F9,
        KeyCode::F10          => Key::F10,
        KeyCode::F11          => Key::F11,
        KeyCode::F12          => Key::F12,
        KeyCode::KeyA         => Key::A,
        KeyCode::KeyB         => Key::B,
        KeyCode::KeyC         => Key::C,
        KeyCode::KeyD         => Key::D,
        KeyCode::KeyE         => Key::E,
        KeyCode::KeyF         => Key::F,
        KeyCode::KeyG         => Key::G,
        KeyCode::KeyH         => Key::H,
        KeyCode::KeyI         => Key::I,
        KeyCode::KeyJ         => Key::J,
        KeyCode::KeyK         => Key::K,
        KeyCode::KeyL         => Key::L,
        KeyCode::KeyM         => Key::M,
        KeyCode::KeyN         => Key::N,
        KeyCode::KeyO         => Key::O,
        KeyCode::KeyP         => Key::P,
        KeyCode::KeyQ         => Key::Q,
        KeyCode::KeyR         => Key::R,
        KeyCode::KeyS         => Key::S,
        KeyCode::KeyT         => Key::T,
        KeyCode::KeyU         => Key::U,
        KeyCode::KeyV         => Key::V,
        KeyCode::KeyW         => Key::W,
        KeyCode::KeyX         => Key::X,
        KeyCode::KeyY         => Key::Y,
        KeyCode::KeyZ         => Key::Z,
        KeyCode::Digit0 | KeyCode::Numpad0 => Key::Num0,
        KeyCode::Digit1 | KeyCode::Numpad1 => Key::Num1,
        KeyCode::Digit2 | KeyCode::Numpad2 => Key::Num2,
        KeyCode::Digit3 | KeyCode::Numpad3 => Key::Num3,
        KeyCode::Digit4 | KeyCode::Numpad4 => Key::Num4,
        KeyCode::Digit5 | KeyCode::Numpad5 => Key::Num5,
        KeyCode::Digit6 | KeyCode::Numpad6 => Key::Num6,
        KeyCode::Digit7 | KeyCode::Numpad7 => Key::Num7,
        KeyCode::Digit8 | KeyCode::Numpad8 => Key::Num8,
        KeyCode::Digit9 | KeyCode::Numpad9 => Key::Num9,
        _ => return None,
    })
}