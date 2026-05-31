import { EventTimeline } from "./components/EventTimeline";
import { TitleBar } from "./components/TItleBar";

import "./App.css";

export default function App() {
  return (
    <div class="app">
      <main class="app-content">
        <TitleBar />
        <EventTimeline refreshInterval={5000} />
      </main>
    </div>
  );
}
