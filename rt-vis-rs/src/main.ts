import { createApp } from "vue";
import App from "./App.vue";

// Disable default browser context menu for a native desktop app feel
document.addEventListener("contextmenu", (e) => {
  e.preventDefault();
});

createApp(App).mount("#app");
