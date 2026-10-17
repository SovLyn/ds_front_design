# Deploying the atlas

The output of `trunk build --release` is a static bundle. There is no server,
database, API key or runtime font/image request.

```bash
trunk build --release
# dist/
# ├── index.html
# ├── deep-space-atlas-<hash>.js
# └── deep-space-atlas-<hash>_bg.wasm
```

## Hosting requirements

1. **HTTPS** (or localhost). WebGPU is unavailable in insecure contexts.
2. Serve `.wasm` as `application/wasm`.
3. Do not rewrite `index.html` for unknown paths; the app has no client-side
   router yet. A single `index.html` is enough.
4. gzip or brotli the wasm and JS. The wasm is the large asset; the fonts are
   inside it, so there is no separate font request.

No special COOP/COEP headers are required. No SharedArrayBuffer, threads or
service worker are used.

## GitHub Pages

```bash
trunk build --release --public-url /<repo-name>/
# copy dist/ to the gh-pages branch (or use actions/deploy-pages)
```

Trunk’s `--public-url` rewrites the asset URLs. If you deploy to a user/org page
(`https://<user>.github.io/`), omit the path.

GitHub Pages serves `.wasm` correctly on current infrastructure. If you use a
custom domain, enable HTTPS.

## Netlify

`netlify.toml`:

```toml
[build]
  command = "trunk build --release"
  publish = "dist"

[[headers]]
  for = "/*.wasm"
  [headers.values]
    Content-Type = "application/wasm"
    Cache-Control = "public, max-age=31536000, immutable"

[[headers]]
  for = "/*.js"
  [headers.values]
    Cache-Control = "public, max-age=31536000, immutable"
```

Trunk hashes asset names, so immutable caching is safe for the hashed files.
Keep `index.html` short-lived (`no-cache`) if you deploy frequently.

## Cloudflare Pages

- Build command: `trunk build --release`
- Output directory: `dist`
- Install Rust and Trunk in the build image, or use a prebuilt Trunk binary.
- Cloudflare serves wasm with the correct MIME type; verify with
  `curl -I https://your.site/..._bg.wasm`.

## Nginx

```nginx
types {
    application/wasm wasm;
}

location / {
    root /var/www/atlas;
    try_files $uri $uri/ /index.html;
}

location ~* \.(js|wasm)$ {
    root /var/www/atlas;
    add_header Cache-Control "public, max-age=31536000, immutable";
}
```

If you host under a subpath, build with `--public-url /subpath/` so the wasm and
JS URLs are prefixed.

## Subpath hosting

```bash
trunk build --release --public-url /atlas/
```

The canvas and fallback are relative to the document; the hashed JS/wasm use the
public URL. Do not hand-edit `dist/index.html`.

## Cache strategy

| Asset | Cache |
| --- | --- |
| `index.html` | `no-cache` or a short max-age |
| hashed `.js` / `.wasm` | `immutable`, one year |
| screenshots in `docs/` | not part of `dist`; documentation only |

The app does not use a service worker, so a new deploy is visible on the next
navigation once `index.html` is refreshed.

## Verifying a deployment

1. Open the site in Chrome/Edge 113+ or Safari 26+ over HTTPS.
2. Confirm the boot overlay fades and the cover star field appears.
3. Open DevTools → Console. There should be no WGSL validation errors and no
   panics.
4. Check `navigator.gpu` in the console; it should be an object.
5. If you see the fallback, check the secure context, the browser version and
   the `.wasm` MIME type.
6. Test one full page turn with the wheel and with the keyboard.
7. Test a narrow viewport (390 × 844) and a 2× device pixel ratio.

## Troubleshooting

**The page stays on the boot overlay.** The wasm failed to load or eframe failed
to start. Check the network panel, the `.wasm` MIME type and the console. The
boot overlay adds an error message after 18 s; the Rust guard adds `no-webgpu`
and shows the fallback if the capability check fails.

**The fallback appears on a browser that supports WebGPU.** The page is probably
not a secure context, or `navigator.gpu` is disabled by a flag/policy. Try
`http://localhost` or HTTPS.

**The scene is black but the UI is visible.** The WebGPU adapter/device failed
after startup. Check the console for wgpu errors. On Linux, the browser may need
a Vulkan driver or an experimental WebGPU flag.

**The layout is too small / zoomed on a phone.** The canvas backing store is
re-asserted by `index.html`; if you embed the canvas in a custom page, preserve
the `#atlas-canvas` sizing rules and the DPR fit script.

**The wasm is large.** The bundle contains eframe, egui, wgpu and the subsetted
fonts. Enable Trunk’s `wasm-opt` integration and install Binaryen if size is
critical. Do not remove the font subsets unless you also provide a reliable
CJK fallback.
