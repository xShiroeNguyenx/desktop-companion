import { state, debugLog } from './core.js';

// Phát audio TTS do Rust gửi sang (pet:tts:audio) và mấp máy miệng theo âm
// lượng thật của giọng nói (ý tưởng từ Open-LLM-VTuber: đo RMS rồi đặt
// ParamMouthOpenY, không cần phân tích âm vị).
//
// Rust tổng hợp từng câu tuần tự nên audio tới đúng thứ tự; ở đây chỉ cần một
// hàng đợi. `gen` tăng mỗi lượt đọc mới — lượt mới tới thì bỏ ngay lượt cũ.

let ctx = null;
let analyser = null;
let samples = null;
let currentGen = -1;
let queue = [];
// null = rảnh; true = đang giải mã; AudioBufferSourceNode = đang phát.
let playing = null;
let mouth = 0;
let hookedModel = null;

// Độ nhạy miệng: RMS của giọng nói thường ~0.03–0.2.
const MOUTH_GAIN = 7;
const MOUTH_SMOOTH = 0.45;

function ensureContext() {
  if (!ctx) {
    ctx = new AudioContext();
    analyser = ctx.createAnalyser();
    analyser.fftSize = 1024;
    samples = new Float32Array(analyser.fftSize);
    analyser.connect(ctx.destination);
  }
  if (ctx.state === 'suspended') {
    ctx.resume().catch((e) => debugLog('TTS: AudioContext resume bị chặn: ' + e));
  }
  return ctx;
}

// Chính sách autoplay của WebView2 chỉ cho AudioContext chạy sau một tương tác
// của người dùng — mở khoá ở lần bấm đầu tiên vào pet.
window.addEventListener('pointerdown', () => {
  try { ensureContext(); } catch (_) { /* ignore */ }
}, { capture: true });

function base64ToBuffer(b64) {
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes.buffer;
}

export function handleTtsAudio(data) {
  if (window.__AUDIO_MUTED__ || !data || !data.data) return;
  if (data.gen < currentGen) return;
  if (data.gen > currentGen) {
    stopTts();
    currentGen = data.gen;
  }
  queue.push(data.data);
  if (!playing) void playNext();
}

export function stopTts(gen) {
  if (typeof gen === 'number' && gen > currentGen) currentGen = gen;
  queue = [];
  if (playing && typeof playing.stop === 'function') {
    try {
      playing.onended = null;
      playing.stop();
    } catch (_) { /* đã dừng */ }
  }
  playing = null;
}

async function playNext() {
  const b64 = queue.shift();
  if (!b64) {
    playing = null;
    return;
  }
  const gen = currentGen;
  playing = true;
  try {
    const c = ensureContext();
    const buffer = await c.decodeAudioData(base64ToBuffer(b64));
    // Bị dừng / có lượt mới trong lúc giải mã → lượt mới tự lo phát.
    if (gen !== currentGen || playing !== true) return;
    const src = c.createBufferSource();
    src.buffer = buffer;
    src.connect(analyser);
    src.onended = () => {
      if (playing === src) {
        playing = null;
        void playNext();
      }
    };
    playing = src;
    hookMouth();
    src.start();
  } catch (e) {
    debugLog('TTS: không phát được audio: ' + e);
    if (gen === currentGen) {
      playing = null;
      void playNext();
    }
  }
}

function currentLoudness() {
  if (!analyser || !playing || typeof playing.stop !== 'function') return 0;
  analyser.getFloatTimeDomainData(samples);
  let sum = 0;
  for (let i = 0; i < samples.length; i++) sum += samples[i] * samples[i];
  return Math.min(1, Math.sqrt(sum / samples.length) * MOUTH_GAIN);
}

// Đặt miệng trong 'beforeModelUpdate' — chạy sau motion/expression của chính
// model nên không bị motion idle hay preset biểu cảm ghi đè (xem rotation.js).
function hookMouth() {
  const internal = state.model && state.model.internalModel;
  if (!internal || hookedModel === internal) return;
  hookedModel = internal;
  internal.on('beforeModelUpdate', () => {
    const target = currentLoudness();
    mouth += (target - mouth) * MOUTH_SMOOTH;
    if (target === 0 && mouth < 0.01) {
      mouth = 0;
      return; // im lặng: để biểu cảm tự quyết khẩu hình
    }
    try {
      internal.coreModel.setParameterValueById('ParamMouthOpenY', mouth);
    } catch (_) { /* model không có tham số này */ }
  });
}
