import { useEffect, useState } from "react";

import { createUseStyles } from "react-jss";

import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";

import DataValue from "./DataValue";
import Duration from "./Duration";
import TargetComplianceGauge from "./TargetComplianceGauge";
import WorkoutGraph from "./WorkoutGraph";

import { Workout } from "./types/Workout";
import { FitnessEquipmentUpdate } from "./types/FitnessEquipmentUpdate";
import { HeartRateUpdate } from "./types/HeartRateUpdate";
import { WorkoutStatus } from "./types/WorkoutStatus";
import { TauriEvent } from "./types";

const useStyles = createUseStyles({
  container: {
    display: "flex",
    flexDirection: "column",
    justifyContent: "space-between",
    height: "100%",
  },
  dataFields: {
    margin: [16, 48],
  },
  row: {
    display: "flex",
    flexDirection: "row",
    justifyContent: "space-between",
    padding: [48, 0],
    textAlign: "center",
  },
});

function WorkoutMain({ workout }: { workout: Workout }) {
  const classes = useStyles();

  const [error, setError] = useState<null | string>(null);
  const [stepIndex, setStepIndex] = useState(0);
  const [stepElapsed, setStepElapsed] = useState(0);
  const [heartRate, setHeartRate] = useState<null | number>(null);
  const [cadence, setCadence] = useState<null | number>(null);
  const [power, setPower] = useState<null | number>(null);

  useEffect(() => {
    const cleanup = listen(
      "workout_status",
      (event: TauriEvent<WorkoutStatus>) => {
        console.log("workout_status", "event", event);
        setStepIndex(event.payload.step_index);
        setStepElapsed(event.payload.step_elapsed);
      },
    );

    return () => {
      cleanup.then((f) => f());
    };
  }, []);

  useEffect(() => {
    const cleanup = listen(
      "heart_rate",
      (event: TauriEvent<HeartRateUpdate>) => {
        console.log("heart_rate", "event", event);
        setHeartRate(event.payload.value);
      },
    );

    return () => {
      cleanup.then((f) => f());
    };
  }, []);

  useEffect(() => {
    const cleanup = listen(
      "fitness_equipment_data",
      (event: TauriEvent<FitnessEquipmentUpdate>) => {
        console.log("fitness_equipment_data", "event", event);
        setCadence(event.payload.cadence);
        setPower(event.payload.power);
      },
    );

    return () => {
      cleanup.then((f) => f());
    };
  }, []);

  const startWorkout = async () => {
    try {
      await invoke("start_workout");
    } catch (error) {
      if (typeof error === "string") {
        setError(error);
      }
    }
  };
  const step = workout.steps[stepIndex];

  console.log("step", step);

  const workoutElapsed =
    workout.steps.slice(0, stepIndex).reduce((a, e) => a + e.duration, 0) +
    stepElapsed;

  return (
    <div className={classes.container}>
      <button onClick={startWorkout}>GO!</button>
      <div className={classes.dataFields}>
        <div className={classes.row}>
          <Duration title="Total Elapsed" seconds={workoutElapsed} />
          <Duration
            title="Lap Remaining"
            seconds={workout.steps[stepIndex].duration - stepElapsed}
          />
          <Duration title="Lap Elapsed" seconds={stepElapsed} />
        </div>
        <div className={classes.row}>
          <DataValue title="Heart Rate" unit="BPM" value={heartRate} />
          <DataValue title="Power" unit="W" value={power} />
          <DataValue title="Cadence" unit="RPM" value={cadence} />
        </div>
        <div className={classes.row}>
          <TargetComplianceGauge
            target={{
              minimum: step.target_range[0],
              maximum: step.target_range[1],
            }}
            value={power}
          />
        </div>
      </div>
      <WorkoutGraph
        workout={workout}
        stepIndex={stepIndex}
        stepElapsed={stepElapsed}
      />
      {error !== null && <div>{error}</div>}
    </div>
  );
}

export default WorkoutMain;
