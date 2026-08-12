const metrics = [
  { label: "Files & Photos", value: <>14.204</>, detail: "↑ 340 added today", tone: "cyan" },
  { label: "Smart Search", value: <>Ready</>, detail: "● Works on this device", tone: "green" },
  { label: "Search Accuracy", value: <>94,2<small>%</small></>, detail: "Looking good" },
  { label: "Space Saved", value: <>12,8 <small>GB</small></>, detail: "Files optimized", tone: "cyan" },
];

export function MetricGrid() {
  return (
    <section className="metric-grid" aria-label="Library summary">
      {metrics.map((metric) => (
        <article className="metric-card" key={metric.label}>
          <p>{metric.label}</p>
          <strong>{metric.value}</strong>
          <span className={metric.tone}>{metric.detail}</span>
        </article>
      ))}
    </section>
  );
}
