import { getAssetFromKV } from "@cloudflare/kv-asset-handler"

// Sites code pulled from
// https://github.com/cloudflare/worker-sites-template

const DEBUG = false

async function handleEvent(event) {
  const url = new URL(event.request.url)
  // Handle push route with WASM
  if (url.pathname.startsWith(`/push`)) {
    return await handlePushRequest(event.request)
  }
  let options = {}

  try {
    if (DEBUG) {
      // customize caching
      options.cacheControl = {
        bypassCache: true,
      }
    }

    const page = await getAssetFromKV(event, options)

    // allow headers to be altered
    const response = new Response(page.body, page)

    response.headers.set("X-XSS-Protection", "1; mode=block")
    response.headers.set("X-Content-Type-Options", "nosniff")
    response.headers.set("X-Frame-Options", "DENY")
    response.headers.set("Referrer-Policy", "unsafe-url")
    response.headers.set("Feature-Policy", "none")

    return response
  } catch (e) {
    // if an error is thrown try to serve the asset at 404.html
    if (!DEBUG) {
      try {
        let notFoundResponse = await getAssetFromKV(event, {
          mapRequestToAsset: (req) =>
            new Request(`${new URL(req.url).origin}/404.html`, req),
        })

        return new Response(notFoundResponse.body, {
          ...notFoundResponse,
          status: 404,
        })
      } catch (e) {}
    }

    return new Response(e.message || e.toString(), { status: 500 })
  }
}

// Modifications for webpack type based on
// https://github.com/cloudflare/rustwasm-worker-template/pull/7
async function handlePushRequest(request) {
  const { greet } = await import("./pkg")
  const greeting = await greet()
  return new Response(greeting, { status: 200 })
}

addEventListener("fetch", (event) => {
  try {
    event.respondWith(handleEvent(event))
  } catch (e) {
    if (DEBUG) {
      return event.respondWith(
        new Response(e.message || e.toString(), {
          status: 500,
        })
      )
    }
    event.respondWith(new Response("Internal Error", { status: 500 }))
  }
})
