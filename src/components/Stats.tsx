import { useEffect, useState } from "react";
import { api, StatsSnapshot } from "../lib/tauri";

export default function Stats() {
  const [stats, setStats] = useState<StatsSnapshot | null>(null);

  useEffect(() => {
    let alive = true;
    const load = async () => {
      const s = await api.getStats();
      if (alive) setStats(s);
    };
    load();
    const id = setInterval(load, 2000);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, []);

  if (!stats) return <div>Loading...</div>;

  return (
    <div>
      <h2>Stats</h2>
      <p>
        Requests today: <strong>{stats.requests_today}</strong> · Errors:{" "}
        <strong>{stats.errors_today}</strong>
      </p>
      <h3 style={{ fontSize: 13 }}>Recent requests</h3>
      <table className="log-table">
        <thead>
          <tr>
            <th>Time</th>
            <th>Domain</th>
            <th>Mode</th>
            <th>Status</th>
            <th>Latency</th>
          </tr>
        </thead>
        <tbody>
          {stats.recent.map((e, i) => (
            <tr key={i}>
              <td>{new Date(e.timestamp).toLocaleTimeString()}</td>
              <td>{e.domain}</td>
              <td className={e.intercepted ? "intercepted" : ""}>
                {e.intercepted ? "intercept" : "tunnel"}
              </td>
              <td className={e.error ? "err" : ""}>
                {e.error ?? (e.status ?? "—")}
              </td>
              <td>{e.latency_ms ? `${e.latency_ms}ms` : "—"}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
