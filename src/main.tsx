import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";

import App from "@/App";
import { initI18n } from "@/i18n";
import "@/index.css";

async function bootstrap(): Promise<void> {
  // Language must be resolved before the first render, otherwise the UI paints
  // in the wrong direction and visibly snaps round.
  await initI18n();

  const container = document.getElementById("root");
  if (!container) {
    throw new Error("Root element #root not found in index.html");
  }

  createRoot(container).render(
    <StrictMode>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </StrictMode>,
  );
}

void bootstrap();
