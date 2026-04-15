use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use bevy_tweening::TweeningPlugin;

mod camera;
mod components;
#[cfg(not(target_arch = "wasm32"))]
mod debug;
mod game_data;
mod hints;
mod input_config;
mod input_intent;
mod interaction;
mod inventory;
mod item_preview;
mod navigation;
mod objectives;
mod save;
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
        "content/title.md",
        "content/game_over.md",
    }
}

use pages::*;

impl Resource for Page {}
impl Resource for AppState {}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "LitAdventure".into(),
                    ..default()
                }),
                ..default()
            })
            .set(bevy::asset::AssetPlugin {
                meta_check: bevy::asset::AssetMetaCheck::Never,
                ..default()
            })
        )
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
        .add_plugins(input_config::InputConfigPlugin)
        .add_plugins(input_intent::InputIntentPlugin)
        .add_plugins(save::SavePlugin)
        .add_plugins(item_preview::ItemPreviewPlugin)
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
        .add_systems(EguiPrimaryContextPass, (drive_overlay_focus, render_ui).chain())
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
                sync_overlays.run_if(state_changed::<states::GameState>),
                handle_game_won,
                handle_game_over,
            ),
        );

    #[cfg(not(target_arch = "wasm32"))]
    app.add_plugins(debug::DebugPlugin);

    app.run();
}

// -- One-time UI initialization --

fn init_ui_state(mut state: ResMut<'_, AppState>) {
    state.show_title = true;
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
    state.inv_hint = if inv.items.is_empty() {
        "Your pockets are empty.".into()
    } else if inv.items.len() >= 2 {
        "Drag or click two items to combine.".into()
    } else {
        "1 item collected.".into()
    };
}

fn sync_hints(
    mut state: ResMut<'_, AppState>,
    hint_text: Res<'_, hints::HintText>,
) {
    state.hint_text.clone_from(&hint_text.0);
}

fn sync_overlays(
    mut state: ResMut<'_, AppState>,
    game_state: Res<'_, State<states::GameState>>,
) {
    let gs = *game_state.get();
    state.show_title = gs == states::GameState::Title;
    state.show_help = gs == states::GameState::Paused;
    state.show_victory = gs == states::GameState::Won;
    state.show_game_over = gs == states::GameState::GameOver;

    // Only show game panels when playing or paused
    let in_game = gs == states::GameState::Playing || gs == states::GameState::Paused;
    state.show_room_info = in_game;
    state.show_inventory = in_game;
    state.show_objectives = in_game;
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

/// Listen for GameOver messages and transition state.
fn handle_game_over(
    mut reader: MessageReader<interaction::GameOver>,
    mut next: ResMut<NextState<states::GameState>>,
) {
    for _ in reader.read() {
        next.set(states::GameState::GameOver);
    }
}


/// Drive egui keyboard focus during overlay states (Title, Pause, Victory, GameOver).
/// Continuously nudges focus toward the first button until one has focus.
/// Tab cycling between buttons is handled natively by egui's Focus::begin_pass.
fn drive_overlay_focus(
    mut ctxs: EguiContexts<'_, '_>,
    game_state: Res<State<states::GameState>>,
) -> Result {
    let is_overlay = matches!(
        game_state.get(),
        states::GameState::Title
            | states::GameState::Paused
            | states::GameState::Won
            | states::GameState::GameOver
    );
    if !is_overlay {
        return Ok(());
    }

    let ctx = ctxs.ctx_mut()?;
    // Every frame during an overlay: if nothing has focus, push toward first focusable button.
    // Overlay windows use .resizable(false).interactable(false) to prevent resize handles
    // and the window body from stealing focus before buttons.
    if ctx.memory(|mem| mem.focused().is_none()) {
        ctx.memory_mut(|mem| mem.move_focus(egui::FocusDirection::Next));
    }
    Ok(())
}

// -- UI Rendering --

fn render_ui(
    mut ctxs: EguiContexts<'_, '_>,
    mut state: ResMut<'_, AppState>,
    mut inv: ResMut<'_, inventory::Inventory>,
    mut feedback: ResMut<'_, interaction::FeedbackText>,
    mut objectives: ResMut<'_, objectives::Objectives>,
    mut camera_ctrl: ResMut<'_, camera::CameraController>,
    mut hint_state: ResMut<'_, hints::HintState>,
    mut combine_state: ResMut<'_, input_intent::CombineState>,
    game_state: Res<'_, State<states::GameState>>,
    mut next_game_state: ResMut<'_, NextState<states::GameState>>,
    mut save_events: MessageWriter<'_, save::SaveRequested>,
    mut load_events: MessageWriter<'_, save::LoadRequested>,
    item_previews: Res<'_, item_preview::ItemPreviews>,
    mut preview_events: MessageWriter<'_, item_preview::PreviewRequested>,
    mut room_transition_events: MessageWriter<'_, navigation::RoomTransition>,
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

    // Right panel: inventory (draggable item images + litui hint text)
    if state.show_inventory {
        // Pre-collect (item_id, name, tex_id) before borrowing ctxs mutably
        let tex_ids: Vec<(String, String, Option<egui::TextureId>)> = inv
            .items
            .iter()
            .map(|item| {
                let tex_id = item_previews
                    .previews
                    .get(&item.item_id)
                    .and_then(|h| ctxs.image_id(h));
                (item.item_id.clone(), item.name.clone(), tex_id)
            })
            .collect();

        // Track combine result from drag-drop or click-select (applied after panel)
        let mut combine_pair: Option<(String, String)> = None;

        // Snapshot combine mode state before the panel closure borrows it
        let combine_active = combine_state.active;
        let combine_cursor = combine_state.cursor;
        let combine_first = combine_state.first_selection.clone();

        egui::SidePanel::right("inventory_panel")
            .default_width(200.0)
            .show(ctxs.ctx_mut()?, |ui| {
                if combine_active {
                    ui.colored_label(egui::Color32::GOLD, "-- Combine Mode --");
                }

                if !tex_ids.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        for (idx, (item_id, name, tex_id)) in tex_ids.iter().enumerate() {
                            let drag_id = egui::Id::new(("inv_item", item_id.as_str()));
                            // Highlight: gold if selected as first item, cyan if cursor
                            let is_first = combine_first.as_deref() == Some(item_id.as_str());
                            let is_cursor = combine_active && idx == combine_cursor;
                            let stroke = if is_first {
                                egui::Stroke::new(2.0, egui::Color32::GOLD)
                            } else if is_cursor {
                                egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE)
                            } else {
                                egui::Stroke::NONE
                            };
                            let frame = egui::Frame::new()
                                .inner_margin(egui::Margin::same(4))
                                .stroke(stroke);

                            let (_, dropped) =
                                ui.dnd_drop_zone::<String, _>(frame, |ui| {
                                    ui.dnd_drag_source(drag_id, item_id.clone(), |ui| {
                                        ui.vertical(|ui| {
                                            if let Some(tid) = tex_id {
                                                let img = egui::Image::new(
                                                    egui::load::SizedTexture::new(
                                                        *tid,
                                                        egui::vec2(64.0, 64.0),
                                                    ),
                                                )
                                                .sense(egui::Sense::click());
                                                ui.add(img);
                                            }
                                            ui.small(name);
                                        });
                                    });
                                });

                            // Drag-drop combine
                            if let Some(dragged_id) = dropped {
                                if *dragged_id != *item_id {
                                    combine_pair =
                                        Some((dragged_id.to_string(), item_id.clone()));
                                }
                            }
                        }
                    });
                    ui.separator();
                }

                // Litui handles hint text
                render_inventory(ui, &mut state);
            });

        // Consume keyboard combine result (from CombineState)
        if combine_pair.is_none() {
            if let Some(pair) = combine_state.pending_combine.take() {
                combine_pair = Some(pair);
            }
        }

        // Apply combine result outside the panel closure (needs &mut inv)
        if let Some((a, b)) = combine_pair {
            if let Some(result_name) = inv.combine(&a, &b) {
                feedback.0 = format!("You created a {}!", result_name);
                if let Some(result_item) = inv.items.last() {
                    preview_events.write(item_preview::PreviewRequested {
                        item_id: result_item.item_id.clone(),
                        name: result_item.name.clone(),
                    });
                }
            } else {
                feedback.0 = "These don't seem to work together.".into();
            }
        }
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
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .interactable(false)
            .show(ctxs.ctx_mut()?, |ui| {
                render_help(ui, &mut state);
            });
    }

    // Window: victory screen
    if state.show_victory {
        egui::Window::new("Victory")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .interactable(false)
            .show(ctxs.ctx_mut()?, |ui| {
                render_victory(ui, &mut state);
            });
    }

    // Window: game over screen
    if state.show_game_over {
        egui::Window::new("Game Over")
            .collapsible(false)
            .title_bar(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .interactable(false)
            .show(ctxs.ctx_mut()?, |ui| {
                render_game_over(ui, &mut state);
            });
    }

    // Window: title screen
    if state.show_title {
        egui::Window::new("Title")
            .collapsible(false)
            .title_bar(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .interactable(false)
            .show(ctxs.ctx_mut()?, |ui| {
                render_title(ui, &mut state);
            });
    }

    // Handle button events
    if state.on_resume_count > 0 {
        state.on_resume_count = 0;
        if *game_state.get() == states::GameState::Paused {
            next_game_state.set(states::GameState::Playing);
        }
    }
    if state.on_start_count > 0 {
        state.on_start_count = 0;
        next_game_state.set(states::GameState::Loading);
    }

    // All restart buttons (victory, help/pause, game over) do the same thing
    let any_restart = state.on_restart_count > 0
        || state.on_restart_help_count > 0
        || state.on_restart_gameover_count > 0;
    if any_restart {
        state.on_restart_count = 0;
        state.on_restart_help_count = 0;
        state.on_restart_gameover_count = 0;
        inv.items.clear();
        feedback.0 = "You look around the room.".into();
        objectives.all.clear();
        camera_ctrl.history.clear();
        camera_ctrl.current_spot = Some("room_overview".into());
        camera_ctrl.transitioning = false;
        hint_state.idle_time = 0.0;
        hint_state.available_tier = 0;
        // Reload the starting room (despawns current, loads study.glb)
        room_transition_events.write(navigation::RoomTransition {
            target_room: "study".into(),
            entry_spot: "room_overview".into(),
        });
        next_game_state.set(states::GameState::Title);
    }

    // Handle save/load
    if state.on_save_count > 0 {
        state.on_save_count = 0;
        save_events.write(save::SaveRequested);
    }
    if state.on_continue_count > 0 {
        state.on_continue_count = 0;
        next_game_state.set(states::GameState::Loading);
        load_events.write(save::LoadRequested);
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
        assert!(state.inv_hint.is_empty());
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
        let _title = Page::Title;
        let _game_over = Page::GameOver;
        assert_eq!(Page::ALL.len(), 7);
    }

    #[test]
    fn page_default_is_room_info() {
        let page = Page::default();
        assert_eq!(page, Page::RoomInfo);
    }
}
