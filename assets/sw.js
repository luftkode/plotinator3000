var cacheName = "plotinator3000-pwa";
var filesToCache = [
    "./",
    "./index.html",
    "./plotinator3000.js",
    "./plotinator3000_bg.wasm",
];

/* Start the service worker and cache all of the app's content */
self.addEventListener("install", function (e) {
    e.waitUntil(
        caches.open(cacheName).then(function (cache) {
            return cache.addAll(filesToCache);
        }),
    );
});

/* Always try to fetch first, fall back to cache if fetch fails */
self.addEventListener("fetch", function (e) {
    e.respondWith(
        fetch(e.request)
            .then(function (response) {
                // If fetch succeeds, clone and cache the response for future use
                if (response && response.status === 200) {
                    var responseToCache = response.clone();
                    caches.open(cacheName).then(function (cache) {
                        cache.put(e.request, responseToCache);
                    });
                }
                return response;
            })
            .catch(function () {
                // If fetch fails, try to get from cache
                return caches.match(e.request);
            }),
    );
});
