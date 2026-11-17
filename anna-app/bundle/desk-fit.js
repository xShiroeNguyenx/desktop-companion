// How a desktop game page (desk/game.html, desk/guess.html) fits the Anna window.
// The pages are drawn on a fixed canvas and scaled with `html { zoom }`:
//   wide window  → the desktop's own landscape canvas, 900×560
//   narrow window (the chat panel, ~440×530) → the portrait canvas of desk-narrow.css
// so the game fills the panel as it is, and the window never has to be stretched.
// A classic script (the pages are not modules) that also loads under node --test.
(function (root) {
  const WIDE = { w: 900, h: 560 };
  const NARROW = { w: 450, h: 560 };

  /** { narrow, zoom } for a window of w×h CSS pixels. */
  function deskFit(w, h) {
    const narrow = w < h * 1.2;
    const canvas = narrow ? NARROW : WIDE;
    return { narrow, zoom: Math.max(0.3, Math.min(w / canvas.w, h / canvas.h)) };
  }

  root.deskFit = deskFit;
})(typeof window !== 'undefined' ? window : globalThis);
