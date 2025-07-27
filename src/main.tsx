import React from "react";
import ReactDOM from "react-dom/client";
import WindowRouter from "./WindowRouter";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <WindowRouter />
  </React.StrictMode>
);