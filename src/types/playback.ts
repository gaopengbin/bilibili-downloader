export interface PlaybackInfo {
  session_id: string;
  source: string;
  title: string;
  cid: number;
  page: number;
  duration: number;
  quality: number;
  qualities: { id: number; label: string }[];
  parts: { page: number; title: string; duration: number }[];
}

export interface VideoDanmaku {
  text: string;
  time: number;
  mode: 0 | 1 | 2;
  color: string;
}
