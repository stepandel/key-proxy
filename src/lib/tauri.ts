import { invoke } from "@tauri-apps/api/core";

export type Rule = {
  id: string;
  domain: string;
  enabled: boolean;
  header_name: string;
  label: string;
};

export type RuleInput = {
  domain: string;
  label: string;
  header_name: string;
  credential?: string | null;
};

export type ProxyStatus = {
  active: boolean;
  port: number;
  error: string | null;
};

export type LogEntry = {
  timestamp: string;
  domain: string;
  status: number | null;
  latency_ms: number;
  intercepted: boolean;
  error: string | null;
};

export type StatsSnapshot = {
  requests_today: number;
  errors_today: number;
  recent: LogEntry[];
};

export const api = {
  getRules: () => invoke<Rule[]>("get_rules"),
  addRule: (rule: RuleInput) => invoke<Rule>("add_rule", { rule }),
  updateRule: (id: string, rule: RuleInput) =>
    invoke<void>("update_rule", { id, rule }),
  deleteRule: (id: string) => invoke<void>("delete_rule", { id }),
  toggleRule: (id: string, enabled: boolean) =>
    invoke<void>("toggle_rule", { id, enabled }),
  getProxyStatus: () => invoke<ProxyStatus>("get_proxy_status"),
  setProxyActive: (active: boolean) =>
    invoke<ProxyStatus>("set_proxy_active", { active }),
  getStats: () => invoke<StatsSnapshot>("get_stats"),
  getCaTrusted: () => invoke<boolean>("get_ca_trusted"),
  trustCa: () => invoke<void>("trust_ca"),
  untrustCa: () => invoke<void>("untrust_ca"),
  exportCaCert: (path: string) => invoke<void>("export_ca_cert", { path }),
  regenerateCa: () => invoke<void>("regenerate_ca"),
  getPort: () => invoke<number>("get_port"),
  setPort: (port: number) => invoke<void>("set_port", { port }),
  openSettings: () => invoke<void>("open_settings"),
  quitApp: () => invoke<void>("quit_app"),
};
