import { getAssetFromKV } from "@cloudflare/kv-asset-handler"

const DEBUG = false

class AttributeRewriter {
  constructor(attributeName) {
    this.attributeName = attributeName
  }

  element(element) {
    const attribute = element.getAttribute(this.attributeName)
    if (attribute) {
      element.setAttribute(
        this.attributeName,
        attribute.replace("{{VAPID_PUBLIC_KEY}}", process.env.VAPID_PUBLIC_KEY)
      )
    }
  }
}

function rewriteEnvironmentVars(res) {
  const rewriter = new HTMLRewriter().on(
    "meta",
    new AttributeRewriter("content")
  )
  return rewriter.transform(res)
}

async function handleEvent(event) {
  const url = new URL(event.request.url)
  // Handle push route with WASM
  if (url.pathname.match(/\/register\/?/)) {
    return await handleRegisterRequest(event.request)
  } else if (url.pathname.match(/\/diff\/?/)) {
    return await handleDiffRequest(event.request)
  } else if (url.pathname.match(/\/push\/?/) && DEBUG) {
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
    const response = rewriteEnvironmentVars(new Response(page.body, page))

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

async function handleRegisterRequest(request) {
  try {
    const { register_subscription } = wasm_bindgen
    await wasm_bindgen(wasm)
    await register_subscription(await request.json())
    return new Response("registered", { status: 200 })
  } catch (e) {
    return new Response(e.stack || err)
  }
}

async function handleDiffRequest(request) {
  try {
    const { check_diffs_and_push } = wasm_bindgen
    await wasm_bindgen(wasm)
    await check_diffs_and_push()
    return new Response("", { status: 200 })
  } catch (e) {
    return new Response(e.stack || err)
  }
}

// Used for debugging, manually trigger a run
async function handlePushRequest(request) {
  try {
    const { check_diffs_and_push } = wasm_bindgen
    await wasm_bindgen(wasm)
    await check_diffs_and_push()
    return new Response("Success", { status: 200 })
  } catch (e) {
    return new Response(e.stack)
  }
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

addEventListener("scheduled", async (event) => {
  try {
    const { check_diffs_and_push } = wasm_bindgen
    await wasm_bindgen(wasm)
    event.waitUntil(check_diffs_and_push())
  } catch (e) {
    console.error(e)
  }
})
