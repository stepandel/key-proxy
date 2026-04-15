import { useState } from "react";
import { api, RuleInput } from "../lib/tauri";

type Props = {
  onClose: () => void;
  onSaved: () => void;
  initial?: { id: string; domain: string; label: string; header_name: string };
};

export default function AddRuleModal({ onClose, onSaved, initial }: Props) {
  const [domain, setDomain] = useState(initial?.domain ?? "");
  const [label, setLabel] = useState(initial?.label ?? "");
  const [headerName, setHeaderName] = useState(initial?.header_name ?? "Authorization");
  const [credential, setCredential] = useState("");
  const [saving, setSaving] = useState(false);

  async function save() {
    setSaving(true);
    try {
      const input: RuleInput = {
        domain,
        label,
        header_name: headerName,
        credential: credential.trim() ? credential : null,
      };
      if (initial) {
        await api.updateRule(initial.id, input);
      } else {
        await api.addRule(input);
      }
      onSaved();
      onClose();
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>{initial ? "Edit rule" : "Add rule"}</h2>
        <div className="field">
          <label>Domain</label>
          <input
            type="text"
            placeholder="api.anthropic.com"
            value={domain}
            onChange={(e) => setDomain(e.target.value)}
          />
        </div>
        <div className="field">
          <label>Label</label>
          <input
            type="text"
            placeholder="Anthropic"
            value={label}
            onChange={(e) => setLabel(e.target.value)}
          />
        </div>
        <div className="field">
          <label>Header name</label>
          <input
            type="text"
            value={headerName}
            onChange={(e) => setHeaderName(e.target.value)}
          />
        </div>
        <div className="field">
          <label>
            Credential value{" "}
            {initial && <span style={{ color: "#888" }}>(leave blank to keep)</span>}
          </label>
          <input
            type="password"
            placeholder={initial ? "••••••••" : "Bearer sk-..."}
            value={credential}
            onChange={(e) => setCredential(e.target.value)}
          />
        </div>
        <div className="modal-actions">
          <button onClick={onClose}>Cancel</button>
          <button
            className="primary"
            disabled={saving || !domain || !headerName}
            onClick={save}
          >
            {saving ? "Saving..." : "Save"}
          </button>
        </div>
      </div>
    </div>
  );
}
