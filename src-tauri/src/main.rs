// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod workout;

use antrs::node;
use antrs::profile::{fitness_equipment, heart_rate_monitor};
use log::{debug, error, trace};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use tauri::{State, Window};
use workout::Workout;

struct AppState {
    config: config::AppConfig,
    node: Mutex<Option<node::Node>>,
    hrm_channel: RwLock<Option<u8>>,
    fe_channel: RwLock<Option<u8>>,
    workout: Mutex<Option<Workout>>,
}

#[derive(Clone, serde::Serialize)]
struct HeartRateUpdate {
    value: u8,
    timestamp: u128,
}

#[derive(Clone, serde::Serialize)]
struct FitnessEquipmentUpdate {
    cadence: Option<u8>,
    power: Option<u16>,
}

#[derive(Clone, serde::Serialize)]
struct WorkoutStatus {
    step_index: usize,
    step_elapsed: u64,
}

#[tauri::command]
async fn open_node(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    debug!("opening node");
    let mut node = state.node.lock().unwrap();

    if let Some(_) = *node {
        return Ok(());
    }

    let nb = antrs::node::NodeBuilder::new(state.config.network_key);
    let mut created = nb.build();

    match created.open() {
        Ok(_) => trace!("node opened successfully!"),
        Err(e) => {
            error!("failed to open node: {}", e);
            return Err(format!("failed to open node: {}", e));
        }
    }

    *node = Some(created);

    Ok(())
}

#[tauri::command]
async fn open_hrm(state: State<'_, Arc<AppState>>, window: Window) -> Result<(), String> {
    debug!("opening heart rate monitor");

    let mut node = state.node.lock().unwrap();

    if let Some(ref mut node) = *node {
        let (hrm, receiver) =
            heart_rate_monitor::new_paired(state.config.devices.heart_rate_monitor.into());
        let channel = node
            .assign_channel(Box::new(hrm))
            .map_err(|e| format!("assigning channel: {}", e))?;

        std::thread::spawn(move || {
            for message in receiver.iter() {
                window
                    .emit(
                        "heart_rate",
                        HeartRateUpdate {
                            value: message.computed_heart_rate,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis(),
                        },
                    )
                    .unwrap();
            }
        });

        {
            let mut hrm_channel = state.hrm_channel.write().unwrap();
            *hrm_channel = Some(channel);
        }

        Ok(())
    } else {
        Err("node not open".into())
    }
}

#[tauri::command]
async fn open_fitness_equipment(
    state: State<'_, Arc<AppState>>,
    window: Window,
) -> Result<(), String> {
    debug!("opening fitness equipment");

    let mut node = state.node.lock().unwrap();

    if let Some(ref mut node) = *node {
        let (fe, receiver) =
            fitness_equipment::new_paired(state.config.devices.fitness_equipment.into());
        let channel = node
            .assign_channel(Box::new(fe))
            .map_err(|e| format!("assigning channel: {}", e))?;

        std::thread::spawn(move || {
            for message in receiver.iter() {
                if let fitness_equipment::FitnessEquipmentData::StationaryBike(data) = message {
                    window
                        .emit(
                            "fitness_equipment_data",
                            FitnessEquipmentUpdate {
                                cadence: data.cadence,
                                power: data.instantaneous_power,
                            },
                        )
                        .unwrap();
                }
            }
        });

        {
            let mut fe_channel = state.fe_channel.write().unwrap();
            *fe_channel = Some(channel);
        }

        Ok(())
    } else {
        Err("node not open".into())
    }
}

#[tauri::command]
async fn load_workout(state: State<'_, Arc<AppState>>) -> Result<Workout, String> {
    let wko = workout::load_workout()?;

    {
        let mut state_wko = state.workout.lock().unwrap();
        *state_wko = Some(wko.clone());
    }

    Ok(wko)
}

#[tauri::command]
async fn start_workout(state: State<'_, Arc<AppState>>, window: Window) -> Result<(), String> {
    let state_wko = state.workout.lock().unwrap();

    let wko = match state_wko.clone() {
        Some(wko) => wko,
        None => return Err("no workout loaded".into()),
    };

    let mut step_start = std::time::Instant::now();
    let mut step_index = 0;
    {
        let power: u16 = wko.steps[step_index].set_point.try_into().unwrap();

        debug!(
            "starting workout at step: {}, need to set power to {}",
            step_index, power,
        );
        let channel = state.fe_channel.read().unwrap();

        if let Some(channel) = *channel {
            let erg = fitness_equipment::target_power_message(channel, power * 4);
            let node = state.node.lock().unwrap();
            if let Some(ref node) = &*node {
                if let Err(e) = node.write_message(erg, core::time::Duration::from_secs(1)) {
                    error!("failed to write message: {}", e);
                }
            } else {
                error!("no node");
            }
        } else {
            error!("no channel for fitness equipment");
        }
    }

    loop {
        let mut step_elapsed = (step_start.elapsed() * 1).as_secs();

        let step_changed = if step_elapsed >= wko.steps[step_index].duration.into() {
            if wko.steps.len() > step_index + 1 {
                step_index += 1;
                step_start = std::time::Instant::now();
                step_elapsed = 0;
            } else {
                break;
            }

            true
        } else {
            false
        };

        if step_changed {
            let power: u16 = wko.steps[step_index].set_point.try_into().unwrap();

            debug!("new step: {}, need to set power to {}", step_index, power,);

            {
                let channel = state.fe_channel.read().unwrap();

                if let Some(channel) = *channel {
                    let erg = fitness_equipment::target_power_message(channel, power * 4);
                    let node = state.node.lock().unwrap();
                    if let Some(ref node) = &*node {
                        if let Err(e) = node.write_message(erg, core::time::Duration::from_secs(1))
                        {
                            error!("failed to write message: {}", e);
                        }
                    } else {
                        error!("no node");
                    }
                } else {
                    error!("no channel for fitness equipment");
                }
            }
        }

        window
            .emit(
                "workout_status",
                WorkoutStatus {
                    step_index,
                    step_elapsed,
                },
            )
            .map_err(|e| format!("emit workout_status: {}", e))?;

        std::thread::sleep(core::time::Duration::from_millis(200));
    }

    Ok(())
}

fn main() {
    env_logger::builder()
        .format_timestamp_millis()
        .filter_level(log::LevelFilter::Debug)
        .filter(Some("tao"), log::LevelFilter::Warn)
        .target(env_logger::Target::Stdout)
        .init();

    let app_config = std::fs::read_to_string("appconfig.toml").expect("read config file");
    let app_config: config::AppConfig = toml::from_str(&app_config).expect("parse config file");
    trace!("loaded application config: {:?}", app_config);

    let state = Arc::new(AppState {
        config: app_config,
        node: None.into(),
        fe_channel: None.into(),
        hrm_channel: None.into(),
        workout: None.into(),
    });

    let mut sleep_lock = nosleep::NoSleep::new().unwrap();
    sleep_lock
        .start(nosleep::NoSleepType::PreventUserIdleDisplaySleep)
        .expect("sleep lock start should succeed");

    let app = tauri::Builder::default()
        .manage(Arc::clone(&state))
        .invoke_handler(tauri::generate_handler![
            open_node,
            open_fitness_equipment,
            open_hrm,
            load_workout,
            start_workout
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(move |_handle, event| match event {
        tauri::RunEvent::ExitRequested { .. } => {
            trace!("cleaning up");
            let mut node = state.node.lock().unwrap();
            if let Some(ref mut node) = *node {
                if let Err(e) = node.close() {
                    error!("closing node: {}", e);
                }
            }
        }
        _ => {}
    });
}
