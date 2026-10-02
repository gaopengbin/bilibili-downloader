import type { VideoDanmaku } from '@/types/playback';

export function parseDanmaku(xml: string): VideoDanmaku[] {
  const document = new DOMParser().parseFromString(xml, 'application/xml');
  if (document.querySelector('parsererror') || document.documentElement.tagName !== 'i') {
    throw new Error('弹幕数据格式不正确');
  }
  const comments: VideoDanmaku[] = [];
  for (const node of document.querySelectorAll('d')) {
    const fields = (node.getAttribute('p') || '').split(',');
    const time = Number(fields[0]);
    const mode = Number(fields[1]);
    const color = Number(fields[3]);
    const text = node.textContent?.trim();
    if (!text || !Number.isFinite(time) || time < 0 || ![1, 2, 3, 4, 5].includes(mode)) continue;
    comments.push({
      text,
      time,
      mode: mode === 5 ? 1 : mode === 4 ? 2 : 0,
      color: Number.isInteger(color) && color >= 0 && color <= 0xffffff
        ? `#${color.toString(16).padStart(6, '0')}` : '#ffffff',
    });
  }
  return comments.sort((a, b) => a.time - b.time);
}
