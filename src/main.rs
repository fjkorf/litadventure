use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use bevy_tweening::TweeningPlugin;

mod camera;
mod components;
mod debug;
mod game_data;
mod hints;
mod interaction;
mod inventory;
mod navigation;
mod objectives;
mod scene;
mod states;

mod pages {
    use egui;
    use litui::*;

    define_litui_app! {
        parent: "content/_app.md",
        "content/room_info.md",
        "content/inventory.md",
        "content/objectives.md",
        "content/help.md",
        "content/victory.md",
    }
}

use pages::*;

impl Resource for Page {}
impl Resource for AppState {}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "LitAdventure".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MeshPickingPlugin)
        .add_plugins(bevy_skein::SkeinPlugin::default())
        .add_plugins(EguiPlugin::default())
        .add_plugins(TweeningPlugin)
        .add_plugins(camera::CameraPlugin)
        .add_plugins(scene::ScenePlugin)
        .add_plugins(interaction::InteractionPlugin)
        .add_plugins(inventory::InventoryPlugin)
        .add_plugins(objectives::ObjectivesPlugin)
        .add_plugins(navigation::NavigationPlugin)
        .add_plugins(states::StatesPlugin)
        .add_plugins(hints::HintsPlugin)
        .add_plugins(game_data::GameDataPlugin)
        .add_plugins(debug::DebugPlugin)
        .register_type::<components::Clickable>()
        .register_type::<components::CameraSpot>()
        .register_type::<components::InventoryItem>()
        .register_type::<components::ObjectState>()
        .register_type::<components::NavigatesTo>()
        .register_type::<components::ParentSpot>()
        .register_type::<components::RequiresItem>()
        .register_type::<components::ContainedInName>()
        .register_type::<components::TweenConfig>()
        .init_resource::<Page>()
        .init_resource::<AppState>()
        .add_systems(Startup, init_ui_state)
        .add_systems(EguiPrimaryContextPass, render_ui)
        .add_systems(
            Update,
            (
                sync_room_info.run_if(resource_changed::<navigation::CurrentRoom>),
                sync_feedback.run_if(
                    resource_changed::<interaction::FeedbackText>
                        .or(resource_changed::<camera::CameraController>),
                ),
                sync_objectives.run_if(resource_changed::<objectives::Objectives>),
                sync_inventory.run_if(resource_changed::<inventory::Inventory>),
                sync_hints.run_if(resource_changed::<hints::HintText>),
                sync_win.run_if(state_changed::<states::GameState>),
                handle_game_won,
            ),
        )
        .run();
}

// -- One-time UI initialization --

fn init_ui_state(mut state: ResMut<'_, AppState>) {
    state.show_room_info = true;
    state.show_inventory = true;
    state.show_objectives = true;
    state.feedback_text = "You look around the room.".into();
}

// -- Focused sync systems --

fn sync_room_info(
    mut state: ResMut<'_, AppState>,
    room: Res<'_, navigation::CurrentRoom>,
) {
    state.room_name.clone_from(&room.display_name);
    state.room_desc.clone_from(&room.description);
}

fn sync_feedback(
    mut state: ResMut<'_, AppState>,
    feedback: Res<'_, interaction::FeedbackText>,
    camera_ctrl: Res<'_, camera::CameraController>,
) {
    if camera_ctrl.transitioning {
        state.feedback_text = "...".into();
    } else if !feedback.0.is_empty() {
        state.feedback_text.clone_from(&feedback.0);
    }
}

fn sync_objectives(
    mut state: ResMut<'_, AppState>,
    obj: Res<'_, objectives::Objectives>,
) {
    state.current_objective = obj.current_text().into();
}

fn sync_inventory(
    mut state: ResMut<'_, AppState>,
    inv: Res<'_, inventory::Inventory>,
) {
    state.inv_items = inv
        .items
        .iter()
        .map(|item| Inv_itemsRow {
            name: item.name.clone(),
            qty: item.qty.to_string(),
        })
        .collect();

    state.inv_hint = if inv.items.is_empty() {
        "Your pockets are empty.".into()
    } else {
        format!("{} item(s) collected.", inv.items.len())
    };
}

fn sync_hints(
    mut state: ResMut<'_, AppState>,
    hint_text: Res<'_, hints::HintText>,
) {
    state.hint_text.clone_from(&hint_text.0);
}

fn sync_win(
    mut state: ResMut<'_, AppState>,
    game_state: Res<'_, State<states::GameState>>,
) {
    let gs = *game_state.get();
    state.show_help = gs == states::GameState::Paused;
    state.show_victory = gs == states::GameState::Won;
}

/// Listen for GameWon messages and transition state.
fn handle_game_won(
    mut reader: MessageReader<interaction::GameWon>,
    mut next: ResMut<NextState<states::GameState>>,
) {
    for _ in reader.read() {
        next.set(states::GameState::Won);
    }
}

// -- UI Rendering --

fn render_ui(
    mut ctxs: EguiContexts<'_, '_>,
    mut state: ResMut<'_, AppState>,
    mut inv: ResMut<'_, inventory::Inventory>,
    mut feedback: ResMut<'_, interaction::FeedbackText>,
    game_state: Res<'_, State<states::GameState>>,
    mut next_game_state: ResMut<'_, NextState<states::GameState>>,
    mut exit: MessageWriter<'_, AppExit>,
    mut loaders_installed: Local<bool>,
) -> Result {
    if !*loaders_installed {
        egui_extras::install_image_loaders(ctxs.ctx_mut()?);
        *loaders_installed = true;
    }

    // Top panel: room name and description
    if state.show_room_info {
        egui::TopBottomPanel::top("room_info_panel").show(ctxs.ctx_mut()?, |ui| {
            render_room_info(ui, &mut state);
        });
    }

    // Right panel: inventory
    if state.show_inventory {
        egui::SidePanel::right("inventory_panel")
            .default_width(200.0)
            .show(ctxs.ctx_mut()?, |ui| {
                render_inventory(ui, &mut state);
            });
    }

    // Bottom panel: feedback + objectives + hints
    if state.show_objectives {
        egui::TopBottomPanel::bottom("objectives_panel").show(ctxs.ctx_mut()?, |ui| {
            render_objectives(ui, &mut state);
        });
    }

    // Window: help overlay (paused)
    if state.show_help {
        egui::Window::new("Help")
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctxs.ctx_mut()?, |ui| {
                render_help(ui, &mut state);
            });
    }

    // Window: victory screen
    if state.show_victory {
        egui::Window::new("Victory")
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctxs.ctx_mut()?, |ui| {
                render_victory(ui, &mut state);
            });
    }

    // Handle button events
    if state.on_resume_count > 0 {
        state.on_resume_count = 0;
        if *game_state.get() == states::GameState::Paused {
            next_game_state.set(states::GameState::Playing);
        }
    }
    if state.on_quit_count > 0 {
        state.on_quit_count = 0;
        exit.write(AppExit::Success);
    }

    // Handle combine button
    if state.on_combine_count > 0 {
        state.on_combine_count = 0;
        if inv.items.len() >= 2 {
            let a = inv.items[0].item_id.clone();
            let b = inv.items[1].item_id.clone();
            if let Some(result_name) = inv.combine(&a, &b) {
                feedback.0 = format!("You created a {}!", result_name);
            } else {
                feedback.0 = "These don't seem to work together.".into();
            }
        } else {
            feedback.0 = "You need at least two items to combine.".into();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_state_has_expected_fields() {
        let state = AppState::default();
        assert!(state.room_name.is_empty());
        assert!(state.room_desc.is_empty());
        assert!(state.feedback_text.is_empty());
        assert!(state.current_objective.is_empty());
        assert!(state.inv_items.is_empty());
        assert!(!state.show_help);
        assert!(!state.show_victory);
    }

    #[test]
    fn page_enum_has_expected_variants() {
        let _room = Page::RoomInfo;
        let _inv = Page::Inventory;
        let _obj = Page::Objectives;
        let _help = Page::Help;
        let _vic = Page::Victory;
        assert_eq!(Page::ALL.len(), 5);
    }

    #[test]
    fn page_default_is_room_info() {
        let page = Page::default();
        assert_eq!(page, Page::RoomInfo);
    }
}
