import React from "react";
import ReactDOM from "react-dom/client";
import App from "@/app/App";
import { CollectionsProvider } from "@/components/layout/CollectionsProvider";
import "@/styles/globals.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <CollectionsProvider><App /></CollectionsProvider>
  </React.StrictMode>,
);
