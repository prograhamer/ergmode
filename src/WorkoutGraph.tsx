import { useEffect, useMemo, useState } from "react";

import * as d3 from "d3";

import { Workout } from "./types/Workout";

function WorkoutGraph({
  workout,
  stepIndex,
  stepElapsed,
}: {
  workout: Workout;
  stepIndex: number;
  stepElapsed: number;
}) {
  const [dimensions, setDimensions] = useState({
    height: window.innerHeight,
    width: window.innerWidth,
  });

  useEffect(() => {
    function handleResize() {
      setDimensions({
        height: window.innerHeight,
        width: window.innerWidth,
      });
    }

    window.addEventListener("resize", handleResize);
  }, []);

  const data = useMemo(() => {
    const data = [];
    let previous_end = 0;
    for (const step of workout.steps) {
      data.push({
        x: previous_end,
        width: step.duration,
        height: step.set_point,
      });
      previous_end += step.duration;
    }
    return data;
  }, [workout]);

  const totalDuration = workout.steps.reduce((a, e) => a + e.duration, 0);
  const cursorPosition =
    workout.steps.slice(0, stepIndex).reduce((a, e) => a + e.duration, 0) +
    stepElapsed;
  const maxHeight = data.reduce((a, e) => Math.max(a, e.height), 0);

  const marginTop = 0;
  const marginRight = 8;
  const marginBottom = 8;
  const marginLeft = 8;
  const width = dimensions.width;
  const height = dimensions.height > 800 ? 400 : dimensions.height / 2;

  const x = d3
    .scaleLinear()
    .domain([0, totalDuration])
    .range([marginLeft, width - marginRight]);
  const y = d3
    .scaleLinear()
    .domain([0, maxHeight])
    .range([height - marginBottom, marginTop]);

  return (
    <svg
      width={width}
      height={height}
      style={{ maxWidth: "100%", height: "auto" }}
    >
      <g fill="steelblue" stroke="currentColor" strokeWidth={1}>
        {data.map((d, i) => (
          <rect
            key={i}
            x={x(d.x)}
            width={x(d.width) - x(0)}
            y={y(d.height)}
            height={y(0) - y(d.height)}
          />
        ))}
        <line
          style={{ strokeWidth: 4 }}
          x1={x(cursorPosition)}
          y1={y(300)}
          x2={x(cursorPosition)}
          y2={y(0)}
        />
      </g>
    </svg>
  );
}

export default WorkoutGraph;
