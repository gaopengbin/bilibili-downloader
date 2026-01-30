import { createApp } from "vue";
import { createPinia } from 'pinia';
import App from "./App.vue";
import 'element-plus/dist/index.css';
import './styles/index.css';
// import router from './router'; // 路由暂时禁用，等组件拆分完成后启用

const app = createApp(App);
const pinia = createPinia();

app.use(pinia);
// app.use(router); // 路由暂时禁用
app.mount("#app");
