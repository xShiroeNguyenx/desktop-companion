// The desktop game pages (bundle/desk/) are pages of this app too: connect
// them to Anna like index.html does, so the host sees a live window, and let
// the character say her bubble lines out loud with the app's voice.
import { spokenText, voiceDenied } from './core.js';
import { connectHost, load } from './host.js';
import { Voice } from './voice.js';

const voice = new Voice();
let anna = null;
let on = false;
/** A line that came before the first click (the browser allows no sound yet). */
let waiting = '';

const lang = new URLSearchParams(location.search).get('glang') || 'en';
const soundOn = () => {
  // The game's own "Sound" switch (settings panel) silences her here too.
  const box = document.getElementById('optSound');
  return !box || box.checked;
};

function speak(text) {
  const line = spokenText(text);
  if (!on || !anna || !line || !soundOn()) return;
  if (!voice.ctx) {
    waiting = line;
    return;
  }
  voice.speak(anna, line, lang).catch((e) => {
    console.warn('[companion] game voice', e);
    // Voice not granted: stop asking, and give the game its own sounds back.
    if (voiceDenied(e)) setOn(false);
  });
}

function setOn(value) {
  on = value;
  // desk-shim.js mutes the games' recorded clips while she reads the lines herself.
  window.DC_SPEAKS = value;
  if (!value) voice.stop();
}

for (const type of ['pointerdown', 'keydown']) {
  window.addEventListener(
    type,
    () => {
      voice.unlock();
      if (waiting) {
        const line = waiting;
        waiting = '';
        speak(line);
      }
    },
    true,
  );
}

// desk-shim.js reads the runtime's latest token from here for the way back.
connectHost().then(async (host) => {
  window.DC_HOST = host;
  anna = host;
  if (!host) return;
  // The same switch as the panel and the floating pet (billed on the user's Anna quota).
  const settings = await load('settings', {});
  setOn(!!settings.voice);
  if (!on) return;
  const bubble = document.getElementById('bubble');
  if (!bubble) return;
  let timer;
  const read = () => {
    clearTimeout(timer);
    // The games write a line, then fill in names: read it once it has settled.
    timer = setTimeout(() => speak(bubble.textContent), 120);
  };
  new MutationObserver(read).observe(bubble, { childList: true, characterData: true, subtree: true });
  read();
});
