# Page Diff Push Notifications

Cloudflare worker for getting web push notifications when an HTML page's content changes

## TODO:

- Implement page request
- Correct server response for generating push notification
- Cron trigger for KV polling
- Check request on cron trigger, send notification and delete key if changed

## Setup

```
wrangler preview --watch
```
