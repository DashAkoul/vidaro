import React from "react";
import ReactDOM from "react-dom/client";

import App from "./App";
import "./styles/app.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("root element is missing from index.html");
}

// The webview is an application shell, not a browser: suppress the default
// context menu (Back / Reload / Save as / Inspect). Ctrl+V still pastes into
// the URL field.
window.addEventListener("contextmenu", (event) => {
  event.preventDefault();
});

ReactDOM.createRoot(container).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
