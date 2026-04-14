use bevy::prelude::*;
use serde::Deserialize;

/// Raw RON format with string key names.
#[derive(Deserialize, Asset, TypePath, Debug, Clone)]
struct InputConfigRaw {
    pub back: String,
    pub return_to_center: String,
    pub pause: String,
    pub cycle_next: String,
    pub confirm: String,
    pub debug_overlay: String,
}

/// Rebindable key bindings. Loaded from `assets/settings/keybindings.ron`.
#[derive(Resource, Debug, Clone)]
pub struct InputConfig {
    pub back: KeyCode,
    pub return_to_center: KeyCode,
    pub pause: KeyCode,
    pub cycle_next: KeyCode,
    pub confirm: KeyCode,
    pub debug_overlay: KeyCode,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            back: KeyCode::Escape,
            return_to_center: KeyCode::Space,
            pause: KeyCode::KeyP,
            cycle_next: KeyCode::Tab,
            confirm: KeyCode::Enter,
            debug_overlay: KeyCode::F1,
        }
    }
}

fn parse_key(s: &str) -> Option<KeyCode> {
    match s {
        "Escape" => Some(KeyCode::Escape),
        "Space" => Some(KeyCode::Space),
        "Enter" => Some(KeyCode::Enter),
        "Tab" => Some(KeyCode::Tab),
        "Backspace" => Some(KeyCode::Backspace),
        "F1" => Some(KeyCode::F1),
        "F2" => Some(KeyCode::F2),
        "F3" => Some(KeyCode::F3),
        "F4" => Some(KeyCode::F4),
        "F5" => Some(KeyCode::F5),
        "KeyA" => Some(KeyCode::KeyA),
        "KeyB" => Some(KeyCode::KeyB),
        "KeyC" => Some(KeyCode::KeyC),
        "KeyD" => Some(KeyCode::KeyD),
        "KeyE" => Some(KeyCode::KeyE),
        "KeyF" => Some(KeyCode::KeyF),
        "KeyG" => Some(KeyCode::KeyG),
        "KeyH" => Some(KeyCode::KeyH),
        "KeyI" => Some(KeyCode::KeyI),
        "KeyJ" => Some(KeyCode::KeyJ),
        "KeyK" => Some(KeyCode::KeyK),
        "KeyL" => Some(KeyCode::KeyL),
        "KeyM" => Some(KeyCode::KeyM),
        "KeyN" => Some(KeyCode::KeyN),
        "KeyO" => Some(KeyCode::KeyO),
        "KeyP" => Some(KeyCode::KeyP),
        "KeyQ" => Some(KeyCode::KeyQ),
        "KeyR" => Some(KeyCode::KeyR),
        "KeyS" => Some(KeyCode::KeyS),
        "KeyT" => Some(KeyCode::KeyT),
        "KeyU" => Some(KeyCode::KeyU),
        "KeyV" => Some(KeyCode::KeyV),
        "KeyW" => Some(KeyCode::KeyW),
        "KeyX" => Some(KeyCode::KeyX),
        "KeyY" => Some(KeyCode::KeyY),
        "KeyZ" => Some(KeyCode::KeyZ),
        "Digit1" => Some(KeyCode::Digit1),
        "Digit2" => Some(KeyCode::Digit2),
        "Digit3" => Some(KeyCode::Digit3),
        "Digit4" => Some(KeyCode::Digit4),
        "Digit5" => Some(KeyCode::Digit5),
        "ArrowUp" => Some(KeyCode::ArrowUp),
        "ArrowDown" => Some(KeyCode::ArrowDown),
        "ArrowLeft" => Some(KeyCode::ArrowLeft),
        "ArrowRight" => Some(KeyCode::ArrowRight),
        _ => None,
    }
}

/// Handle to the keybindings asset.
#[derive(Resource, Default)]
pub struct InputConfigHandle(pub Option<Handle<InputConfigRaw>>);

fn load_input_config(mut handle: ResMut<InputConfigHandle>, asset_server: Res<AssetServer>) {
    handle.0 = Some(asset_server.load("settings/keybindings.ron"));
}

fn apply_input_config(
    handle: Res<InputConfigHandle>,
    assets: Res<Assets<InputConfigRaw>>,
    mut config: ResMut<InputConfig>,
    mut applied: Local<bool>,
) {
    if *applied {
        return;
    }
    let Some(h) = &handle.0 else { return };
    let Some(raw) = assets.get(h) else { return };

    if let Some(k) = parse_key(&raw.back) { config.back = k; }
    if let Some(k) = parse_key(&raw.return_to_center) { config.return_to_center = k; }
    if let Some(k) = parse_key(&raw.pause) { config.pause = k; }
    if let Some(k) = parse_key(&raw.cycle_next) { config.cycle_next = k; }
    if let Some(k) = parse_key(&raw.confirm) { config.confirm = k; }
    if let Some(k) = parse_key(&raw.debug_overlay) { config.debug_overlay = k; }

    *applied = true;
}

pub struct InputConfigPlugin;

impl Plugin for InputConfigPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            bevy_common_assets::ron::RonAssetPlugin::<InputConfigRaw>::new(&[
                "keybindings.ron",
            ]),
        )
        .init_resource::<InputConfigHandle>()
        .init_resource::<InputConfig>()
        .add_systems(Startup, load_input_config)
        .add_systems(Update, apply_input_config);
    }
}
