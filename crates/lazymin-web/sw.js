const CACHE = "lazymin-pwa-v1";
const ASSETS = [
  "/manifest.webmanifest",
  "/favicon.ico",
  "/icons/icon-192-dark.png",
  "/icons/icon-512-dark.png",
  "/icons/icon-192-light.png",
  "/icons/icon-512-light.png",
  "/icons/icon-dark-mode.svg",
  "/icons/icon-light-mode.svg",
  "/sw.js",
];

function isPrecachedPath(pathname) {
  return ASSETS.includes(pathname);
}

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(CACHE).then((cache) => cache.addAll(ASSETS)),
  );
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys().then((keys) => {
      return Promise.all(
        keys.filter((k) => k !== CACHE).map((k) => caches.delete(k)),
      );
    }),
  );
  self.clients.claim();
});

self.addEventListener("fetch", (event) => {
  if (event.request.method !== "GET") {
    return;
  }

  const req = event.request;
  const url = new URL(req.url);

  if (url.origin !== self.location.origin) {
    return;
  }

  if (req.mode === "navigate") {
    event.respondWith(fetch(req));
    return;
  }

  const accept = req.headers.get("accept") || "";
  if (accept.includes("text/html")) {
    event.respondWith(fetch(req));
    return;
  }

  if (isPrecachedPath(url.pathname)) {
    event.respondWith(
      caches.match(req).then((hit) => hit || fetch(req)),
    );
    return;
  }

  event.respondWith(fetch(req));
});
