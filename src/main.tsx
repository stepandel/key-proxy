import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Settings from "./components/Settings";
import "./styles.css";

const isSettings = window.location.hash.startsWith("#/settings");

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>{isSettings ? <Settings /> : <App />}</React.StrictMode>
);
