// https://serviceworke.rs/push-rich_service-worker_doc.html

// Register event listener for the 'push' event.
self.addEventListener("push", function (event) {
  const diffUrl = decodeURIComponent(new URL(location).searchParams.get("diff"))
  // Keep the service worker alive until the notification is created.
  event.waitUntil(
    // https://notifications.spec.whatwg.org/
    // https://developer.mozilla.org/en-US/docs/Web/API/notification
    self.registration.showNotification("URL Has Changed", {
      body: `${diffUrl} changed`,
    })
  )
})
