import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ActivityOverlayWindow } from "./components/overlay/ActivityOverlayWindow";
import "./styles/tokens.css";
import "./styles/index.css";
import { initNativeDesktopBehavior } from "./services/nativeDesktop";
import { ErrorBoundary } from "./components/common/ErrorBoundary";

// Initialize native desktop optimizations
initNativeDesktopBehavior();

const isOverlay = typeof window !== "undefined" && window.location.hash === "#overlay";

if (isOverlay) {
  document.documentElement.classList.add("overlay-mode");
  document.body.classList.add("overlay-mode");
  document.documentElement.style.background = "transparent";
  document.documentElement.style.backgroundColor = "transparent";
  document.body.style.background = "transparent";
  document.body.style.backgroundColor = "transparent";
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary sectionName={isOverlay ? "ActivityOverlay" : "Application"}>
      {isOverlay ? <ActivityOverlayWindow /> : <App />}
    </ErrorBoundary>
  </React.StrictMode>,
);
