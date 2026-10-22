import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/tokens.css";
import "./styles/index.css";
import { initNativeDesktopBehavior } from "./services/nativeDesktop";

// Initialize native desktop optimizations
initNativeDesktopBehavior();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
