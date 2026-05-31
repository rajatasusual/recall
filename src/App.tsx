import { EventTimeline } from "./components/EventTimeline";
import { TitleBar } from "./components/TItleBar";

import "./App.css";

export default function App() {
  return (
    <div class="app">
      <TitleBar />
      <main class="app-content">
        <EventTimeline refreshInterval={5000} />
      </main>
    </div>
  );
}
