import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router';

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    name: 'Home',
    component: () => import('@/views/Home.vue'),
    meta: {
      title: '首页',
    },
  },
  {
    path: '/bilibili',
    name: 'Bilibili',
    component: () => import('@/views/BilibiliView.vue'),
    meta: {
      title: '哔哩哔哩',
      platform: 'bilibili',
    },
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

// 路由切换时更新标题
router.beforeEach((to, _from, next) => {
  const title = to.meta.title as string;
  if (title) {
    document.title = `${title} - 视频下载器`;
  }
  next();
});

export default router;
