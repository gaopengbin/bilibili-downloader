import { createRouter, createWebHistory } from 'vue-router';

const routes = [
  {
    path: '/',
    name: 'Home',
    component: () => import('@/views/Home.vue'),
  },
  {
    path: '/bilibili',
    name: 'Bilibili',
    // 暂时使用 App.vue，后续迁移到 BilibiliView.vue
    component: () => import('@/App.vue'),
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
