import { getCurrentWindow } from "@tauri-apps/api/window";
import { EventTimeline } from "./components/EventTimeline";
import { QuickOverlay } from "./components/QuickOverlay";
import { TitleBar } from "./components/TItleBar";

import "./App.css";

export default function App() {
  if (getCurrentWindow().label === "quick_overlay") {
    return <QuickOverlay />;
  }

  return (
    <div class="app">
      <main class="app-content">
        <TitleBar />
        <EventTimeline refreshInterval={5000} />
      </main>
    </div>
  );
}
