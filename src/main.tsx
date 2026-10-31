import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import { installMock } from "./dev/mock";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/components.css";
import "./styles/shell.css";

if (import.meta.env.VITE_MOCK) installMock();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
