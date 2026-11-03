// The desktop game pages (bundle/desk/) are pages of this app too: connect
// them to Anna like index.html does, so the host sees a live window.
import { connectHost } from './host.js';

// desk-shim.js reads the runtime's latest token from here for the way back.
connectHost().then((anna) => {
  window.DC_HOST = anna;
});
