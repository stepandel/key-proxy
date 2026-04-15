import { Rule } from "../lib/tauri";

type Props = {
  rules: Rule[];
  onToggle: (id: string, enabled: boolean) => void;
};

export default function RuleList({ rules, onToggle }: Props) {
  if (rules.length === 0) {
    return (
      <div style={{ padding: 24, textAlign: "center", color: "#888" }}>
        No rules yet. Add one to start injecting credentials.
      </div>
    );
  }
  return (
    <div className="rule-list">
      {rules.map((r) => (
        <div key={r.id} className="rule">
          <div className="info">
            <span className="domain">{r.domain}</span>
            <span className="label">
              {r.label} · {r.header_name}
            </span>
          </div>
          <div className="right">
            <div
              className={`toggle ${r.enabled ? "on" : ""}`}
              onClick={() => onToggle(r.id, !r.enabled)}
              role="switch"
              aria-checked={r.enabled}
            />
          </div>
        </div>
      ))}
    </div>
  );
}
