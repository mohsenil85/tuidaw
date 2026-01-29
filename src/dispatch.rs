use std::path::PathBuf;

use crate::audio::{self, AudioEngine};
use crate::panes::{FileBrowserPane, MixerPane, PianoRollPane, ServerPane, StripEditPane, StripPane};
use crate::scd_parser;
use crate::state::{CustomSynthDef, MixerSelection, ParamSpec, StripState};
use crate::ui::{Action, FileSelectAction, Frame, PaneManager};

/// Default path for save file
pub fn default_rack_path() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home)
            .join(".config")
            .join("tuidaw")
            .join("default.sqlite")
    } else {
        PathBuf::from("default.sqlite")
    }
}

/// Dispatch an action. Returns true if the app should quit.
pub fn dispatch_action(
    action: &Action,
    panes: &mut PaneManager,
    audio_engine: &mut AudioEngine,
    app_frame: &mut Frame,
    active_notes: &mut Vec<(u32, u8, u32)>,
) -> bool {
    match action {
        Action::Quit => return true,
        Action::AddStrip(_) => {
            panes.dispatch_to("strip", action);
            if audio_engine.is_running() {
                if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                    let _ = audio_engine.rebuild_strip_routing(strip_pane.state());
                }
            }
            panes.switch_to("strip");
        }
        Action::DeleteStrip(strip_id) => {
            let strip_id = *strip_id;
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                strip_pane.state_mut().remove_strip(strip_id);
            }
            if audio_engine.is_running() {
                if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                    let _ = audio_engine.rebuild_strip_routing(strip_pane.state());
                }
            }
        }
        Action::EditStrip(id) => {
            let strip_data = panes
                .get_pane_mut::<StripPane>("strip")
                .and_then(|sp| sp.state().strip(*id).cloned());
            if let Some(strip) = strip_data {
                if let Some(edit) = panes.get_pane_mut::<StripEditPane>("strip_edit") {
                    edit.set_strip(&strip);
                }
                panes.switch_to("strip_edit");
            }
        }
        Action::UpdateStrip(id) => {
            let id = *id;
            // Apply edits from strip_edit pane back to the strip
            let edits = panes.get_pane_mut::<StripEditPane>("strip_edit")
                .map(|edit| {
                    let mut dummy = crate::state::strip::Strip::new(id, crate::state::OscType::Saw);
                    edit.apply_to(&mut dummy);
                    dummy
                });
            if let Some(edited) = edits {
                if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                    if let Some(strip) = strip_pane.state_mut().strip_mut(id) {
                        strip.source = edited.source;
                        strip.source_params = edited.source_params;
                        strip.filter = edited.filter;
                        strip.effects = edited.effects;
                        strip.amp_envelope = edited.amp_envelope;
                        strip.polyphonic = edited.polyphonic;

                        // Handle track toggle
                        if edited.has_track != strip.has_track {
                            strip.has_track = edited.has_track;
                        }
                    }
                    // Sync piano roll tracks
                    let strips: Vec<(u32, bool)> = strip_pane.state().strips.iter()
                        .map(|s| (s.id, s.has_track))
                        .collect();
                    let pr = &mut strip_pane.state_mut().piano_roll;
                    for (sid, has_track) in strips {
                        if has_track && !pr.tracks.contains_key(&sid) {
                            pr.add_track(sid);
                        } else if !has_track && pr.tracks.contains_key(&sid) {
                            pr.remove_track(sid);
                        }
                    }
                }
            }
            if audio_engine.is_running() {
                if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                    let _ = audio_engine.rebuild_strip_routing(strip_pane.state());
                }
            }
            // Don't switch pane - stay in edit
        }
        Action::SaveRack => {
            let path = default_rack_path();
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            // Sync session state
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                app_frame.session.time_signature = strip_pane.state().piano_roll.time_signature;
            }
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                if let Err(e) = strip_pane.state().save(&path, &app_frame.session) {
                    eprintln!("Failed to save: {}", e);
                }
            }
            let name = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("default")
                .to_string();
            app_frame.set_project_name(name);
        }
        Action::LoadRack => {
            let path = default_rack_path();
            if path.exists() {
                match StripState::load(&path) {
                    Ok((state, loaded_session)) => {
                        if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                            strip_pane.set_state(state);
                        }
                        app_frame.session = loaded_session;
                        let name = path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("default")
                            .to_string();
                        app_frame.set_project_name(name);
                    }
                    Err(e) => {
                        eprintln!("Failed to load: {}", e);
                    }
                }
            }
        }
        Action::ConnectServer => {
            let result = audio_engine.connect("127.0.0.1:57110");
            if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                match result {
                    Ok(()) => {
                        // Load built-in synthdefs
                        let synthdef_dir = std::path::Path::new("synthdefs");
                        let builtin_result = audio_engine.load_synthdefs(synthdef_dir);

                        // Also load custom synthdefs from config dir
                        let config_dir = config_synthdefs_dir();
                        let custom_result = if config_dir.exists() {
                            audio_engine.load_synthdefs(&config_dir)
                        } else {
                            Ok(())
                        };

                        match (builtin_result, custom_result) {
                            (Ok(()), Ok(())) => {
                                server.set_status(audio::ServerStatus::Connected, "Connected");
                            }
                            (Err(e), _) | (_, Err(e)) => {
                                server.set_status(
                                    audio::ServerStatus::Connected,
                                    &format!("Connected (synthdef warning: {})", e),
                                );
                            }
                        }

                        if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                            let _ = audio_engine.rebuild_strip_routing(strip_pane.state());
                        }
                    }
                    Err(e) => {
                        server.set_status(audio::ServerStatus::Error, &e.to_string())
                    }
                }
            }
        }
        Action::DisconnectServer => {
            audio_engine.disconnect();
            if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                server.set_status(audio_engine.status(), "Disconnected");
                server.set_server_running(audio_engine.server_running());
            }
        }
        Action::StartServer => {
            let result = audio_engine.start_server();
            if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                match result {
                    Ok(()) => {
                        server.set_status(audio::ServerStatus::Running, "Server started");
                        server.set_server_running(true);
                    }
                    Err(e) => {
                        server.set_status(audio::ServerStatus::Error, &e);
                        server.set_server_running(false);
                    }
                }
            }
        }
        Action::StopServer => {
            audio_engine.stop_server();
            if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                server.set_status(audio::ServerStatus::Stopped, "Server stopped");
                server.set_server_running(false);
            }
        }
        Action::CompileSynthDefs => {
            let scd_path = std::path::Path::new("synthdefs/compile.scd");
            match audio_engine.compile_synthdefs_async(scd_path) {
                Ok(()) => {
                    if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                        server.set_status(audio_engine.status(), "Compiling synthdefs...");
                    }
                }
                Err(e) => {
                    if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                        server.set_status(audio_engine.status(), &e);
                    }
                }
            }
        }
        Action::LoadSynthDefs => {
            // Load built-in synthdefs
            let synthdef_dir = std::path::Path::new("synthdefs");
            let builtin_result = audio_engine.load_synthdefs(synthdef_dir);

            // Also load custom synthdefs from config dir
            let config_dir = config_synthdefs_dir();
            let custom_result = if config_dir.exists() {
                audio_engine.load_synthdefs(&config_dir)
            } else {
                Ok(())
            };

            if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                match (builtin_result, custom_result) {
                    (Ok(()), Ok(())) => {
                        server.set_status(audio_engine.status(), "Synthdefs loaded (built-in + custom)");
                    }
                    (Err(e), _) => {
                        server.set_status(audio_engine.status(), &format!("Error loading built-in: {}", e));
                    }
                    (_, Err(e)) => {
                        server.set_status(audio_engine.status(), &format!("Error loading custom: {}", e));
                    }
                }
            }
        }
        Action::SetStripParam(strip_id, ref param, value) => {
            // Real-time param update - update state and audio
            let _ = strip_id;
            let _ = param;
            let _ = value;
            // TODO: implement real-time param setting on audio engine
        }
        Action::MixerMove(delta) => {
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                strip_pane.state_mut().mixer_move(*delta);
            }
        }
        Action::MixerJump(direction) => {
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                strip_pane.state_mut().mixer_jump(*direction);
            }
        }
        Action::MixerAdjustLevel(delta) => {
            let mut bus_update: Option<(u8, f32, bool, f32)> = None;
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                let state = strip_pane.state_mut();
                match state.mixer_selection {
                    MixerSelection::Strip(idx) => {
                        if let Some(strip) = state.strips.get_mut(idx) {
                            strip.level = (strip.level + delta).clamp(0.0, 1.0);
                        }
                    }
                    MixerSelection::Bus(id) => {
                        if let Some(bus) = state.bus_mut(id) {
                            bus.level = (bus.level + delta).clamp(0.0, 1.0);
                        }
                        if let Some(bus) = state.bus(id) {
                            let mute = state.effective_bus_mute(bus);
                            bus_update = Some((id, bus.level, mute, bus.pan));
                        }
                    }
                    MixerSelection::Master => {
                        state.master_level = (state.master_level + delta).clamp(0.0, 1.0);
                    }
                }
            }
            if audio_engine.is_running() {
                if let Some((bus_id, level, mute, pan)) = bus_update {
                    let _ = audio_engine.set_bus_mixer_params(bus_id, level, mute, pan);
                }
                // Rebuild to update strip output levels
                if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                    let _ = audio_engine.rebuild_strip_routing(strip_pane.state());
                }
            }
        }
        Action::MixerToggleMute => {
            let mut bus_update: Option<(u8, f32, bool, f32)> = None;
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                let state = strip_pane.state_mut();
                match state.mixer_selection {
                    MixerSelection::Strip(idx) => {
                        if let Some(strip) = state.strips.get_mut(idx) {
                            strip.mute = !strip.mute;
                        }
                    }
                    MixerSelection::Bus(id) => {
                        if let Some(bus) = state.bus_mut(id) {
                            bus.mute = !bus.mute;
                        }
                        if let Some(bus) = state.bus(id) {
                            let mute = state.effective_bus_mute(bus);
                            bus_update = Some((id, bus.level, mute, bus.pan));
                        }
                    }
                    MixerSelection::Master => {
                        state.master_mute = !state.master_mute;
                    }
                }
            }
            if audio_engine.is_running() {
                if let Some((bus_id, level, mute, pan)) = bus_update {
                    let _ = audio_engine.set_bus_mixer_params(bus_id, level, mute, pan);
                }
                if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                    let _ = audio_engine.rebuild_strip_routing(strip_pane.state());
                }
            }
        }
        Action::MixerToggleSolo => {
            let mut bus_updates: Vec<(u8, f32, bool, f32)> = Vec::new();
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                let state = strip_pane.state_mut();
                match state.mixer_selection {
                    MixerSelection::Strip(idx) => {
                        if let Some(strip) = state.strips.get_mut(idx) {
                            strip.solo = !strip.solo;
                        }
                    }
                    MixerSelection::Bus(id) => {
                        if let Some(bus) = state.bus_mut(id) {
                            bus.solo = !bus.solo;
                        }
                    }
                    MixerSelection::Master => {}
                }
                for bus in &state.buses {
                    let mute = state.effective_bus_mute(bus);
                    bus_updates.push((bus.id, bus.level, mute, bus.pan));
                }
            }
            if audio_engine.is_running() {
                for (bus_id, level, mute, pan) in bus_updates {
                    let _ = audio_engine.set_bus_mixer_params(bus_id, level, mute, pan);
                }
                if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                    let _ = audio_engine.rebuild_strip_routing(strip_pane.state());
                }
            }
        }
        Action::MixerCycleSection => {
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                strip_pane.state_mut().mixer_cycle_section();
            }
        }
        Action::MixerCycleOutput => {
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                strip_pane.state_mut().mixer_cycle_output();
            }
        }
        Action::MixerCycleOutputReverse => {
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                strip_pane.state_mut().mixer_cycle_output_reverse();
            }
        }
        Action::MixerAdjustSend(bus_id, delta) => {
            let bus_id = *bus_id;
            let delta = *delta;
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                let state = strip_pane.state_mut();
                if let MixerSelection::Strip(idx) = state.mixer_selection {
                    if let Some(strip) = state.strips.get_mut(idx) {
                        if let Some(send) = strip.sends.iter_mut().find(|s| s.bus_id == bus_id) {
                            send.level = (send.level + delta).clamp(0.0, 1.0);
                        }
                    }
                }
            }
        }
        Action::MixerToggleSend(bus_id) => {
            let bus_id = *bus_id;
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                let state = strip_pane.state_mut();
                if let MixerSelection::Strip(idx) = state.mixer_selection {
                    if let Some(strip) = state.strips.get_mut(idx) {
                        if let Some(send) = strip.sends.iter_mut().find(|s| s.bus_id == bus_id) {
                            send.enabled = !send.enabled;
                            if send.enabled && send.level <= 0.0 {
                                send.level = 0.5;
                            }
                        }
                    }
                }
            }
            if audio_engine.is_running() {
                if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                    let _ = audio_engine.rebuild_strip_routing(strip_pane.state());
                }
            }
        }
        Action::PianoRollToggleNote => {
            if let Some(pr_pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
                let pitch = pr_pane.cursor_pitch();
                let tick = pr_pane.cursor_tick();
                let dur = pr_pane.default_duration();
                let vel = pr_pane.default_velocity();
                let track = pr_pane.current_track();
                if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                    strip_pane.state_mut().piano_roll.toggle_note(track, pitch, tick, dur, vel);
                }
            }
        }
        Action::PianoRollAdjustDuration(delta) => {
            let delta = *delta;
            if let Some(pr_pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
                pr_pane.adjust_default_duration(delta);
            }
        }
        Action::PianoRollAdjustVelocity(delta) => {
            let delta = *delta;
            if let Some(pr_pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
                pr_pane.adjust_default_velocity(delta);
            }
        }
        Action::PianoRollPlayStop => {
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                let pr = &mut strip_pane.state_mut().piano_roll;
                pr.playing = !pr.playing;
                if !pr.playing {
                    pr.playhead = 0;
                    if audio_engine.is_running() {
                        audio_engine.release_all_voices();
                    }
                    active_notes.clear();
                }
            }
            // Clear recording if stopping via normal play/stop
            if let Some(pr_pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
                pr_pane.set_recording(false);
            }
        }
        Action::PianoRollPlayStopRecord => {
            let is_playing = panes
                .get_pane_mut::<StripPane>("strip")
                .map(|sp| sp.state().piano_roll.playing)
                .unwrap_or(false);

            if !is_playing {
                // Start playing + recording
                if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                    strip_pane.state_mut().piano_roll.playing = true;
                }
                if let Some(pr_pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
                    pr_pane.set_recording(true);
                }
            } else {
                // Stop playing + recording
                if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                    let pr = &mut strip_pane.state_mut().piano_roll;
                    pr.playing = false;
                    pr.playhead = 0;
                }
                if audio_engine.is_running() {
                    audio_engine.release_all_voices();
                }
                active_notes.clear();
                if let Some(pr_pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
                    pr_pane.set_recording(false);
                }
            }
        }
        Action::PianoRollToggleLoop => {
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                let pr = &mut strip_pane.state_mut().piano_roll;
                pr.looping = !pr.looping;
            }
        }
        Action::PianoRollSetLoopStart => {
            if let Some(pr_pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
                let tick = pr_pane.cursor_tick();
                if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                    strip_pane.state_mut().piano_roll.loop_start = tick;
                }
            }
        }
        Action::PianoRollSetLoopEnd => {
            if let Some(pr_pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
                let tick = pr_pane.cursor_tick();
                if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                    strip_pane.state_mut().piano_roll.loop_end = tick;
                }
            }
        }
        Action::PianoRollChangeTrack(delta) => {
            let delta = *delta;
            let track_count = panes
                .get_pane_mut::<StripPane>("strip")
                .map(|sp| sp.state().piano_roll.track_order.len())
                .unwrap_or(0);
            if let Some(pr_pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
                pr_pane.change_track(delta, track_count);
            }
        }
        Action::PianoRollCycleTimeSig => {
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                let pr = &mut strip_pane.state_mut().piano_roll;
                pr.time_signature = match pr.time_signature {
                    (4, 4) => (3, 4),
                    (3, 4) => (6, 8),
                    (6, 8) => (5, 4),
                    (5, 4) => (7, 8),
                    _ => (4, 4),
                };
            }
        }
        Action::PianoRollTogglePolyMode => {
            let track_idx = panes
                .get_pane_mut::<PianoRollPane>("piano_roll")
                .map(|pr| pr.current_track());
            if let Some(idx) = track_idx {
                if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                    if let Some(track) = strip_pane.state_mut().piano_roll.track_at_mut(idx) {
                        track.polyphonic = !track.polyphonic;
                    }
                }
            }
        }
        Action::PianoRollJump(_direction) => {
            if let Some(pr_pane) = panes.get_pane_mut::<PianoRollPane>("piano_roll") {
                pr_pane.jump_to_end();
            }
        }
        Action::PianoRollPlayNote(pitch, velocity) => {
            let pitch = *pitch;
            let velocity = *velocity;
            // Get the current track's strip_id and polyphonic mode
            let track_info: Option<(u32, bool)> = {
                let track_idx = panes
                    .get_pane_mut::<PianoRollPane>("piano_roll")
                    .map(|pr| pr.current_track());
                if let Some(idx) = track_idx {
                    panes
                        .get_pane_mut::<StripPane>("strip")
                        .and_then(|sp| sp.state().piano_roll.track_at(idx))
                        .map(|t| (t.module_id, t.polyphonic))
                } else {
                    None
                }
            };

            if let Some((strip_id, polyphonic)) = track_info {
                if audio_engine.is_running() {
                    // Spawn voice
                    let vel_f = velocity as f32 / 127.0;
                    let state = panes
                        .get_pane_mut::<StripPane>("strip")
                        .map(|sp| sp.state().clone());
                    if let Some(state) = state {
                        let _ = audio_engine.spawn_voice(strip_id, pitch, vel_f, 0.0, polyphonic, &state);
                        let duration_ticks = 240; // Half beat for staccato feel
                        active_notes.push((strip_id, pitch, duration_ticks));
                    }
                }

                // Record note if recording
                let recording_info = panes
                    .get_pane_mut::<PianoRollPane>("piano_roll")
                    .filter(|pr| pr.is_recording())
                    .map(|pr| (pr.current_track(), pr.default_duration(), pr.default_velocity()));
                if let Some((track_idx, duration, vel)) = recording_info {
                    if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                        let playhead = strip_pane.state().piano_roll.playhead;
                        strip_pane.state_mut().piano_roll.toggle_note(track_idx, pitch, playhead, duration, vel);
                    }
                }
            }
        }
        Action::StripPlayNote(pitch, velocity) => {
            let pitch = *pitch;
            let velocity = *velocity;
            // Get the selected strip's id and polyphonic mode
            let strip_info: Option<(u32, bool)> = panes
                .get_pane_mut::<StripPane>("strip")
                .and_then(|sp| sp.state().selected_strip().map(|s| (s.id, s.polyphonic)));

            if let Some((strip_id, polyphonic)) = strip_info {
                if audio_engine.is_running() {
                    let vel_f = velocity as f32 / 127.0;
                    let state = panes
                        .get_pane_mut::<StripPane>("strip")
                        .map(|sp| sp.state().clone());
                    if let Some(state) = state {
                        let _ = audio_engine.spawn_voice(strip_id, pitch, vel_f, 0.0, polyphonic, &state);
                        let duration_ticks = 240;
                        active_notes.push((strip_id, pitch, duration_ticks));
                    }
                }
            }
        }
        Action::UpdateSession(ref session) => {
            app_frame.session = session.clone();
            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                strip_pane.state_mut().piano_roll.time_signature = session.time_signature;
                strip_pane.state_mut().piano_roll.bpm = session.bpm as f32;
            }
            panes.switch_to("strip");
        }
        Action::OpenFileBrowser(ref file_action) => {
            if let Some(fb) = panes.get_pane_mut::<FileBrowserPane>("file_browser") {
                fb.open_for(file_action.clone(), None);
            }
            panes.switch_to("file_browser");
        }
        Action::ImportCustomSynthDef(ref path) => {
            // Read and parse the .scd file
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    match scd_parser::parse_scd_file(&content) {
                        Ok(parsed) => {
                            // Create params with inferred ranges
                            let params: Vec<ParamSpec> = parsed
                                .params
                                .iter()
                                .map(|(name, default)| {
                                    let (min, max) =
                                        scd_parser::infer_param_range(name, *default);
                                    ParamSpec {
                                        name: name.clone(),
                                        default: *default,
                                        min,
                                        max,
                                    }
                                })
                                .collect();

                            // Create the custom synthdef entry
                            let synthdef_name = parsed.name.clone();
                            let custom = CustomSynthDef {
                                id: 0, // Will be set by registry.add()
                                name: parsed.name.clone(),
                                synthdef_name: synthdef_name.clone(),
                                source_path: path.clone(),
                                params,
                            };

                            // Register it
                            if let Some(strip_pane) = panes.get_pane_mut::<StripPane>("strip") {
                                let _id = strip_pane.state_mut().custom_synthdefs.add(custom);
                            }

                            // Copy the .scd file to the config synthdefs directory
                            let config_dir = config_synthdefs_dir();
                            let _ = std::fs::create_dir_all(&config_dir);

                            // Copy .scd file
                            if let Some(filename) = path.file_name() {
                                let dest = config_dir.join(filename);
                                let _ = std::fs::copy(path, &dest);
                            }

                            // Compile and load the synthdef
                            match compile_and_load_synthdef(path, &config_dir, &synthdef_name, audio_engine) {
                                Ok(_) => {
                                    if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                                        server.set_status(audio_engine.status(), &format!("Loaded custom synthdef: {}", synthdef_name));
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Failed to compile/load synthdef: {}", e);
                                    if let Some(server) = panes.get_pane_mut::<ServerPane>("server") {
                                        server.set_status(audio_engine.status(), &format!("Import error: {}", e));
                                    }
                                }
                            }

                            // Switch back to add pane
                            panes.switch_to("add");
                        }
                        Err(e) => {
                            eprintln!("Failed to parse .scd file: {}", e);
                            panes.switch_to("add");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read .scd file: {}", e);
                    panes.switch_to("add");
                }
            }
        }
        _ => {}
    }
    false
}

/// Get the config directory for custom synthdefs
fn config_synthdefs_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home)
            .join(".config")
            .join("tuidaw")
            .join("synthdefs")
    } else {
        PathBuf::from("synthdefs")
    }
}

/// Find sclang executable, checking common locations
fn find_sclang() -> Option<PathBuf> {
    // Check if sclang is in PATH
    if let Ok(output) = std::process::Command::new("which").arg("sclang").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }

    // Common macOS locations
    let candidates = [
        "/Applications/SuperCollider.app/Contents/MacOS/sclang",
        "/Applications/SuperCollider/SuperCollider.app/Contents/MacOS/sclang",
        "/usr/local/bin/sclang",
        "/opt/homebrew/bin/sclang",
    ];

    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Compile a .scd file using sclang and load it into scsynth
fn compile_and_load_synthdef(
    scd_path: &std::path::Path,
    output_dir: &std::path::Path,
    synthdef_name: &str,
    audio_engine: &mut AudioEngine,
) -> Result<(), String> {
    // Find sclang
    let sclang = find_sclang().ok_or_else(|| {
        "sclang not found. Install SuperCollider or add sclang to PATH.".to_string()
    })?;

    // Read the original .scd file
    let scd_content = std::fs::read_to_string(scd_path)
        .map_err(|e| format!("Failed to read .scd file: {}", e))?;

    // Replace directory references with the actual output directory
    // Handle both patterns: `dir ? thisProcess...` and just `thisProcess...`
    let output_dir_str = format!("\"{}\"", output_dir.display());
    let modified_content = scd_content
        .replace("dir ? thisProcess.nowExecutingPath.dirname", &output_dir_str)
        .replace("thisProcess.nowExecutingPath.dirname", &output_dir_str);

    // Wrap in a block that exits when done
    let compile_script = format!(
        "(\n{}\n\"SUCCESS\".postln;\n0.exit;\n)",
        modified_content
    );

    // Write temp compile script
    let temp_script = std::env::temp_dir().join("tuidaw_compile_custom.scd");
    std::fs::write(&temp_script, &compile_script)
        .map_err(|e| format!("Failed to write compile script: {}", e))?;

    // Run sclang with a timeout by spawning and waiting
    let mut child = std::process::Command::new(&sclang)
        .arg(&temp_script)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run sclang: {}", e))?;

    // Wait up to 30 seconds for compilation
    let timeout = std::time::Duration::from_secs(30);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err("sclang compilation timed out".to_string());
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => return Err(format!("Error waiting for sclang: {}", e)),
        }
    }

    let output = child.wait_with_output()
        .map_err(|e| format!("Failed to get sclang output: {}", e))?;

    // Check for errors (but ignore common non-error messages)
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Look for actual errors, not just any "ERROR" in output
    let has_error = stderr.lines().any(|line| {
        line.contains("ERROR:") || line.contains("FAILURE")
    }) || stdout.lines().any(|line| {
        line.starts_with("ERROR:") || line.contains("FAILURE")
    });

    if has_error {
        return Err(format!("sclang error: {}{}", stdout, stderr));
    }

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_script);

    // Load the .scsyndef into scsynth if connected
    if audio_engine.is_running() {
        let scsyndef_path = output_dir.join(format!("{}.scsyndef", synthdef_name));
        if scsyndef_path.exists() {
            audio_engine.load_synthdef_file(&scsyndef_path)?;
        } else {
            // Try loading all synthdefs from the directory as fallback
            audio_engine.load_synthdefs(output_dir)?;
        }
    }

    Ok(())
}
