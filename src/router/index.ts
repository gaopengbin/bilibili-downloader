import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router';

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    name: 'Home',
    component: () => import('@/views/BilibiliView.vue'),
    meta: {
      title: 'B站视频下载器',
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
