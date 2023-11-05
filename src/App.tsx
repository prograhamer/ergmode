import { useState } from "react";

import { createUseStyles } from "react-jss";

import { invoke } from "@tauri-apps/api/tauri";

import WorkoutMain from "./WorkoutMain";

const useStyles = createUseStyles({
  container: {
    width: "100vw",
    height: "100vh",
  },
});

function App() {
  const classes = useStyles();

  const [error, setError] = useState("");
  const [workout, setWorkout] = useState(null);

  const setup = async () => {
    try {
      setError("");
      await invoke("open_node");
      await invoke("open_hrm");
      await invoke("open_fitness_equipment");
    } catch (error) {
      if (typeof error === "string") {
        setError(error);
      }
    }
  };

  const loadWorkout = async () => {
    try {
      setWorkout(await invoke("load_workout"));
    } catch (error) {
      if (typeof error === "string") {
        setError(error);
      }
    }
  };

  return (
    <div className={classes.container}>
      {error != "" && <p>Error message: {error}</p>}
      {workout === null ? (
        <>
          <button onClick={setup}>SET IT UP</button>
          <button onClick={loadWorkout}>Load Workout</button>
        </>
      ) : (
        <WorkoutMain workout={workout} />
      )}
    </div>
  );
}

export default App;
