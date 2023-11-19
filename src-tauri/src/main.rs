// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod executor;
mod workout;

use antrs::node;
use antrs::profile::{fitness_equipment, heart_rate_monitor};
use log::{debug, error, info, trace, warn};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::Duration;
use tauri::{State, Window};
use ts_rs::TS;
use workout::Workout;

include!(concat!(env!("OUT_DIR"), "/ant_network_key.rs"));

#[derive(Clone)]
pub struct FitnessEquipmentState {
    pub channel: u8,
    pub receiver: crossbeam_channel::Receiver<fitness_equipment::CommandStatusData>,
}

struct AppState {
    config: config::AppConfig,
    node: Arc<Mutex<Option<node::Node>>>,
    hrm_channel: RwLock<Option<u8>>,
    fe_state: RwLock<Option<FitnessEquipmentState>>,
    workout: Mutex<Option<Workout>>,
    workout_status: Arc<Mutex<Option<executor::WorkoutStatus>>>,
}

#[derive(Clone, serde::Serialize, TS)]
#[ts(export, export_to = "../src/types/HeartRateUpdate.ts")]
struct HeartRateUpdate {
    value: u8,
    timestamp: u128,
}

#[derive(Clone, serde::Serialize, TS)]
#[ts(export, export_to = "../src/types/FitnessEquipmentUpdate.ts")]
struct FitnessEquipmentUpdate {
    cadence: Option<u8>,
    power: Option<u16>,
}

#[tauri::command]
fn open_node(state: State<'_, Arc<AppState>>, window: Window) {
    let state = Arc::clone(&state);

    std::thread::spawn(move || {
        debug!("opening node");
        let mut node = state.node.lock().unwrap();

        while node.is_none() {
            let nb = antrs::node::NodeBuilder::new(ANT_NETWORK_KEY);
            let mut created = nb.build();

            match created.open() {
                Ok(_) => {
                    trace!("node opened successfully!");
                    *node = Some(created);
                }
                Err(e) => {
                    warn!("failed to open node: {}", e);
                }
            }

            std::thread::sleep(Duration::from_secs(5));
        }

        if let Err(e) = window.emit("node_connected", true) {
            error!("failed to emit node_connected: {}", e);
        }
    });
}

#[tauri::command]
async fn open_hrm(state: State<'_, Arc<AppState>>, window: Window) -> Result<(), String> {
    debug!("opening heart rate monitor");

    let mut nd = state.node.lock().unwrap();

    if let Some(ref mut nd) = *nd {
        let (hrm, receiver) =
            heart_rate_monitor::new_paired(state.config.devices.heart_rate_monitor.into());
        let channel = nd
            .assign_channel(
                Box::new(hrm),
                Some(node::ChannelOptions {
                    low_priority_search_timeout: Some(255),
                    search_timeout: Some(0),
                }),
            )
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
            .assign_channel(
                Box::new(fe),
                Some(node::ChannelOptions {
                    low_priority_search_timeout: Some(255),
                    search_timeout: Some(0),
                }),
            )
            .map_err(|e| format!("assigning channel: {}", e))?;

        let (s, r) = crossbeam_channel::unbounded();

        {
            let mut fe_state = state.fe_state.write().unwrap();
            *fe_state = Some(FitnessEquipmentState {
                channel,
                receiver: r,
            });
        }

        std::thread::spawn(move || {
            for message in receiver.iter() {
                match message {
                    fitness_equipment::FitnessEquipmentData::StationaryBike(data) => {
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
                    fitness_equipment::FitnessEquipmentData::CommandStatus(data) => {
                        s.send(data)
                            .expect("send to control loop channel should succeed");
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    } else {
        Err("node not open".into())
    }
}

#[tauri::command]
async fn load_workout(state: State<'_, Arc<AppState>>, data: String) -> Result<Workout, String> {
    let wko = workout::from_data_url(data, 0.)?;

    trace!("load_workout: wko = {:?}", wko);

    {
        let mut state_wko = state.workout.lock().unwrap();
        *state_wko = Some(wko.clone());
    }

    Ok(wko)
}

#[tauri::command]
async fn start_workout(state: State<'_, Arc<AppState>>, window: Window) -> Result<(), String> {
    let wko = {
        let wko = state.workout.lock().unwrap();
        match wko.clone() {
            Some(wko) => wko,
            None => return Err("no workout loaded".into()),
        }
    };

    let fe_state = {
        let fe = state.fe_state.read().unwrap();
        match *fe {
            Some(ref fe) => fe.clone(),
            None => return Err("fitness equipment not connected".into()),
        }
    };

    let wko_exec = executor::Executor::new(
        Arc::clone(&state.node),
        Arc::clone(&state.workout_status),
        wko,
        fe_state,
    );

    info!("starting workout");

    let wko_handle = std::thread::spawn(move || wko_exec.execute());

    while !wko_handle.is_finished() {
        let status = *state.workout_status.lock().unwrap();

        if let Some(status) = status {
            window
                .emit("workout_status", status)
                .map_err(|e| format!("emit workout_status: {}", e))?;
        } else {
            debug!("no workout status in UI update loop");
        }

        std::thread::sleep(core::time::Duration::from_millis(200));
    }

    info!("workout complete, exiting UI update loop");

    Ok(())
}

fn main() {
    env_logger::builder()
        .format_timestamp_millis()
        .filter_level(log::LevelFilter::Trace)
        .filter(Some("tao"), log::LevelFilter::Warn)
        .target(env_logger::Target::Stdout)
        .init();

    let app_config = std::fs::read_to_string("appconfig.toml").expect("read config file");
    let app_config: config::AppConfig = toml::from_str(&app_config).expect("parse config file");
    trace!("loaded application config: {:?}", app_config);

    let state = Arc::new(AppState {
        config: app_config,
        node: Arc::new(Mutex::new(None)),
        fe_state: None.into(),
        hrm_channel: None.into(),
        workout: None.into(),
        workout_status: Arc::new(Mutex::new(None)),
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

    app.run(move |_handle, event| {
        if let tauri::RunEvent::Exit = event {
            trace!("cleaning up");
            let mut node = state.node.lock().unwrap();
            if let Some(ref mut node) = *node {
                if let Err(e) = node.close() {
                    error!("closing node: {}", e);
                }
            }
        }
    });
}
