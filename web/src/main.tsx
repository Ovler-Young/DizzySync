import React from "react";
import { createRoot } from "react-dom/client";
import { I18nProvider } from "./i18n.tsx";
import { Providers } from "./Providers.tsx";
import "./styles.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("Root element #root was not found");
}

createRoot(root).render(
  <React.StrictMode>
    <I18nProvider>
      <Providers />
    </I18nProvider>
  </React.StrictMode>,
);
