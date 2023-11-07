import DataValue from "./DataValue";

function Duration({ seconds, title }: { seconds: number; title: string }) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const displaySeconds = seconds % 60;
  let formatted;
  if (hours > 0) {
    formatted = `${hours}h ${minutes}m ${displaySeconds}s`;
  } else {
    formatted =
      minutes > 0 ? `${minutes}m ${displaySeconds}s` : `${displaySeconds}s`;
  }

  return <DataValue title={title} value={formatted} />;
}

export default Duration;
