# Page Diff Push Notifications

Cloudflare worker for subscribing to a browser push notification when an HTML page's content changes

## Setup

Generate keys for VAPID authentication with `npm run keys`.

Preview the worker locally with:

```
wrangler preview --watch
```

## TODO:

- Basic UI
- Eventually IndexedDB for showing existing subscriptions? Allow for deleting?
