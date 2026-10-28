import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

// Disable right-click context menu globally
document.addEventListener('contextmenu', (e) => {
  // Allow context menu only for input/textarea elements
  const target = e.target as HTMLElement;
  if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
    return;
  }
  e.preventDefault();
}, true);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
