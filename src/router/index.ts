import { createRouter, createWebHistory } from 'vue-router';

// 暂时使用懒加载占位，后续创建视图组件后启用
const routes = [
  {
    path: '/',
    name: 'Home',
    // component: () => import('@/views/Home.vue'),
    redirect: '/bilibili', // 暂时重定向到 B站
  },
  {
    path: '/bilibili',
    name: 'Bilibili',
    // component: () => import('@/views/BilibiliView.vue'),
    component: () => import('@/App.vue'), // 暂时使用 App.vue
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
