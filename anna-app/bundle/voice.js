// Pet voice through Anna's host TTS (audio.speak, billed per character on the
// user's quota — so it is opt-in). Plays via Web Audio, which also gives the
// level for lip-sync, and needs no extra CSP (no <audio> src / media-src).
import { unwrap } from './core.js';

const MAX_CHARS = 400;

export class Voice {
  constructor(onLevel) {
    this.onLevel = onLevel || (() => {});
    this.ctx = null;
    this.current = null;
  }

  /** Call from a click handler: browsers only allow audio after a user gesture. */
  unlock() {
    if (!this.ctx) this.ctx = new AudioContext();
    if (this.ctx.state === 'suspended') this.ctx.resume();
  }

  stop() {
    if (this.current) {
      try {
        this.current.stop();
      } catch {
        /* already stopped */
      }
      this.current = null;
    }
    this.onLevel(0);
  }

  async speak(anna, text, language) {
    const clean = String(text || '').replace(/\s+/g, ' ').trim().slice(0, MAX_CHARS);
    if (!clean || !anna || !anna.audio || !this.ctx) return;
    this.stop();
    const res = unwrap(await anna.audio.speak({ text: clean, language, format: 'mp3', delivery: 'inline' }));
    let bytes;
    if (res && res.audioBase64) {
      bytes = Uint8Array.from(atob(res.audioBase64), (c) => c.charCodeAt(0)).buffer;
    } else if (res && res.url) {
      bytes = await (await fetch(res.url)).arrayBuffer();
    } else {
      throw new Error('no audio returned');
    }
    const buffer = await this.ctx.decodeAudioData(bytes);
    const src = this.ctx.createBufferSource();
    const analyser = this.ctx.createAnalyser();
    analyser.fftSize = 512;
    src.buffer = buffer;
    src.connect(analyser);
    analyser.connect(this.ctx.destination);
    this.current = src;
    const data = new Uint8Array(analyser.fftSize);
    const tick = () => {
      if (this.current !== src) return;
      analyser.getByteTimeDomainData(data);
      let sum = 0;
      for (const v of data) sum += ((v - 128) / 128) ** 2;
      this.onLevel(Math.min(1, Math.sqrt(sum / data.length) * 4));
      requestAnimationFrame(tick);
    };
    await new Promise((resolve) => {
      src.onended = resolve;
      src.start();
      tick();
    });
    if (this.current === src) this.stop();
  }

  /** Short two-note chime (focus timer), no network. */
  chime() {
    if (!this.ctx) return;
    const t = this.ctx.currentTime;
    [660, 880].forEach((f, i) => {
      const o = this.ctx.createOscillator();
      const g = this.ctx.createGain();
      o.frequency.value = f;
      g.gain.setValueAtTime(0.0001, t + i * 0.18);
      g.gain.exponentialRampToValueAtTime(0.2, t + i * 0.18 + 0.02);
      g.gain.exponentialRampToValueAtTime(0.0001, t + i * 0.18 + 0.35);
      o.connect(g).connect(this.ctx.destination);
      o.start(t + i * 0.18);
      o.stop(t + i * 0.18 + 0.4);
    });
  }
}
