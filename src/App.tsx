import { useState, ChangeEvent } from "react";

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

  const loadWorkout = async (event: ChangeEvent<HTMLInputElement>) => {
    if (event.target.files && event.target.files.length === 1) {
      const reader = new FileReader();
      reader.onload = async (evt) => {
        if (evt.target && evt.target.result) {
          try {
            setWorkout(
              await invoke("load_workout", { data: evt.target.result }),
            );
          } catch (error) {
            if (typeof error === "string") {
              setError(error);
            }
          }
        }
      };
      reader.readAsDataURL(event.target.files[0]);
    }
  };

  return (
    <div className={classes.container}>
      {error != "" && <p>Error message: {error}</p>}
      {workout === null ? (
        <>
          <button onClick={setup}>SET IT UP</button>
          <input type="file" accept=".fit" onChange={loadWorkout} />
        </>
      ) : (
        <WorkoutMain workout={workout} />
      )}
    </div>
  );
}

export default App;
