use std::collections::HashMap;
use std::io::BufReader;

use base64::{engine::general_purpose, Engine as _};
use fit_file::fit_file;
use ts_rs::TS;
use url::Url;

#[derive(Clone, Debug, PartialEq, serde::Serialize, TS)]
#[ts(export, export_to = "../src/types/Workout.ts")]
pub struct Workout {
    pub title: String,
    pub steps: Vec<WorkoutStep>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, TS)]
#[ts(export, export_to = "../src/types/WorkoutStep.ts")]
pub struct WorkoutStep {
    pub set_point: u32,
    pub target_power: (u32, u32),
    pub target_cadence: Option<(u32, u32)>,
    pub duration: u32,
}

struct WorkoutConstructor {
    workout: Option<Workout>,
    error: Option<String>,
    steps: Vec<WorkoutStep>,
    step_indices: HashMap<u16, usize>,
}

fn fit_message_callback(
    _timestamp: u32,
    global_msg_num: u16,
    _local_msg_num: u8,
    message_index: u16,
    fields: Vec<fit_file::FitFieldValue>,
    data: &mut WorkoutConstructor,
) {
    // stop processing after first error
    if data.error.is_some() {
        return;
    }

    if global_msg_num == fit_file::GLOBAL_MSG_NUM_WORKOUT {
        let wko = fit_file::FitWorkoutMsg::new(fields);
        if let Some(title) = wko.workout_name {
            data.workout = Some(Workout {
                title,
                steps: data.steps.clone(),
            });
        } else {
            data.error = Some("missing workout title".into());
        }
    } else if global_msg_num == fit_file::GLOBAL_MSG_NUM_WORKOUT_STEP {
        let step = fit_file::FitWorkoutStepMsg::new(message_index, fields);

        if let Some(duration_type) = step.duration_type {
            if duration_type == fit_file::WORKOUT_STEP_DURATION_TIME {
                let target_power = if step.target_type == Some(fit_file::WORKOUT_STEP_TARGET_POWER)
                {
                    match target_from_fields(
                        step.target_value,
                        step.custom_target_low,
                        step.custom_target_high,
                    ) {
                        Ok(v) => {
                            if v.0 >= 1000 && v.1 >= 1000 {
                                Some((v.0 - 1000, v.1 - 1000))
                            } else {
                                data.error = Some("power based on FTP % not supported".into());
                                return;
                            }
                        }
                        Err(e) => {
                            data.error = Some(e);
                            return;
                        }
                    }
                } else if step.secondary_target_type == Some(fit_file::WORKOUT_STEP_TARGET_POWER) {
                    match target_from_fields(
                        step.secondary_target_value,
                        step.secondary_custom_target_low,
                        step.secondary_custom_target_high,
                    ) {
                        Ok(v) => {
                            if v.0 >= 1000 && v.1 >= 1000 {
                                Some((v.0 - 1000, v.1 - 1000))
                            } else {
                                data.error = Some("power based on FTP % not supported".into());
                                return;
                            }
                        }
                        Err(e) => {
                            data.error = Some(e);
                            return;
                        }
                    }
                } else {
                    None
                };

                let target_power = if let Some(target_power) = target_power {
                    target_power
                } else {
                    data.error = Some("no power target".into());
                    return;
                };

                let target_cadence = if step.target_type
                    == Some(fit_file::WORKOUT_STEP_TARGET_CADENCE)
                {
                    target_from_fields(
                        step.target_value,
                        step.custom_target_low,
                        step.custom_target_high,
                    )
                    .ok()
                } else if step.secondary_target_type == Some(fit_file::WORKOUT_STEP_TARGET_CADENCE)
                {
                    target_from_fields(
                        step.secondary_target_value,
                        step.secondary_custom_target_low,
                        step.secondary_custom_target_high,
                    )
                    .ok()
                } else {
                    None
                };

                if let Some(duration) = step.duration_value {
                    let duration = duration / 1000;
                    let workout_step = WorkoutStep {
                        set_point: ((target_power.0 + target_power.1) / 2),
                        target_cadence,
                        target_power,
                        duration,
                    };

                    if let Some(ref mut workout) = data.workout {
                        workout.steps.push(workout_step);
                        data.step_indices
                            .insert(message_index, workout.steps.len() - 1);
                    } else {
                        data.steps.push(workout_step);
                        data.step_indices
                            .insert(message_index, data.steps.len() - 1);
                    }
                } else {
                    data.error = Some("duration_value missing".into());
                    return;
                }
            } else if duration_type == fit_file::WORKOUT_STEP_DURATION_REPEAT_UNTIL_STEPS_COMPLETE {
                if let Some(target_index) = step.duration_value {
                    if let Some(repetitions) = step.target_value {
                        let steps = if let Some(ref mut workout) = data.workout {
                            &mut workout.steps
                        } else {
                            &mut data.steps
                        };

                        let repeated_steps = if let Some(target_step_index) =
                            data.step_indices.get(&target_index.try_into().unwrap())
                        {
                            let len = steps.len();

                            let mut t = vec![];

                            for i in *target_step_index..len {
                                t.push(steps.get(i).unwrap().clone());
                            }
                            t
                        } else {
                            data.error = Some("no matching target step for repeat".into());
                            return;
                        };

                        for _ in 0..repetitions - 1 {
                            steps.extend(repeated_steps.clone());
                        }
                    } else {
                        data.error = Some("target_value missing".into());
                        return;
                    }
                } else {
                    data.error = Some("duration_value missing".into());
                    return;
                }
            } else if duration_type == fit_file::WORKOUT_STEP_DURATION_OPEN {
                return;
            } else {
                println!("step: {:?}", step);
                data.error = Some("unsupported duration type".into());
                return;
            }
        } else {
            data.error = Some("duration_type missing".into());
            return;
        };
    }
}

fn target_from_fields(
    target_value: Option<u32>,
    custom_target_low: Option<u32>,
    custom_target_high: Option<u32>,
) -> Result<(u32, u32), String> {
    if target_value == Some(0) {
        if let Some(target_low) = custom_target_low {
            if let Some(target_high) = custom_target_high {
                Ok((target_low, target_high))
            } else {
                Err("custom_target_high missing".into())
            }
        } else {
            Err("custom_target_low missing".into())
        }
    } else {
        Err("zones not supported".into())
    }
}

pub fn from_data_url(url: String) -> Result<Workout, String> {
    let url = Url::parse(&url).map_err(|e| format!("parse URL: {}", e))?;

    if url.scheme() != "data"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.cannot_be_a_base()
    {
        return Err("invalid data URL".into());
    }

    let parts = url.path().split(',').collect::<Vec<_>>();

    if parts.len() != 2 || parts[0] != "application/octet-stream;base64" {
        return Err("invalid data URL".into());
    }

    let data = general_purpose::STANDARD
        .decode(parts[1])
        .map_err(|e| format!("decode base64: {}", e))?;

    load_workout(&*data)
}

pub fn load_workout(data: impl std::io::Read) -> Result<Workout, String> {
    let mut constructor = WorkoutConstructor {
        workout: None,
        error: None,
        steps: Vec::new(),
        step_indices: HashMap::new(),
    };

    let mut reader = BufReader::new(data);

    fit_file::read(&mut reader, fit_message_callback, &mut constructor)
        .map_err(|e| format!("reading fit file: {}", e))?;

    match constructor.workout {
        Some(workout) => Ok(workout),
        None => {
            if let Some(error) = constructor.error {
                Err(error)
            } else {
                Err("unknown error occurred".into())
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::workout::{self, WorkoutStep};
    use std::fs::File;
    use std::io::BufReader;

    #[test]
    fn it_loads_workout_with_power_and_cadence_targets() {
        let file = File::open("./tests/fixtures/power_and_cadence.fit").expect("file loads");
        let mut reader = BufReader::new(file);
        let wko = workout::load_workout(&mut reader).expect("workout loads");

        assert_eq!(
            wko,
            workout::Workout {
                title: "Threshold 4x 8\"".into(),
                steps: vec![
                    WorkoutStep {
                        duration: 480,
                        set_point: 112,
                        target_cadence: None,
                        target_power: (100, 125),
                    },
                    WorkoutStep {
                        duration: 60,
                        set_point: 137,
                        target_cadence: Some((85, 95)),
                        target_power: (125, 150),
                    },
                    WorkoutStep {
                        duration: 60,
                        set_point: 137,
                        target_cadence: Some((95, 105)),
                        target_power: (125, 150),
                    },
                    WorkoutStep {
                        duration: 60,
                        set_point: 137,
                        target_cadence: Some((105, 115)),
                        target_power: (125, 150),
                    },
                    WorkoutStep {
                        duration: 60,
                        set_point: 137,
                        target_cadence: Some((115, 125)),
                        target_power: (125, 150),
                    },
                    WorkoutStep {
                        duration: 60,
                        set_point: 137,
                        target_cadence: Some((105, 115)),
                        target_power: (125, 150),
                    },
                    WorkoutStep {
                        duration: 60,
                        set_point: 137,
                        target_cadence: Some((95, 105)),
                        target_power: (125, 150),
                    },
                    WorkoutStep {
                        duration: 60,
                        set_point: 137,
                        target_cadence: Some((85, 95)),
                        target_power: (125, 150),
                    },
                    WorkoutStep {
                        duration: 300,
                        set_point: 112,
                        target_cadence: None,
                        target_power: (100, 125),
                    },
                    WorkoutStep {
                        duration: 480,
                        set_point: 244,
                        target_cadence: None,
                        target_power: (238, 250),
                    },
                    WorkoutStep {
                        set_point: 125,
                        target_power: (112, 138),
                        target_cadence: None,
                        duration: 120
                    },
                    WorkoutStep {
                        duration: 480,
                        set_point: 244,
                        target_cadence: None,
                        target_power: (238, 250),
                    },
                    WorkoutStep {
                        set_point: 125,
                        target_power: (112, 138),
                        target_cadence: None,
                        duration: 120
                    },
                    WorkoutStep {
                        duration: 480,
                        set_point: 244,
                        target_cadence: None,
                        target_power: (238, 250),
                    },
                    WorkoutStep {
                        set_point: 125,
                        target_power: (112, 138),
                        target_cadence: None,
                        duration: 120
                    },
                    WorkoutStep {
                        duration: 480,
                        set_point: 244,
                        target_cadence: None,
                        target_power: (238, 250),
                    },
                    WorkoutStep {
                        set_point: 125,
                        target_power: (112, 138),
                        target_cadence: None,
                        duration: 120
                    },
                    WorkoutStep {
                        set_point: 112,
                        target_power: (100, 125),
                        target_cadence: None,
                        duration: 600
                    },
                ],
            }
        );
    }
}
