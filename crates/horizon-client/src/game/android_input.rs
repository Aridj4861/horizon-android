//! Extra desktop keys, a touch keyboard and menu scrolling on Android.
use super::*;
use horizon_client::{
    debug_keys::KeyAction,
    osk::Output,
    touch_ui::{Action, Gesture},
};
use winit::event::{Touch, TouchPhase};

impl Game<'_> {
    pub(crate) fn android_apply_display_scale(&mut self) {
        if let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer)
            && let Err(error) = crate::display::apply(window, renderer)
        {
            crate::display::set_scale(crate::display::RenderScale::default());
            log::warn!("Android display scale: {error}");
        }
    }

    pub(super) fn android_keypad_menu_key(
        &mut self,
        events: &ActiveEventLoop,
        event: &winit::event::KeyEvent,
    ) -> Result<bool> {
        if !self.settings.keypad.enabled
            || !self.pad_in_menu()
            || crate::platform::active()
            || self.ui.options.selected_key.is_some()
        {
            return Ok(false);
        }
        let Some(key) = horizon_client::keypad::resolve_key(event.physical_key, &event.logical_key)
        else {
            return Ok(false);
        };
        let Some(button) = self.settings.keypad.menu_button(key) else {
            return Ok(false);
        };
        if !event.repeat {
            self.pad.in_use = true;
            self.pad_menu_button(events, button, event.state == ElementState::Pressed)?;
        }
        Ok(true)
    }
    /// A freely moving physical pointer can use the same utility buttons as touch.
    pub(super) fn android_aux_mouse(
        &mut self,
        events: &ActiveEventLoop,
        id: WindowId,
        event: &WindowEvent,
    ) -> bool {
        const POINTER: u64 = u64::MAX - 2;
        if !self.settings.keypad.enabled || (self.captured && crate::keyboard_mouse_input()) {
            return false;
        }
        let WindowEvent::MouseInput {
            device_id,
            state,
            button: MouseButton::Left,
            ..
        } = event
        else {
            return false;
        };
        if *device_id == DeviceId::dummy() {
            return false;
        }
        let Some(batch) = self.gui_scale() else {
            return false;
        };
        let point = [self.cursor.0, self.cursor.1];
        let normalized = [point[0] / batch.width, point[1] / batch.height];
        let utility = self
            .pad
            .touch_ui
            .action(batch.width, point, self.pad_in_menu())
            .is_some()
            || self.pad.touch_ui.owns_key(POINTER)
            || self.pad.touch.owns_pointer(POINTER)
            || (!self.pad_in_menu() && self.pad.touch.auxiliary_at(normalized));
        if !utility {
            return false;
        }
        let touch = Touch {
            device_id: *device_id,
            phase: if *state == ElementState::Pressed {
                TouchPhase::Started
            } else {
                TouchPhase::Ended
            },
            location: winit::dpi::PhysicalPosition::new(
                f64::from(point[0] * batch.scale),
                f64::from(point[1] * batch.scale),
            ),
            force: None,
            id: POINTER,
        };
        self.window_event(events, id, WindowEvent::Touch(touch));
        true
    }

    pub(super) fn android_custom_key_held(&self, key: KeyCode) -> bool {
        self.pad.touch.held_custom_tokens().any(|token| {
            if let Some(mapping) = token.strip_prefix("action:") {
                horizon_client::keybinds::key_code(horizon_client::keybinds::KeyBinds::default_of(
                    mapping,
                )) == Some(key)
            } else {
                horizon_client::keybinds::key_code(token).is_some_and(|physical| {
                    self.settings
                        .keys
                        .translate_all(physical)
                        .contains(&Ok(key))
                })
            }
        })
    }
    pub(super) fn android_custom_mouse_held(&self, button: MouseButton) -> bool {
        self.pad.touch.held_custom_tokens().any(|token| {
            token.strip_prefix("action:").is_some_and(|mapping| {
                horizon_client::keybinds::mouse_button(
                    horizon_client::keybinds::KeyBinds::default_of(mapping),
                ) == Some(button)
            })
        })
    }
    pub(super) fn android_custom_events(&mut self) {
        for (name, down) in self.pad.touch.key_events() {
            if let Some(mapping) = name.strip_prefix("action:") {
                let default = horizon_client::keybinds::KeyBinds::default_of(mapping);
                let result = if let Some(key) = horizon_client::keybinds::key_code(default) {
                    self.mapped_key(key, down, false)
                } else if let Some(button) = horizon_client::keybinds::mouse_button(default) {
                    self.play_mouse(button, down)
                } else {
                    Ok(())
                };
                if let Err(error) = result {
                    log::warn!("Custom touch action: {error}");
                }
                if down && self.screen == Screen::Chat {
                    self.android_show_keyboard();
                }
                continue;
            }
            let Some(key) = horizon_client::keybinds::key_code(&name) else {
                continue;
            };
            if down {
                self.pad.custom_keys.insert(key);
            } else {
                self.pad.custom_keys.remove(&key);
            }
            if self.pad.hardware_keys.contains(&key) {
                continue;
            }
            if (!self.pad_in_menu() || !down)
                && horizon_client::android_runtime::keypad_key(key, down)
            {
                continue;
            }
            if down {
                self.physical_keys.insert(key);
            } else {
                self.physical_keys.remove(&key);
            }
            self.mod_key(key, down, false);
            let action = if down {
                KeyAction::Press
            } else {
                KeyAction::Release
            };
            if self.debug_key(key, action) {
                continue;
            }
            for mapped in self.settings.keys.translate_all(key) {
                let result = match mapped {
                    Ok(code) => self.mapped_key(code, down, false),
                    Err(button) => self.play_mouse(button, down),
                };
                if let Err(error) = result {
                    log::warn!("Custom touch key: {error}");
                }
            }
            if down && matches!(key, KeyCode::KeyT | KeyCode::Slash) && self.screen == Screen::Chat
            {
                self.android_show_keyboard();
            }
        }
    }

    pub(super) fn android_aux_touch(
        &mut self,
        t: &Touch,
        events: &ActiveEventLoop,
        id: WindowId,
    ) -> Result<bool> {
        if t.phase == TouchPhase::Cancelled {
            self.pad.touch_ui.cancel();
            self.android_release_touch_keys(true);
            return Ok(true);
        }
        if t.phase == TouchPhase::Ended && self.pad.touch_ui.owns_key(t.id) {
            if let Some(key) = self.pad.touch_ui.key_up(t.id) {
                self.android_key_up(key);
            }
            return Ok(true);
        }
        let size = self.window.as_ref().unwrap().inner_size();
        let b = HudBatch::new([size.width, size.height]);
        let p = [t.location.x as f32 / b.scale, t.location.y as f32 / b.scale];
        if let Some(action) = self.pad.touch_ui.action(b.width, p, self.pad_in_menu()) {
            if t.phase == TouchPhase::Started {
                self.pad.touch.clear();
                self.pad.touch.ignore(t.id);
                match action {
                    Action::LatchF3 => {
                        let was = self.pad.touch_ui.key_held(KeyCode::F3);
                        self.pad.touch_ui.f3_latched = !self.pad.touch_ui.f3_latched;
                        let now = self.pad.touch_ui.key_held(KeyCode::F3);
                        if !was && now {
                            self.physical_keys.insert(KeyCode::F3);
                            self.debug_key(KeyCode::F3, KeyAction::Press);
                        }
                        if was && !now {
                            self.android_key_up(KeyCode::F3);
                        }
                    }
                    Action::Pick => {
                        self.play_mouse(MouseButton::Middle, true)?;
                        self.play_mouse(MouseButton::Middle, false)?;
                        self.pad.touch_ui.expanded = false;
                    }
                    Action::DebugCombo(key) => {
                        let held = self.pad.touch_ui.key_held(KeyCode::F3);
                        if !held {
                            self.debug_key(KeyCode::F3, KeyAction::Press);
                        }
                        self.debug_key(key, KeyAction::Press);
                        self.debug_key(key, KeyAction::Release);
                        if !held {
                            self.debug_key(KeyCode::F3, KeyAction::Release);
                        }
                        self.pad.touch_ui.expanded = false;
                    }
                    Action::More => self.pad.touch_ui.expanded = !self.pad.touch_ui.expanded,
                    Action::Keyboard => {
                        if crate::platform::active() {
                            crate::platform::hide();
                        } else {
                            self.android_show_keyboard();
                        }
                    }
                    Action::Key(key) => {
                        if key != KeyCode::F3 && !self.pad.touch_ui.key_held(KeyCode::F3) {
                            self.pad.touch_ui.expanded = false;
                        }
                        if key == KeyCode::Escape {
                            crate::platform::hide();
                            self.pad_back()?;
                        } else if matches!(
                            key,
                            KeyCode::Tab | KeyCode::ShiftLeft | KeyCode::ControlLeft
                        ) {
                            let down = !self.physical_keys.remove(&key);
                            if down {
                                self.physical_keys.insert(key);
                            }
                            self.mapped_key(key, down, false)?;
                        } else {
                            if !self.pad.touch_ui.key_down(t.id, key) {
                                return Ok(true);
                            }
                            self.physical_keys.insert(key);
                            if !self.debug_key(key, KeyAction::Press) {
                                self.key_pressed(key)?;
                            }
                            if matches!(key, KeyCode::KeyT | KeyCode::Slash)
                                && self.screen == Screen::Chat
                            {
                                self.android_show_keyboard();
                            }
                        }
                    }
                }
            }
            return Ok(true);
        }
        let _ = (events, id);
        Ok(false)
    }
    pub(super) fn android_show_keyboard(&mut self) {
        // The in-world keyboard button needs a real editor to receive text.
        if self.title.is_none() && self.screen == Screen::Playing {
            self.open_chat("");
        }
        let initial = if self.screen == Screen::Chat && self.title.is_none() {
            self.chat_key_code(KeyCode::End, None);
            self.ui
                .chat_screen
                .as_ref()
                .map(|s| s.value().to_owned())
                .unwrap_or_default()
        } else if let Some(t) = self.title.as_mut() {
            t.field_key(KeyCode::End, false, false);
            t.focused_field()
                .map(|(_, s, _)| s.to_owned())
                .unwrap_or_default()
        } else if self.screen == Screen::Creative {
            self.creative_screen.search.value.clone()
        } else {
            String::new()
        };
        crate::platform::show(&initial);
    }
    pub(super) fn android_poll_keyboard(&mut self) {
        self.android_custom_events();
        for result in crate::gallery::poll() {
            self.system_message(horizon_client::text::Text::literal(
                result.unwrap_or_else(|e| format!("Screenshot export failed: {e}")),
            ));
        }
        for out in crate::platform::poll() {
            let done = matches!(out, Output::Enter | Output::Close);
            self.pad_osk_output(out);
            if done {
                crate::platform::hide();
            }
        }
        // Title screens also use Playing: their focused text fields must keep
        // the IME until the user confirms or closes the editor.
        if self.title.is_none() && self.screen == Screen::Playing && crate::platform::active() {
            crate::platform::hide();
        }
    }
    fn android_key_up(&mut self, key: KeyCode) {
        if self.pad.custom_keys.contains(&key) {
            return;
        }
        self.physical_keys.remove(&key);
        self.keys.remove(&key);
        self.debug_key(key, KeyAction::Release);
    }
    pub(super) fn android_release_touch_keys(&mut self, cancelled: bool) {
        if cancelled {
            self.pad.hardware_keys.clear();
            crate::platform::cancel_vibration();
            self.pad.touch.clear();
            self.pad.touch_aim = None;
            if self.pad.touch.editor.is_some() {
                self.pad.touch.cancel_editor();
                self.pause = false;
            }
        }
        self.android_custom_events();
        for key in self.pad.touch_ui.release_keys() {
            if cancelled && key == KeyCode::F3 {
                self.physical_keys.remove(&key);
                self.debug_keys.modifier_down = false;
                self.debug_keys.used_debug_key_as_modifier = false;
                if self.screen == Screen::GameModeSwitcher {
                    self.close_gamemode_switcher(false);
                }
            } else {
                self.android_key_up(key);
            }
        }
    }
    pub(super) fn android_menu_touch(&mut self, t: &Touch, events: &ActiveEventLoop, id: WindowId) {
        let size = self.window.as_ref().unwrap().inner_size();
        let b = HudBatch::new([size.width, size.height]);
        let p = [t.location.x as f32 / b.scale, t.location.y as f32 / b.scale];
        let direct = self.pad.touch_ui.direct_pointer(t.id, p, t.phase);
        self.window_event(
            events,
            id,
            WindowEvent::CursorMoved {
                device_id: t.device_id,
                position: t.location,
            },
        );
        if direct {
            if t.phase != TouchPhase::Moved {
                self.window_event(
                    events,
                    id,
                    WindowEvent::MouseInput {
                        device_id: t.device_id,
                        button: MouseButton::Left,
                        state: if t.phase == TouchPhase::Started {
                            ElementState::Pressed
                        } else {
                            ElementState::Released
                        },
                    },
                );
            }
            return;
        }
        match self
            .pad
            .touch_ui
            .gesture(t.id, [t.location.x as f32, t.location.y as f32], t.phase)
        {
            Gesture::Tap => {
                let field = crate::platform::field_at(p);
                for state in [ElementState::Pressed, ElementState::Released] {
                    self.window_event(
                        events,
                        id,
                        WindowEvent::MouseInput {
                            device_id: t.device_id,
                            button: MouseButton::Left,
                            state,
                        },
                    );
                }
                if field {
                    self.android_show_keyboard();
                }
            }
            Gesture::Scroll(y) => self.window_event(
                events,
                id,
                WindowEvent::MouseWheel {
                    device_id: t.device_id,
                    delta: MouseScrollDelta::LineDelta(0.0, y),
                    phase: TouchPhase::Moved,
                },
            ),
            Gesture::None => {}
        }
    }
}
