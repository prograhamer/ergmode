import { createUseStyles } from "react-jss";

const useStyles = createUseStyles({
  container: {},
  title: {
    fontWeight: 600,
    marginBottom: 8,
  },
  icon: {
    display: "inline",
  },
  value: {
    fontSize: 24,
  },
});

function HeartRateDisplay({ heartRate }) {
  const classes = useStyles();

  return (
    <div className={classes.container}>
      <div className={classes.title}>Heart Rate</div>
      <div className={classes.value}>
        <span className={classes.icon}>❤️</span>
        &nbsp;
        {heartRate === undefined ? "No data" : heartRate}
      </div>
    </div>
  );
}

export default HeartRateDisplay;
