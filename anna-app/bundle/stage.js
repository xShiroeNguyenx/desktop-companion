// Live2D stage: model loading and framing, look-at-cursor, head pats / pokes,
// emotion reactions, lip-sync from the voice level, and floating particles.

// emotion tag → [expression name in model3.json, motion group]
const EMOTION_FX = {
  happy: ['Smile', 'TapBody'],
  sad: ['Sad', 'Sad'],
  angry: ['Angry', 'Shake'],
  shy: ['Shy', 'Shy'],
  surprised: ['Surprised', 'Surprised'],
  wink: ['Wink', 'Tilt'],
  thinking: [null, 'Think'],
};

export class Stage {
  /** framing: 'bust' (head to thigh, room for a bubble on top) | 'full' (whole body, feet at the bottom). */
  constructor(el, canvas, { onPat, onPoke, framing = 'bust' } = {}) {
    this.el = el;
    this.canvas = canvas;
    this.framing = framing;
    this.onPat = onPat || (() => {});
    this.onPoke = onPoke || (() => {});
    this.pixi = null;
    this.model = null;
    this.mouth = 0;
    this.rub = null;
    this.bindPointer();
  }

  async load(url) {
    if (!this.pixi) {
      this.pixi = new PIXI.Application({
        view: this.canvas,
        backgroundAlpha: 0,
        antialias: true,
        autoStart: true,
        resolution: window.devicePixelRatio || 1,
        autoDensity: true,
      });
      new ResizeObserver(() => this.fit()).observe(this.el);
    }
    const model = await PIXI.live2d.Live2DModel.from(url, { autoInteract: false });
    if (this.model) {
      this.pixi.stage.removeChild(this.model);
      this.model.destroy();
    }
    this.model = model;
    this.pixi.stage.addChild(model);
    // Lip-sync: write the mouth after motions update, so idle motions don't close it.
    model.internalModel.on('beforeModelUpdate', () => {
      if (this.mouth > 0.01) model.internalModel.coreModel.setParameterValueById('ParamMouthOpenY', this.mouth);
    });
    this.fit();
  }

  fit() {
    const { pixi, model } = this;
    if (!pixi || !model) return;
    const w = this.el.clientWidth;
    const h = this.el.clientHeight;
    pixi.renderer.resize(w, h);
    model.scale.set(1);
    if (this.framing === 'full') {
      // Whole body standing on the bottom edge; the bubble floats over the head.
      const scale = Math.min((h * 0.8) / model.height, (w * 0.98) / model.width);
      model.scale.set(scale);
      model.x = (w - model.width) / 2;
      model.y = h - model.height - 4;
      return;
    }
    // Head-to-thigh framing; the top ~22% is left for the speech bubble.
    const scale = Math.min((h * 1.45) / model.height, (w * 0.95) / model.width);
    model.scale.set(scale);
    model.x = (w - model.width) / 2;
    model.y = h * 0.22;
  }

  hasGroup(group) {
    const defs = (this.model && this.model.internalModel && this.model.internalModel.motionManager.definitions) || {};
    return !!defs[group];
  }

  motion(group) {
    if (!this.model) return;
    // Not every model has every group (snow-lolita has no Shy / Think): fall back to a head tilt.
    const g = this.hasGroup(group) ? group : this.hasGroup('Tilt') ? 'Tilt' : null;
    if (g) this.model.motion(g);
  }

  react(emotion) {
    if (!this.model || !emotion) return;
    const [expression, motion] = EMOTION_FX[emotion] || [];
    try {
      if (expression) this.express(expression);
      if (motion) this.motion(motion);
    } catch (e) {
      console.warn('[companion] react', e);
    }
  }

  /**
   * Show an expression for `ms`, then go back to the neutral face. Live2D keeps an
   * expression until it is replaced, so without the reset she would smile forever
   * (the desktop app also gives every expression a duration).
   */
  express(name, ms = 5000) {
    if (!this.model) return;
    this.model.expression(name);
    clearTimeout(this.faceTimer);
    this.faceTimer = setTimeout(() => {
      const em = this.model && this.model.internalModel.motionManager.expressionManager;
      if (em) em.resetExpression();
    }, ms);
  }

  setMouth(level) {
    this.mouth = Math.max(0, Math.min(1, level));
  }

  /** 'head' | 'body' | null for a point in stage coordinates. */
  part(x, y) {
    const m = this.model;
    if (!m) return null;
    const b = m.getBounds();
    if (x < b.x || x > b.x + b.width || y < b.y || y > b.y + b.height) return null;
    return y < b.y + b.height * 0.2 ? 'head' : 'body';
  }

  bindPointer() {
    const local = (e) => {
      const r = this.el.getBoundingClientRect();
      return { x: e.clientX - r.left, y: e.clientY - r.top };
    };
    this.el.addEventListener('pointermove', (e) => {
      const p = local(e);
      if (this.model) this.model.focus(p.x, p.y);
      // Rubbing back and forth over the head counts as a pat.
      if (this.rub && this.part(p.x, p.y) === 'head') {
        this.rub.dist += Math.abs(p.x - this.rub.x);
        this.rub.x = p.x;
        if (this.rub.dist > 160) {
          this.rub = null;
          this.pat(p);
        }
      }
    });
    this.el.addEventListener('pointerleave', () => {
      if (this.model) this.model.focus(this.el.clientWidth / 2, this.el.clientHeight / 2);
      this.rub = null;
    });
    this.el.addEventListener('pointerdown', (e) => {
      if (e.target !== this.canvas) return;
      const p = local(e);
      const part = this.part(p.x, p.y);
      if (part === 'head') this.rub = { x: p.x, dist: 0, at: p };
      else if (part === 'body') this.poke(p);
    });
    this.el.addEventListener('pointerup', (e) => {
      if (this.rub) {
        const at = this.rub.at;
        this.rub = null;
        this.pat(at);
      }
    });
  }

  pat(p) {
    if (this.model) {
      this.express('Smile');
      this.motion('TapHead');
    }
    this.burst(['💗', '💕', '✨'], p, 6);
    this.onPat();
  }

  poke(p) {
    if (this.model) {
      this.express('Surprised', 3000);
      this.motion('Shake');
    }
    this.burst(['💢', '❗'], p, 3);
    this.onPoke();
  }

  /** Floating emoji particles at a stage point (centre top when omitted). */
  burst(glyphs, p, count = 5) {
    const at = p || { x: this.el.clientWidth / 2, y: this.el.clientHeight * 0.35 };
    for (let i = 0; i < count; i++) {
      const s = document.createElement('span');
      s.className = 'particle';
      s.textContent = glyphs[i % glyphs.length];
      s.style.left = `${at.x + (Math.random() - 0.5) * 60}px`;
      s.style.top = `${at.y + (Math.random() - 0.5) * 20}px`;
      s.style.animationDelay = `${i * 70}ms`;
      this.el.appendChild(s);
      setTimeout(() => s.remove(), 1600 + i * 70);
    }
  }
}
