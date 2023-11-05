import DataValue from "./DataValue";

function Duration({ seconds, title }: { seconds: number; title: string }) {
  const minutes = Math.floor((seconds % 3600) / 60);
  const displaySeconds = seconds % 60;
  const formatted =
    minutes > 0 ? `${minutes}m ${displaySeconds}s` : `${displaySeconds}s`;

  return <DataValue title={title} value={formatted} />;
}

export default Duration;
