import { createApp } from "vue";
import { createPinia } from "pinia";
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "@fontsource/jetbrains-mono/400.css";
import "./style.css";
import App from "./App.vue";
createApp(App).use(createPinia()).mount("#app");
