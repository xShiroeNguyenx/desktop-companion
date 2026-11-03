// Right-click menu on the pet: the desktop app's radial ring
// (app/media/webview/radial-menu.js) ported to the Anna bundle. Entries fan out
// on a ring around the click; the hovered entry's name shows in the centre pill.
// A group replaces the ring with its own entries and turns the centre into a
// back button, so the menu stays one ring deep in a small window.
//
// Entries: { icon, label, action } or { icon, label, id, items: [...] }. An entry
// may carry `img` (shown instead of the icon, e.g. a model preview) and
// `active` (highlighted, e.g. the current model).

const ITEM_SIZE = 34;
const RADIUS = 92;
const MIN_RADIUS = 56;
const MARGIN = 8;

const clamp = (v, min, max) => (max < min ? (min + max) / 2 : Math.min(Math.max(v, min), max));

/**
 * @param {object} o
 * @param {() => Array} o.getItems fresh root entries, read on every open
 * @param {(action: string) => void} o.onAction
 * @param {() => {title: string, back: string, close: string}} o.labels read on every open (language can change)
 */
export function createRadialMenu({ getItems, onAction, labels }) {
  const root = document.createElement('div');
  root.className = 'radial';
  const ring = document.createElement('div');
  ring.className = 'radial-ring';
  const center = document.createElement('button');
  center.type = 'button';
  center.className = 'radial-center';
  root.append(ring, center);
  document.body.appendChild(root);

  let open = false;
  let text = { title: '', back: '', close: '' };
  /** Drill-down stack: root ring first, the open group (if any) last. */
  let levels = [];
  const current = () => levels[levels.length - 1];
  const centerText = () => (levels.length > 1 ? '↩ ' + current().title : current().title);

  /** Centre of the ring at the click, pulled in so the whole ring fits. */
  function place(x, y) {
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const radius = Math.max(MIN_RADIUS, Math.min(RADIUS, (Math.min(vw, vh) - 2 * MARGIN - ITEM_SIZE) / 2));
    const reach = radius + ITEM_SIZE / 2 + MARGIN;
    root.style.setProperty('--radial-x', clamp(x, reach, vw - reach) + 'px');
    root.style.setProperty('--radial-y', clamp(y, reach, vh - reach) + 'px');
    root.style.setProperty('--radial-radius', radius + 'px');
  }

  function render(level) {
    const isBack = levels.length > 1;
    center.classList.toggle('is-back', isBack);
    center.title = isBack ? text.back : text.close;
    center.textContent = centerText();
    // Collapse to the centre, then let the transition carry each entry out.
    ring.classList.remove('is-out');
    ring.textContent = '';
    level.items.forEach((item, i) => {
      const b = document.createElement('button');
      b.type = 'button';
      b.className = 'radial-item' + (item.active ? ' active' : '');
      b.style.setProperty('--angle', -90 + (360 / level.items.length) * i + 'deg');
      b.style.setProperty('--i', String(i));
      b.dataset.label = item.label;
      b.setAttribute('aria-label', item.label);
      if (item.items) b.dataset.group = item.id;
      else b.dataset.action = item.action;
      if (item.img) {
        const img = document.createElement('img');
        img.src = item.img;
        img.alt = '';
        b.appendChild(img);
      } else {
        b.textContent = item.icon;
      }
      ring.appendChild(b);
    });
    requestAnimationFrame(() => {
      if (open) ring.classList.add('is-out');
    });
  }

  function openAt(x, y) {
    const items = getItems();
    if (!Array.isArray(items) || !items.length) return;
    text = labels();
    levels = [{ title: text.title, items }];
    place(x, y);
    open = true;
    root.classList.add('show');
    render(levels[0]);
  }

  function close() {
    if (!open) return;
    open = false;
    levels = [];
    root.classList.remove('show');
    ring.classList.remove('is-out');
    ring.textContent = '';
  }

  ring.addEventListener('click', (e) => {
    const item = e.target.closest('.radial-item');
    if (!item) return;
    if (item.dataset.group) {
      const group = current().items.find((entry) => entry.id === item.dataset.group);
      if (!group || !group.items) return;
      levels.push({ title: group.label, items: group.items });
      render(current());
      return;
    }
    close();
    if (item.dataset.action) onAction(item.dataset.action);
  });
  ring.addEventListener('mouseover', (e) => {
    const item = e.target.closest('.radial-item');
    if (item) center.textContent = item.dataset.label || '';
  });
  ring.addEventListener('mouseout', (e) => {
    if (open && e.target.closest('.radial-item')) center.textContent = centerText();
  });
  center.addEventListener('click', () => {
    if (levels.length > 1) {
      levels.pop();
      render(current());
    } else {
      close();
    }
  });
  // Click outside the ring (or Esc) closes it.
  window.addEventListener('pointerdown', (e) => {
    if (open && !root.contains(e.target)) close();
  }, true);
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') close();
  });
  window.addEventListener('blur', close);

  return {
    openAt,
    close,
    isOpen: () => open,
    /** Right-clicks (or a long press on touch) inside `area` open the ring there. */
    attach(area) {
      area.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        openAt(e.clientX, e.clientY);
      });
    },
  };
}
