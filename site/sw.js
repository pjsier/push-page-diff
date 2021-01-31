// https://serviceworke.rs/push-rich_service-worker_doc.htm

// Register event listener for the 'push' event.
self.addEventListener("push", function (event) {
  const url = event && event.data ? event.data.text() : "Diff URL"

  // Keep the service worker alive until the notification is created.
  event.waitUntil(
    self.registration.showNotification("URL Has Changed", {
      body: url,
    })
  )
})
