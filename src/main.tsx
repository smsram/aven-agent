import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/tokens.css";
import "./styles/index.css";
import { initNativeDesktopBehavior } from "./services/nativeDesktop";

import { ErrorBoundary } from "./components/common/ErrorBoundary";

// Initialize native desktop optimizations
initNativeDesktopBehavior();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary sectionName="Application">
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
);
