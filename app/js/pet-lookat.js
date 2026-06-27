// Phase 2B — look-at-cursor for the standalone pet.
//
// The pet window is small, so the webview only receives `mousemove` while the
// cursor is physically over it. To make the model follow the cursor anywhere on
// screen (and even while click-through is on), the Rust hook streams the global
// cursor position via the `cursor-pos` event (bridge.js → window.__onCursorPos__).
//
// We convert screen coords → window client coords and re-dispatch a synthetic
// `mousemove`, which interaction.js's existing follow listener picks up and
// feeds to rotation.js's applyFollowFocus(). No changes to the reused runtime.
(function () {
  // How far (in px) beyond the pet window edge the cursor can be and still be
  // tracked. Outside this margin the model recenters (looks forward) instead of
  // straining to follow a cursor on the far side of the screen.
  const RANGE_MARGIN = 220;

  let following = false;

  // Map a global screen point to this window's client coordinate space.
  // window.screenX/Y are CSS px of the window's top-left; the Rust hook reports
  // physical px, so divide by devicePixelRatio to match the CSS coordinate
  // system the webview's getBoundingClientRect() uses.
  window.__onCursorPos__ = function (pos) {
    if (!window.__FOCUS_FOLLOW__) return;
    const dpr = window.devicePixelRatio || 1;
    const clientX = pos.x / dpr - window.screenX;
    const clientY = pos.y / dpr - window.screenY;

    const w = window.innerWidth;
    const h = window.innerHeight;
    // Inside the window, or within RANGE_MARGIN of any edge → track.
    const inRange =
      clientX >= -RANGE_MARGIN && clientX <= w + RANGE_MARGIN &&
      clientY >= -RANGE_MARGIN && clientY <= h + RANGE_MARGIN;

    if (inRange) {
      following = true;
      window.dispatchEvent(new MouseEvent('mousemove', { clientX, clientY, bubbles: false }));
    } else if (following) {
      // Just left the range → recenter once so the head returns to neutral.
      following = false;
      document.documentElement.dispatchEvent(new MouseEvent('mouseleave', { bubbles: false }));
    }
  };
})();
