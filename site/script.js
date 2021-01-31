function urlBase64ToUint8Array(base64String) {
  const padding = "=".repeat((4 - (base64String.length % 4)) % 4)
  const base64 = (base64String + padding).replace(/\-/g, "+").replace(/_/g, "/")
  const rawData = window.atob(base64)
  return Uint8Array.from([...rawData].map((char) => char.charCodeAt(0)))
}

function requestNotification(diff) {
  // Should only call on user action
  window.Notification.requestPermission((status) =>
    console.log("Notification Permissiong status:", status)
  )

  if (window.Notification.permission === "denied") {
    console.error("Not permitted")
    return
  }
  if (!navigator.serviceWorker) {
    console.error("Service worker not supported")
    return
  }

  // Register and get the notification details and send them to our back end server.
  navigator.serviceWorker
    .register(`sw.js`)
    .then((registration) =>
      // Use the PushManager to get the user's subscription to the push service.
      registration.pushManager.getSubscription().then(async (subscription) => {
        // If a subscription was found, return it.
        if (subscription) {
          return subscription
        }

        const vapidPublicKey = document.querySelector(
          `meta[name="vapid-public-key"]`
        ).content
        const convertedVapidKey = urlBase64ToUint8Array(vapidPublicKey)
        return registration.pushManager.subscribe({
          userVisibleOnly: true,
          applicationServerKey: convertedVapidKey,
        })
      })
    )
    .then((subscription) => {
      const {
        endpoint,
        keys: { auth, p256dh },
      } = JSON.parse(JSON.stringify(subscription))
      console.log({ endpoint, auth, p256dh, diff })
      fetch("./register", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ endpoint, auth, p256dh, diff }),
      }).then((res) => {
        // TODO: Save to IndexedDB for display?
      })
    })
}

function unsubscribeNotifications() {
  navigator.serviceWorker.ready.then((reg) => {
    reg.pushManager.getSubscription().then((subscription) => {
      subscription
        .unsubscribe()
        .then((successful) => {
          console.log("unsubscribed")
        })
        .catch((e) => {
          console.log("unsubscribe failed")
        })
    })
  })
}

document.addEventListener("DOMContentLoaded", () => {
  console.log("testing")
  document.getElementById("diff-form").addEventListener("submit", (e) => {
    e.preventDefault()
    requestNotification(document.getElementById("diff-input").value)
  })
  document.getElementById("unsubscribe").addEventListener("click", () => {
    unsubscribeNotifications()
  })
})
