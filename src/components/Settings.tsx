import { useEffect, useState } from "react";
import { api, Rule } from "../lib/tauri";
import AddRuleModal from "./AddRuleModal";
import Stats from "./Stats";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";

type Tab = "rules" | "general" | "stats";

export default function Settings() {
  const [tab, setTab] = useState<Tab>("rules");

  return (
    <div className="settings-layout">
      <nav className="sidebar">
        <button
          className={`tab ${tab === "rules" ? "active" : ""}`}
          onClick={() => setTab("rules")}
        >
          Rules
        </button>
        <button
          className={`tab ${tab === "general" ? "active" : ""}`}
          onClick={() => setTab("general")}
        >
          General
        </button>
        <button
          className={`tab ${tab === "stats" ? "active" : ""}`}
          onClick={() => setTab("stats")}
        >
          Stats
        </button>
      </nav>
      <div className="content">
        {tab === "rules" && <RulesTab />}
        {tab === "general" && <GeneralTab />}
        {tab === "stats" && <Stats />}
      </div>
    </div>
  );
}

function RulesTab() {
  const [rules, setRules] = useState<Rule[]>([]);
  const [editing, setEditing] = useState<Rule | null>(null);
  const [adding, setAdding] = useState(false);

  const load = async () => setRules(await api.getRules());
  useEffect(() => {
    load();
  }, []);

  return (
    <div>
      <h2>Rules</h2>
      <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: 10 }}>
        <button className="primary" onClick={() => setAdding(true)}>
          + Add rule
        </button>
      </div>
      <div style={{ border: "1px solid #ececec", borderRadius: 6, background: "#fff" }}>
        {rules.length === 0 ? (
          <div style={{ padding: 16, color: "#888" }}>No rules yet.</div>
        ) : (
          rules.map((r) => (
            <div
              key={r.id}
              className="rule"
              style={{ cursor: "pointer" }}
              onClick={() => setEditing(r)}
            >
              <div className="info">
                <span className="domain">{r.domain}</span>
                <span className="label">
                  {r.label} · {r.header_name}
                </span>
              </div>
              <div className="right">
                <div
                  className={`toggle ${r.enabled ? "on" : ""}`}
                  onClick={async (e) => {
                    e.stopPropagation();
                    await api.toggleRule(r.id, !r.enabled);
                    load();
                  }}
                />
                <button
                  className="danger"
                  onClick={async (e) => {
                    e.stopPropagation();
                    if (confirm(`Delete rule for ${r.domain}?`)) {
                      await api.deleteRule(r.id);
                      load();
                    }
                  }}
                >
                  Delete
                </button>
              </div>
            </div>
          ))
        )}
      </div>
      {adding && (
        <AddRuleModal onClose={() => setAdding(false)} onSaved={load} />
      )}
      {editing && (
        <AddRuleModal
          initial={editing}
          onClose={() => setEditing(null)}
          onSaved={load}
        />
      )}
    </div>
  );
}

function GeneralTab() {
  const [port, setPort] = useState<number>(7777);
  const [trusted, setTrusted] = useState(false);
  const [busy, setBusy] = useState(false);

  const refresh = async () => {
    setPort(await api.getPort());
    setTrusted(await api.getCaTrusted());
  };
  useEffect(() => {
    refresh();
  }, []);

  return (
    <div>
      <h2>General</h2>
      <div className="field" style={{ maxWidth: 200 }}>
        <label>Proxy port</label>
        <input
          type="number"
          value={port}
          onChange={(e) => setPort(parseInt(e.target.value, 10) || 0)}
          onBlur={() => port > 0 && api.setPort(port)}
        />
      </div>

      <h3 style={{ fontSize: 13, marginTop: 20 }}>Certificate Authority</h3>
      <p style={{ color: "#666", fontSize: 12 }}>
        KeyProxy issues a local CA to terminate TLS for configured domains.{" "}
        {trusted ? (
          <span style={{ color: "#34c759" }}>● Trusted by system</span>
        ) : (
          <span style={{ color: "#ff3b30" }}>● Not trusted</span>
        )}
      </p>
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        <button
          className="primary"
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            try {
              await api.trustCa();
              await refresh();
            } catch (e) {
              alert(`Trust failed: ${e}`);
            } finally {
              setBusy(false);
            }
          }}
        >
          Trust CA Certificate
        </button>
        <button
          disabled={busy}
          onClick={async () => {
            const path = await saveDialog({
              defaultPath: "keyproxy-ca.pem",
              filters: [{ name: "PEM", extensions: ["pem"] }],
            });
            if (path) await api.exportCaCert(path);
          }}
        >
          Export CA Certificate
        </button>
        <button
          disabled={busy}
          onClick={async () => {
            if (!confirm("Regenerate CA? All existing trust will be invalidated.")) return;
            setBusy(true);
            try {
              await api.regenerateCa();
              await refresh();
            } finally {
              setBusy(false);
            }
          }}
        >
          Regenerate CA
        </button>
        <button
          className="danger"
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            try {
              await api.untrustCa();
              await refresh();
            } finally {
              setBusy(false);
            }
          }}
        >
          Remove Trust
        </button>
      </div>
    </div>
  );
}
