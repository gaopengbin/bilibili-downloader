import sharp from 'sharp';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const iconsDir = path.join(__dirname, '..', 'src-tauri', 'icons');

// B站小电视 SVG（粉色背景 + 白色图标）
const createSvg = (size) => `
<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 512 512">
  <defs>
    <linearGradient id="bg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#fb7299"/>
      <stop offset="100%" style="stop-color:#f85b82"/>
    </linearGradient>
  </defs>
  <!-- 圆角背景 -->
  <rect width="512" height="512" rx="100" ry="100" fill="url(#bg)"/>
  <!-- 小电视图标 (缩放并居中) -->
  <g transform="translate(96, 96) scale(13.33)">
    <path fill="#fff" d="M17.813 4.653h.854c1.51.054 2.769.578 3.773 1.574 1.004.995 1.524 2.249 1.56 3.76v7.36c-.036 1.51-.556 2.769-1.56 3.773s-2.262 1.524-3.773 1.56H5.333c-1.51-.036-2.769-.556-3.773-1.56S.036 18.858 0 17.347v-7.36c.036-1.511.556-2.765 1.56-3.76 1.004-.996 2.262-1.52 3.773-1.574h.774l-1.174-1.12a1.234 1.234 0 0 1-.373-.906c0-.356.124-.659.373-.907l.027-.027c.267-.249.573-.373.92-.373.347 0 .653.124.92.373L9.653 4.44c.071.071.134.142.187.213h4.267a.836.836 0 0 1 .16-.213l2.853-2.747c.267-.249.573-.373.92-.373.347 0 .662.151.929.4.267.249.391.551.391.907 0 .355-.124.657-.373.906l-1.174 1.12zM5.333 7.24c-.746.018-1.373.276-1.88.773-.506.498-.769 1.13-.786 1.894v7.52c.017.764.28 1.395.786 1.893.507.498 1.134.756 1.88.773h13.334c.746-.017 1.373-.275 1.88-.773.506-.498.769-1.129.786-1.893v-7.52c-.017-.765-.28-1.396-.786-1.894-.507-.497-1.134-.755-1.88-.773H5.333zM8 11.107c.373 0 .684.124.933.373.25.249.383.569.4.96v1.173c-.017.391-.15.711-.4.96-.249.25-.56.374-.933.374s-.684-.125-.933-.374c-.25-.249-.383-.569-.4-.96V12.44c0-.373.129-.689.386-.947.258-.257.574-.386.947-.386zm8 0c.373 0 .684.124.933.373.25.249.383.569.4.96v1.173c-.017.391-.15.711-.4.96-.249.25-.56.374-.933.374s-.684-.125-.933-.374c-.25-.249-.383-.569-.4-.96V12.44c.017-.391.15-.711.4-.96.249-.249.56-.373.933-.373z"/>
  </g>
</svg>
`;

async function generateIcons() {
  // 确保目录存在
  if (!fs.existsSync(iconsDir)) {
    fs.mkdirSync(iconsDir, { recursive: true });
  }

  const sizes = [32, 128, 256];
  
  // 生成 PNG 图标
  for (const size of sizes) {
    const svg = createSvg(512);
    const pngBuffer = await sharp(Buffer.from(svg))
      .resize(size, size)
      .png()
      .toBuffer();
    
    const filename = size === 256 ? '128x128@2x.png' : `${size}x${size}.png`;
    fs.writeFileSync(path.join(iconsDir, filename), pngBuffer);
    console.log(`Generated ${filename}`);
  }

  // 生成 ICO 文件 (Windows)
  // ICO 需要多个尺寸: 16, 32, 48, 256
  const icoSizes = [16, 32, 48, 256];
  const icoBuffers = [];
  
  for (const size of icoSizes) {
    const svg = createSvg(512);
    const pngBuffer = await sharp(Buffer.from(svg))
      .resize(size, size)
      .png()
      .toBuffer();
    icoBuffers.push({ size, buffer: pngBuffer });
  }
  
  // 创建 ICO 文件
  const icoBuffer = createIco(icoBuffers);
  fs.writeFileSync(path.join(iconsDir, 'icon.ico'), icoBuffer);
  console.log('Generated icon.ico');

  // 生成 ICNS (macOS) - 简化版，只用最大的 PNG
  const svg512 = createSvg(512);
  const png512 = await sharp(Buffer.from(svg512))
    .resize(512, 512)
    .png()
    .toBuffer();
  
  // 对于 macOS，我们生成一个简单的 PNG 文件重命名为 icns
  // 实际上 Tauri 会处理这个
  fs.writeFileSync(path.join(iconsDir, 'icon.icns'), png512);
  console.log('Generated icon.icns (as PNG, Tauri will handle conversion)');

  console.log('All icons generated successfully!');
}

// 创建 ICO 文件格式
function createIco(images) {
  // ICO 文件头
  const headerSize = 6;
  const dirEntrySize = 16;
  
  // 计算数据偏移
  let dataOffset = headerSize + (dirEntrySize * images.length);
  
  // 文件头
  const header = Buffer.alloc(headerSize);
  header.writeUInt16LE(0, 0);  // Reserved
  header.writeUInt16LE(1, 2);  // Type (1 = ICO)
  header.writeUInt16LE(images.length, 4);  // Number of images
  
  // 目录项
  const dirEntries = [];
  const imageDataBuffers = [];
  
  for (const img of images) {
    const entry = Buffer.alloc(dirEntrySize);
    entry.writeUInt8(img.size >= 256 ? 0 : img.size, 0);  // Width
    entry.writeUInt8(img.size >= 256 ? 0 : img.size, 1);  // Height
    entry.writeUInt8(0, 2);  // Color palette
    entry.writeUInt8(0, 3);  // Reserved
    entry.writeUInt16LE(1, 4);  // Color planes
    entry.writeUInt16LE(32, 6);  // Bits per pixel
    entry.writeUInt32LE(img.buffer.length, 8);  // Size of image data
    entry.writeUInt32LE(dataOffset, 12);  // Offset to image data
    
    dirEntries.push(entry);
    imageDataBuffers.push(img.buffer);
    dataOffset += img.buffer.length;
  }
  
  return Buffer.concat([header, ...dirEntries, ...imageDataBuffers]);
}

generateIcons().catch(console.error);
