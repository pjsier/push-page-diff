# Page Diff Push Notifications

[![Build status](https://github.com/pjsier/push-page-diff/workflows/CICD/badge.svg)](https://github.com/pjsier/push-page-diff/actions?query=workflow%3ACICD)

[Cloudflare Worker](https://workers.cloudflare.com/) for subscribing to a browser push notification when an HTML page's content changes. You can see the live version at [push-page-diff.pjsier.workers.dev](https://push-page-diff.pjsier.workers.dev/).

Includes workarounds for customizing a webpack build in a Cloudflare Worker using Rust. Also includes a WASM-compatible setup for web push authentication and encryption.

## Setup

You'll need Rust, node.js, and wrangler installed. You'll need to replace the account-specific values in `wrangler.toml` with your own. Install dependencies with:

```
npm install
```

Then you'll need to generate keys for VAPID authentication by running:

```
npm run keys
```

Then copy `.env.sample` to `.env` and fill in the output values for `VAPID_PUBLIC_KEY` and `VAPID_PRIVATE_KEY`. `VAPID_SUBJECT` should be a `mailto:` link to an email of your choice.

Run the worker locally at [localhost:8787](http://localhost:8787) with:

```
wrangler dev
```
