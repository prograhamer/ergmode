use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use fit_file::fit_file;
use ts_rs::TS;

#[derive(Clone, serde::Serialize, TS)]
#[ts(export, export_to = "../src/types/Workout.ts")]
pub struct Workout {
    pub title: String,
    pub steps: Vec<WorkoutStep>,
}

#[derive(Clone, serde::Serialize, TS)]
#[ts(export, export_to = "../src/types/WorkoutStep.ts")]
pub struct WorkoutStep {
    pub set_point: u32,
    pub target_range: (u32, u32),
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
                let (low, high) = if step.target_type == Some(fit_file::WORKOUT_STEP_TARGET_POWER) {
                    if step.target_value == Some(0) {
                        if let Some(target_low) = step.custom_target_low {
                            if let Some(target_high) = step.custom_target_high {
                                if target_low >= 1000 && target_high > 1000 {
                                    (target_low - 1000, target_high - 1000)
                                } else {
                                    data.error = Some("power based on FTP not supported".into());
                                    return;
                                }
                            } else {
                                data.error = Some("custom_target_high missing".into());
                                return;
                            }
                        } else {
                            data.error = Some("custom_target_low missing".into());
                            return;
                        }
                    } else {
                        data.error = Some("power zones not supported".into());
                        return;
                    }
                } else {
                    data.error = Some("unsupported target type".into());
                    return;
                };

                if let Some(duration) = step.duration_value {
                    let duration = duration / 1000;
                    let workout_step = WorkoutStep {
                        set_point: ((low + high) / 2),
                        target_range: (low, high),
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

pub fn load_workout() -> Result<Workout, String> {
    let mut constructor = WorkoutConstructor {
        workout: None,
        error: None,
        steps: Vec::new(),
        step_indices: HashMap::new(),
    };

    let file = File::open("/Users/graham/Code/fitparse/2023-11-07_VO2max-Int.fit")
        .map_err(|e| format!("opening file: {}", e))?;
    let mut reader = BufReader::new(file);

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

    //     Ok(Workout {
    //         title: "Test Workout #1".to_string().into(),
    //         steps: vec![
    //             WorkoutStep {
    //                 power: 120,
    //                 duration: 60,
    //             },
    //             WorkoutStep {
    //                 power: 130,
    //                 duration: 60,
    //             },
    //             WorkoutStep {
    //                 power: 140,
    //                 duration: 60,
    //             },
    //             WorkoutStep {
    //                 power: 150,
    //                 duration: 60,
    //             },
    //             WorkoutStep {
    //                 power: 160,
    //                 duration: 60 * 22,
    //             },
    //             WorkoutStep {
    //                 power: 150,
    //                 duration: 60,
    //             },
    //             WorkoutStep {
    //                 power: 140,
    //                 duration: 60,
    //             },
    //             WorkoutStep {
    //                 power: 130,
    //                 duration: 60,
    //             },
    //             WorkoutStep {
    //                 power: 120,
    //                 duration: 60,
    //             },
    //         ],
    //     })
}
