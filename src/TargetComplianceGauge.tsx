import { createUseStyles } from "react-jss";

import * as d3 from "d3";

const useStyles = createUseStyles({
  container: {
    margin: [0, "auto"],
  },
  title: {
    fontSize: 24,
    fontWeight: 600,
    textAlign: "center",
  },
});

function TargetComplianceGauge({ target, value }) {
  const classes = useStyles();

  const inTargetRange = target.maximum - target.minimum;
  const targetMidpoint = (target.maximum + target.minimum) / 2;
  let displayRange = inTargetRange * 2;
  let displayMinimum = targetMidpoint - displayRange / 2;
  let displayMaximum = targetMidpoint + displayRange / 2;

  let displayValue = Math.max(Math.min(displayMaximum, value), displayMinimum);

  const marginTop = 8;
  const marginRight = 8;
  const marginBottom = 8;
  const marginLeft = 8;
  const width = 800;
  const height = 140;
  const bottomGutter = 30;
  const textMargin = 30;

  const x = d3
    .scaleLinear()
    .domain([displayMinimum, displayMaximum])
    .range([marginLeft, width - marginRight]);

  return (
    <div className={classes.container}>
      <div className={classes.title}>Target: Power</div>
      <svg height={height} width={width}>
        <defs>
          <linearGradient id="compliance-gradient">
            <stop offset="0%" stopColor="#E31C1C" />
            <stop offset="10%" stopColor="#ED7827" />
            <stop offset="20%" stopColor="#FBBE27" />
            <stop offset="40%" stopColor="#15EA15" />
            <stop offset="50%" stopColor="#15EA15" />
            <stop offset="60%" stopColor="#15EA15" />
            <stop offset="80%" stopColor="#FBBE27" />
            <stop offset="90%" stopColor="#ED7827" />
            <stop offset="100%" stopColor="#E31C1C" />
          </linearGradient>
        </defs>
        <g fill="url(#compliance-gradient)" stroke="darkgreen" strokeWidth={1}>
          <rect
            x={x(displayMinimum)}
            width={x(displayMaximum) - x(displayMinimum)}
            y={textMargin}
            height={height - bottomGutter - textMargin}
          />
          <line
            x1={x(target.minimum)}
            y1={textMargin}
            x2={x(target.minimum)}
            y2={height - bottomGutter}
            strokeWidth={3}
          />
          <line
            x1={x(target.maximum)}
            y1={textMargin}
            x2={x(target.maximum)}
            y2={height - bottomGutter}
            strokeWidth={3}
          />
        </g>
        <path
          d={`M ${x(displayValue)} ${
            height - bottomGutter
          } l15 ${bottomGutter} l-30 0 Z`}
          fill="currentColor"
        />
        <text
          x={x(target.minimum)}
          y="0"
          fill="currentColor"
          dominantBaseline="hanging"
          textAnchor="middle"
          fontSize={24}
        >
          {target.minimum}
        </text>
        <text
          x={x(target.maximum)}
          y="0"
          fill="currentColor"
          dominantBaseline="hanging"
          textAnchor="middle"
          fontSize={24}
        >
          {target.maximum}
        </text>
      </svg>
    </div>
  );
}

export default TargetComplianceGauge;
