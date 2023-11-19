use core::time::Duration;
use log::{debug, error, trace, warn};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use ts_rs::TS;

use antrs::{
    message::{Message, MessageCode, MessageID},
    node,
    profile::fitness_equipment,
};

use crate::workout;

#[derive(Clone, Copy, serde::Serialize, TS)]
#[ts(export, export_to = "../src/types/WorkoutStatus.ts")]
pub struct WorkoutStatus {
    pub step_index: usize,
    pub step_elapsed: u32,
}

pub struct Executor {
    fe_state: super::FitnessEquipmentState,
    node: Arc<Mutex<Option<node::Node>>>,
    status: Arc<Mutex<Option<WorkoutStatus>>>,
    workout: workout::Workout,
}

enum CommandState {
    None,
    Pending(u16, Instant),
    Acked(u16),
}

impl Executor {
    pub fn new(
        node: Arc<Mutex<Option<node::Node>>>,
        status: Arc<Mutex<Option<WorkoutStatus>>>,
        workout: workout::Workout,
        fe_state: super::FitnessEquipmentState,
    ) -> Executor {
        Executor {
            fe_state,
            node,
            status,
            workout,
        }
    }

    fn power_control_loop(
        node: Arc<Mutex<Option<node::Node>>>,
        control: crossbeam_channel::Receiver<u16>,
        fe_state: super::FitnessEquipmentState,
    ) {
        let channel = fe_state.channel;

        let matcher = move |message| {
            if let Message::ChannelResponseEvent(data) = message {
                data.channel == channel
                    && data.message_id == MessageID::ChannelEvent
                    && (data.message_code == MessageCode::EventTransferTXCompleted
                        || data.message_code == MessageCode::TransferInProgress)
            } else {
                false
            }
        };

        let send_timeout = Duration::from_millis(100);
        let tx_timeout = Duration::from_millis(400);
        let command_status_timeout = Duration::from_millis(500);

        let mut command_state = CommandState::None;
        let mut desired_power = None;

        loop {
            if let Some(request) = control.try_iter().last() {
                debug!("updating desired power: {}", request);
                desired_power = Some(request);
            }

            for command_status in fe_state.receiver.try_iter() {
                debug!("received command status: {:?}", command_status);
                if command_status.command_id
                    == Into::<u8>::into(fitness_equipment::Command::TargetPower)
                {
                    if command_status.command_status == antrs::message::CommandStatus::Pass {
                        if let Some(acked_power) = command_status.target_power {
                            let power = acked_power / 4;

                            debug!("updating acked power: {}", power);
                            command_state = CommandState::Acked(power);
                        }
                    } else {
                        warn!(
                            "command status is not a pass: {:?}",
                            command_status.command_status
                        );
                    }
                }
            }

            if let Some(desired_power) = desired_power {
                let command_required = match command_state {
                    CommandState::None => {
                        trace!("no command status, sending command");
                        true
                    }
                    CommandState::Pending(power, sent_at) => {
                        if power != desired_power || sent_at.elapsed() > command_status_timeout {
                            warn!("pending command status, sending command; power: {}, desired_power: {}, sent_at.elapsed(): {}", power, desired_power, sent_at.elapsed().as_millis());
                            true
                        } else {
                            false
                        }
                    }
                    CommandState::Acked(power) => {
                        if power != desired_power {
                            trace!("acked command status, sending command; power: {}, desired_power: {}", power, desired_power);
                            true
                        } else {
                            false
                        }
                    }
                };

                if command_required {
                    let erg = fitness_equipment::target_power_message(
                        fe_state.channel,
                        desired_power * 4,
                    );

                    let mut send_data_page_request = false;

                    let node = node.lock().unwrap();
                    if let Some(ref node) = *node {
                        match node.wait_for_message_after(Box::new(matcher), tx_timeout, || {
                            node.write_message(erg, send_timeout)
                        }) {
                            Ok(_) => {
                                send_data_page_request = true;
                            }
                            Err(node::Error::Timeout) => {
                                warn!("timeout waiting for command channel event");
                            }
                            Err(e) => {
                                error!("writing power command message: {}", e);
                            }
                        }

                        if send_data_page_request {
                            match node.wait_for_message_after(Box::new(matcher), tx_timeout, || {
                                node.write_message(
                                    antrs::message::request_data_page(fe_state.channel, 71),
                                    send_timeout,
                                )
                            }) {
                                Ok(_) => {
                                    command_state =
                                        CommandState::Pending(desired_power, Instant::now());
                                }
                                Err(e) => {
                                    error!("writing data page request message: {}", e);
                                }
                            }
                        }
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn execute(self) {
        let node = Arc::clone(&self.node);
        let (sender, receiver) = crossbeam_channel::unbounded();
        let fe_state = self.fe_state;

        std::thread::spawn(move || Self::power_control_loop(node, receiver, fe_state));

        let workout_start = Instant::now();
        let mut step_index = 0;

        let power: u16 = self.workout.steps[step_index].set_point.try_into().unwrap();
        sender.send(power).unwrap();

        debug!(
            "starting workout at step: {}, need to set power to {}",
            step_index, power,
        );

        loop {
            let (step_elapsed, step_changed) = {
                let elapsed = workout_start.elapsed().as_millis();

                let step_start = self.workout.steps[0..step_index]
                    .iter()
                    .fold(0, |a, e| a + e.duration as u128 * 1000);

                let threshold = step_start + self.workout.steps[step_index].duration as u128 * 1000;

                if elapsed > threshold {
                    step_index += 1;

                    if step_index >= self.workout.steps.len() {
                        trace!("workout complete, exiting executor");
                        break;
                    }

                    trace!("new step: {}, elapsed: {}", step_index, elapsed);

                    (elapsed - threshold, true)
                } else {
                    (elapsed - step_start, false)
                }
            };

            if step_changed {
                let power: u16 = self.workout.steps[step_index].set_point.try_into().unwrap();

                debug!("new step: {}, set target power to {}", step_index, power);

                sender.send(power).unwrap();
            }

            {
                let step_elapsed = (step_elapsed / 1000) as u32;

                let mut status = self.status.lock().unwrap();
                match &mut *status {
                    Some(ref mut status) => {
                        status.step_index = step_index;
                        status.step_elapsed = step_elapsed;
                    }
                    None => {
                        *status = Some(WorkoutStatus {
                            step_index,
                            step_elapsed,
                        })
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
