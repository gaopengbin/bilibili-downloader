export * from './interface';

// B站平台
export { BilibiliPlatform, bilibiliPlatform } from './bilibili';
export * as bilibiliApi from './bilibili/api';

// 注册所有平台
import { registerPlatform } from './interface';
import { bilibiliPlatform } from './bilibili';

registerPlatform(bilibiliPlatform);
