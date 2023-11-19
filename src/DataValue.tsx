import { createUseStyles } from "react-jss";

const useStyles = createUseStyles({
  container: {
    minWidth: 200,
  },
  title: {
    fontSize: 24,
    fontWeight: 600,
    marginBottom: 16,
  },
  icon: {
    display: "inline",
    marginRight: 8,
  },
  unit: {
    marginLeft: 4,
  },
  value: {
    fontSize: 32,
  },
});

function DataValue({
  icon,
  title,
  unit,
  value,
}: {
  icon?: string;
  title: string;
  unit?: string;
  value: null | number | string;
}) {
  const classes = useStyles();

  return (
    <div className={classes.container}>
      <div className={classes.title}>{title}</div>
      <div className={classes.value}>
        {icon && <span className={classes.icon}>{icon}</span>}
        {value !== null ? value : "NO DATA"}
        {value !== null && unit && <span className={classes.unit}>{unit}</span>}
      </div>
    </div>
  );
}

export default DataValue;
