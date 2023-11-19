import { useEffect, useState, ChangeEvent } from "react";

import { createUseStyles } from "react-jss";

import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";

import { HiSignal, HiSignalSlash } from "react-icons/hi2";

import WorkoutMain from "./WorkoutMain";

import { TauriEvent } from "./types";

const useStyles = createUseStyles({
  container: {
    width: "100vw",
    height: "100vh",
  },
  statusBar: {
    position: "absolute",
    bottom: 0,
    color: "#eeeeee",
    backgroundColor: "#252525",
    borderTop: "1px black solid",
    fontSize: 24,
    width: "100%",
  },
});

function App() {
  const classes = useStyles();

  const [error, setError] = useState("");
  const [workout, setWorkout] = useState(null);
  const [nodeConnected, setNodeConnected] = useState(false);
  const [devicesOpen, setDevicesOpen] = useState(false);

  useEffect(() => {
    const cleanup = listen("node_connected", (event: TauriEvent<boolean>) => {
      setNodeConnected(event.payload);
    });

    return () => {
      cleanup.then((f) => f());
    };
  }, []);

  useEffect(() => {
    invoke("open_node").catch((e) => {
      if (typeof e === "string") {
        setError(e);
      }
    });
  }, []);

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

  const openDevices = async () => {
    try {
      await invoke("open_hrm");
      await invoke("open_fitness_equipment");
      setDevicesOpen(true);
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
          <button
            disabled={devicesOpen || !nodeConnected}
            onClick={openDevices}
          >
            Open Devices
          </button>
          <input
            disabled={!devicesOpen}
            type="file"
            accept=".fit"
            onChange={loadWorkout}
          />
        </>
      ) : (
        <WorkoutMain workout={workout} />
      )}
      <div className={classes.statusBar}>
        {nodeConnected ? <HiSignal /> : <HiSignalSlash />}
      </div>
    </div>
  );
}

export default App;
