import { useEffect, useState } from "react";
import { api, ProxyStatus, Rule, StatsSnapshot } from "./lib/tauri";
import RuleList from "./components/RuleList";
import AddRuleModal from "./components/AddRuleModal";

export default function App() {
  const [status, setStatus] = useState<ProxyStatus | null>(null);
  const [rules, setRules] = useState<Rule[]>([]);
  const [stats, setStats] = useState<StatsSnapshot | null>(null);
  const [adding, setAdding] = useState(false);

  async function refresh() {
    const [s, r, st] = await Promise.all([
      api.getProxyStatus(),
      api.getRules(),
      api.getStats(),
    ]);
    setStatus(s);
    setRules(r);
    setStats(st);
  }

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 1500);
    return () => clearInterval(id);
  }, []);

  const dotClass = !status
    ? "grey"
    : status.error
    ? "red"
    : status.active
    ? rules.every((r) => r.enabled)
      ? "green"
      : "yellow"
    : "grey";

  return (
    <div className="panel">
      <header>
        <div className="title">KeyProxy</div>
        <div>
          <span className={`dot ${dotClass}`} />
          {status?.active ? "Active" : "Paused"}
        </div>
      </header>

      <RuleList
        rules={rules}
        onToggle={async (id, enabled) => {
          await api.toggleRule(id, enabled);
          refresh();
        }}
      />

      <div style={{ padding: "8px 14px", borderTop: "1px solid #ececec" }}>
        <button onClick={() => setAdding(true)}>+ Add rule</button>
      </div>

      <div className="footer">
        <div className="stats">
          {stats
            ? `Requests today: ${stats.requests_today} · Errors: ${stats.errors_today}`
            : "—"}
        </div>
        <div style={{ display: "flex", gap: 6 }}>
          <button onClick={() => api.openSettings()}>Settings</button>
          <button
            className={status?.active ? "" : "primary"}
            onClick={async () => {
              await api.setProxyActive(!status?.active);
              refresh();
            }}
          >
            {status?.active ? "Pause" : "Start"}
          </button>
        </div>
      </div>

      {adding && (
        <AddRuleModal
          onClose={() => setAdding(false)}
          onSaved={refresh}
        />
      )}
    </div>
  );
}
